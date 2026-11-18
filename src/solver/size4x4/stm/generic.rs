//! A generic [`TableSet`]: any number of partitions of tiles 1-15 into groups, each group's
//! table in one of a few [`Format`]s, looked up in both orientations (direct and mirrored).
//!
//! A set is described by a [`Spec`]: its [`Group`]s, each naming its tiles, the partition it
//! belongs to and the format of its table. The heuristic is the maximum, over both orientations,
//! of every partition's additive sum and of the full (blank-counting) value of every group whose
//! table has one and asks for it. Coded tables (2-bit minimum codes, exact bits) are decoded
//! against the parent's values, which the search carries; the root's come from walking downhill
//! in each coded table ([`walk`]).

use std::{hint::select_unpredictable, marker::PhantomData};

use crate::solver::size4x4::stm::{
    large::TableSet,
    pdb::{Dbs, Layout, decode_code, prefetch},
    tables::{Board, Move, permutations},
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
    /// Bytes per placement in the scaled index (`Min2`'s index counts placements).
    pub const fn scale(self) -> u32 {
        match self {
            Self::Min2 => 1,
            Self::Cells => 2,
            Self::Pair => 4,
            Self::Pair8 => 2,
            Self::Bytes16 => 32,
            Self::Bytes8 => 16,
        }
    }

    /// Whether the table holds codes that decode against the parent's value.
    pub const fn coded(self) -> bool {
        matches!(self, Self::Min2 | Self::Cells | Self::Pair | Self::Pair8)
    }

    /// Whether the table holds a full value.
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
    pub const fn with_full(mut self) -> Self {
        assert!(self.format.has_full());
        self.full = true;
        self
    }

    /// Number of tiles.
    pub const fn len(self) -> u32 {
        self.tiles.count_ones()
    }

    /// Number of placements of the group's tiles.
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
    pub const fn bytes(self) -> u64 {
        let n = self.placements();
        match self.format {
            Format::Min2 => n.div_ceil(4),
            f => n * f.scale() as u64,
        }
    }

    /// Whether `tile` belongs to the group.
    #[inline(always)]
    pub const fn has(self, tile: u8) -> bool {
        (self.tiles >> tile) & 1 != 0
    }

    /// Lehmer weight of `tile` (a group tile) in the group's index: tiles in ascending order,
    /// cells in radix 16, 15, ...
    const fn weight(self, tile: u8) -> u32 {
        let i = (self.tiles & ((1 << tile) - 1)).count_ones() as i32;
        let k = self.len() as i32;
        permutations(15 - i, k - 1 - i) as u32
    }

    /// The group's tiles in ascending order, and how many there are.
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
    const fn new(groups: &[Group]) -> Self {
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

/// The generic table set for spec `S`.
pub struct Gen<S, const N: usize>(PhantomData<S>);

/// Both orientations' scaled indices, one per group.
#[derive(Clone, Copy)]
pub struct Indices<const N: usize> {
    idx: [[u32; N]; 2],
    /// The cells holding each `Pair8` group's tiles (unused for other formats).
    occ: [[u16; N]; 2],
}

/// A node's values in both orientations: each group's additive and full value.
#[derive(Clone, Copy)]
pub struct Values<const N: usize> {
    a: [[u8; N]; 2],
    f: [[u8; N]; 2],
}

/// `((value + 1) >> 1) & 1`: the entry bit of `value + 1`.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

impl<S: Spec, const N: usize> Gen<S, N> {
    /// One orientation's indices after move `m`, which moves `tile` into the blank.
    #[inline(always)]
    fn shift_side(
        base: [u32; N],
        occ: [u16; N],
        board: &Board,
        m: &Move,
        side: usize,
        tile: u8,
    ) -> ([u32; N], [u16; N]) {
        let (lo, gap, sign) = (m.lo[side] as usize, m.gap[side] as usize, m.sign[side] != 0);
        let two = S::PARTS > 2;
        // SAFETY: tile is a tile value.
        let row = unsafe { S::SHIFT.contrib.get_unchecked(tile as usize) };
        // SAFETY: as above.
        let own = unsafe { S::SHIFT.wbits.get_unchecked(tile as usize) };
        let mut acc0 = own[0];
        let mut acc1 = if two { own[1] } else { 0 };
        macro_rules! cell {
            ($k:literal) => {{
                // SAFETY: a cell strictly between the two blank positions, and a tile value.
                let packed = unsafe { row.get_unchecked(*board.get_unchecked(lo + $k) as usize) };
                acc0 = acc0.wrapping_add(packed[0]);
                if two {
                    acc1 = acc1.wrapping_add(packed[1]);
                }
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
        let mut occ = occ;
        let swap = (1u16 << lo) | (1u16 << (lo + gap));
        each_group!(N, g, {
            let group = S::GROUPS[g];
            let d = deltas[group.partition as usize];
            out[g] = out[g].wrapping_add(select_unpredictable(group.has(tile), d, 0));
            if matches!(group.format, Format::Pair8) {
                occ[g] ^= select_unpredictable(group.has(tile), swap, 0);
            }
        });
        (out, occ)
    }

    /// Group `g`'s values (additive, full) for a child in one orientation with index `idx`, the
    /// blank at `blank` and the moved tile `tile`, from the parent's `a` and `f`.
    #[inline(always)]
    #[allow(clippy::too_many_arguments, reason = "one group's inputs")]
    fn decode(
        dbs: Dbs<'_>,
        g: usize,
        idx: u32,
        occ: u16,
        blank: u32,
        tile: u8,
        a: u32,
        f: u32,
    ) -> (u32, u32) {
        let group = S::GROUPS[g];
        let off = S::OFFSETS[g];
        let moved = group.has(tile);
        match group.format {
            Format::Min2 => {
                let code = (dbs.byte(off, idx >> 2) >> ((idx & 3) * 2)) & 3;
                let child = decode_code(a.wrapping_sub(3), (a + 5) / 2, code);
                (select_unpredictable(moved, child, a), 0)
            }
            Format::Cells => {
                let bit = (dbs.half(off, idx) >> blank) & 1;
                let child = a + 1 - 2 * (bit ^ up(a));
                (select_unpredictable(moved, child, a), 0)
            }
            Format::Pair => {
                let word = dbs.word(off, idx) >> (2 * blank);
                let child = a + 1 - 2 * ((word & 1) ^ up(a));
                let full = f + 1 - 2 * (((word >> 1) & 1) ^ up(f));
                (select_unpredictable(moved, child, a), full)
            }
            Format::Pair8 => {
                let rank = (!occ & ((1u16 << blank) - 1)).count_ones();
                let word = dbs.half(off, idx) >> rank;
                let child = a + 1 - 2 * ((word & 1) ^ up(a));
                let full = f + 1 - 2 * (((word >> 8) & 1) ^ up(f));
                (select_unpredictable(moved, child, a), full)
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
            Format::Cells | Format::Pair | Format::Pair8 => dbs.entry(off, idx),
            Format::Bytes16 => dbs.entry(off, idx + 2 * blank),
            Format::Bytes8 => dbs.entry(off, idx + blank),
        }
    }
}

impl<S: Spec, const N: usize> TableSet for Gen<S, N> {
    const LAYOUT: Layout = S::LAYOUT;
    type Indices = Indices<N>;
    type Values = Values<N>;
    type Decoder = Values<N>;
    type Decoded = Values<N>;

    fn root(dbs: Dbs<'_>, boards: &[Board; 2], _blank: usize) -> (Indices<N>, Values<N>, u64) {
        const { assert!(N == S::GROUPS.len(), "N is the number of groups") };
        let mut idx = [[0u32; N]; 2];
        let mut occ = [[0u16; N]; 2];
        let mut values = Values {
            a: [[0; N]; 2],
            f: [[0; N]; 2],
        };
        let mut bound = 0u32;
        for (s, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&t| t == 0).unwrap();
            let mut sums = [0u32; MAX_PARTS];
            for (g, &group) in S::GROUPS.iter().enumerate() {
                let placement = walk::index(group, board);
                idx[s][g] = placement * group.format.scale();
                occ[s][g] = board
                    .iter()
                    .enumerate()
                    .filter(|&(_, &t)| group.has(t))
                    .fold(0, |m, (c, _)| m | (1 << c));
                let (a, f) = if group.format.coded() {
                    let a = walk::value(dbs, S::OFFSETS[g], group, board, false);
                    let f = if group.format.has_full() {
                        walk::value(dbs, S::OFFSETS[g], group, board, true)
                    } else {
                        0
                    };
                    (a, f)
                } else {
                    Self::decode(dbs, g, idx[s][g], occ[s][g], blank as u32, 0, 0, 0)
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
        (Indices { idx, occ }, values, u64::from(bound))
    }

    #[inline(always)]
    fn shifted(base: Indices<N>, boards: &[Board; 2], m: &Move, tiles: [u8; 2]) -> Indices<N> {
        let (i0, o0) = Self::shift_side(base.idx[0], base.occ[0], &boards[0], m, 0, tiles[0]);
        let (i1, o1) = Self::shift_side(base.idx[1], base.occ[1], &boards[1], m, 1, tiles[1]);
        Indices {
            idx: [i0, i1],
            occ: [o0, o1],
        }
    }

    #[inline(always)]
    fn prefetch(dbs: Dbs<'_>, c: &Indices<N>, blanks: [u8; 2]) {
        for s in 0..2 {
            each_group!(N, g, {
                prefetch(Self::address(dbs, g, c.idx[s][g], u32::from(blanks[s])));
            });
        }
    }

    #[inline(always)]
    fn decoder(values: Values<N>) -> Values<N> {
        values
    }

    #[inline(always)]
    fn evaluate(
        dbs: Dbs<'_>,
        c: &Indices<N>,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &Values<N>,
        depth: u64,
    ) -> Option<(u64, Values<N>)> {
        let mut out = *decoder;
        let mut best = 0u32;
        let mut total = 0u32;
        for s in 0..2 {
            let blank = u32::from(blanks[s]);
            let mut sums = [0u32; MAX_PARTS];
            let mut fulls = 0u32;
            each_group!(N, g, {
                let (a, f) = Self::decode(
                    dbs,
                    g,
                    c.idx[s][g],
                    c.occ[s][g],
                    blank,
                    tiles[s],
                    u32::from(decoder.a[s][g]),
                    u32::from(decoder.f[s][g]),
                );
                out.a[s][g] = a as u8;
                out.f[s][g] = f as u8;
                sums[S::GROUPS[g].partition as usize] += a;
                if S::GROUPS[g].full {
                    fulls = fulls.max(f);
                    total += f;
                }
            });
            let mut m = fulls;
            for &sum in &sums[..S::PARTS] {
                m = m.max(sum);
                total += sum;
            }
            if u64::from(m) >= depth {
                return None;
            }
            best = best.max(m);
        }
        Some((u64::from((best << 10) + total), out))
    }

    #[inline(always)]
    fn child(_values: Values<N>, decoded: Values<N>, _tiles: [u8; 2]) -> Values<N> {
        decoded
    }
}

/// Exact values of coded tables at the root, by walking downhill to the goal.
mod walk {
    use super::{Format, Group};
    use crate::solver::size4x4::stm::{
        build::lehmer,
        pdb::{Dbs, decode_code},
        tables::{Board, NEIGHBORS},
    };

    /// Lehmer index of the group's placement on `board` (unscaled).
    pub fn index(group: Group, board: &Board) -> u32 {
        let mut cell = [0u8; 16];
        for (c, &t) in board.iter().enumerate() {
            cell[t as usize] = c as u8;
        }
        let (tiles, k) = group.tile_list();
        let cells: [u8; 16] =
            std::array::from_fn(|i| if i < k { cell[tiles[i] as usize] } else { 0 });
        lehmer(&cells[..k])
    }

    fn nbr(cell: usize) -> u16 {
        NEIGHBORS[cell]
            .iter()
            .filter(|&&n| n >= 0)
            .fold(0, |m, &n| m | (1 << n))
    }

    fn flood(seed: u16, free: u16) -> u16 {
        let mut region = seed;
        loop {
            let mut next = region;
            let mut r = region;
            while r != 0 {
                let c = r.trailing_zeros() as usize;
                r &= r - 1;
                next |= nbr(c) & free;
            }
            if next == region {
                return region;
            }
            region = next;
        }
    }

    /// The entry code of a placement (`cells`, the blank at `blank`): a bit, or a 2-bit code for
    /// `Min2`.
    fn code(dbs: Dbs<'_>, off: isize, group: Group, cells: &[u8], blank: usize, full: bool) -> u32 {
        let p = lehmer(cells);
        match group.format {
            Format::Min2 => (dbs.byte(off, p >> 2) >> ((p & 3) * 2)) & 3,
            Format::Cells => (dbs.half(off, 2 * p) >> blank) & 1,
            Format::Pair => (dbs.word(off, 4 * p) >> (2 * blank + usize::from(full))) & 1,
            Format::Pair8 => {
                let occ = cells.iter().fold(0u16, |m, &c| m | (1 << c));
                let rank = (!occ & ((1u16 << blank) - 1)).count_ones();
                (dbs.half(off, 2 * p) >> (rank + 8 * u32::from(full))) & 1
            }
            Format::Bytes16 | Format::Bytes8 => unreachable!(),
        }
    }

    /// The exact value at `board` of group `group`'s coded table at `off`: its full value when
    /// `full`, else its additive value.
    pub fn value(dbs: Dbs<'_>, off: isize, group: Group, board: &Board, full: bool) -> u32 {
        let (tiles, k) = group.tile_list();
        let mut cells = [0u8; 16];
        let mut blank = 0;
        for (c, &t) in board.iter().enumerate() {
            if t == 0 {
                blank = c;
            } else if group.has(t) {
                let i = (group.tiles & ((1 << t) - 1)).count_ones() as usize;
                cells[i] = c as u8;
            }
        }
        let home: [u8; 16] = std::array::from_fn(|i| if i < k { tiles[i] } else { 0 });
        let parity = if full {
            (blank & 1) as u32
        } else {
            (0..k).fold(0, |p, i| p ^ (u32::from(cells[i] ^ tiles[i]) & 1))
        };
        let min2 = group.format == Format::Min2;
        let start = if min2 {
            512 + 2 * code(dbs, off, group, &cells[..k], blank, full) + parity
        } else {
            256 + 2 * code(dbs, off, group, &cells[..k], blank, full) + parity
        };
        let mut cur = start;
        loop {
            let home_placed = cells[..k] == home[..k];
            if home_placed && (!full || blank == 0) {
                break;
            }
            let occupied = (0..k).fold(0u16, |m, i| m | (1 << cells[i]));
            let free = !occupied;
            let decode = |child_code: u32| {
                if min2 {
                    decode_code(cur - 3, (cur + 5) / 2, child_code)
                } else {
                    let up = ((cur + 1) >> 1) & 1;
                    cur + 1 - 2 * (child_code ^ up)
                }
            };
            let mut stepped = false;
            if full {
                for n in nbr(blank).into_iter_bits() {
                    let mut next = cells;
                    if let Some(j) = (0..k).find(|&j| usize::from(cells[j]) == n) {
                        next[j] = blank as u8;
                    }
                    let v = decode(code(dbs, off, group, &next[..k], n, true));
                    if v < cur {
                        (cells, blank, cur) = (next, n, v);
                        stepped = true;
                        break;
                    }
                }
            } else {
                let region = if min2 { free } else { flood(1 << blank, free) };
                'moves: for j in 0..k {
                    let from = usize::from(cells[j]);
                    for to in (nbr(from) & region).into_iter_bits() {
                        let mut next = cells;
                        next[j] = to as u8;
                        let v = decode(code(dbs, off, group, &next[..k], from, false));
                        if v < cur {
                            (cells, blank, cur) = (next, from, v);
                            stepped = true;
                            break 'moves;
                        }
                    }
                }
            }
            assert!(stepped, "every entry but the goal's has a lower neighbour");
        }
        start - cur
    }

    /// Iterating the set bits of a mask.
    trait Bits {
        fn into_iter_bits(self) -> impl Iterator<Item = usize>;
    }

    impl Bits for u16 {
        fn into_iter_bits(self) -> impl Iterator<Item = usize> {
            let mut m = self;
            std::iter::from_fn(move || {
                (m != 0).then(|| {
                    let c = m.trailing_zeros() as usize;
                    m &= m - 1;
                    c
                })
            })
        }
    }
}
