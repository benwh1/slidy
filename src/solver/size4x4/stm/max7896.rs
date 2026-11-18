//! The max(7-8, 9-6) table set with full (blank-counting) terms, in bit
//! tables: the [`TableSet`] the solver uses.
//!
//! Takes the elementwise maximum, in both orientations, of the additive
//! 7-8 sum, the full 1-7+blank and 8-15+blank values, the additive 9-6
//! sum and the full 1-9+blank and 10-15+blank values. The 1-7 and 8-15
//! tables hold one bit per entry, the value mod 4 with the position's
//! parity, and the 1-9 table a 2-bit code, the value mod 8 (plus a full
//! bit per free cell), so the search carries those tables' values down
//! the tree ([`Exact`]) and decodes each child's entries against its
//! parent's values; the 10-15 table holds plain values. The root's values
//! come from walking downhill in each coded table (`walk`).
//!
//! Every table but the 1-9 minimum is exact in the blank; the exact bit
//! tables hold `(value >> 1) & 1`, which decodes exactly against the
//! parent's value because exact tables are consistent (a move changes a
//! value by at most 1; see `bitpack`). The 1-9 minimum over the blank's
//! regions can change by 3 (larger steps were relaxed away, on 0.46% of
//! entries), so it holds two bits per placement, `(value >> 1) & 3`.
//!
//! - 1-7 (`p17b`) and 8-15 (`p815w2`): a little-endian u32 per placement, bit `2 * cell` for the
//!   additive value with the blank at `cell` and bit `2 * cell + 1` for the full one (every move
//!   counted, the blank part of the pattern).
//! - 1-9 (`p19mf`): a mixed table (`Dbs::mix_entry`), the relaxed two-bit minimum of `idea/two-bit`
//!   (`p19m2`) and the full 1-9 bits (`f19x`, `bigbuild --mode full`): a byte per placement, bit
//!   `r` for the blank at the `r`-th cell not holding a tile 1-9.
//! - 10-15 (`p1015w16`): plain values, a little-endian u16 per placement and blank cell, indexed
//!   `placement * 16 + cell`, the additive value in the low byte and the full one in the high byte.
//!
//! `u32` indices throughout: the largest, P(16,9), fits.

use std::hint::select_unpredictable;

use crate::solver::size4x4::stm::{
    large::TableSet,
    pdb::{Dbs, Layout, decode_code, prefetch},
    shift::{Partitions, move_deltas},
    tables::{self, Board, MIRROR, Move},
    walk::{self, Group, Kind, WalkSpec},
};

/// The max(7-8, 9-6) set with full terms ([module docs](self)).
pub struct Max7896;

/// Table sizes in bytes.
const SIZE_17: u64 = 4 * 57_657_600;
const SIZE_815: u64 = 4 * 518_918_400;
const SIZE_19: u64 = 5 * (4_151_347_200 / 4);
const SIZE_1015: u64 = 32 * 5_765_760;

/// The combined file, made by `pdbcombine` from the separate tables.
const LAYOUT: Layout = Layout {
    file: "combined-l4",
    parts: &[
        ("p17b", SIZE_17),
        ("p1015w16", SIZE_1015),
        ("p815w2", SIZE_815),
        ("p19mf", SIZE_19),
    ],
};

const OFF_17: isize = LAYOUT.offset(0);
const OFF_1015: isize = LAYOUT.offset(1);
const OFF_815: isize = LAYOUT.offset(2);
const OFF_19: isize = LAYOUT.offset(3);

// Every displacement fits an x86 addressing mode.
const _: () = assert!(OFF_17 > i32::MIN as isize);
const _: () = assert!(OFF_19 < i32::MAX as isize);

impl Partitions for Max7896 {
    const SPLIT: [u8; 2] = [7, 9];
    /// The 1-7 and 8-15 tables hold a u32 of entry bits per placement
    /// (x4), the 10-15 table 16 u16 entries per placement (x32); the 1-9
    /// index counts placements (5-byte blocks of four).
    const SCALE: [[u32; 2]; 2] = [[4, 4], [1, 32]];
}

/// The groups and tables the root walks: the additive and full 1-7 and
/// 8-15 tables, the 1-9 minimum and the full 1-9 table. The order is
/// [`Exact`]'s.
const GROUPS: [Group; 3] = [
    Group::of::<Max7896>((1, 7)),
    Group::of::<Max7896>((8, 15)),
    Group::of::<Max7896>((1, 9)),
];
const WALKS: [WalkSpec; 6] = [
    WalkSpec::new(OFF_17, Kind::Additive, 0),
    WalkSpec::new(OFF_815, Kind::Additive, 1),
    WalkSpec::new(OFF_17, Kind::Full, 0),
    WalkSpec::new(OFF_815, Kind::Full, 1),
    WalkSpec::new(OFF_19, Kind::Min2, 2),
    WalkSpec::new(OFF_19, Kind::FullCompact, 2),
];

