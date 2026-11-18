//! Exact values of a position's coded pattern-database entries.
//!
//! A coded entry holds `(value >> 1) & 1` or `(value >> 1) & 3` of its state's distance, and a
//! state's colour gives the distance's parity, so the entry gives the distance mod 4 or mod 8.
//! The search decodes its children against their parent's exact values, so at the root each
//! coded table is walked downhill to its goal: some move always leads to a lower distance, and
//! the codes tell which. The walk's length is the root's value.
//!
//! All of a root's walks advance in lockstep, each step's candidate entries prefetched before any
//! walk reads its own, so their cache misses overlap.

use std::hint::select_unpredictable;

use crate::solver::size4x4::stm::utils::{
    Board, NEIGHBOR_MASKS,
    index::{index_of_cells, placement_index, tile_mask},
    prefetch,
};

/// What a coded table holds, and so how a walk moves.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::solver::size4x4::stm) enum Encoding {
    /// `(value >> 1) & 1` of the additive distance: a group tile moves into the blank's region.
    AdditiveBit,
    /// `(value >> 1) & 1` of the full distance: the blank moves.
    FullBit,
    /// `(value >> 1) & 3` of the placement's relaxed minimum additive distance (which a move
    /// changes by 1 or 3): a group tile moves to any neighbouring cell without one.
    RelaxedMinimum,
}

/// A state of a walk: the placement's index, the blank's cell and the cells without a group tile.
#[derive(Clone, Copy, Default)]
pub(in crate::solver::size4x4::stm) struct WalkState {
    pub(in crate::solver::size4x4::stm) index: u32,
    pub(in crate::solver::size4x4::stm) blank: u32,
    pub(in crate::solver::size4x4::stm) free_cells: u16,
}

/// A walk to take: the coded table of `tiles` (which `encoding` describes) from `board`. The
/// [`index_shift_table`] the walks share holds the group in partition `partition`.
///
/// [`index_shift_table`]: super::index::index_shift_table
#[derive(Clone, Copy)]
pub(in crate::solver::size4x4::stm) struct WalkSpec<'a> {
    pub(in crate::solver::size4x4::stm) tiles: &'a [u8],
    pub(in crate::solver::size4x4::stm) partition: usize,
    pub(in crate::solver::size4x4::stm) board: &'a Board,
    pub(in crate::solver::size4x4::stm) encoding: Encoding,
}

/// `CLOSER_NEIGHBORS[tile][cell]`: the cells next to `cell` nearer to `tile`'s home (cell
/// `tile`).
const CLOSER_NEIGHBORS: [[u16; 16]; 16] = {
    // Odd rows run backwards.
    const fn row_column(cell: usize) -> (usize, usize) {
        let row = cell / 4;
        (row, if row % 2 == 1 { 3 - cell % 4 } else { cell % 4 })
    }
    const fn distance(a: usize, b: usize) -> usize {
        let ((row_a, column_a), (row_b, column_b)) = (row_column(a), row_column(b));
        row_a.abs_diff(row_b) + column_a.abs_diff(column_b)
    }
    let mut out = [[0; 16]; 16];
    let mut tile = 0;
    while tile < 16 {
        let mut cell = 0;
        while cell < 16 {
            let mut neighbor = 0;
            while neighbor < 16 {
                if (NEIGHBOR_MASKS[cell] >> neighbor) & 1 != 0
                    && distance(neighbor, tile) < distance(cell, tile)
                {
                    out[tile][cell] |= 1 << neighbor;
                }
                neighbor += 1;
            }
            cell += 1;
        }
        tile += 1;
    }
    out
};

/// The cells reachable from `blank` through `free_cells`, and the cells outside `free_cells`
/// next to them.
fn region_and_border(blank: u32, free_cells: u16) -> (u16, u16) {
    let mut region: u16 = 1 << blank;
    let mut seen = region;
    let mut frontier = region;
    while frontier != 0 {
        let mut grown = 0;
        while frontier != 0 {
            grown |= NEIGHBOR_MASKS[frontier.trailing_zeros() as usize];
            frontier &= frontier - 1;
        }
        seen |= grown;
        frontier = grown & free_cells & !region;
        region |= frontier;
    }
    (region, seen & !free_cells)
}

/// One move of a walk: the tile on `from` moves to `to`, leading to `next`.
#[derive(Clone, Copy, Default)]
struct Step {
    from: u8,
    to: u8,
    next: WalkState,
}

/// One downhill walk through a coded table.
struct Walk<'a> {
    shifts: &'a [[u64; 16]; 16],
    partition: usize,
    encoding: Encoding,
    /// The group's tiles on an otherwise empty board, padded so that an index shift can always
    /// read six cells past the lower of a move's.
    group_board: [u8; 24],
    state: WalkState,
    /// The goal placement's index.
    goal: u32,
    /// The value's parity at the start.
    parity: u32,
    /// The values are known mod `mask + 1`.
    mask: u32,
    /// The current value mod `mask + 1`.
    residue: u32,
    /// How far the walk has come down.
    value: u32,
    /// The cells a group tile may move to, and the group tiles next to them.
    targets: u16,
    border: u16,
    /// The moves listed from the current state: those bringing a tile closer to its home, then
    /// (once those all go up) the others.
    steps: [Step; 24],
    step_count: usize,
    closer_count: usize,
}

