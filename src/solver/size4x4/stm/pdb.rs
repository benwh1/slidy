//! Pattern-database files, preloaded into RAM or mapped.
//!
//! A table set's tables live in one file (made from the separate table
//! files by `pdbcombine`), each at a fixed offset given by the set's
//! [`Layout`], and the solver takes one region holding that file. The
//! only job here is knowing the layout and reading or mapping the file up
//! front (load time is reported separately and excluded from solve
//! benchmarks), plus the entry formats the tables share.

use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read as _, Seek as _, SeekFrom},
    marker::PhantomData,
    path::Path,
};

use memmap2::{Mmap, MmapMut, MmapOptions};

const PAGE: u64 = 4096;

const fn page_up(n: u64) -> u64 {
    n.div_ceil(PAGE) * PAGE
}

/// Where a table set's tables sit in its combined file: in order, each on
/// its own page, zero-padded in between.
///
/// The lookups address every table from one base pointer plus a constant
/// displacement, which x86 limits to +-2 GiB, so the base points halfway
/// to the last table's start and the [`Layout::offset`]s are signed
/// (a set checks its own with a `const` assertion).
pub struct Layout {
    /// The combined file's name inside the table directory.
    pub file: &'static str,
    /// The separate table files and their sizes in bytes.
    pub parts: &'static [(&'static str, u64)],
}

impl Layout {
    /// Where table `part` starts in the combined file.
    #[must_use]
    pub const fn start(&self, part: usize) -> u64 {
        let mut start = 0;
        let mut i = 0;
        while i < part {
            start += page_up(self.parts[i].1);
            i += 1;
        }
        start
    }

    /// Size of the combined file.
    #[must_use]
    pub const fn size(&self) -> u64 {
        let last = self.parts.len() - 1;
        self.start(last) + self.parts[last].1
    }

    /// How far into the region the base pointer of [`Dbs`] lies.
    const fn bias(&self) -> u64 {
        page_up(self.start(self.parts.len() - 1) / 2)
    }

    /// Table `part`'s displacement from the base pointer.
    #[must_use]
    pub const fn offset(&self, part: usize) -> isize {
        self.start(part) as isize - self.bias() as isize
    }
}

/// Hint a table entry into the cache ahead of its read. The tables are
/// far larger than any cache, so every lookup misses without this.
#[inline(always)]
pub(crate) fn prefetch(entry: *const u8) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: a prefetch never faults, whatever the pointer.
    unsafe {
        std::arch::x86_64::_mm_prefetch(entry.cast::<i8>(), std::arch::x86_64::_MM_HINT_T0);
    }
    // Non-x86 builds skip the hint; the loads still warm the cache.
    #[cfg(not(target_arch = "x86_64"))]
    let _ = entry;
}

/// Write `dir/<layout.file>` from the separate tables in `dir`. The file is
/// written under a temporary name and renamed when complete, so a failed
/// run never leaves a truncated combined file behind.
pub fn combine(dir: &Path, layout: &Layout) -> Result<(), Box<dyn Error>> {
    let partial = dir.join(format!("{}.partial", layout.file));
    let mut out = File::create(&partial)?;
    for (part, &(name, size)) in layout.parts.iter().enumerate() {
        let mut table = File::open(dir.join(name))?;
        if table.metadata()?.len() != size {
            Err(format!("{name} has the wrong size"))?;
        }
        // Seeking past the end leaves a hole, which reads as zeros.
        out.seek(SeekFrom::Start(layout.start(part)))?;
        if io::copy(&mut table, &mut out)? != size {
            Err(format!("{name} changed while being copied"))?;
        }
    }
    out.sync_all()?;
    fs::rename(&partial, dir.join(layout.file))?;
    Ok(())
}

/// A set's combined file in memory: read into RAM, or mapped.
pub struct Region {
    bytes: Bytes,
    layout: &'static Layout,
}

enum Bytes {
    Preloaded(MmapMut),
    Mapped(Mmap),
}

impl Region {
    /// A region for `layout` whose tables `fill(part, bytes)` writes (the bytes start zeroed).
    pub fn build(
        layout: &'static Layout,
        mut fill: impl FnMut(usize, &mut [u8]),
    ) -> Result<Self, Box<dyn Error>> {
        let mut bytes = MmapOptions::new().len(layout.size() as usize).map_anon()?;
        #[cfg(target_os = "linux")]
        {
            bytes.advise(memmap2::Advice::HugePage)?;
            let _ = bytes.advise(memmap2::Advice::PopulateWrite);
        }
        for (part, &(_, size)) in layout.parts.iter().enumerate() {
            let start = layout.start(part) as usize;
            fill(part, &mut bytes[start..start + size as usize]);
        }
        Ok(Self {
            bytes: Bytes::Preloaded(bytes),
            layout,
        })
    }

