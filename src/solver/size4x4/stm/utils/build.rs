//! Breadth-first searches over the placements of a group of tiles, for building pattern
//! databases.
//!
//! A state is a placement of the group's tiles and the blank's cell (one without a group tile).
//! Its additive distance counts only the moves of the group's tiles, the other tiles being
//! indistinguishable from the blank; its full distance counts every move. Each search keeps its
//! states' distances mod 3 in 2 bits (3 marks an unvisited state), so even a 9-tile group's 29
//! billion states fit in 7.3 GB, and reports every state once with its distance.

use std::{
    sync::atomic::{
        AtomicU8, AtomicU64, AtomicUsize,
        Ordering::{AcqRel, Relaxed},
    },
    thread,
};

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::utils::{
        NEIGHBOR_MASKS,
        index::{cells_of_index, index_of_cells, index_shift_table, placement_count},
    },
    statistics::PdbIterationStats,
};

/// Which moves a distance counts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::solver::size4x4::stm) enum Cost {
    /// Only moves of the group's tiles.
    Additive,
    /// Every move.
    Full,
}

/// The residue that marks an unvisited state.
const UNVISITED: u8 = 3;

/// `bytes` viewed as atomics, for writing from several threads.
pub(in crate::solver::size4x4::stm) fn as_atomic(bytes: &mut [u8]) -> &[AtomicU8] {
    // SAFETY: `AtomicU8` has the same layout as `u8`, and the borrow is exclusive.
    unsafe { std::slice::from_raw_parts(bytes.as_mut_ptr().cast::<AtomicU8>(), bytes.len()) }
}

/// The cells reachable from the cells of `seed` through the cells of `free_cells`.
#[must_use]
pub(super) fn reachable_region(seed: u16, free_cells: u16) -> u16 {
    let mut region = seed;
    loop {
        let mut grown = region;
        let mut cells = region;
        while cells != 0 {
            grown |= NEIGHBOR_MASKS[cells.trailing_zeros() as usize];
            cells &= cells - 1;
        }
        grown &= free_cells;
        if grown == region {
            return region;
        }
        region = grown;
    }
}

/// How many of the cells of `free_cells` are below `blank`: the blank's rank among them.
fn blank_rank(free_cells: u16, blank: u32) -> usize {
    (free_cells & ((1 << blank) - 1)).count_ones() as usize
}

/// One placement of a group's tiles.
struct Placement {
    /// Each group tile's label on its cell, 0 elsewhere.
    board: [u8; 16],
    /// The cells without a group tile.
    free_cells: u16,
    /// The placement's index.
    index: u32,
}

impl Placement {
    fn new(index: u32, tiles: &[u8]) -> Self {
        let cells = cells_of_index(index, tiles.len());
        let mut board = [0; 16];
        for (&cell, &tile) in cells.iter().zip(tiles) {
            board[usize::from(cell)] = tile;
        }
        let free_cells = (0..16)
            .filter(|&cell| board[cell] == 0)
            .fold(0, |mask, cell| mask | (1 << cell));
        Self {
            board,
            free_cells,
            index,
        }
    }

    /// The index after moving the tile on `from` to `to`, and the cells then free. `shifts` is
    /// the group's [`index_shift_table`].
    fn after_move(&self, shifts: &[[u64; 16]; 16], from: u32, to: u32) -> (u32, u16) {
        let row = &shifts[usize::from(self.board[from as usize])];
        let (lower, higher) = (from.min(to), from.max(to));
        let delta = (lower + 1..higher).fold(row[0] as u32, |sum, cell| {
            sum + row[usize::from(self.board[cell as usize])] as u32
        });
        let index = if to > from {
            self.index + delta
        } else {
            self.index - delta
        };
        (index, self.free_cells ^ (1 << from) ^ (1 << to))
    }
}

/// A search's states' distances mod 3, 2 bits each; [`UNVISITED`] marks an unvisited state.
struct Residues(Vec<u8>);

impl Residues {
    /// The byte holding `state`'s residue, as an atomic.
    fn byte(&self, state: usize) -> &AtomicU8 {
        // SAFETY: the bytes are only accessed through this, atomically.
        unsafe { &*(&raw const self.0[state >> 2]).cast::<AtomicU8>() }
    }

    fn get(&self, state: usize) -> u8 {
        (self.byte(state).load(Relaxed) >> (2 * (state & 3))) & 3
    }

    /// Marks unvisited `state` as at distance `residue` mod 3; false when it was visited.
    fn claim(&self, state: usize, residue: u8) -> bool {
        let shift = 2 * (state & 3);
        self.byte(state)
            .try_update(AcqRel, Relaxed, |byte| {
                ((byte >> shift) & 3 == UNVISITED)
                    .then_some(byte & !((UNVISITED ^ residue) << shift))
            })
            .is_ok()
    }
}