/// One orientation's table state: the Lehmer indices of both partitions
/// (7-8 and 9-6), scaled to their tables, and the blank's rank among the
/// cells without a tile 1-9 (which picks its bit in the full 1-9 byte).
#[derive(Clone, Copy)]
pub struct Side {
    i17: u32,
    i815: u32,
    i19: u32,
    i1015: u32,
    rank: u8,
}

/// Both orientations' [`Side`]s, stored per field: the pairs' padding
/// would grow the search's candidates, and every copy of them, by 4 bytes.
#[derive(Clone, Copy)]
pub struct Indices {
    i17: [u32; 2],
    i815: [u32; 2],
    i19: [u32; 2],
    i1015: [u32; 2],
    rank: [u8; 2],
}

impl Indices {
    #[inline(always)]
    fn side(&self, s: usize) -> Side {
        Side {
            i17: self.i17[s],
            i815: self.i815[s],
            i19: self.i19[s],
            i1015: self.i1015[s],
            rank: self.rank[s],
        }
    }

    #[inline(always)]
    fn new([d, m]: [Side; 2]) -> Self {
        Self {
            i17: [d.i17, m.i17],
            i815: [d.i815, m.i815],
            i19: [d.i19, m.i19],
            i1015: [d.i1015, m.i1015],
            rank: [d.rank, m.rank],
        }
    }
}

impl Side {
    fn root(board: &Board) -> Self {
        let blank = tables::blank_position(board);
        Self {
            i17: 4 * tables::lehmer_index(board, 1, 7) as u32,
            i815: 4 * tables::lehmer_index(board, 8, 15) as u32,
            i19: tables::lehmer_index(board, 1, 9) as u32,
            i1015: 32 * tables::lehmer_index(board, 10, 15) as u32,
            rank: board[..blank].iter().filter(|&&tile| tile >= 10).count() as u8,
        }
    }

    /// The state after move `m` (orientation `side`) on `board`, which
    /// moves `tile` into the blank.
    #[inline(always)]
    fn shifted(self, board: &Board, m: &Move, side: usize, tile: u8) -> Self {
        let (lo, gap, up) = (m.lo[side] as usize, m.gap[side] as usize, m.sign[side]);
        let delta = move_deltas::<Max7896>(board, lo, gap, up != 0, tile as usize);
        // Deltas are signed; wrapping adds compute the mod-2^32 sums, and
        // the range checks (which also catch wrapped-around underflows) run
        // in debug only.
        let [[d17, d815], [d19, d1015]] = delta.shift;
        let shifted = Self {
            i17: self.i17.wrapping_add(d17 as u32),
            i815: self.i815.wrapping_add(d815 as u32),
            i19: self.i19.wrapping_add(d19 as u32),
            i1015: self.i1015.wrapping_add(d1015 as u32),
            rank: child_rank(self.rank, up, tile, delta.passed[1]),
        };
        debug_assert!(u64::from(shifted.i17) < SIZE_17);
        debug_assert!(u64::from(shifted.i815) < SIZE_815);
        debug_assert!(5 * (u64::from(shifted.i19) / 4) < SIZE_19);
        debug_assert!(u64::from(shifted.i1015) < SIZE_1015);
        shifted
    }

    /// Prefetch the child's 7-8 word lines.
    #[inline(always)]
    fn prefetch_78(self, dbs: Dbs<'_>) {
        prefetch(dbs.entry(OFF_17, self.i17));
        prefetch(dbs.entry(OFF_815, self.i815));
    }

    /// A position's 10-15 values, (additive, full), straight from the
    /// table.
    #[inline(always)]
    fn values1015(self, dbs: Dbs<'_>, blank: u8) -> (u32, u32) {
        let pair = dbs.half(OFF_1015, self.i1015 + 2 * u32::from(blank));
        (pair & 0xFF, pair >> 8)
    }

