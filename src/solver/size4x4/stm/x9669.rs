//! The max(9-6, 6-9) table set: two partitions, each a 9-tile group with
//! exact-blank additive and full bits in one pair table and a 6-tile
//! group in plain bytes, 17.0 GB, [`TableSet`] `X9669`.
//!
//! Takes the elementwise maximum, in both orientations, of the additive
//! 9-6 sum (tiles 1-9 and 10-15), the additive 6-9 sum (1-6 and 7-15)
//! and the full (blank-counting) 1-9, 10-15, 1-6 and 7-15 values (lab,
//! 2k: 1.78 M nodes against `max7896`'s 2.75 M), reading four table
//! lines per child and orientation like `max7896`. The 9-tile tables hold
//! one bit per entry, `(value >> 1) & 1`, which with the position's parity
//! decodes exactly against the parent's value (exact tables are
//! consistent); the search carries those values ([`Exact`]).
//!
//! - 1-9 (`p19pair`) and 7-15 (`p715pair`): `bitpack pair8` of `bigbuild`'s additive and full
//!   compact tables, a little-endian u16 per placement, additive bits low and full bits high, bit
//!   `r` for the blank at the `r`-th cell not holding a group tile.
//! - 10-15 (`p1015w16`) and 1-6 (`p16w16`): plain values, a little-endian u16 per placement and
//!   blank cell, indexed `placement * 16 + cell`, additive low byte, full high byte (`genbuild
//!   --exact16`, both modes, interleaved).
//!
//! No combined file is needed: the solver reads the parts into place.

use std::hint::select_unpredictable;

use crate::solver::size4x4::stm::{
    large::TableSet,
    pdb::{Dbs, Layout, prefetch},
    shift::{Partitions, move_deltas},
    tables::{self, Board, MIRROR, Move},
    walk::{self, Group, Kind, WalkSpec},
};

/// The max(9-6, 6-9) set ([module docs](self)).
pub struct X9669;

/// Table sizes in bytes.
const SIZE_19: u64 = 2 * 4_151_347_200;
const SIZE_715: u64 = SIZE_19;
const SIZE_6: u64 = 32 * 5_765_760;

const LAYOUT: Layout = Layout {
    file: "combined-x9669",
    parts: &[
        ("p1015w16", SIZE_6),
        ("p16w16", SIZE_6),
        ("p19pair", SIZE_19),
        ("p715pair", SIZE_715),
    ],
};

const OFF_1015: isize = LAYOUT.offset(0);
const OFF_16: isize = LAYOUT.offset(1);
const OFF_19: isize = LAYOUT.offset(2);
const OFF_715: isize = LAYOUT.offset(3);

// The two 8.3 GB tables cannot both start within +-2 GiB of one base, so
// unlike `max7896` not every offset fits an x86 displacement: the far ones
// are 64-bit constants the compiler keeps in registers.

impl Partitions for X9669 {
    const SPLIT: [u8; 2] = [9, 6];
    /// The 6-tile tables hold 16 u16 entries per placement (x32); the
    /// 9-tile indices count placements (a u16 each, beyond u32 byte
    /// offsets).
    const SCALE: [[u32; 2]; 2] = [[1, 32], [32, 1]];
}

/// The groups and tables the root walks: the additive and full 1-9 and
/// 7-15 values, in [`Exact`]'s order.
const GROUPS: [Group; 2] = [Group::of::<X9669>((1, 9)), Group::of::<X9669>((7, 15))];
const WALKS: [WalkSpec; 4] = [
    WalkSpec::new(OFF_19, Kind::PairAdditive, 0),
    WalkSpec::new(OFF_19, Kind::PairFull, 0),
    WalkSpec::new(OFF_715, Kind::PairAdditive, 1),
    WalkSpec::new(OFF_715, Kind::PairFull, 1),
];

/// One orientation's table state: the four groups' indices, scaled, and
/// the blank's rank among the cells without a tile 1-9 (`r19`) and
/// without a tile 7-15 (`r715`), which pick its bits in the pair tables.
#[derive(Clone, Copy)]
pub struct Side {
    i19: u32,
    i1015: u32,
    i16: u32,
    i715: u32,
    r19: u8,
    r715: u8,
}