/// The fixed parameters of one search.
struct Search<'a, F> {
    /// The group's [`index_shift_table`].
    shifts: [[u64; 16]; 16],
    residues: Residues,
    /// How many cells have no group tile, so states per placement.
    free_count: usize,
    cost: Cost,
    visit: &'a F,
}

impl<F> Search<'_, F>
where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    /// The state of placement `index` with the blank on `blank`, given the free cells.
    fn state(&self, index: u32, free_cells: u16, blank: u32) -> usize {
        index as usize * self.free_count + blank_rank(free_cells, blank)
    }

    /// One placement's part of a layer: when pushing, finds the unvisited neighbours of its
    /// states at residue `current`; when pulling, finds its unvisited states with a neighbour
    /// there. Either way they are at distance `depth + 1`, so residue `next`. Returns how many
    /// states it found.
    fn expand_placement(
        &self,
        placement: &Placement,
        pulling: bool,
        current: u8,
        next: u8,
        depth: u8,
    ) -> u64 {
        let mut found = 0;
        let mut claim = |index: u32, free_cells: u16, blank: u32| {
            let rank = blank_rank(free_cells, blank);
            if self
                .residues
                .claim(index as usize * self.free_count + rank, next)
            {
                (self.visit)(index, blank, rank, depth + 1);
                found += 1;
            }
        };
        // The states this layer starts from: unvisited ones when pulling, else the layer's.
        let source = if pulling { UNVISITED } else { current };
        let (index, free_cells) = (placement.index, placement.free_cells);
        match self.cost {
            Cost::Full => {
                let mut blanks = free_cells;
                while blanks != 0 {
                    let blank = blanks.trailing_zeros();
                    blanks &= blanks - 1;
                    if self.residues.get(self.state(index, free_cells, blank)) != source {
                        continue;
                    }
                    let mut neighbors = NEIGHBOR_MASKS[blank as usize];
                    while neighbors != 0 {
                        let neighbor = neighbors.trailing_zeros();
                        neighbors &= neighbors - 1;
                        // The blank moves to `neighbor`, moving the tile there (if any) to
                        // `blank`.
                        let (next_index, next_free) = if placement.board[neighbor as usize] == 0 {
                            (index, free_cells)
                        } else {
                            placement.after_move(&self.shifts, neighbor, blank)
                        };
                        if !pulling {
                            claim(next_index, next_free, neighbor);
                        } else if self
                            .residues
                            .get(self.state(next_index, next_free, neighbor))
                            == current
                        {
                            claim(index, free_cells, blank);
                            break;
                        }
                    }
                }
            }
            Cost::Additive => {
                // The blank's region (its cells reachable without moving a group tile) shares
                // one distance, so each region is handled once, from its lowest cell.
                let mut remaining = free_cells;
                while remaining != 0 {
                    let region = reachable_region(remaining.isolate_lowest_one(), free_cells);
                    remaining &= !region;
                    let lowest = region.trailing_zeros();
                    if self.residues.get(self.state(index, free_cells, lowest)) != source {
                        continue;
                    }
                    // Moves of a group tile next to the region into it.
                    let mut movable = (0..16)
                        .filter(|&cell| NEIGHBOR_MASKS[cell] & region != 0)
                        .fold(0u16, |mask, cell| mask | (1 << cell))
                        & !free_cells;
                    'moves: while movable != 0 {
                        let from = movable.trailing_zeros();
                        movable &= movable - 1;
                        let mut targets = NEIGHBOR_MASKS[from as usize] & region;
                        while targets != 0 {
                            let to = targets.trailing_zeros();
                            targets &= targets - 1;
                            let (next_index, next_free) =
                                placement.after_move(&self.shifts, from, to);
                            if !pulling {
                                let mut cells = reachable_region(1 << from, next_free);
                                while cells != 0 {
                                    claim(next_index, next_free, cells.trailing_zeros());
                                    cells &= cells - 1;
                                }
                            } else if self.residues.get(self.state(next_index, next_free, from))
                                == current
                            {
                                let mut cells = region;
                                while cells != 0 {
                                    claim(index, free_cells, cells.trailing_zeros());
                                    cells &= cells - 1;
                                }
                                break 'moves;
                            }
                        }
                    }
                }
            }
        }
        found
    }
}

