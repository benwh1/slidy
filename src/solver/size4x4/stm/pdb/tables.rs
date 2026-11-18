//! The lookups shared by every table set ([`Tables`]): any number of partitions of tiles 1-15
//! into groups, each group's table in one of a few [`Format`]s, looked up in both orientations
//! (direct and mirrored).
//!
//! A set is described by a [`Spec`]: its [`Group`]s, each naming its tiles, the partition it
//! belongs to and the format of its table. The heuristic is the maximum, over both orientations,
//! of every partition's additive sum and of the full (blank-counting) value of every group whose
//! table has one and asks for it. Coded tables (2-bit minimum codes, exact bits) are decoded
//! against the parent's values, which the search carries; the root's come from walking downhill
//! in each coded table (`walk`).

use std::{error::Error, hint::select_unpredictable, marker::PhantomData, path::Path};

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, build,
        group::{Format, Group, MAX_PARTS, Spec},
        region::{Dbs, Region, decode_code, prefetch},
    },
    utils::{Board, Move},
};

/// Per-tile Lehmer weights for every partition, scaled to byte offsets in the moved tile's
/// group's table, with partition `p` in 32-bit lane `p % 2` of word `p / 2`, so a scanned cell
/// costs one or two adds.
pub struct Shift {
    /// `contrib[piece][tile]`: how a `tile` the moved `piece` passes shifts its groups' indices.
    contrib: [[[u64; 2]; 16]; 16],
    /// The moved piece's own weight, which every move adds.
    wbits: [[u64; 2]; 16],
}

impl Shift {
    pub(super) const fn new(groups: &[Group]) -> Self {
        let mut contrib = [[[0u64; 2]; 16]; 16];
        let mut wbits = [[0u64; 2]; 16];
        let mut g = 0;
        while g < groups.len() {
            let group = groups[g];
            let p = group.partition as usize;
            let (word, lane) = (p / 2, 32 * (p % 2));
            let scale = group.format.scale() as u64;
            let mut piece = 1;
            while piece < 16 {
                if group.has(piece) {
                    let w = group.weight(piece) as u64 * scale;
                    let mut other = 0;
                    while other < 16 {
                        let c = if !group.has(other) {
                            w
                        } else if other > piece {
                            group.weight(other) as u64 * scale + w
                        } else {
                            0
                        };
                        assert!(c < 1 << 32);
                        contrib[piece as usize][other as usize][word] |= c << lane;
                        other += 1;
                    }
                    wbits[piece as usize][word] |= w << lane;
                }
                piece += 1;
            }
            g += 1;
        }
        Self { contrib, wbits }
    }
}

/// Unrolls `$body` for `$g` in `0..MAX_GROUPS`, skipping the groups a set does not have (a
/// compile-time test).
macro_rules! each_group {
    ($n_:ident, $g:ident, $body:block) => {{
        each_group!(@one $n_, $g, 0, $body);
        each_group!(@one $n_, $g, 1, $body);
        each_group!(@one $n_, $g, 2, $body);
        each_group!(@one $n_, $g, 3, $body);
        each_group!(@one $n_, $g, 4, $body);
        each_group!(@one $n_, $g, 5, $body);
    }};
    (@one $n_:ident, $g:ident, $n:literal, $body:block) => {
        if $n < $n_ {
            #[allow(clippy::allow_attributes, unused_variables, reason = "unrolled")]
            let $g: usize = $n;
            $body
        }
    };
}

/// Unrolls `$body` for `$p` in `0..MAX_PARTS`, skipping the partitions a set does not have.
macro_rules! each_part {
    ($s:ty, $p:ident, $body:block) => {{
        each_part!(@one $s, $p, 0, $body);
        each_part!(@one $s, $p, 1, $body);
        each_part!(@one $s, $p, 2, $body);
        each_part!(@one $s, $p, 3, $body);
    }};
    (@one $s:ty, $p:ident, $n:literal, $body:block) => {
        if $n < <$s>::PARTS {
            let $p: usize = $n;
            $body
        }
    };
}

/// The tables of the set that spec `S` describes (`N` is its number of groups), and their
/// lookups.
pub struct Tables<S, const N: usize> {
    region: Region,
    /// The region's [`Dbs`] base, which a lookup adds its table's offset to.
    base: *const u8,
    spec: PhantomData<S>,
}

// SAFETY: the tables are never written after construction, so they can be shared and sent like
// the `Region` they point into.
unsafe impl<S, const N: usize> Send for Tables<S, N> {}
// SAFETY: as for `Send`.
unsafe impl<S, const N: usize> Sync for Tables<S, N> {}

