//! Exact values of the root's coded table entries.
//!
//! A bit table's entry holds only `(value >> 1) & 1` (one bit) or
//! `(value >> 1) & 3` (a 2-bit code): with the value's parity, the value
//! mod 4 or mod 8. The search needs exact values to decode its children
//! against, so at the root each table is walked downhill to its goal,
//! where the value is 0: from any other entry some move leads to a lower
//! entry (the first step of a shortest path in the table's abstract space;
//! for the additive tables the first move of a group tile, moves of other
//! tiles being free), and every listed move changes the value by an amount
//! its code determines (exactly 1, or 1 or 3 for a 2-bit code), so each
//! step decodes against the last. The walk's length is the root's value.
//! A table set lists its walks ([`Group`], [`WalkSpec`]); all of them, in
//! both orientations, advance in lockstep, so their misses overlap.

use std::{hint::select_unpredictable, marker::PhantomData};

use crate::solver::size4x4::stm::{
    pdb::{Dbs, decode_code, prefetch},
    shift::Partitions,
    tables::{self, Board, NEIGHBORS},
};

/// How a table's entry is read: its format.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Additive bit of a u32 word per placement, `2 * blank cell`.
    Additive,
    /// Full (every move counted) bit of the same word, `2 * blank cell + 1`.
    Full,
    /// Full bit in a mixed table's byte per placement ([`Dbs::mix_byte`]),
    /// at the blank's rank among the cells without a group tile.
    FullCompact,
    /// A mixed table's 2-bit code per placement ([`Dbs::mix_code`]), of a
    /// table minimising over the blank's regions: a tile move changes the
    /// value by 1 or 3, and the blank does not matter.
    Min2,
    /// Additive bit in a pair table's u16 per placement (`bitpack pair8`):
    /// the low byte, at the blank's rank among the cells without a group
    /// tile.
    PairAdditive,
    /// Full bit in the same u16's high byte.
    PairFull,
}

impl Kind {
    /// Whether the table is full (every move counted).
    const fn full(self) -> bool {
        matches!(self, Self::Full | Self::FullCompact | Self::PairFull)
    }
}

/// A tile group whose tables a set walks.
pub struct Group {
    /// The group's tiles, `lo..=hi`.
    pub tiles: (u8, u8),
    /// Bytes per placement in its tables.
    pub scale: u32,
    /// Which [`Partitions`] partition its index shifts belong to.
    pub partition: usize,
    /// The goal index (every tile home), scaled.
    goal: u32,
}

impl Group {
    /// Tiles `lo..=hi`, a group of one of `P`'s partitions.
    pub const fn of<P>((lo, hi): (u8, u8)) -> Self
    where
        P: Partitions,
    {
        // A partition's groups are `1..=SPLIT[p]` and `SPLIT[p] + 1..=15`.
        let second = (lo != 1) as usize;
        let split = if lo == 1 { hi } else { lo - 1 };
        let partition = (split != P::SPLIT[0]) as usize;
        let scale = P::SCALE[partition][second];
        Self {
            tiles: (lo, hi),
            scale,
            partition,
            goal: scale * tables::home_index(lo, hi) as u32,
        }
    }
}

/// One walk: a table of group `group` (an index into the set's groups),
/// read as `kind` at displacement `offset`.
pub struct WalkSpec {
    offset: isize,
    kind: Kind,
    group: usize,
}

impl WalkSpec {
    pub const fn new(offset: isize, kind: Kind, group: usize) -> Self {
        Self {
            offset,
            kind,
            group,
        }
    }
}

/// Neighbour cells of each cell, as a mask.
const NBR: [u16; 16] = {
    let mut out = [0u16; 16];
    let mut c = 0;
    while c < 16 {
        let mut d = 0;
        while d < 4 {
            let n = NEIGHBORS[c][d];
            if n >= 0 {
                out[c] |= 1 << n;
            }
            d += 1;
        }
        c += 1;
    }
    out
};

/// Row and column of an internal cell (odd rows run backwards).
const fn row_col(cell: usize) -> (usize, usize) {
    let row = cell / 4;
    (row, if row % 2 == 1 { 3 - cell % 4 } else { cell % 4 })
}

