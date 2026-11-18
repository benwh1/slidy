//! A generic [`TableSet`]: any number of partitions of tiles 1-15 into groups, each group's
//! table in one of a few [`Format`]s, looked up in both orientations (direct and mirrored).
//!
//! A set is described by a [`Spec`]: its [`Group`]s, each naming its tiles, the partition it
//! belongs to and the format of its table. The heuristic is the maximum, over both orientations,
//! of every partition's additive sum and of the full (blank-counting) value of every group whose
//! table has one and asks for it. Coded tables (2-bit minimum codes, exact bits) are decoded
//! against the parent's values, which the search carries; the root's come from walking downhill
//! in each coded table (`walk`).

use std::{hint::select_unpredictable, marker::PhantomData};

use crate::solver::size4x4::stm::{
    large::TableSet,
    pdb::region::{Dbs, Layout, decode_code, prefetch},
    utils::{Board, Move, permutations},
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
    #[must_use]
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
            Format::Pair8 => 2 * n,
            f @ (Format::Cells | Format::Pair | Format::Bytes16 | Format::Bytes8) => {
                n * f.scale() as u64
            }
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

/// The generic table set for spec `S`.
pub struct Gen<S, const N: usize>(PhantomData<S>);

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

impl<S: Spec, const N: usize> Gen<S, N> {
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

impl<S: Spec, const N: usize> TableSet for Gen<S, N> {
    const LAYOUT: Layout = S::LAYOUT;
    type Indices = Indices<N>;
    type Values = Values<N>;
    type Decoder = Decoder<N>;
    type Decoded = Values<N>;

    fn root(dbs: Dbs<'_>, boards: &[Board; 2], _blank: usize) -> (Indices<N>, Values<N>, u64) {
        const { assert!(N == S::GROUPS.len(), "N is the number of groups") };
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
    fn prefetch(dbs: Dbs<'_>, c: &Indices<N>, blanks: [u8; 2]) {
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
        dbs: Dbs<'_>,
        c: &Indices<N>,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &Decoder<N>,
        depth: u64,
    ) -> Option<(u64, Values<N>)> {
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