impl<S: Spec, const N: usize> Tables<S, N> {
    /// Tables held in `region`, which must have `S`'s layout.
    #[must_use]
    pub fn from_region(region: Region) -> Self {
        const { assert!(N == S::GROUPS.len(), "N is the number of groups") };
        assert!(
            region.layout().size() == S::LAYOUT.size(),
            "the region has the set's layout"
        );
        let base = region.as_dbs().base();
        Self {
            region,
            base,
            spec: PhantomData,
        }
    }

    /// Builds the tables in memory on `threads` threads.
    #[must_use]
    pub fn build(threads: usize) -> Self {
        let region = Region::build(&S::LAYOUT, |part, out| {
            let group = S::GROUPS[part];
            build::encode(group, &build::values(group, threads), out, threads);
        })
        .expect("allocating the tables");
        Self::from_region(region)
    }

    /// Reads the tables from `dir`: the set's combined file (see [`Self::save`]), or else its
    /// separate table files. Only the files' sizes are checked: other tables of the right sizes
    /// give wrong solutions (or a panic), though never undefined behaviour.
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self::from_region(Region::preload(dir, &S::LAYOUT)?))
    }

    /// Maps the set's combined file in `dir` read-only, so that processes using the same file
    /// share its memory. See [`Self::load`].
    pub fn map(dir: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self::from_region(Region::map(dir, &S::LAYOUT)?))
    }

    /// Saves the tables to `dir` as the set's combined file.
    pub fn save(&self, dir: &Path) -> Result<(), Box<dyn Error>> {
        self.region.save(dir)
    }

    /// The tables' size in bytes.
    #[must_use]
    pub const fn size() -> u64 {
        S::LAYOUT.size()
    }

    #[inline(always)]
    fn dbs(&self) -> Dbs<'_> {
        // SAFETY: the base comes from this region, which lives as long as `self`.
        unsafe { Dbs::from_base(self.base) }
    }

    /// One orientation's indices after move `m`, which moves `tile` into the blank.
    #[inline(always)]
    fn shift_side(
        base: [u32; N],
        rank: [u8; N],
        board: &Board,
        m: &Move,
        side: usize,
        tile: u8,
    ) -> ([u32; N], [u8; N]) {
        let (lo, gap, sign) = (m.lo[side] as usize, m.gap[side] as usize, m.sign[side] != 0);
        let two = S::PARTS > 2;
        // SAFETY: tile is a tile value.
        let row = unsafe { S::SHIFT.contrib.get_unchecked(tile as usize) };
        // SAFETY: as above.
        let own = unsafe { S::SHIFT.wbits.get_unchecked(tile as usize) };
        let mut acc0 = own[0];
        let mut acc1 = if two { own[1] } else { 0 };
        // Each `Pair8` group's tiles among the cells passed.
        let mut inside = [0u8; N];
        macro_rules! cell {
            ($k:literal) => {{
                // SAFETY: a cell strictly between the two blank positions.
                let other = unsafe { *board.get_unchecked(lo + $k) };
                // SAFETY: a tile value.
                let packed = unsafe { row.get_unchecked(other as usize) };
                acc0 = acc0.wrapping_add(packed[0]);
                if two {
                    acc1 = acc1.wrapping_add(packed[1]);
                }
                each_group!(N, g, {
                    if matches!(S::GROUPS[g].format, Format::Pair8) {
                        inside[g] += ((S::GROUPS[g].tiles >> other) & 1) as u8;
                    }
                });
            }};
        }
        match gap {
            1 => {}
            3 => {
                cell!(1);
                cell!(2);
            }
            5 => {
                cell!(1);
                cell!(2);
                cell!(3);
                cell!(4);
            }
            _ => {
                cell!(1);
                cell!(2);
                cell!(3);
                cell!(4);
                cell!(5);
                cell!(6);
            }
        }
        let mask = 0u64.wrapping_sub(u64::from(sign));
        let neg = |x: u64| (x ^ mask).wrapping_sub(mask) as u32;
        let deltas = [
            neg(acc0 & 0xFFFF_FFFF),
            neg(acc0 >> 32),
            neg(acc1 & 0xFFFF_FFFF),
            neg(acc1 >> 32),
        ];
        let mut out = base;
        let mut rank = rank;
        each_group!(N, g, {
            let group = S::GROUPS[g];
            let d = deltas[group.partition as usize];
            out[g] = out[g].wrapping_add(select_unpredictable(group.has(tile), d, 0));
            if matches!(group.format, Format::Pair8) {
                // The cells that change sides of the blank: the passed ones and the moved tile's.
                let passed = (gap as u8 - 1) - inside[g] + u8::from(!group.has(tile));
                let r = rank[g];
                rank[g] = select_unpredictable(sign, r + passed, r.wrapping_sub(passed));
            }
        });
        (out, rank)
    }

    /// Group `g`'s values (additive, full) for a child in one orientation with index `idx`, the
    /// blank at `blank` and the moved tile `tile`, decoded against the parent's `h`.
    #[inline(always)]
    fn decode(
        dbs: Dbs<'_>,
        g: usize,
        (idx, rank): (u32, u8),
        blank: u32,
        tile: u8,
        h: Half,
    ) -> (u32, u32) {
        let group = S::GROUPS[g];
        let off = S::OFFSETS[g];
        let moved = group.has(tile);
        match group.format {
            Format::Min2 => {
                let code = (dbs.byte(off, idx >> 2) >> ((idx & 3) * 2)) & 3;
                let child = decode_code(h.m, h.f, code);
                (select_unpredictable(moved, child, h.a), 0)
            }
            Format::Cells => {
                let y = (dbs.half(off, idx) >> blank) ^ h.m;
                let child = h.a + 1 - ((y << 1) & 2);
                (select_unpredictable(moved, child, h.a), 0)
            }
            Format::Pair => {
                let y = (dbs.word(off, idx) >> (2 * blank)) ^ h.m;
                let child = h.a + 1 - ((y << 1) & 2);
                (
                    select_unpredictable(moved, child, h.a),
                    h.f.wrapping_sub(y & 2),
                )
            }
            Format::Pair8 => {
                let y = (dbs.pair(off, idx) >> rank) ^ h.m;
                let child = h.a + 1 - ((y << 1) & 2);
                (
                    select_unpredictable(moved, child, h.a),
                    h.f.wrapping_sub((y >> 7) & 2),
                )
            }
            Format::Bytes16 => {
                let h = dbs.half(off, idx + 2 * blank);
                (h & 0xFF, h >> 8)
            }
            Format::Bytes8 => (dbs.byte(off, idx + blank), 0),
        }
    }

    /// The address of group `g`'s entry for index `idx` with the blank at `blank`.
    #[inline(always)]
    fn address(dbs: Dbs<'_>, g: usize, idx: u32, blank: u32) -> *const u8 {
        let off = S::OFFSETS[g];
        match S::GROUPS[g].format {
            Format::Min2 => dbs.entry(off, idx >> 2),
            Format::Cells | Format::Pair => dbs.entry(off, idx),
            Format::Pair8 => dbs.pair_entry(off, idx),
            Format::Bytes16 => dbs.entry(off, idx + 2 * blank),
            Format::Bytes8 => dbs.entry(off, idx + blank),
        }
    }
}