/// Both orientations' [`Side`]s, stored per field (no padding per side).
#[derive(Clone, Copy)]
pub struct Indices {
    i19: [u32; 2],
    i1015: [u32; 2],
    i16: [u32; 2],
    i715: [u32; 2],
    r19: [u8; 2],
    r715: [u8; 2],
}

impl Indices {
    #[inline(always)]
    fn side(&self, s: usize) -> Side {
        Side {
            i19: self.i19[s],
            i1015: self.i1015[s],
            i16: self.i16[s],
            i715: self.i715[s],
            r19: self.r19[s],
            r715: self.r715[s],
        }
    }

    #[inline(always)]
    fn new([d, m]: [Side; 2]) -> Self {
        Self {
            i19: [d.i19, m.i19],
            i1015: [d.i1015, m.i1015],
            i16: [d.i16, m.i16],
            i715: [d.i715, m.i715],
            r19: [d.r19, m.r19],
            r715: [d.r715, m.r715],
        }
    }
}

impl Side {
    fn root(board: &Board) -> Self {
        let blank = tables::blank_position(board);
        let before =
            |keep: fn(u8) -> bool| board[..blank].iter().filter(|&&t| keep(t)).count() as u8;
        Self {
            i19: tables::lehmer_index(board, 1, 9) as u32,
            i1015: 32 * tables::lehmer_index(board, 10, 15) as u32,
            i16: 32 * tables::lehmer_index(board, 1, 6) as u32,
            i715: tables::lehmer_index(board, 7, 15) as u32,
            r19: before(|t| t >= 10),
            r715: before(|t| t <= 6),
        }
    }

    /// The state after move `m` (orientation `side`) on `board`, which
    /// moves `tile` into the blank.
    #[inline(always)]
    fn shifted(self, board: &Board, m: &Move, side: usize, tile: u8) -> Self {
        let (lo, gap, up) = (m.lo[side] as usize, m.gap[side] as usize, m.sign[side]);
        let delta = move_deltas::<X9669>(board, lo, gap, up != 0, tile as usize);
        // Deltas are signed; wrapping adds compute the mod-2^32 sums.
        let [[d19, d1015], [d16, d715]] = delta.shift;
        // The blank passes the moved tile's cell and the cells between: the
        // tiles among them outside a 9-tile group change the blank's rank.
        let passed16 = (gap as u8 - 1) - delta.passed[1];
        let shifted = Self {
            i19: self.i19.wrapping_add(d19 as u32),
            i1015: self.i1015.wrapping_add(d1015 as u32),
            i16: self.i16.wrapping_add(d16 as u32),
            i715: self.i715.wrapping_add(d715 as u32),
            r19: child_rank(self.r19, up, tile >= 10, delta.passed[0]),
            r715: child_rank(self.r715, up, tile <= 6, passed16),
        };
        debug_assert!(2 * u64::from(shifted.i19) < SIZE_19);
        debug_assert!(2 * u64::from(shifted.i715) < SIZE_715);
        debug_assert!(u64::from(shifted.i1015) < SIZE_6);
        debug_assert!(u64::from(shifted.i16) < SIZE_6);
        shifted
    }

    /// A 6-tile group's values, (additive, full), straight from its table.
    #[inline(always)]
    fn values6(dbs: Dbs<'_>, offset: isize, index: u32, blank: u8) -> (u32, u32) {
        let pair = dbs.half(offset, index + 2 * u32::from(blank));
        (pair & 0xFF, pair >> 8)
    }

    /// A child's estimates in one partition: (additive sum, full 6-tile,
    /// full 9-tile) and its 9-tile value. Both 9-tile bits sit at the
    /// blank's rank (additive low byte, full high); the additive value
    /// changes only when one of the group's tiles moves (`moved`), the full
    /// one on every move.
    #[inline(always)]
    #[allow(clippy::too_many_arguments, reason = "one partition's inputs")]
    fn decode(
        dbs: Dbs<'_>,
        [offset9, offset6]: [isize; 2],
        [index9, index6]: [u32; 2],
        rank: u8,
        blank: u8,
        moved: bool,
        d: &Half,
    ) -> ([u32; 3], u32) {
        let y = (dbs.pair(offset9, index9) >> rank) ^ d.m;
        let a = select_unpredictable(moved, d.a + 1 - ((y << 1) & 2), d.a);
        let (a6, f6) = Self::values6(dbs, offset6, index6, blank);
        ([a + a6, f6, d.f - ((y >> 7) & 2)], a)
    }