const fn distance(a: usize, b: usize) -> usize {
    let ((r0, c0), (r1, c1)) = (row_col(a), row_col(b));
    r0.abs_diff(r1) + c0.abs_diff(c1)
}

/// Bits of a byte in reverse order.
const REVERSE: [u8; 256] = {
    let mut out = [0u8; 256];
    let mut b = 0;
    while b < 256 {
        out[b] = (b as u8).reverse_bits();
        b += 1;
    }
    out
};

/// `STEP_TO[d][cell]`: the neighbour of `cell` in direction `d` (0: the
/// next cell in the row's order, 1: the previous one, 2: the row below, 3:
/// the row above), when there is one (a row's last cell and the next
/// row's first are neighbours too, but vertically: direction 2).
const STEP_TO: [[u8; 16]; 4] = {
    let mut out = [[0u8; 16]; 4];
    let mut c = 0;
    while c < 16 {
        let below = 8 * (c / 4) + 7;
        out[0][c] = (c + 1) as u8;
        out[1][c] = c.wrapping_sub(1) as u8;
        out[2][c] = below.wrapping_sub(c) as u8;
        out[3][c] = below.wrapping_sub(c + 8) as u8;
        c += 1;
    }
    out
};

/// Bits of a cell in all four direction lanes (lane `d` is bits
/// `16 * d..16 * d + 16`).
const LANES: u64 = 0x0001_0001_0001_0001;

/// `CLOSER_LANES[tile][cell]`: bit `16 * d + cell` for every direction `d`
/// that takes `tile` from `cell` nearer to its home.
const CLOSER_LANES: [[u64; 16]; 16] = {
    let mut out = [[0u64; 16]; 16];
    let mut tile = 0;
    while tile < 16 {
        let mut cell = 0;
        while cell < 16 {
            let mut d = 0;
            while d < 4 {
                let n = STEP_TO[d][cell] as usize;
                let in_row = match d {
                    0 => cell % 4 != 3,
                    1 => cell % 4 != 0,
                    _ => true,
                };
                if in_row && n < 16 && distance(n, tile) < distance(cell, tile) {
                    out[tile][cell] |= 1 << (16 * d + cell);
                }
                d += 1;
            }
            cell += 1;
        }
        tile += 1;
    }
    out
};

/// Lane `d`: the cells whose neighbour in direction `d` is in `cells`.
#[inline(always)]
fn toward(cells: u16) -> u64 {
    let byte = |m: u16| u16::from(REVERSE[usize::from(m & 0xFF)]);
    // A byte's reversal maps each cell of its upper row to the one above
    // (low nibble) and each of its lower row to the one below (high).
    let (r0, r1, r2) = (byte(cells), byte(cells >> 4), byte(cells >> 8));
    let next = (cells >> 1) & 0x7777;
    let previous = (cells << 1) & 0xEEEE;
    let above = (r0 & 0x0F) | ((r1 & 0x0F) << 4) | ((r2 & 0x0F) << 8);
    let below = (r0 & 0xF0) | ((r1 & 0xF0) << 4) | ((r2 & 0xF0) << 8);
    u64::from(next)
        | (u64::from(previous) << 16)
        | (u64::from(above) << 32)
        | (u64::from(below) << 48)
}

/// The cells in `cells` and their neighbours: the union of the
/// [`toward`] lanes (adjacency is symmetric).
#[inline(always)]
fn spread(cells: u16) -> u16 {
    let lanes = toward(cells);
    cells | (lanes | (lanes >> 16) | (lanes >> 32) | (lanes >> 48)) as u16
}

/// Cells reachable from `seed` through `free` cells.
fn flood(seed: u16, free: u16) -> u16 {
    let mut region = seed;
    loop {
        let next = spread(region) & free;
        if next == region {
            return region;
        }
        region = next;
    }
}

/// One move of a walk: the entry it leads to and how the group changes.
#[derive(Clone, Copy, Default)]
struct Step {
    index: u32,
    /// Cell the moved tile leaves (where the blank goes) and enters.
    from: u8,
    to: u8,
}