/// Both orientations' scaled indices, one per group.
#[derive(Clone, Copy)]
pub struct Indices<const N: usize> {
    idx: [[u32; N]; 2],
    /// Each `Pair8` group's blank rank: how many cells below the blank's hold none of the
    /// group's tiles (unused for other formats).
    rank: [[u8; N]; 2],
}

/// A node's values in both orientations: each group's additive and full value.
#[derive(Clone, Copy)]
pub struct Values<const N: usize> {
    a: [[u8; N]; 2],
    f: [[u8; N]; 2],
}

/// `((value + 1) >> 1) & 1`: the entry bit of `value + 1`. A move changes an exact value by
/// exactly 1 and an entry holds `(v >> 1) & 1`, so a child's bit matches this exactly when its
/// value is `value + 1`.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

/// One group's values at a node, set up for decoding its children's entries.
#[derive(Clone, Copy)]
pub struct Half {
    /// The additive value (kept when none of the group's tiles moves).
    a: u32,
    /// `Min2`: the middle code `(a + 5) / 2`; full formats: the full value + 1.
    f: u32,
    /// `Min2`: the lowest candidate `a - 3`; bit formats: the entry bits of `value + 1` where
    /// the child's bits sit (a child's bit that differs went down).
    m: u32,
}

impl Half {
    #[inline(always)]
    fn new(format: Format, a: u8, f: u8) -> Self {
        let (a, f) = (u32::from(a), u32::from(f));
        match format {
            Format::Min2 => Self {
                a,
                f: u32::midpoint(a, 5),
                m: a.wrapping_sub(3),
            },
            Format::Cells => Self { a, f: 0, m: up(a) },
            Format::Pair => Self {
                a,
                f: f + 1,
                m: up(a) | (up(f) << 1),
            },
            Format::Pair8 => Self {
                a,
                f: f + 1,
                m: up(a) | (up(f) << 8),
            },
            Format::Bytes16 | Format::Bytes8 => Self { a, f, m: 0 },
        }
    }
}