    /// A child's 7-8 estimates: (additive sum, full 1-7, full 8-15). Only
    /// the moved tile's group changes its additive value; which one it is
    /// is unpredictable, so the bit is selected without a branch. Both full
    /// values change on every move. Each word also gives the full estimate:
    /// same entry, no extra load.
    #[inline(always)]
    fn decode78(self, dbs: Dbs<'_>, blank: u8, tile: u8, d: &Decoder) -> [u32; 3] {
        let y = (dbs.word(OFF_17, self.i17) >> (2 * blank)) ^ d.m17;
        let e = (dbs.word(OFF_815, self.i815) >> (2 * blank)) ^ d.m815;
        let x = select_unpredictable(tile <= 7, y, e);
        [d.s78 - ((x << 1) & 2), d.f17 - (y & 2), d.f815 - (e & 2)]
    }

    /// A child's 9-6 estimates: (additive sum, full 10-15, full 1-9) and
    /// its 1-9 value. The full 1-9 bit sits at the blank's rank among the
    /// cells without a tile 1-9; the full value changes on every move.
    #[inline(always)]
    fn decode96(self, dbs: Dbs<'_>, blank: u8, tile: u8, d: &Decoder) -> ([u32; 3], u32) {
        let a19 = decode_code(d.b19, d.mid19, dbs.mix_code(OFF_19, self.i19));
        let a19 = select_unpredictable(tile <= 9, a19, d.a19);
        let e = (dbs.mix_byte(OFF_19, self.i19) >> self.rank) ^ d.m19f;
        let (a1015, f1015) = self.values1015(dbs, blank);
        ([a19 + a1015, f1015, d.f19 - ((e << 1) & 2)], a19)
    }
}

/// Exact values of the bit tables for one orientation: the additive 1-7
/// and 8-15 values, the full (blank-counting) 1-7+blank and 8-15+blank
/// ones and the 1-9 value (the minimum over the blank's regions), in
/// [`WALKS`] order. (The 10-15 table holds plain values, read directly.)
#[derive(Clone, Copy)]
pub struct Exact {
    a17: u8,
    a815: u8,
    f17: u8,
    f815: u8,
    a19: u8,
    f19: u8,
}

impl Exact {
    /// The child's values from its decoded estimates; the moved `tile`
    /// says which 7-8 group took the step.
    #[inline(always)]
    fn child(self, d: Decoded, tile: u8) -> Self {
        let small = tile <= 7;
        Self {
            a17: select_unpredictable(small, d.h78 - self.a815, self.a17),
            a815: select_unpredictable(small, self.a815, d.h78 - self.a17),
            f17: d.f17,
            f815: d.f815,
            a19: d.a19,
            f19: d.f19,
        }
    }

    /// Admissible estimate from these values and the 10-15 ones.
    fn bound(self, (a1015, f1015): (u32, u32)) -> u64 {
        let [a17, a815, f17, f815, a19, f19] =
            [self.a17, self.a815, self.f17, self.f815, self.a19, self.f19].map(u32::from);
        u64::from(
            (a17 + a815)
                .max(f17)
                .max(f815)
                .max(a19 + a1015)
                .max(f19)
                .max(f1015),
        )
    }
}

/// A child's decoded estimates in one orientation: the additive 7-8 sum,
/// the full 1-7 and 8-15 values and the 1-9 value.
#[derive(Clone, Copy)]
pub struct Decoded {
    h78: u8,
    f17: u8,
    f815: u8,
    a19: u8,
    f19: u8,
}

/// `((value + 1) >> 1) & 1`: the entry bit of `value + 1`. A move changes
/// a value by exactly 1 and an entry holds `(v >> 1) & 1`, so a child's
/// entry bit matches this exactly when its value is `value + 1`.
const fn up(value: u8) -> u32 {
    ((value as u32 + 1) >> 1) & 1
}

/// One node's values, one orientation, set up so a child decodes with
/// an XOR: with `y` the child's entry bits XOR the mask, the child's value
/// is the `+ 1` field minus 2 when `y`'s bit is set (the value went down).
#[derive(Clone, Copy)]
pub struct Decoder {
    /// Additive 7-8 sum + 1 (one 7-8 group takes the step).
    s78: u32,
    /// Masks for the 1-7 and 8-15 words (additive bit 0, full bit 1).
    m17: u32,
    m815: u32,
    /// Full 1-7 and 8-15 values + 1.
    f17: u32,
    f815: u32,
    /// 1-9 value (kept when a tile 10-15 moves), less 3, and
    /// `(value + 5) / 2` ([`decode_code`]).
    a19: u32,
    b19: u32,
    mid19: u32,
    /// Full 1-9 value + 1 and mask.
    f19: u32,
    m19f: u32,
}