/// Searches the states of `tiles` (at most 9 of them, each home on the cell of its label) for
/// `cost`'s distances, calling `visit(placement, blank, rank, distance)` once for every state
/// (`rank`: how many cells below the blank's have no group tile), from several threads. Reports
/// each layer to `config`'s callback.
pub(in crate::solver::size4x4::stm) fn breadth_first_search<F>(
    tiles: &[u8],
    cost: Cost,
    config: &PdbConfig,
    visit: F,
) where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    let placements = placement_count(tiles.len());
    let free_count = 16 - tiles.len();
    let states = placements * free_count;
    let search = Search {
        shifts: index_shift_table([&[tiles], &[]]),
        residues: Residues(vec![u8::MAX; states.div_ceil(4)]),
        free_count,
        cost,
        visit: &visit,
    };
    let threads = thread::available_parallelism().map_or(1, usize::from);

    // The goal: every tile home, and the blank anywhere (additive) or home (full).
    let home = Placement::new(index_of_cells(tiles), tiles);
    let mut layer_sizes = vec![0u64];
    let mut blanks = home.free_cells;
    while blanks != 0 {
        let blank = blanks.trailing_zeros();
        blanks &= blanks - 1;
        if cost == Cost::Additive || blank == 0 {
            let rank = blank_rank(home.free_cells, blank);
            search
                .residues
                .claim(home.index as usize * free_count + rank, 0);
            visit(home.index, blank, rank, 0);
            layer_sizes[0] += 1;
        }
    }
    let report = |depth: usize, layer_sizes: &[u64]| {
        if let Some(callback) = &config.end_of_iter_callback {
            callback(PdbIterationStats {
                depth: depth as u64,
                new: layer_sizes[depth],
                total: layer_sizes.iter().sum(),
            });
        }
    };
    report(0, &layer_sizes);

    for depth in 0.. {
        let (current, next) = ((depth % 3) as u8, ((depth + 1) % 3) as u8);
        // Expanding the layer's states (and the earlier ones with the same residue) or checking
        // the unvisited ones, whichever is fewer.
        let push_work: u64 = layer_sizes.iter().rev().step_by(3).sum();
        let pull_work = states as u64 - layer_sizes.iter().sum::<u64>();
        let pulling = pull_work < push_work;
        let source = if pulling { UNVISITED } else { current };
        let found = AtomicU64::new(0);
        let chunk_len = placements.div_ceil(threads * 64).max(1);
        let next_chunk = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    let mut found_here = 0;
                    loop {
                        let start = next_chunk.fetch_add(chunk_len, Relaxed);
                        if start >= placements {
                            break;
                        }
                        for index in start..(start + chunk_len).min(placements) {
                            let first_state = index * free_count;
                            if !(first_state..first_state + free_count)
                                .any(|state| search.residues.get(state) == source)
                            {
                                continue;
                            }
                            let placement = Placement::new(index as u32, tiles);
                            found_here += search.expand_placement(
                                &placement,
                                pulling,
                                current,
                                next,
                                depth as u8,
                            );
                        }
                    }
                    found.fetch_add(found_here, Relaxed);
                });
            }
        });
        layer_sizes.push(found.into_inner());
        report(depth + 1, &layer_sizes);
        if layer_sizes[depth + 1] == 0 {
            return;
        }
    }
}

/// Lowers each placement's value in `minimums` (by steps of 2, keeping it admissible and its
/// parity) until moving one of `tiles` to a neighbouring cell changes it by at most 3.
pub(in crate::solver::size4x4::stm) fn relax_minimums(tiles: &[u8], minimums: &mut [u8]) {
    let shifts = index_shift_table([&[tiles], &[]]);
    let minimums = as_atomic(minimums);
    let threads = thread::available_parallelism().map_or(1, usize::from);
    let chunk_len = minimums.len().div_ceil(threads);
    // Values only fall, and a stale neighbour only lowers less, so racing passes are sound;
    // they repeat until one changes nothing.
    loop {
        let changed = thread::scope(|scope| {
            #[expect(
                clippy::needless_collect,
                reason = "every thread starts before any is joined"
            )]
            let handles: Vec<_> = (0..threads)
                .map(|thread| {
                    let shifts = &shifts;
                    scope.spawn(move || {
                        let mut changed = false;
                        let end = ((thread + 1) * chunk_len).min(minimums.len());
                        for index in thread * chunk_len..end {
                            let placement = Placement::new(index as u32, tiles);
                            let current = minimums[index].load(Relaxed);
                            let mut lowest = current;
                            let mut occupied = !placement.free_cells;
                            while occupied != 0 {
                                let from = occupied.trailing_zeros();
                                occupied &= occupied - 1;
                                let mut targets =
                                    NEIGHBOR_MASKS[from as usize] & placement.free_cells;
                                while targets != 0 {
                                    let to = targets.trailing_zeros();
                                    targets &= targets - 1;
                                    let (neighbor, _) = placement.after_move(shifts, from, to);
                                    let neighbor = minimums[neighbor as usize].load(Relaxed);
                                    lowest = lowest.min(neighbor + 3);
                                }
                            }
                            if lowest < current {
                                minimums[index].store(lowest, Relaxed);
                                changed = true;
                            }
                        }
                        changed
                    })
                })
                .collect();
            handles
                .into_iter()
                .fold(false, |changed, handle| handle.join().unwrap() | changed)
        });
        if !changed {
            return;
        }
    }
}