/// A node's values in both orientations set up for decoding.
#[derive(Clone, Copy)]
pub struct Decoder<const N: usize> {
    h: [[Half; N]; 2],
}

impl<S: Spec, const N: usize> Pdb for Tables<S, N> {
    type Indices = Indices<N>;
    type Values = Values<N>;
    type Decoder = Decoder<N>;
    type Decoded = Values<N>;

    fn root(&self, boards: &[Board; 2], _blank: usize) -> (Indices<N>, Values<N>, u64) {
        let dbs = self.dbs();
        let mut idx = [[0u32; N]; 2];
        let mut rank = [[0u8; N]; 2];
        let mut values = Values {
            a: [[0; N]; 2],
            f: [[0; N]; 2],
        };
        let walked = walk::root_values::<S>(dbs, boards);
        let mut bound = 0u32;
        for (s, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&t| t == 0).unwrap();
            let mut sums = [0u32; MAX_PARTS];
            for (g, &group) in S::GROUPS.iter().enumerate() {
                let placement = walk::index(group, board);
                idx[s][g] = placement * group.format.scale();
                rank[s][g] = board[..blank].iter().filter(|&&t| !group.has(t)).count() as u8;
                let (a, f) = if group.format.coded() {
                    walked[s][g]
                } else {
                    Self::decode(
                        dbs,
                        g,
                        (idx[s][g], rank[s][g]),
                        blank as u32,
                        0,
                        Half::new(group.format, 0, 0),
                    )
                };
                values.a[s][g] = a as u8;
                values.f[s][g] = f as u8;
                sums[group.partition as usize] += a;
                if group.full {
                    bound = bound.max(f);
                }
            }
            bound = bound.max(sums.into_iter().max().unwrap_or(0));
        }
        (Indices { idx, rank }, values, u64::from(bound))
    }

    #[inline(always)]
    fn shifted(base: Indices<N>, boards: &[Board; 2], m: &Move, tiles: [u8; 2]) -> Indices<N> {
        let (i0, o0) = Self::shift_side(base.idx[0], base.rank[0], &boards[0], m, 0, tiles[0]);
        let (i1, o1) = Self::shift_side(base.idx[1], base.rank[1], &boards[1], m, 1, tiles[1]);
        Indices {
            idx: [i0, i1],
            rank: [o0, o1],
        }
    }

    #[inline(always)]
    fn prefetch(&self, c: &Indices<N>, blanks: [u8; 2]) {
        let dbs = self.dbs();
        // In the order evaluate reads them.
        each_part!(S, p, {
            for s in 0..2 {
                each_group!(N, g, {
                    if S::GROUPS[g].partition as usize == p {
                        prefetch(Self::address(dbs, g, c.idx[s][g], u32::from(blanks[s])));
                    }
                });
            }
        });
    }

    #[inline(always)]
    fn decoder(values: Values<N>) -> Decoder<N> {
        Decoder {
            h: std::array::from_fn(|s| {
                std::array::from_fn(|g| {
                    Half::new(S::GROUPS[g].format, values.a[s][g], values.f[s][g])
                })
            }),
        }
    }

    /// Short-circuits in prune order: each partition in both orientations, a partition's
    /// estimate tested as soon as its groups are decoded. The key is the maximum in the high bits
    /// and the sum of all the estimates below.
    #[inline(always)]
    fn evaluate(
        &self,
        c: &Indices<N>,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &Decoder<N>,
        depth: u64,
    ) -> Option<(u64, Values<N>)> {
        let dbs = self.dbs();
        let mut out = Values {
            a: [[0; N]; 2],
            f: [[0; N]; 2],
        };
        let mut best = 0u32;
        let mut total = 0u32;
        each_part!(S, p, {
            for s in 0..2 {
                let blank = u32::from(blanks[s]);
                let mut sum = 0u32;
                let mut m = 0u32;
                each_group!(N, g, {
                    if S::GROUPS[g].partition as usize == p {
                        let (a, f) = Self::decode(
                            dbs,
                            g,
                            (c.idx[s][g], c.rank[s][g]),
                            blank,
                            tiles[s],
                            decoder.h[s][g],
                        );
                        out.a[s][g] = a as u8;
                        out.f[s][g] = f as u8;
                        sum += a;
                        if S::GROUPS[g].full {
                            m = m.max(f);
                            total += f;
                        }
                    }
                });
                m = m.max(sum);
                total += sum;
                if u64::from(m) >= depth {
                    return None;
                }
                best = best.max(m);
            }
        });
        Some((u64::from((best << 10) + total), out))
    }

    #[inline(always)]
    fn child(_values: Values<N>, decoded: Values<N>, _tiles: [u8; 2]) -> Values<N> {
        decoded
    }
}

mod walk;