impl Decoder {
    fn new(v: Exact) -> Self {
        Self {
            s78: u32::from(v.a17) + u32::from(v.a815) + 1,
            m17: up(v.a17) | (up(v.f17) << 1),
            m815: up(v.a815) | (up(v.f815) << 1),
            f17: u32::from(v.f17) + 1,
            f815: u32::from(v.f815) + 1,
            a19: u32::from(v.a19),
            b19: u32::from(v.a19).wrapping_sub(3),
            mid19: u32::midpoint(u32::from(v.a19), 5),
            f19: u32::from(v.f19) + 1,
            m19f: up(v.f19),
        }
    }
}

/// The child's blank rank from the parent's `rank`, when `tile` moves
/// into the blank's cell and the blank goes `up` (to a higher cell) or
/// down past `passed` tiles 10-15: the cells the blank passes, the moved
/// tile's old cell among them, are the only ones that change sides.
#[inline(always)]
fn child_rank(rank: u8, up: u8, tile: u8, passed: u8) -> u8 {
    let passed = u8::from(tile >= 10) + passed;
    select_unpredictable(up != 0, rank + passed, rank.wrapping_sub(passed))
}

/// Exact values of the coded tables at the root, both orientations.
fn root_exact(dbs: Dbs<'_>, boards: &[Board; 2]) -> [Exact; 2] {
    walk::root_values::<Max7896, 3, 6>(dbs, &GROUPS, &WALKS, boards).map(
        |[a17, a815, f17, f815, a19, f19]| Exact {
            a17,
            a815,
            f17,
            f815,
            a19,
            f19,
        },
    )
}

impl TableSet for Max7896 {
    const LAYOUT: Layout = LAYOUT;
    type Indices = Indices;
    type Values = [Exact; 2];
    type Decoder = [Decoder; 2];
    type Decoded = [Decoded; 2];

    fn root(dbs: Dbs<'_>, boards: &[Board; 2], blank: usize) -> (Indices, [Exact; 2], u64) {
        let sides = boards.each_ref().map(Side::root);
        let values = root_exact(dbs, boards);
        let blanks = [blank as u8, MIRROR[blank]];
        let distance = (0..2)
            .map(|s| values[s].bound(sides[s].values1015(dbs, blanks[s])))
            .max()
            .unwrap_or(0);
        (Indices::new(sides), values, distance)
    }

    #[inline(always)]
    fn shifted(base: Indices, boards: &[Board; 2], m: &Move, tiles: [u8; 2]) -> Indices {
        let side = |s: usize| base.side(s).shifted(&boards[s], m, s, tiles[s]);
        Indices::new([side(0), side(1)])
    }

    #[inline(always)]
    fn prefetch(dbs: Dbs<'_>, c: &Indices, blanks: [u8; 2]) {
        let (direct, mirrored) = (c.side(0), c.side(1));
        direct.prefetch_78(dbs);
        mirrored.prefetch_78(dbs);
        prefetch(dbs.mix_entry(OFF_19, direct.i19, 0));
        prefetch(dbs.entry(OFF_1015, direct.i1015 + 2 * u32::from(blanks[0])));
        prefetch(dbs.mix_entry(OFF_19, mirrored.i19, 0));
        // The block's last byte: a second line for the 4 in 64 blocks that
        // straddle two.
        prefetch(dbs.mix_entry(OFF_19, direct.i19, 4));
        prefetch(dbs.mix_entry(OFF_19, mirrored.i19, 4));
        prefetch(dbs.entry(OFF_1015, mirrored.i1015 + 2 * u32::from(blanks[1])));
    }

    #[inline(always)]
    fn decoder(values: [Exact; 2]) -> [Decoder; 2] {
        [Decoder::new(values[0]), Decoder::new(values[1])]
    }