    /// The combined file in `dir`, checked against the layout's size.
    fn open(dir: &Path, layout: &Layout) -> Result<File, Box<dyn Error>> {
        let file = File::open(dir.join(layout.file))?;
        if file.metadata()?.len() != layout.size() {
            Err("pattern database has the wrong size")?;
        }
        Ok(file)
    }

    /// Read the combined file into an anonymous mapping advised for
    /// transparent huge pages. With THP defrag set to `madvise` (the
    /// common default) only advised regions get direct compaction, so on
    /// a machine whose memory is fragmented by page cache a plain
    /// allocation ends up partly on 4 KB pages, and every table lookup
    /// there costs a page walk.
    ///
    /// Without a combined file, the separate tables are read into place
    /// instead (a large set then needs no second copy on disk).
    pub fn preload(dir: &Path, layout: &'static Layout) -> Result<Self, Box<dyn Error>> {
        let mut bytes = MmapOptions::new().len(layout.size() as usize).map_anon()?;
        #[cfg(target_os = "linux")]
        {
            bytes.advise(memmap2::Advice::HugePage)?;
            // Fault the region in now, before the reads fill the page
            // cache: allocated later, a large set's region ended up a
            // quarter on 4 KB pages (17 GB set: 12.7 GB huge), and every
            // lookup there costs a page walk. (Best effort: older kernels
            // lack the advice.)
            let _ = bytes.advise(memmap2::Advice::PopulateWrite);
        }
        if dir.join(layout.file).exists() {
            Self::open(dir, layout)?.read_exact(&mut bytes)?;
        } else {
            for (part, &(name, size)) in layout.parts.iter().enumerate() {
                let mut table = File::open(dir.join(name))?;
                if table.metadata()?.len() != size {
                    Err(format!("{name} has the wrong size"))?;
                }
                let start = layout.start(part) as usize;
                table.read_exact(&mut bytes[start..start + size as usize])?;
            }
        }
        Ok(Self {
            bytes: Bytes::Preloaded(bytes),
            layout,
        })
    }

    /// Map the combined file read-only, shared between processes: every
    /// process mapping it shares the page cache's frames, so N solvers
    /// cost ~7.7 GB total, not N. `MAP_POPULATE` pays all page faults
    /// inside the mapping call, so the first solve runs with no warmup
    /// slowdown.
    pub fn map(dir: &Path, layout: &'static Layout) -> Result<Self, Box<dyn Error>> {
        let file = Self::open(dir, layout)?;
        // SAFETY: the mapping is read-only and the size was checked, so
        // every page is backed by the file; this assumes the file is not
        // truncated or rewritten while mapped.
        let map = unsafe {
            MmapOptions::new()
                .len(layout.size() as usize)
                .populate()
                .map(&file)?
        };
        Ok(Self {
            bytes: Bytes::Mapped(map),
            layout,
        })
    }

    #[must_use]
    pub fn as_dbs(&self) -> Dbs<'_> {
        let start = match &self.bytes {
            Bytes::Preloaded(bytes) => bytes.as_ptr(),
            Bytes::Mapped(map) => map.as_ptr(),
        };
        Dbs {
            // SAFETY: the bias lies inside the file's bytes.
            base: unsafe { start.add(self.layout.bias() as usize) },
            _region: PhantomData,
        }
    }
}

/// Shareable view of a set's tables: one base pointer, the tables at its
/// [`Layout::offset`]s.
#[derive(Clone, Copy)]
pub struct Dbs<'a> {
    base: *const u8,
    _region: PhantomData<&'a [u8]>,
}

// SAFETY: a `Dbs` only reads the region it borrows, which nothing writes
// while it is borrowed, so threads can share and send it like the
// `&'a [u8]` it stands for.
unsafe impl Send for Dbs<'_> {}
// SAFETY: as for `Send`.
unsafe impl Sync for Dbs<'_> {}

