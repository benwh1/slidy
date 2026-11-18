//! Breadth-first searches over the placements of a group of tiles, for building pattern
//! databases.
//!
//! A state is a placement of the group's tiles and the blank's cell (one without a group tile).
//! Its additive distance counts only the moves of the group's tiles, the other tiles being
//! indistinguishable from the blank; its full distance counts every move. Each search keeps 2
//! bits per state, whether it is unvisited, in the layer being expanded, in the next layer or
//! done, so even a 9-tile group's 29 billion states fit in 7.3 GB, and reports every state once
//! with its distance.

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

/// The mark of an unvisited state.
const UNVISITED: u8 = 3;

/// The mark of a state in a layer already expanded.
const DONE: u8 = 0;

/// The mark of the states at distance `depth` while they are a frontier: the layer being
/// expanded or the next one. The two alternate, and before a layer is expanded the previous one
/// is marked [`DONE`], so the next layer can take its mark.
const fn frontier_mark(depth: usize) -> u8 {
    1 + (depth & 1) as u8
}

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
    #[inline(always)]
    fn new(index: u32, tiles: &[u8]) -> Self {
        let cells = cells_of_index(index, tiles.len());
        let mut board = [0; 16];
        let mut free_cells = u16::MAX;
        for (&cell, &tile) in cells.iter().zip(tiles) {
            board[usize::from(cell)] = tile;
            free_cells &= !(1 << cell);
        }
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

/// The low bit of each 2-bit field of a word of marks.
const LOW_BITS: u64 = 0x5555_5555_5555_5555;

/// How many states' marks a word holds.
const MARKS_PER_WORD: usize = 32;

/// The low bits of the fields of `word` holding `mark`.
const fn fields_marked(word: u64, mark: u8) -> u64 {
    // A field is zero exactly where it holds `mark`.
    let difference = word ^ (mark as u64 * LOW_BITS);
    !(difference | (difference >> 1)) & LOW_BITS
}

/// A search's states' marks, 2 bits each ([`UNVISITED`], [`DONE`] or a [`frontier_mark`]), 32 to
/// a word; state `s` is field `s % 32` of word `s / 32`.
struct Marks(Vec<AtomicU64>);

impl Marks {
    /// Every one of `states` states unvisited.
    fn new(states: usize) -> Self {
        let words = states.div_ceil(MARKS_PER_WORD);
        Self(
            std::iter::repeat_with(|| AtomicU64::new(u64::MAX))
                .take(words)
                .collect(),
        )
    }

    /// Marks every state marked `mark` as [`DONE`], using `threads` threads.
    fn retire(&mut self, mark: u8, threads: usize) {
        let chunk_len = self.0.len().div_ceil(threads).max(1);
        thread::scope(|scope| {
            for chunk in self.0.chunks_mut(chunk_len) {
                scope.spawn(move || {
                    for word in chunk {
                        let word = word.get_mut();
                        let matching = fields_marked(*word, mark);
                        *word = (*word & !(matching * 3)) | (matching * u64::from(DONE));
                    }
                });
            }
        });
    }

    /// The word holding the marks of states `32 * index..32 * (index + 1)`.
    fn word(&self, index: usize) -> u64 {
        self.0[index].load(Relaxed)
    }

    fn get(&self, state: usize) -> u8 {
        ((self.word(state / MARKS_PER_WORD) >> (2 * (state % MARKS_PER_WORD))) & 3) as u8
    }

    /// Marks unvisited `state` with `mark`; false when it was visited.
    fn claim(&self, state: usize, mark: u8) -> bool {
        let shift = 2 * (state % MARKS_PER_WORD);
        self.0[state / MARKS_PER_WORD]
            .try_update(AcqRel, Relaxed, |word| {
                ((word >> shift) & 3 == u64::from(UNVISITED))
                    .then_some(word & !(u64::from(UNVISITED ^ mark) << shift))
            })
            .is_ok()
    }
}

/// The fixed parameters of one search.
struct Search<'a, F> {
    /// The group's [`index_shift_table`].
    shifts: [[u64; 16]; 16],
    marks: Marks,
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
    /// states marked `current` (the layer at distance `depth`); when pulling, finds its unvisited
    /// states with a neighbour there. Either way they are at distance `depth + 1`, and are marked
    /// `next`. Returns how many states it found.
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
                .marks
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
                    if self.marks.get(self.state(index, free_cells, blank)) != source {
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
                        } else if self.marks.get(self.state(next_index, next_free, neighbor))
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
                    if self.marks.get(self.state(index, free_cells, lowest)) != source {
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
                            } else if self.marks.get(self.state(next_index, next_free, from))
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
    let mut search = Search {
        shifts: index_shift_table([&[tiles], &[]]),
        marks: Marks::new(states),
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
                .marks
                .claim(home.index as usize * free_count + rank, frontier_mark(0));
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
        let (current, next) = (frontier_mark(depth), frontier_mark(depth + 1));
        if depth > 0 {
            // The previous layer is done, and its mark is the next layer's.
            search.marks.retire(next, threads);
        }
        let search = &search;
        // Expanding the layer's states or checking the unvisited ones, whichever is fewer.
        let push_work = layer_sizes[depth];
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
                        let end = (start + chunk_len).min(placements);
                        // Finds the chunk's placements with a state marked `source`, a word of
                        // marks at a time. No state gains the mark `source` during a layer (the
                        // frontier's are fixed, and claimed states lose `UNVISITED`), so a word
                        // without one can be skipped.
                        let end_state = end * free_count;
                        let mut state = start * free_count;
                        while state < end_state {
                            let word_index = state / MARKS_PER_WORD;
                            let word_end = (word_index + 1) * MARKS_PER_WORD;
                            let mut matching = fields_marked(search.marks.word(word_index), source)
                                & (u64::MAX << (2 * (state % MARKS_PER_WORD)));
                            if end_state < word_end {
                                matching &= (1 << (2 * (end_state % MARKS_PER_WORD))) - 1;
                            }
                            if matching == 0 {
                                state = word_end;
                                continue;
                            }
                            let matched = word_index * MARKS_PER_WORD
                                + matching.trailing_zeros() as usize / 2;
                            let index = matched / free_count;
                            let placement = Placement::new(index as u32, tiles);
                            found_here += search.expand_placement(
                                &placement,
                                pulling,
                                current,
                                next,
                                depth as u8,
                            );
                            state = (index + 1) * free_count;
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

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU16, Ordering::Relaxed},
        time::Instant,
    };

    use super::*;

    /// The xxh3 hash of every state's distance, as `breadth_first_search` reports it, laid out
    /// by state (`placement * free_count + rank`); checks each state is reported exactly once.
    fn distances_hash(tiles: &[u8], cost: Cost) -> u64 {
        let free_count = 16 - tiles.len();
        let mut distances = vec![u8::MAX; placement_count(tiles.len()) * free_count];
        let reports = AtomicU64::new(0);
        let atomic_distances = as_atomic(&mut distances);
        breadth_first_search(
            tiles,
            cost,
            &PdbConfig::default(),
            |index, _, rank, distance| {
                let state = index as usize * free_count + rank;
                assert_eq!(atomic_distances[state].swap(distance, Relaxed), u8::MAX);
                reports.fetch_add(1, Relaxed);
            },
        );
        assert_eq!(reports.into_inner(), distances.len() as u64);
        xxhash_rust::xxh3::xxh3_64(&distances)
    }

    /// Checks each case's distances against its known-answer hash (from the original search).
    fn check_known_answers(cases: &[(&[u8], Cost, u64)]) {
        let mut failures = Vec::new();
        for &(tiles, cost, expected) in cases {
            let hash = distances_hash(tiles, cost);
            if hash != expected {
                failures.push(format!("{tiles:?} {}: {hash:#018x}", cost_name(cost)));
            }
        }
        assert!(failures.is_empty(), "mismatched hashes: {failures:?}");
    }

    fn cost_name(cost: Cost) -> &'static str {
        match cost {
            Cost::Additive => "Additive",
            Cost::Full => "Full",
        }
    }

    #[test]
    fn six_tile_distances_match_known_answers() {
        check_known_answers(&[
            (&[1, 2, 3, 4, 5, 6], Cost::Additive, 0xa3d7_92a5_52c0_b480),
            (&[1, 2, 3, 4, 5, 6], Cost::Full, 0x87ba_0e77_c076_77c2),
            (
                &[10, 11, 12, 13, 14, 15],
                Cost::Additive,
                0xe364_0c2d_b91f_8db9,
            ),
            (&[10, 11, 12, 13, 14, 15], Cost::Full, 0xa4b9_81a1_acb0_e6bb),
        ]);
    }

    #[test]
    #[ignore = "about a minute"]
    fn seven_tile_distances_match_known_answers() {
        check_known_answers(&[
            (
                &[1, 2, 3, 4, 5, 6, 7],
                Cost::Additive,
                0xf0da_925d_bc7a_8326,
            ),
            (&[1, 2, 3, 4, 5, 6, 7], Cost::Full, 0x8e60_fc5a_8fb6_e1d9),
        ]);
    }

    /// Times the search for the groups and costs in `STM_BFS_BENCH` (e.g. `7a,7f,8f`: the first
    /// 7 or 8 tiles, additive or full), recording each state's distance as the huge database's
    /// 9-tile tables do: bit 1 of it, at the blank's rank in its placement's word for the cost.
    #[test]
    #[ignore = "a benchmark"]
    fn time_breadth_first_search() {
        let cases = std::env::var("STM_BFS_BENCH").unwrap_or_else(|_| "7a,7f".to_owned());
        for case in cases.split(',') {
            let (count, cost) = case.split_at(case.len() - 1);
            let tiles: Vec<u8> = (1..=count.parse().unwrap()).collect();
            let cost = if cost == "f" {
                Cost::Full
            } else {
                Cost::Additive
            };
            let cost_word = usize::from(cost == Cost::Full);
            let bits: Vec<AtomicU16> = std::iter::repeat_with(|| AtomicU16::new(0))
                .take(2 * placement_count(tiles.len()))
                .collect();
            let start = Instant::now();
            breadth_first_search(
                &tiles,
                cost,
                &PdbConfig::default(),
                |index, _, rank, distance| {
                    bits[2 * index as usize + cost_word]
                        .fetch_or(u16::from((distance >> 1) & 1) << rank, Relaxed);
                },
            );
            std::hint::black_box(&bits);
            println!(
                "BENCH {case} ({} tiles, {}): {:.2} s",
                tiles.len(),
                cost_name(cost),
                start.elapsed().as_secs_f64()
            );
        }
    }
}