    #[inline(always)]
    fn decode96(self, dbs: Dbs<'_>, blank: u8, tile: u8, d: &Decoder) -> ([u32; 3], u32) {
        let (offsets, indices) = ([OFF_19, OFF_1015], [self.i19, self.i1015]);
        Self::decode(dbs, offsets, indices, self.r19, blank, tile <= 9, &d.h19)
    }

    #[inline(always)]
    fn decode69(self, dbs: Dbs<'_>, blank: u8, tile: u8, d: &Decoder) -> ([u32; 3], u32) {
        let (offsets, indices) = ([OFF_715, OFF_16], [self.i715, self.i16]);
        Self::decode(dbs, offsets, indices, self.r715, blank, tile >= 7, &d.h715)
    }
}

/// The child's blank rank from the parent's `rank`, when the blank goes
/// `up` (to a higher cell) or down past `passed` cells outside the group
/// plus the moved tile's (`outside` when the moved tile is not one of the
/// group's): the only cells that change sides.
#[inline(always)]
fn child_rank(rank: u8, up: u8, outside: bool, passed: u8) -> u8 {
    let passed = u8::from(outside) + passed;
    select_unpredictable(up != 0, rank + passed, rank.wrapping_sub(passed))
}

/// Exact values of the pair tables for one orientation: the additive and
/// full 1-9 and 7-15 values, in [`WALKS`] order.
#[derive(Clone, Copy)]
pub struct Exact {
    a19: u8,
    f19: u8,
    a715: u8,
    f715: u8,
}

impl Exact {
    /// Admissible estimate from these values and the 6-tile groups' ones.
    fn bound(self, (a1015, f1015): (u32, u32), (a16, f16): (u32, u32)) -> u64 {
        let [a19, f19, a715, f715] = [self.a19, self.f19, self.a715, self.f715].map(u32::from);
        u64::from(
            (a19 + a1015)
                .max(f19)
                .max(f1015)
                .max(a715 + a16)
                .max(f715)
                .max(f16),
        )
    }
}

/// A child's decoded 9-tile values in one orientation: the same fields
/// as [`Exact`], which they become.
pub type Decoded = Exact;

/// `((value + 1) >> 1) & 1`: the entry bit of `value + 1`. A move changes
/// a value by exactly 1 and an entry holds `(v >> 1) & 1`, so a child's
/// entry bit matches this exactly when its value is `value + 1`.
const fn up(value: u8) -> u32 {
    ((value as u32 + 1) >> 1) & 1
}

/// One 9-tile group's values set up for decoding a child's pair entry.
#[derive(Clone, Copy)]
pub struct Half {
    /// Additive value (kept when no group tile moves).
    a: u32,
    /// Full value + 1.
    f: u32,
    /// Mask for the u16: the entry bits of `value + 1` (additive bit 0,
    /// full bit 8); a child's bit that differs went down.
    m: u32,
}

impl Half {
    fn new(a: u8, f: u8) -> Self {
        Self {
            a: u32::from(a),
            f: u32::from(f) + 1,
            m: up(a) | (up(f) << 8),
        }
    }
}

/// One node's values, one orientation, set up for decoding.
#[derive(Clone, Copy)]
pub struct Decoder {
    h19: Half,
    h715: Half,
}

/// Exact values of the pair tables at the root, both orientations.
fn root_exact(dbs: Dbs<'_>, boards: &[Board; 2]) -> [Exact; 2] {
    walk::root_values::<X9669, 2, 4>(dbs, &GROUPS, &WALKS, boards).map(|[a19, f19, a715, f715]| {
        Exact {
            a19,
            f19,
            a715,
            f715,
        }
    })
}

