//! Exact values of a position's coded pattern-database entries.
//!
//! A coded entry holds `(value >> 1) & 1` or `(value >> 1) & 3` of its state's distance, and a
//! state's colour gives the distance's parity, so the entry gives the distance mod 4 or mod 8.
//! The search decodes its children against their parent's exact values, so at the root each
//! coded table is walked downhill to its goal: some move always leads to a lower distance, and
//! the codes tell which. The walk's length is the root's value.

use crate::solver::size4x4::stm::utils::{
    Board, NEIGHBOR_MASKS,
    build::reachable_region,
    index::{index_of_cells, placement_index, tile_mask},
    prefetch,
};

/// What a coded table holds, and so how a walk moves.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// `(value >> 1) & 1` of the additive distance: a group tile moves into the blank's region.
    AdditiveBit,
    /// `(value >> 1) & 1` of the full distance: the blank moves.
    FullBit,
    /// `(value >> 1) & 3` of the placement's relaxed minimum additive distance (which a move
    /// changes by 1 or 3): a group tile moves to any neighbouring cell without one.
    RelaxedMinimum,
}

/// A state of a walk: the placement's index, the blank's cell and the cells without a group tile.
#[derive(Clone, Copy)]
pub struct WalkState {
    pub index: u32,
    pub blank: u32,
    pub free_cells: u16,
}

/// The value at `board` of the coded table of `tiles` (which `encoding` describes), whose codes
/// `code_of` reads (and `address_of` locates, for prefetching). `shifts` is an
/// [`index_shift_table`] that holds the group in partition `partition`.
///
/// # Panics
///
/// Panics if the table's codes lead nowhere, which only wrong tables do.
///
/// [`index_shift_table`]: super::index::index_shift_table
pub fn exact_value(
    tiles: &[u8],
    shifts: &[[u64; 16]; 16],
    partition: usize,
    board: &Board,
    encoding: Encoding,
    code_of: impl Fn(WalkState) -> u32,
    address_of: impl Fn(WalkState) -> *const u8,
) -> u8 {
    let group = tile_mask(tiles);
    let mut group_board: Board = board.map(|tile| if (group >> tile) & 1 != 0 { tile } else { 0 });
    let mut state = WalkState {
        index: placement_index(tiles, board),
        blank: board.iter().position(|&tile| tile == 0).unwrap() as u32,
        free_cells: (0..16)
            .filter(|&cell| group_board[cell] == 0)
            .fold(0, |mask, cell| mask | (1 << cell)),
    };
    let goal = index_of_cells(tiles);
    // The value's parity: in the serpentine order a cell's colour is its parity, and an additive
    // move moves a group tile, a full one the blank.
    let parity = if encoding == Encoding::FullBit {
        state.blank & 1
    } else {
        tiles.iter().fold(0, |parity, &tile| {
            let cell = board
                .iter()
                .position(|&cell_tile| cell_tile == tile)
                .unwrap() as u32;
            parity ^ ((cell ^ u32::from(tile)) & 1)
        })
    };
    let modulus = if encoding == Encoding::RelaxedMinimum {
        8
    } else {
        4
    };
    // The value mod `modulus`.
    let mut residue = parity | (code_of(state) << 1);
    let mut value = 0u32;
    while state.index != goal || (encoding == Encoding::FullBit && state.blank != 0) {
        // The moves: (from, to), the tile on `from` moving to `to`.
        let mut moves = [(0u32, 0u32); 48];
        let mut move_count = 0;
        let mut push = |from: u32, to: u32| {
            moves[move_count] = (from, to);
            move_count += 1;
        };
        match encoding {
            Encoding::FullBit => {
                let mut neighbors = NEIGHBOR_MASKS[state.blank as usize];
                while neighbors != 0 {
                    push(neighbors.trailing_zeros(), state.blank);
                    neighbors &= neighbors - 1;
                }
            }
            Encoding::AdditiveBit | Encoding::RelaxedMinimum => {
                let reachable = if encoding == Encoding::AdditiveBit {
                    reachable_region(1 << state.blank, state.free_cells)
                } else {
                    state.free_cells
                };
                let mut occupied = !state.free_cells;
                while occupied != 0 {
                    let from = occupied.trailing_zeros();
                    occupied &= occupied - 1;
                    let mut targets = NEIGHBOR_MASKS[from as usize] & reachable;
                    while targets != 0 {
                        push(from, targets.trailing_zeros());
                        targets &= targets - 1;
                    }
                }
            }
        }
        let after = |(from, to): (u32, u32)| {
            let tile = group_board[from as usize];
            if tile == 0 {
                // The blank moves to a cell without a group tile.
                return WalkState {
                    blank: from,
                    ..state
                };
            }
            let row = &shifts[usize::from(tile)];
            let lane = |entry: u64| (entry >> (32 * partition)) as u32;
            let (lower, higher) = (from.min(to), from.max(to));
            let delta = (lower + 1..higher).fold(lane(row[0]), |sum, cell| {
                sum + lane(row[usize::from(group_board[cell as usize])])
            });
            WalkState {
                index: if to > from {
                    state.index + delta
                } else {
                    state.index - delta
                },
                blank: from,
                free_cells: state.free_cells ^ (1 << from) ^ (1 << to),
            }
        };
        for &step in &moves[..move_count] {
            prefetch(address_of(after(step)));
        }
        // A neighbour's value is 1 or 3 away, and those values have different codes; the lower
        // ones are 1 and (for `RelaxedMinimum`) 3 less.
        let code_if_lower_by = |decrease: u32| ((residue + modulus - decrease) % modulus) >> 1;
        let descent = moves[..move_count].iter().find_map(|&step| {
            let neighbor = after(step);
            let code = code_of(neighbor);
            [1, 3]
                .into_iter()
                .take(if encoding == Encoding::RelaxedMinimum {
                    2
                } else {
                    1
                })
                .find(|&decrease| code_if_lower_by(decrease) == code)
                .map(|decrease| (step, neighbor, decrease))
        });
        let ((from, to), neighbor, decrease) = descent.expect("the table's codes lead downhill");
        group_board[to as usize] = group_board[from as usize];
        group_board[from as usize] = 0;
        state = neighbor;
        residue = (residue + modulus - decrease) % modulus;
        value += decrease;
        assert!(value < 256, "the table's codes lead downhill");
    }
    value as u8
}