/// One downhill walk through a bit table, from a group's placement (and
/// blank) at the root to the table's goal.
struct Walk<'t, P> {
    dbs: Dbs<'t>,
    offset: isize,
    kind: Kind,
    /// The [`Partitions`] partition the group's index shifts belong to.
    partition: usize,
    /// The group's tiles on an otherwise empty board, padded so an index
    /// shift can always scan six cells.
    board: [u8; 24],
    /// Cells without a group tile.
    free: u16,
    blank: u8,
    /// The blank's region (additive tables), refreshed each step.
    region: u16,
    /// The group's moves toward home, as [`CLOSER_LANES`] bits.
    closer: u64,
    index: u32,
    goal: u32,
    /// The starting value plus an unknown multiple of 4.
    start: u32,
    /// The current value on the same offset.
    value: u32,
    steps: [Step; 32],
    count: usize,
    /// The starting value's parity.
    parity: u8,
    partitions: PhantomData<P>,
}

/// A group's tiles at the root, shared by the walks on its table.
#[derive(Clone, Copy)]
struct Placement {
    /// The group's tiles on an otherwise empty board (see [`Walk`]).
    board: [u8; 24],
    free: u16,
    /// Parity of the additive value: of the tiles' colour distances.
    parity: u8,
    closer: u64,
    index: u32,
    goal: u32,
}

impl Placement {
    fn new(position: &Board, group: &Group) -> Self {
        let (lo, hi) = group.tiles;
        let mut board = [0; 24];
        let mut free = 0xFFFF;
        let mut parity = 0;
        let mut closer = 0;
        for (cell, &tile) in position.iter().enumerate() {
            if (lo..hi + 1).contains(&tile) {
                board[cell] = tile;
                free &= !(1 << cell);
                closer |= CLOSER_LANES[tile as usize][cell];
                // A tile move flips the tile's checkerboard colour, which in
                // the serpentine order is the cell's parity.
                parity ^= (cell as u8 ^ tile) & 1;
            }
        }
        let mut placed = [0; 16];
        placed.copy_from_slice(&board[..16]);
        Self {
            board,
            free,
            parity,
            closer,
            index: group.scale * tables::lehmer_index(&placed, lo, hi) as u32,
            goal: group.goal,
        }
    }
}

impl<'t, P: Partitions> Walk<'t, P> {
    /// A walk from `placed` (with the blank at `blank`), its start entry
    /// prefetched: [`Self::anchor`] reads it once every walk is set up,
    /// so the starting misses overlap.
    fn new(
        dbs: Dbs<'t>,
        offset: isize,
        kind: Kind,
        partition: usize,
        placed: &Placement,
        blank: u8,
    ) -> Self {
        // Every move flips the blank's colour; its home is cell 0.
        let parity = if kind.full() {
            blank & 1
        } else {
            placed.parity
        };
        let walk = Self {
            dbs,
            offset,
            kind,
            partition,
            board: placed.board,
            free: placed.free,
            blank,
            region: 0,
            closer: placed.closer,
            index: placed.index,
            goal: placed.goal,
            start: 0,
            value: 0,
            steps: [Step::default(); 32],
            count: 0,
            parity,
            partitions: PhantomData,
        };
        prefetch(walk.entry(kind, walk.index));
        walk
    }

    /// The address of placement `index`'s entry.
    #[inline(always)]
    fn entry(&self, kind: Kind, index: u32) -> *const u8 {
        match kind {
            Kind::Min2 | Kind::FullCompact => self.dbs.mix_entry(self.offset, index, 0),
            Kind::PairAdditive | Kind::PairFull => self.dbs.pair_entry(self.offset, index),
            Kind::Additive | Kind::Full => self.dbs.entry(self.offset, index),
        }
    }

    /// Read the starting entry: the value mod 4, offset by 256 to keep the
    /// (falling) walk value far from zero.
    fn anchor(&mut self) {
        let kind = self.kind;
        self.start =
            256 + 2 * self.bit(kind, self.index, self.blank, self.free) + u32::from(self.parity);
        if kind == Kind::Min2 {
            // A two-bit code: the value mod 8.
            self.start += 256;
        }
        self.value = self.start;
    }