    /// Short-circuit loads in prune order, decoding the child's entries
    /// against the node's values. The key is the full max in the high
    /// bits; ties (common) go to the smaller sum of all the estimates: in
    /// the final iteration that child is more often on a solution path
    /// (-2.4% instructions on 10k), and counting the full estimates in
    /// the tie-break too measured -3.4% nodes (lab, 2k).
    #[inline(always)]
    fn evaluate(
        dbs: Dbs<'_>,
        c: &Indices,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &[Decoder; 2],
        depth: u64,
    ) -> Option<(u64, [Decoded; 2])> {
        let within = |e: [u32; 3]| u64::from(e[0].max(e[1]).max(e[2])) < depth;
        let e78 = |s: usize| c.side(s).decode78(dbs, blanks[s], tiles[s], &decoder[s]);
        let e96 = |s: usize| c.side(s).decode96(dbs, blanks[s], tiles[s], &decoder[s]);
        let e78d = e78(0);
        if !within(e78d) {
            return None;
        }
        let e78m = e78(1);
        if !within(e78m) {
            return None;
        }
        let (e96d, a19d) = e96(0);
        if !within(e96d) {
            return None;
        }
        let (e96m, a19m) = e96(1);
        if !within(e96m) {
            return None;
        }
        // The max fills the high bits; the sum of all twelve estimates
        // (at most 12 * 80) fits below.
        let [[h78d, f17d, f815d], [h78m, f17m, f815m]] = [e78d, e78m];
        let [[h96d, f1015d, f19d], [h96m, f1015m, f19m]] = [e96d, e96m];
        let best = h78d
            .max(f17d.max(f815d))
            .max(h78m.max(f17m.max(f815m)))
            .max(h96d.max(f1015d).max(h96m.max(f1015m)))
            .max(f19d.max(f19m));
        let key = (best << 10)
            + (h78d + h78m + h96d + h96m)
            + (f17d + f17m + f1015d + f1015m)
            + (f815d + f815m + f19d + f19m);
        let decoded = |[h78, f17, f815]: [u32; 3], [_, _, f19]: [u32; 3], a19: u32| Decoded {
            h78: h78 as u8,
            f17: f17 as u8,
            f815: f815 as u8,
            a19: a19 as u8,
            f19: f19 as u8,
        };
        Some((
            u64::from(key),
            [decoded(e78d, e96d, a19d), decoded(e78m, e96m, a19m)],
        ))
    }

    #[inline(always)]
    fn child(values: [Exact; 2], decoded: [Decoded; 2], tiles: [u8; 2]) -> [Exact; 2] {
        [
            values[0].child(decoded[0], tiles[0]),
            values[1].child(decoded[1], tiles[1]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Root walks alone on the scrambles in `$WALK_SET` (one per line),
    /// `$WALK_REPS` times over: prints the time per root and a checksum
    /// of every value, which must not change. Needs `pdbs/combined-l4`.
    #[test]
    #[ignore = "needs the 7.7 GB tables and a scramble file"]
    fn bench_root_walks() {
        let set = std::fs::read_to_string(std::env::var("WALK_SET").unwrap()).unwrap();
        let reps: usize = std::env::var("WALK_REPS").map_or(1, |r| r.parse().unwrap());
        let boards: Vec<[Board; 2]> = set
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let internal = tables::to_internal(&tables::parse_scramble(l).unwrap());
                [internal, tables::reflect(&internal)]
            })
            .collect();
        let tables = crate::solver::size4x4::stm::pdb::Region::preload(
            std::path::Path::new("pdbs"),
            &LAYOUT,
        )
        .unwrap();
        let dbs = tables.as_dbs();
        let mut sum = 0u64;
        let start = std::time::Instant::now();
        for _ in 0..reps {
            for pair in &boards {
                for v in root_exact(dbs, pair) {
                    for x in [v.a17, v.a815, v.f17, v.f815, v.a19, v.f19] {
                        sum = sum.wrapping_mul(31).wrapping_add(u64::from(x));
                    }
                }
            }
        }
        let per = start.elapsed().as_secs_f64() / (reps * boards.len()) as f64;
        println!("{:.2} us per root, checksum {sum:016x}", per * 1e6);
    }

    /// Incremental indices and full 1-9 ranks match recomputing them from
    /// the moved board, for every move of 3000 random boards, both
    /// orientations.
    #[test]
    fn shifts_match_full_recompute() {
        for board in tables::random_boards(3000) {
            let boards = [board, tables::reflect(&board)];
            let list = &tables::MOVES[tables::blank_position(&board)][15];
            for m in &list.moves[..list.n as usize] {
                for (side, before) in boards.iter().enumerate() {
                    let next = m.next[side] as usize;
                    let tile = before[next];
                    let mut after = *before;
                    after[tables::blank_position(before)] = tile;
                    after[next] = 0;
                    let shifted = Side::root(before).shifted(before, m, side, tile);
                    let expected = Side::root(&after);
                    assert_eq!(
                        (
                            shifted.i17,
                            shifted.i815,
                            shifted.i19,
                            shifted.i1015,
                            shifted.rank
                        ),
                        (
                            expected.i17,
                            expected.i815,
                            expected.i19,
                            expected.i1015,
                            expected.rank
                        )
                    );
                    // 9-group indices stay in-table.
                    assert!(5 * (u64::from(shifted.i19) / 4) < SIZE_19);
                }
            }
        }
    }
}