impl<'a> Walk<'a> {
    fn new(shifts: &'a [[u64; 16]; 16], spec: &WalkSpec<'_>) -> Self {
        let WalkSpec {
            tiles,
            partition,
            board,
            encoding,
        } = *spec;
        let group = tile_mask(tiles);
        let mut group_board = [0; 24];
        for (cell, &tile) in group_board.iter_mut().zip(board) {
            *cell = if (group >> tile) & 1 != 0 { tile } else { 0 };
        }
        let state = WalkState {
            index: placement_index(tiles, board),
            blank: board.iter().position(|&tile| tile == 0).unwrap() as u32,
            free_cells: (0..16)
                .filter(|&cell| group_board[cell] == 0)
                .fold(0, |mask, cell| mask | (1 << cell)),
        };
        // The value's parity: in the serpentine order a cell's colour is its parity, and an
        // additive move moves a group tile, a full one the blank.
        let parity = if encoding == Encoding::FullBit {
            state.blank & 1
        } else {
            (0..16)
                .filter(|&cell| group_board[cell as usize] != 0)
                .fold(0, |parity, cell| {
                    parity ^ ((cell ^ u32::from(group_board[cell as usize])) & 1)
                })
        };
        Self {
            shifts,
            partition,
            encoding,
            group_board,
            state,
            goal: index_of_cells(tiles),
            parity,
            mask: if encoding == Encoding::RelaxedMinimum {
                7
            } else {
                3
            },
            residue: 0,
            value: 0,
            targets: 0,
            border: 0,
            steps: [Step::default(); 24],
            step_count: 0,
            closer_count: 0,
        }
    }

    /// The state after the tile on `from` moves to `to`.
    #[inline(always)]
    fn after(&self, from: u32, to: u32) -> WalkState {
        let row = &self.shifts[usize::from(self.group_board[from as usize] & 15)];
        let lane = |entry: u64| (entry >> (32 * (self.partition & 1))) as u32;
        let lower = from.min(to) as usize;
        let gap = from.abs_diff(to) as usize;
        // The cells strictly between: at most six, read whatever the gap, so that the loop has
        // no branches to mispredict.
        let mut delta = lane(row[0]);
        for k in 1..7 {
            let passed = lane(row[usize::from(self.group_board[lower + k] & 15)]);
            delta = delta.wrapping_add(select_unpredictable(k < gap, passed, 0));
        }
        WalkState {
            index: if to > from {
                self.state.index.wrapping_add(delta)
            } else {
                self.state.index.wrapping_sub(delta)
            },
            blank: from,
            free_cells: self.state.free_cells ^ (1 << from) ^ (1 << to),
        }
    }

    /// How much lower than the current value a neighbour with code `code` is, if it is lower:
    /// a neighbour's value is 1 or 3 away, and those values have different codes.
    #[inline(always)]
    fn decrease(&self, code: u32) -> Option<u32> {
        let code_if_lower_by =
            |decrease: u32| (self.residue.wrapping_sub(decrease) & self.mask) >> 1;
        if code_if_lower_by(1) == code {
            Some(1)
        } else if self.encoding == Encoding::RelaxedMinimum && code_if_lower_by(3) == code {
            Some(3)
        } else {
            None
        }
    }

    /// Moves to `next` (by the tile on `from` moving to `to`), `decrease` lower.
    fn take(&mut self, from: u32, to: u32, next: WalkState, decrease: u32) {
        self.group_board[to as usize] = self.group_board[from as usize];
        self.group_board[from as usize] = 0;
        self.state = next;
        self.residue = self.residue.wrapping_sub(decrease) & self.mask;
        self.value += decrease;
        assert!(self.value < 256, "the table's codes lead downhill");
    }

    /// Lists the moves from the current state that bring a tile closer to its home (the likely
    /// ways down), prefetching their entries; false when the walk is at its goal. A full walk
    /// first takes every downhill move of the blank alone, whose entries are in the line already
    /// read.
    fn list(
        &mut self,
        code_of: impl Fn(WalkState) -> u32,
        address_of: impl Fn(WalkState) -> *const u8,
    ) -> bool {
        let state = self.state;
        (self.targets, self.border) = match self.encoding {
            Encoding::FullBit => {
                loop {
                    if self.state.index == self.goal && self.state.blank == 0 {
                        return false;
                    }
                    let mut cells =
                        NEIGHBOR_MASKS[self.state.blank as usize] & self.state.free_cells;
                    let mut descended = false;
                    while cells != 0 {
                        let cell = cells.trailing_zeros();
                        cells &= cells - 1;
                        let next = WalkState {
                            blank: cell,
                            ..self.state
                        };
                        if let Some(decrease) = self.decrease(code_of(next)) {
                            self.take(cell, cell, next, decrease);
                            descended = true;
                            break;
                        }
                    }
                    if !descended {
                        break;
                    }
                }
                let blank = self.state.blank as usize;
                (1 << blank, NEIGHBOR_MASKS[blank] & !self.state.free_cells)
            }
            Encoding::AdditiveBit => {
                let (region, border) = region_and_border(state.blank, state.free_cells);
                // Moves within the blank's region are free.
                if state.index == self.goal && region & 1 != 0 {
                    return false;
                }
                (region, border)
            }
            Encoding::RelaxedMinimum => {
                if state.index == self.goal {
                    return false;
                }
                (state.free_cells, !state.free_cells)
            }
        };
        self.step_count = 0;
        self.push_moves(true);
        self.closer_count = self.step_count;
        self.prefetch(0..self.closer_count, address_of);
        true
    }