    /// The entry bit for the blank at `blank` with the group placed as
    /// `index` (cells without a group tile: `free`).
    #[inline(always)]
    fn bit(&self, kind: Kind, index: u32, blank: u8, free: u16) -> u32 {
        match kind {
            Kind::Additive => (self.dbs.word(self.offset, index) >> (2 * blank)) & 1,
            Kind::Full => (self.dbs.word(self.offset, index) >> (2 * blank + 1)) & 1,
            Kind::FullCompact => {
                let rank = (free & ((1u16 << blank) - 1)).count_ones();
                (self.dbs.mix_byte(self.offset, index) >> rank) & 1
            }
            Kind::Min2 => self.dbs.mix_code(self.offset, index),
            Kind::PairAdditive | Kind::PairFull => {
                let rank = (free & ((1u16 << blank) - 1)).count_ones();
                let byte = if kind == Kind::PairFull { 8 } else { 0 };
                (self.dbs.pair(self.offset, index) >> (rank + byte)) & 1
            }
        }
    }

    /// Whether the walk has reached its goal (refreshing the blank's
    /// region on the way).
    fn done(&mut self) -> bool {
        if self.kind.full() {
            return self.index == self.goal && self.blank == 0;
        }
        if self.kind == Kind::Min2 {
            // Any free cell counts as the blank's.
            self.region = self.free;
            return self.index == self.goal;
        }
        self.region = flood(1 << self.blank, self.free);
        // Additive values are equal across a blank region.
        self.index == self.goal && self.region & 1 != 0
    }

    /// Index after moving the tile at `from` into `to`: the search's
    /// incremental shift ([`crate::solver::size4x4::stm::shift::move_deltas`]), without its
    /// per-gap branches, which a walk cannot predict.
    #[inline(always)]
    fn shifted(&self, from: usize, to: usize) -> u32 {
        // Cells and tiles are below 16 (the masks only drop bounds checks).
        let (from, to) = (from & 15, to & 15);
        let tile = usize::from(self.board[from] & 15);
        let lo = from.min(to);
        let gap = from.abs_diff(to);
        let row = &P::SHIFT.contrib[tile];
        let mut acc = P::SHIFT.wbits[tile];
        for k in 1..7 {
            let other = row[usize::from(self.board[lo + k] & 15)];
            acc = acc.wrapping_add(select_unpredictable(k < gap, other, 0));
        }
        // Partition 0's shift rides in the low half, partition 1's in the
        // high.
        let part = if self.partition == 1 { acc >> 32 } else { acc } as u32;
        let delta = if to < from { part.wrapping_neg() } else { part };
        self.index.wrapping_add(delta)
    }

    #[inline(always)]
    fn push(&mut self, kind: Kind, from: usize, to: usize) {
        let index = self.shifted(from, to);
        prefetch(self.entry(kind, index));
        // A move crosses one of the board's 24 edges, so there are fewer
        // than 32 (the mask only drops the bounds check).
        self.steps[self.count & 31] = Step {
            index,
            from: from as u8,
            to: to as u8,
        };
        self.count += 1;
    }

    /// List the moves from here that bring a group tile `closer` to home
    /// (the likely ways down) or the others, prefetching their entries.
    /// A full table's blank-only moves count as closer.
    fn list(&mut self, closer: bool) {
        // One copy of the body per kind, each with its reads fixed.
        match self.kind {
            Kind::Additive => self.list_as(Kind::Additive, closer),
            Kind::Full => self.list_as(Kind::Full, closer),
            Kind::FullCompact => self.list_as(Kind::FullCompact, closer),
            Kind::Min2 => self.list_as(Kind::Min2, closer),
            Kind::PairAdditive => self.list_as(Kind::PairAdditive, closer),
            Kind::PairFull => self.list_as(Kind::PairFull, closer),
        }
    }

    #[inline(always)]
    fn list_as(&mut self, kind: Kind, closer: bool) {
        self.count = 0;
        // The cells a group tile may step into: the blank's for a full
        // table, the blank's region for an additive one (moves within it
        // are free), every free cell for the 1-9 minimum.
        let into = if kind.full() {
            let blank = if closer {
                self.descend(kind)
            } else {
                self.blank as usize
            };
            if self.index == self.goal && blank == 0 {
                return;
            }
            1 << blank
        } else {
            self.region
        };
        let toward_home = if closer { self.closer } else { !self.closer };
        let mut moves = toward(into) & (u64::from(!self.free) * LANES) & toward_home;
        while moves != 0 {
            let bit = moves.trailing_zeros() as usize;
            moves &= moves - 1;
            let (d, from) = (bit >> 4, bit & 15);
            self.push(kind, from, usize::from(STEP_TO[d][from]));
        }
    }