impl TableSet for X9669 {
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
            .map(|s| {
                let v1015 = Side::values6(dbs, OFF_1015, sides[s].i1015, blanks[s]);
                let v16 = Side::values6(dbs, OFF_16, sides[s].i16, blanks[s]);
                values[s].bound(v1015, v16)
            })
            .max()
            .unwrap_or(0);
        (Indices::new(sides), values, distance)
    }

    #[inline(always)]
    fn shifted(base: Indices, boards: &[Board; 2], m: &Move, tiles: [u8; 2]) -> Indices {
        // (Two calls, not a closure: the closure was not inlined, and its
        // results came back through memory.)
        Indices::new([
            base.side(0).shifted(&boards[0], m, 0, tiles[0]),
            base.side(1).shifted(&boards[1], m, 1, tiles[1]),
        ])
    }

    #[inline(always)]
    fn prefetch(dbs: Dbs<'_>, c: &Indices, blanks: [u8; 2]) {
        let (direct, mirrored) = (c.side(0), c.side(1));
        // In the order evaluate reads them.
        prefetch(dbs.pair_entry(OFF_19, direct.i19));
        prefetch(dbs.entry(OFF_1015, direct.i1015 + 2 * u32::from(blanks[0])));
        prefetch(dbs.pair_entry(OFF_19, mirrored.i19));
        prefetch(dbs.entry(OFF_1015, mirrored.i1015 + 2 * u32::from(blanks[1])));
        prefetch(dbs.pair_entry(OFF_715, direct.i715));
        prefetch(dbs.entry(OFF_16, direct.i16 + 2 * u32::from(blanks[0])));
        prefetch(dbs.pair_entry(OFF_715, mirrored.i715));
        prefetch(dbs.entry(OFF_16, mirrored.i16 + 2 * u32::from(blanks[1])));
    }

    #[inline(always)]
    fn decoder(values: [Exact; 2]) -> [Decoder; 2] {
        let decoder = |v: Exact| Decoder {
            h19: Half::new(v.a19, v.f19),
            h715: Half::new(v.a715, v.f715),
        };
        [decoder(values[0]), decoder(values[1])]
    }

    /// Short-circuit loads in prune order (9-6 both orientations, then
    /// 6-9), decoding the child's entries against the node's values. The
    /// key is the max in the high bits and the sum of all the estimates
    /// below, as in `max7896`.
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
        let e96 = |s: usize| c.side(s).decode96(dbs, blanks[s], tiles[s], &decoder[s]);
        let e69 = |s: usize| c.side(s).decode69(dbs, blanks[s], tiles[s], &decoder[s]);
        let (e96d, a19d) = e96(0);
        if !within(e96d) {
            return None;
        }
        let (e96m, a19m) = e96(1);
        if !within(e96m) {
            return None;
        }
        let (e69d, a715d) = e69(0);
        if !within(e69d) {
            return None;
        }
        let (e69m, a715m) = e69(1);
        if !within(e69m) {
            return None;
        }
        // The max fills the high bits; the sum of all twelve estimates
        // (at most 12 * 80) fits below.
        let [[h96d, f1015d, f19d], [h96m, f1015m, f19m]] = [e96d, e96m];
        let [[h69d, f16d, f715d], [h69m, f16m, f715m]] = [e69d, e69m];
        let best = h96d
            .max(f1015d.max(f19d))
            .max(h96m.max(f1015m.max(f19m)))
            .max(h69d.max(f16d).max(h69m.max(f16m)))
            .max(f715d.max(f715m));
        let key = (best << 10)
            + (h96d + h96m + h69d + h69m)
            + (f1015d + f1015m + f16d + f16m)
            + (f19d + f19m + f715d + f715m);
        let decoded = |a19: u32, f19: u32, a715: u32, f715: u32| Decoded {
            a19: a19 as u8,
            f19: f19 as u8,
            a715: a715 as u8,
            f715: f715 as u8,
        };
        Some((
            u64::from(key),
            [
                decoded(a19d, f19d, a715d, f715d),
                decoded(a19m, f19m, a715m, f715m),
            ],
        ))
    }

    #[inline(always)]
    fn child(_values: [Exact; 2], decoded: [Decoded; 2], _tiles: [u8; 2]) -> [Exact; 2] {
        // Each partition has one coded group, so the decoded values are
        // the child's.
        decoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Incremental indices and both ranks match recomputing them from the
    /// moved board, for every move of 3000 random boards, both
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
                    let s = Side::root(before).shifted(before, m, side, tile);
                    let e = Side::root(&after);
                    assert_eq!(
                        (s.i19, s.i1015, s.i16, s.i715, s.r19, s.r715),
                        (e.i19, e.i1015, e.i16, e.i715, e.r19, e.r715)
                    );
                }
            }
        }
    }
}