    /// Lists the moves from the current state that do not bring a tile closer to its home,
    /// after the others, prefetching their entries.
    fn list_others(&mut self, address_of: impl Fn(WalkState) -> *const u8) {
        self.push_moves(false);
        self.prefetch(self.closer_count..self.step_count, address_of);
    }

    /// Lists the moves that bring a tile `closer` to its home, or those that do not.
    #[inline(always)]
    fn push_moves(&mut self, closer: bool) {
        let mut border = self.border;
        while border != 0 {
            let from = border.trailing_zeros() as usize;
            border &= border - 1;
            let nearer = CLOSER_NEIGHBORS[usize::from(self.group_board[from] & 15)][from];
            let mut cells =
                NEIGHBOR_MASKS[from] & self.targets & if closer { nearer } else { !nearer };
            while cells != 0 {
                let to = cells.trailing_zeros();
                cells &= cells - 1;
                self.steps[self.step_count] = Step {
                    from: from as u8,
                    to: to as u8,
                    next: WalkState::default(),
                };
                self.step_count += 1;
            }
        }
    }

    /// Works out where the listed moves `steps` lead, and prefetches their entries.
    fn prefetch(
        &mut self,
        steps: std::ops::Range<usize>,
        address_of: impl Fn(WalkState) -> *const u8,
    ) {
        for k in steps {
            let Step { from, to, .. } = self.steps[k];
            let next = self.after(u32::from(from), u32::from(to));
            self.steps[k].next = next;
            prefetch(address_of(next));
        }
    }

    /// Takes the first of the listed moves `steps` that goes down, if any.
    fn step(&mut self, steps: std::ops::Range<usize>, code_of: impl Fn(WalkState) -> u32) -> bool {
        for k in steps {
            let Step { from, to, next } = self.steps[k];
            if let Some(decrease) = self.decrease(code_of(next)) {
                self.take(u32::from(from), u32::from(to), next, decrease);
                return true;
            }
        }
        false
    }
}

/// The values at their boards of the coded tables `walks` describe, whose codes `code_of` reads
/// and `address_of` locates (for prefetching), both given the walk's position in `walks`.
/// `shifts` is an [`index_shift_table`] holding every walk's group.
///
/// # Panics
///
/// Panics if a table's codes lead nowhere, which only wrong tables do.
///
/// [`index_shift_table`]: super::index::index_shift_table
pub(in crate::solver::size4x4::stm) fn exact_values<const N: usize>(
    shifts: &[[u64; 16]; 16],
    walks: [WalkSpec<'_>; N],
    code_of: impl Fn(usize, WalkState) -> u32,
    address_of: impl Fn(usize, WalkState) -> *const u8,
) -> [u8; N] {
    let mut walks = walks.map(|spec| Walk::new(shifts, &spec));
    for (w, walk) in walks.iter().enumerate() {
        prefetch(address_of(w, walk.state));
    }
    for (w, walk) in walks.iter_mut().enumerate() {
        walk.residue = walk.parity | (code_of(w, walk.state) << 1);
    }
    // Each round lists every unfinished walk's moves and takes the first of each one's likely
    // moves that goes down; the walks whose likely moves all go up then try their others.
    let mut walking = [true; N];
    loop {
        for (w, walk) in walks.iter_mut().enumerate() {
            walking[w] =
                walking[w] && walk.list(|state| code_of(w, state), |state| address_of(w, state));
        }
        if !walking.contains(&true) {
            break;
        }
        let mut stuck = [false; N];
        for (w, walk) in walks.iter_mut().enumerate() {
            if walking[w] && !walk.step(0..walk.closer_count, |state| code_of(w, state)) {
                walk.list_others(|state| address_of(w, state));
                stuck[w] = true;
            }
        }
        for (w, walk) in walks.iter_mut().enumerate() {
            if stuck[w] {
                let range = walk.closer_count..walk.step_count;
                let stepped = walk.step(range, |state| code_of(w, state));
                assert!(stepped, "the table's codes lead downhill");
            }
        }
    }
    walks.map(|walk| walk.value as u8)
}