    /// A full table's bits for the current placement, one per cell (bit
    /// `c`: the blank at `c`).
    #[inline(always)]
    fn cell_bits(&self, kind: Kind) -> u16 {
        if kind == Kind::Full {
            // Gather the word's odd bits.
            let mut x = (self.dbs.word(self.offset, self.index) >> 1) & 0x5555_5555;
            x = (x | (x >> 1)) & 0x3333_3333;
            x = (x | (x >> 2)) & 0x0F0F_0F0F;
            x = (x | (x >> 4)) & 0x00FF_00FF;
            return (x | (x >> 8)) as u16;
        }
        // A compact byte: bit `r` belongs to the `r`-th free cell.
        let byte = if kind == Kind::PairFull {
            self.dbs.pair(self.offset, self.index) >> 8
        } else {
            self.dbs.mix_byte(self.offset, self.index)
        };
        let mut free = self.free;
        let mut out = 0;
        let mut r = 0;
        while free != 0 {
            let cell = free.trailing_zeros();
            free &= free.wrapping_sub(1);
            out |= (((byte >> r) & 1) as u16) << (cell & 15);
            r += 1;
        }
        out
    }

    /// Full tables: take every downhill move that carries no group tile
    /// (the placement stays, so its bits are in the line already loaded:
    /// no index shift, no new line), stopping at the goal; returns the
    /// blank.
    #[inline(always)]
    fn descend(&mut self, kind: Kind) -> usize {
        let bits = self.cell_bits(kind);
        let mut blank = self.blank as usize;
        while !(self.index == self.goal && blank == 0) {
            // A neighbour is 1 lower when its bit differs from `value + 1`'s.
            let up = 0u16.wrapping_sub((((self.value + 1) >> 1) & 1) as u16);
            let down = NBR[blank] & self.free & (bits ^ up);
            if down == 0 {
                break;
            }
            blank = down.trailing_zeros() as usize;
            self.value -= 1;
        }
        self.blank = blank as u8;
        blank
    }

    /// Take the first listed move down; false when none of them is (true
    /// when a full walk's descent already reached the goal).
    fn step(&mut self) -> bool {
        match self.kind {
            Kind::Additive => self.step_as(Kind::Additive),
            Kind::Full => self.step_as(Kind::Full),
            Kind::FullCompact => self.step_as(Kind::FullCompact),
            Kind::Min2 => self.step_as(Kind::Min2),
            Kind::PairAdditive => self.step_as(Kind::PairAdditive),
            Kind::PairFull => self.step_as(Kind::PairFull),
        }
    }

    #[inline(always)]
    fn step_as(&mut self, kind: Kind) -> bool {
        if self.count == 0 && kind.full() && self.index == self.goal && self.blank == 0 {
            return true;
        }
        let up = ((self.value + 1) >> 1) & 1;
        let (b, mid) = (self.value.wrapping_sub(3), u32::midpoint(self.value, 5));
        for k in 0..self.count {
            let s = self.steps[k & 31];
            let (from, to) = (usize::from(s.from & 15), usize::from(s.to & 15));
            let free = self.free ^ (1 << from) ^ (1 << to);
            // The blank ends where the tile was.
            let bit = self.bit(kind, s.index, from as u8, free);
            let value = if kind == Kind::Min2 {
                decode_code(b, mid, bit)
            } else {
                self.value + 1 - 2 * (bit ^ up)
            };
            if value < self.value {
                let tile = self.board[from];
                self.board[to] = tile;
                self.board[from] = 0;
                self.closer =
                    (self.closer & !(LANES << from)) | CLOSER_LANES[usize::from(tile & 15)][to];
                self.free = free;
                self.blank = from as u8;
                self.index = s.index;
                self.value = value;
                self.count = 0;
                return true;
            }
        }
        self.count = 0;
        false
    }