impl Dbs<'_> {
    /// A view whose base is `base`, for tests of the index arithmetic
    /// (nothing may be read through it).
    #[cfg(test)]
    pub(crate) fn for_test(base: *const u8) -> Self {
        Dbs {
            base,
            _region: PhantomData,
        }
    }

    /// Address of a table entry, for prefetching: `offset` is a table's
    /// [`Layout::offset`] and `index` the pre-scaled index.
    #[inline(always)]
    pub(crate) fn entry(self, offset: isize, index: u32) -> *const u8 {
        // SAFETY: every index used in the search is a valid Lehmer code by
        // construction, so the pointer stays inside the region.
        unsafe { self.base.offset(offset).add(index as usize) }
    }

    /// A little-endian u32 at a pre-scaled index: a placement's bit word
    /// ([`Self::entry`]).
    #[inline(always)]
    pub(crate) fn word(self, offset: isize, index: u32) -> u32 {
        // SAFETY: as `entry`; the word lies inside the table.
        unsafe { self.entry(offset, index).cast::<u32>().read_unaligned() }
    }

    /// A byte at a pre-scaled index ([`Self::entry`]).
    #[inline(always)]
    pub(crate) fn byte(self, offset: isize, index: u32) -> u32 {
        // SAFETY: as `entry`.
        u32::from(unsafe { *self.entry(offset, index) })
    }

    /// A little-endian u16 at a pre-scaled index ([`Self::entry`]).
    #[inline(always)]
    pub(crate) fn half(self, offset: isize, index: u32) -> u32 {
        // SAFETY: as `entry`; the half-word lies inside the table.
        u32::from(unsafe { self.entry(offset, index).cast::<u16>().read_unaligned() })
    }

    /// Address of placement `index`'s u16 in a pair table at `offset`
    /// (`bitpack pair8`: a group's additive bits in the low byte, its full
    /// bits in the high). The table may exceed 4 GiB, hence the `usize`
    /// arithmetic.
    #[inline(always)]
    pub(crate) fn pair_entry(self, offset: isize, index: u32) -> *const u8 {
        // SAFETY: as `entry`; the u16 lies inside the table.
        unsafe { self.base.offset(offset).add(2 * index as usize) }
    }

    /// Placement `index`'s u16 in a pair table ([`Self::pair_entry`]).
    #[inline(always)]
    pub(crate) fn pair(self, offset: isize, index: u32) -> u32 {
        // SAFETY: as `pair_entry`.
        u32::from(unsafe {
            self.pair_entry(offset, index)
                .cast::<u16>()
                .read_unaligned()
        })
    }

    /// Address of byte `byte` of placement `index`'s block in a mixed
    /// table at `offset`: 5-byte blocks of four placements, a byte of their
    /// four 2-bit codes ([`Self::mix_code`]) and then a byte each
    /// ([`Self::mix_byte`]). The table may exceed 4 GiB, hence the `usize`
    /// arithmetic.
    #[inline(always)]
    pub(crate) fn mix_entry(self, offset: isize, index: u32, byte: usize) -> *const u8 {
        // SAFETY: as `entry`; the block lies inside the table.
        unsafe {
            self.base
                .offset(offset)
                .add(5 * (index as usize >> 2) + byte)
        }
    }

    /// Placement `index`'s 2-bit code in a mixed table, `(value >> 1) & 3`
    /// of a value with known parity: the value mod 8 ([`decode_code`]).
    #[inline(always)]
    pub(crate) fn mix_code(self, offset: isize, index: u32) -> u32 {
        // SAFETY: the entry lies inside the table.
        (u32::from(unsafe { *self.mix_entry(offset, index, 0) }) >> ((index & 3) * 2)) & 3
    }

    /// Placement `index`'s own byte in a mixed table.
    #[inline(always)]
    pub(crate) fn mix_byte(self, offset: isize, index: u32) -> u32 {
        // SAFETY: the entry lies inside the table.
        u32::from(unsafe { *self.mix_entry(offset, index, 1 + (index & 3) as usize) })
    }
}

/// The value with 2-bit `code` one move from one of value `parent`, in a
/// table whose moves change the value by 1 or 3, always flipping the
/// parity: four candidates of one parity, whose codes count up mod 4.
/// `b` is `parent - 3`, `mid` is `(parent + 5) / 2`.
#[inline(always)]
pub(crate) const fn decode_code(b: u32, mid: u32, code: u32) -> u32 {
    b.wrapping_add(2 * (code.wrapping_sub(mid) & 3))
}
