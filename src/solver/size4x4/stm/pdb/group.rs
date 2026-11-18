//! Tile groups and the table formats that store their values, and [`Spec`], which describes a
//! table set as a list of groups.

use crate::solver::size4x4::stm::{
    pdb::{region::Layout, tables::Shift},
    utils::permutations,
};

/// The most groups a set may have.
pub const MAX_GROUPS: usize = 6;

/// The most partitions a set may have.
pub const MAX_PARTS: usize = 4;

/// How a group's table stores its values.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    /// The additive value minimised over the blank's cells (relaxed so that a move changes it by
    /// 1 or 3), as a 2-bit code `(value >> 1) & 3` per placement, four per byte.
    Min2,
    /// The exact additive value: bit `cell` of a little-endian u16 per placement holds
    /// `(value >> 1) & 1` for the blank at `cell`.
    Cells,
    /// The exact additive and full values: bits `2 * cell` and `2 * cell + 1` of a little-endian
    /// u32 per placement.
    Pair,
    /// The exact additive and full values of a group of 8 or more tiles: bits `r` and `8 + r` of
    /// a little-endian u16 per placement, for the blank at the `r`-th cell not holding a group
    /// tile.
    Pair8,
    /// Plain values: a little-endian u16 per placement and blank cell, the additive value in the
    /// low byte, the full one in the high byte.
    Bytes16,
    /// Plain additive values: a byte per placement and blank cell.
    Bytes8,
}

impl Format {
    /// Index units per placement: bytes, except that `Min2`'s and `Pair8`'s indices count
    /// placements (so a 9-tile `Pair8` index fits a u32).
    #[must_use]
    pub const fn scale(self) -> u32 {
        match self {
            Self::Min2 => 1,
            Self::Cells => 2,
            Self::Pair => 4,
            Self::Pair8 => 1,
            Self::Bytes16 => 32,
            Self::Bytes8 => 16,
        }
    }

    /// Whether the table holds codes that decode against the parent's value.
    #[must_use]
    pub const fn coded(self) -> bool {
        matches!(self, Self::Min2 | Self::Cells | Self::Pair | Self::Pair8)
    }

    /// Whether the table holds a full value.
    #[must_use]
    pub const fn has_full(self) -> bool {
        matches!(self, Self::Pair | Self::Pair8 | Self::Bytes16)
    }
}

/// One group of tiles and its table.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    /// The group's tiles, as a mask over tile labels (internal labels: tile `t`'s home is
    /// cell `t`).
    pub tiles: u16,
    /// The partition the group belongs to.
    pub partition: u8,
    /// Its table's format.
    pub format: Format,
    /// Whether its full value is a term of the maximum (only for formats that hold one).
    pub full: bool,
}

impl Group {
    /// The group of `tiles` in `partition`, its table in `format`.
    #[must_use]
    pub const fn new(tiles: &[u8], partition: u8, format: Format) -> Self {
        let mut mask = 0u16;
        let mut i = 0;
        while i < tiles.len() {
            assert!(tiles[i] >= 1 && tiles[i] <= 15);
            mask |= 1 << tiles[i];
            i += 1;
        }
        assert!(!matches!(format, Format::Pair8) || mask.count_ones() >= 8);
        Self {
            tiles: mask,
            partition,
            format,
            full: false,
        }
    }

    /// The same group with its full value as a term.
    #[must_use]
    pub const fn with_full(mut self) -> Self {
        assert!(self.format.has_full());
        self.full = true;
        self
    }

    /// Number of tiles.
    #[must_use]
    #[expect(clippy::len_without_is_empty, reason = "a group is never empty")]
    pub const fn len(self) -> u32 {
        self.tiles.count_ones()
    }

    /// Number of placements of the group's tiles.
    #[must_use]
    pub const fn placements(self) -> u64 {
        let mut n = 1u64;
        let mut i = 0;
        while i < self.len() {
            n *= 16 - i as u64;
            i += 1;
        }
        n
    }

    /// Size of its table in bytes.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        let n = self.placements();
        match self.format {
            Format::Min2 => n.div_ceil(4),
            Format::Pair8 => 2 * n,
            f @ (Format::Cells | Format::Pair | Format::Bytes16 | Format::Bytes8) => {
                n * f.scale() as u64
            }
        }
    }

    /// Whether `tile` belongs to the group.
    #[inline(always)]
    #[must_use]
    pub const fn has(self, tile: u8) -> bool {
        (self.tiles >> tile) & 1 != 0
    }

    /// Lehmer weight of `tile` (a group tile) in the group's index: tiles in ascending order,
    /// cells in radix 16, 15, ...
    pub(super) const fn weight(self, tile: u8) -> u32 {
        let i = (self.tiles & ((1 << tile) - 1)).count_ones() as i32;
        let k = self.len() as i32;
        permutations(15 - i, k - 1 - i) as u32
    }

    /// The group's tiles in ascending order, and how many there are.
    #[must_use]
    pub fn tile_list(self) -> ([u8; 16], usize) {
        let mut out = [0; 16];
        let mut k = 0;
        for t in 1..16 {
            if self.has(t) {
                out[k] = t;
                k += 1;
            }
        }
        (out, k)
    }
}

/// The table sizes of `groups`, one table per group, for a [`Layout`].
#[must_use]
pub const fn parts<const N: usize>(groups: &[Group; N]) -> [(&'static str, u64); N] {
    let mut out = [("", 0); N];
    let mut g = 0;
    while g < N {
        out[g].1 = groups[g].bytes();
        g += 1;
    }
    out
}

/// A generic table set's description.
pub trait Spec: 'static {
    /// The groups, one table each. Every partition covers tiles 1-15 exactly once.
    const GROUPS: &'static [Group];
    /// The tables' layout: one part per group, in order (see [`parts`]).
    const LAYOUT: Layout;

    /// Number of partitions.
    const PARTS: usize = count_parts(Self::GROUPS);
    /// Each group's displacement from the base pointer.
    const OFFSETS: [isize; MAX_GROUPS] = offsets(&Self::LAYOUT, Self::GROUPS.len());
    /// The index shift tables.
    const SHIFT: &'static Shift = &Shift::new(Self::GROUPS);
}

const fn count_parts(groups: &[Group]) -> usize {
    assert!(groups.len() <= MAX_GROUPS);
    let mut parts = 0;
    let mut g = 0;
    while g < groups.len() {
        if groups[g].partition as usize + 1 > parts {
            parts = groups[g].partition as usize + 1;
        }
        g += 1;
    }
    assert!(parts <= MAX_PARTS);
    // Every partition covers tiles 1-15 exactly once.
    let mut p = 0;
    while p < parts {
        let mut covered = 0u16;
        let mut g = 0;
        while g < groups.len() {
            if groups[g].partition as usize == p {
                assert!(covered & groups[g].tiles == 0);
                covered |= groups[g].tiles;
            }
            g += 1;
        }
        assert!(covered == 0xFFFE);
        p += 1;
    }
    parts
}

const fn offsets(layout: &Layout, n: usize) -> [isize; MAX_GROUPS] {
    let mut out = [0; MAX_GROUPS];
    let mut g = 0;
    while g < n {
        out[g] = layout.offset(g);
        g += 1;
    }
    out
}