    /// The root value, once the walk is done.
    fn result(&self) -> u8 {
        debug_assert_eq!(self.value % 4, 0, "walk ended off the goal value");
        (self.start - self.value) as u8
    }
}

/// Exact values at the root of the tables `walks` (on the tile groups
/// `groups`), from walks on the position and its mirror image (`boards`):
/// `[direct, mirrored][walk]`.
#[inline(always)]
pub fn root_values<P, const G: usize, const W: usize>(
    dbs: Dbs<'_>,
    groups: &[Group; G],
    walks: &[WalkSpec; W],
    boards: &[Board; 2],
) -> [[u8; W]; 2]
where
    P: Partitions,
{
    let start = |p: &Board| {
        let blank = tables::blank_position(p) as u8;
        // One placement per group, shared by the walks on its tables.
        let placed: [Placement; G] = std::array::from_fn(|g| Placement::new(p, &groups[g]));
        std::array::from_fn(|w| {
            let spec = &walks[w];
            let partition = groups[spec.group].partition;
            Walk::<P>::new(
                dbs,
                spec.offset,
                spec.kind,
                partition,
                &placed[spec.group],
                blank,
            )
        })
    };
    let mut all: [[Walk<'_, P>; W]; 2] = boards.each_ref().map(start);
    for walk in all.iter_mut().flatten() {
        walk.anchor();
    }
    // Per round: list the likely moves of every unfinished walk, step;
    // then list the other moves of the walks those did not take down,
    // step again (which always succeeds).
    let mut active = [[true; W]; 2];
    loop {
        let mut walking = false;
        for (side, on) in all.iter_mut().zip(&mut active) {
            for (walk, on) in side.iter_mut().zip(on) {
                *on = *on && !walk.done();
                if *on {
                    walk.list(true);
                    walking = true;
                }
            }
        }
        if !walking {
            break;
        }
        let mut stuck = [[false; W]; 2];
        let mut any = false;
        for ((side, on), st) in all.iter_mut().zip(&active).zip(&mut stuck) {
            for ((walk, &on), st) in side.iter_mut().zip(on).zip(st) {
                if on && !walk.step() {
                    walk.list(false);
                    *st = true;
                    any = true;
                }
            }
        }
        if any {
            for (side, st) in all.iter_mut().zip(&stuck) {
                for (walk, &st) in side.iter_mut().zip(st) {
                    if st {
                        let stepped = walk.step();
                        assert!(
                            stepped,
                            "every entry but the goal's has a neighbour 1 lower"
                        );
                    }
                }
            }
        }
    }
    all.map(|side| side.map(|walk| walk.result()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::size4x4::stm::max7896::Max7896;

    /// The walk's branch-free shift matches recomputing the index, for
    /// every group, scale and kind of move.
    #[test]
    fn shift_matches_full_recompute() {
        // The groups the walks of the 7-8 and 9-6 tables use.
        let groups = [(1, 7), (8, 15), (1, 9)].map(Group::of::<Max7896>);
        // The table is never read: only the index arithmetic is tested.
        let region = [0u8; 64];
        let dbs = Dbs::for_test(region.as_ptr());
        for board in tables::random_boards(2000) {
            for group in &groups {
                let ((lo, hi), scale) = (group.tiles, group.scale);
                let placed = Placement::new(&board, group);
                let blank = tables::blank_position(&board) as u8;
                let walk =
                    Walk::<Max7896>::new(dbs, 0, Kind::Additive, group.partition, &placed, blank);
                for from in 0..16 {
                    if walk.free & (1 << from) != 0 {
                        continue;
                    }
                    let mut cells = NBR[from] & walk.free;
                    while cells != 0 {
                        let to = cells.trailing_zeros() as usize;
                        cells &= cells - 1;
                        let mut moved = [0u8; 16];
                        moved.copy_from_slice(&walk.board[..16]);
                        moved[to] = moved[from];
                        moved[from] = 0;
                        let expected = scale * tables::lehmer_index(&moved, lo, hi) as u32;
                        assert_eq!(walk.shifted(from, to), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn spread_matches_the_neighbour_table() {
        for (cell, &nbr) in NBR.iter().enumerate() {
            assert_eq!(spread(1 << cell), nbr | (1 << cell), "cell {cell}");
        }
    }
}
