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
        index::{cells_of_index, index_of_cells, index_shift_table, placement_count, tile_mask},
        neighbors, prefetch,
    },
    statistics::PdbIterationStats,
};

/// Which moves a distance counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// `len` zeroed words, allocated zeroed, so that a page is only written when it is first used.
fn zeroed_words(len: usize) -> Box<[AtomicU64]> {
    let layout = std::alloc::Layout::array::<AtomicU64>(len).expect("too many words");
    if layout.size() == 0 {
        return Box::new([]);
    }
    // SAFETY: the layout is not empty. Zeroed memory is a valid `AtomicU64`, and the allocation
    // has the layout of `[AtomicU64]` of length `len`, as the box expects.
    unsafe {
        let words = std::alloc::alloc_zeroed(layout).cast::<AtomicU64>();
        if words.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(std::ptr::slice_from_raw_parts_mut(words, len))
    }
}

/// The cells reachable from the cells of `seed` through the cells of `free_cells`.
#[must_use]
pub(super) fn reachable_region(seed: u16, free_cells: u16) -> u16 {
    let mut region = seed;
    loop {
        let grown = (region | neighbors(region)) & free_cells;
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

    // The goal: every tile home, and the blank home (full) or in its home's region (additive,
    // where moves within a region are free). The group's tiles must free the blank's home before
    // the puzzle is solved, so an additive goal that walls it in is not reached yet.
    let home = Placement::new(index_of_cells(tiles), tiles);
    let mut layer_sizes = vec![0u64];
    let mut blanks = home.free_cells;
    let goal_blanks = if cost == Cost::Additive {
        reachable_region(1, home.free_cells)
    } else {
        1
    };
    while blanks != 0 {
        let blank = blanks.trailing_zeros();
        blanks &= blanks - 1;
        if (goal_blanks >> blank) & 1 != 0 {
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

/// Each free cell's region (its cells reachable through free cells) for every set of free cells
/// of one size, labelled by the region's rank (counting the regions in the order of their lowest
/// cells, from 0), a nibble per cell; cells that are not free are labelled with the number of
/// regions, which no rank is.
struct RegionLabels(Vec<u64>);

impl RegionLabels {
    /// A nibble's lowest bit, in every nibble.
    const NIBBLES: u64 = 0x1111_1111_1111_1111;

    /// The labels of every set of `free_count` free cells.
    fn new(free_count: usize) -> Self {
        let mut labels = vec![0; 1 << 16];
        for free_cells in 0..=u16::MAX {
            if free_cells.count_ones() as usize != free_count {
                continue;
            }
            let (mut cell_labels, mut rank, mut unlabelled) = (u64::MAX, 0, free_cells);
            while unlabelled != 0 {
                let region = reachable_region(unlabelled.isolate_lowest_one(), free_cells);
                unlabelled &= !region;
                for cell in 0..16 {
                    if (region >> cell) & 1 != 0 {
                        cell_labels &= !(15 << (4 * cell));
                        cell_labels |= rank << (4 * cell);
                    }
                }
                rank += 1;
            }
            for cell in 0..16 {
                if (free_cells >> cell) & 1 == 0 {
                    cell_labels &= !(15 << (4 * cell));
                    cell_labels |= rank << (4 * cell);
                }
            }
            labels[usize::from(free_cells)] = cell_labels;
        }
        Self(labels)
    }

    /// How many regions `free_cells` (not every cell) make.
    #[inline(always)]
    fn count(&self, free_cells: u16) -> u32 {
        self.rank(free_cells, (!free_cells).trailing_zeros())
    }

    /// The rank of free cell `cell`'s region.
    #[inline(always)]
    fn rank(&self, free_cells: u16, cell: u32) -> u32 {
        ((self.0[usize::from(free_cells)] >> (4 * cell)) & 15) as u32
    }

    /// The region of rank `rank`.
    #[inline(always)]
    fn region(&self, free_cells: u16, rank: u32) -> u16 {
        // A nibble is zero exactly where the label is `rank`.
        let difference = self.0[usize::from(free_cells)] ^ (u64::from(rank) * Self::NIBBLES);
        let mut matching =
            !(difference | (difference >> 1) | (difference >> 2) | (difference >> 3))
                & Self::NIBBLES;
        // Each nibble's bit to one bit of the result, a pair, then four, then eight at a time.
        matching = (matching | (matching >> 3)) & 0x0303_0303_0303_0303;
        matching = (matching | (matching >> 6)) & 0x000f_000f_000f_000f;
        matching = (matching | (matching >> 12)) & 0x0000_00ff_0000_00ff;
        ((matching | (matching >> 24)) & 0xffff) as u16
    }
}

/// How [`relaxed_minimums`] names a group's placements: a key for each (some keys may name none),
/// and the key that moving one of its tiles leads to.
trait PlacementKeys: Sync {
    /// What the moves from a placement need to know about it.
    type Placement;

    /// One more than the largest key.
    fn len(&self) -> usize;

    /// The key of the placement with the group's tiles on `cells`.
    fn key_of(&self, cells: &[u8]) -> u32;

    /// Placement `key`, and its free cells.
    fn placement(&self, key: u32) -> (Self::Placement, u16);

    /// The key after moving the tile on `from` of `placement` (with key `key`) to `to`.
    fn after_move(&self, placement: &Self::Placement, key: u32, from: u32, to: u32) -> u32;

    /// The minimums (one less than `values`, one per key) in placement index order, one per
    /// placement.
    fn minimums(&self, values: Vec<u8>) -> Vec<u8>;
}

/// Keys that are placement indices.
struct IndexKeys<'a> {
    tiles: &'a [u8],
    /// The group's [`index_shift_table`].
    shifts: [[u64; 16]; 16],
}

impl<'a> IndexKeys<'a> {
    fn new(tiles: &'a [u8]) -> Self {
        Self {
            tiles,
            shifts: index_shift_table([&[tiles], &[]]),
        }
    }
}

impl PlacementKeys for IndexKeys<'_> {
    type Placement = Placement;

    fn len(&self) -> usize {
        placement_count(self.tiles.len())
    }

    fn key_of(&self, cells: &[u8]) -> u32 {
        index_of_cells(cells)
    }

    #[inline(always)]
    fn placement(&self, key: u32) -> (Placement, u16) {
        let placement = Placement::new(key, self.tiles);
        let free_cells = placement.free_cells;
        (placement, free_cells)
    }

    #[inline(always)]
    fn after_move(&self, placement: &Placement, _: u32, from: u32, to: u32) -> u32 {
        placement.after_move(&self.shifts, from, to).0
    }

    fn minimums(&self, mut values: Vec<u8>) -> Vec<u8> {
        for value in &mut values {
            *value = minimum(*value);
        }
        values
    }
}

/// Keys of a nibble per tile holding its cell, tile 0's the highest. A move only changes one
/// nibble, which is cheaper than an index's digits, but some keys (with two tiles on a cell) name
/// no placement, so the keys outnumber the placements by up to 16⁶ to 5,765,760 (6 tiles).
struct NibbleKeys {
    tile_count: usize,
}

impl PlacementKeys for NibbleKeys {
    /// Each tile's nibble's shift, on its cell.
    type Placement = [u8; 16];

    fn len(&self) -> usize {
        1 << (4 * self.tile_count)
    }

    fn key_of(&self, cells: &[u8]) -> u32 {
        cells
            .iter()
            .fold(0, |key, &cell| 16 * key + u32::from(cell))
    }

    #[inline(always)]
    fn placement(&self, key: u32) -> ([u8; 16], u16) {
        let (mut shifts, mut free_cells) = ([0; 16], u16::MAX);
        for shift in (0..4 * self.tile_count).step_by(4) {
            let cell = (key >> shift) & 15;
            shifts[cell as usize] = shift as u8;
            free_cells &= !(1 << cell);
        }
        (shifts, free_cells)
    }

    #[inline(always)]
    fn after_move(&self, placement: &[u8; 16], key: u32, from: u32, to: u32) -> u32 {
        let shift = placement[from as usize];
        key - (from << shift) + (to << shift)
    }

    fn minimums(&self, values: Vec<u8>) -> Vec<u8> {
        in_index_order(self.tile_count, |cells| {
            minimum(values[self.key_of(cells) as usize])
        })
    }
}

/// How many regions [`additive_values`] expands before checking their moves.
const MOVE_BATCH_REGIONS: usize = 16;

/// Each key's least additive distance over the blank's cells, plus 1 (0 for keys naming no
/// placement): what [`breadth_first_search`] finds for [`Cost::Additive`], minimised over each
/// placement's states, but searching the blank's regions (its cells reachable without moving a
/// group tile, which share a distance) instead of every cell. Reports each layer (counting every
/// cell of its regions, as [`breadth_first_search`] does) to `config`'s callback.
///
/// Also gives the keys with a region more than 2 further than their value: the only placements
/// that moving a tile from can lower another's value in [`relax`] (a move into a region leads at
/// most 1 further), so they are all it needs to start from.
fn additive_values<K>(keys: &K, tiles: &[u8], config: &PdbConfig) -> (Vec<u8>, Vec<u32>)
where
    K: PlacementKeys,
{
    let labels = RegionLabels::new(16 - tiles.len());
    let threads = thread::available_parallelism().map_or(1, usize::from);
    let mut values = vec![0; keys.len()];
    let atomic_values = as_atomic(&mut values);
    // A region is `16 * key + its rank`, with a bit set once it is reached. Nearly every
    // placement has at most 4 regions: their bits come first, 4 per key, so that the search's
    // lookups mostly stay in the cache, then 16 per key for the rest.
    let bits = zeroed_words((20 * keys.len()).div_ceil(64));
    let first_bits = 4 * keys.len();
    // A region's bit: its word and the bit's mask.
    let bit_of = |region: u32| {
        let position = if region % 16 < 4 {
            4 * (region as usize / 16) + region as usize % 16
        } else {
            first_bits + region as usize
        };
        (&bits[position / 64], 1u64 << (position % 64))
    };
    let reach = |region: u32| {
        let (word, bit) = bit_of(region);
        word.load(Relaxed) & bit == 0 && word.fetch_or(bit, Relaxed) & bit == 0
    };
    // Reaches `region`, if it is new, giving whether its key's other regions are all unreached
    // when that is known without reading its value: when they all have bits in its word. Read
    // before setting the bit, that misses only other regions of the key reached in the same
    // layer, whose values are the same.
    let reach_first = |region: u32, free_cells: u16| {
        let (word, bit) = bit_of(region);
        let old = word.load(Relaxed);
        if old & bit != 0 || word.fetch_or(bit, Relaxed) & bit != 0 {
            return None;
        }
        let key_bits = 15 << (4 * (region / 16 % 16));
        Some((labels.count(free_cells) <= 4).then_some(old & key_bits == 0))
    };

    // The goal: every tile home, and the blank in its home's region, which holds cell 0 (so has
    // rank 0).
    let home = keys.key_of(tiles);
    let mut frontier = vec![16 * home];
    let mut far = Vec::new();
    reach(16 * home);
    atomic_values[home as usize].store(1, Relaxed);
    let home_region = reachable_region(1, !tile_mask(tiles));
    let mut layer_sizes = vec![u64::from(home_region.count_ones())];
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
    // Only the callback needs the layers' sizes in cells.
    let counting = config.end_of_iter_callback.is_some();

    for depth in 0u8.. {
        let next_chunk = AtomicUsize::new(0);
        let chunk_len = frontier.len().div_ceil(threads * 64).max(1);
        // Small layers are not worth starting every thread for.
        let layer_threads = frontier.len().div_ceil(1 << 14).min(threads);
        let (layer, reach_first, bit_of, labels) = (&frontier, &reach_first, &bit_of, &labels);
        let layers: Vec<(Vec<u32>, u64, Vec<u32>)> = thread::scope(|scope| {
            // Every thread starts before any is joined.
            let handles: Vec<_> = std::iter::repeat_with(|| {
                scope.spawn(|| {
                    let (mut next, mut found, mut far) = (Vec::new(), 0, Vec::new());
                    // A batch of moves' regions, with their free cells, whose bits are
                    // fetched before any is checked.
                    let mut moves: Vec<(u32, u16)> = Vec::with_capacity(1024);
                    // The new regions whose keys' values must be read, fetched first too.
                    let mut unsure: Vec<u32> = Vec::with_capacity(1024);
                    loop {
                        let start = next_chunk.fetch_add(chunk_len, Relaxed);
                        if start >= layer.len() {
                            break;
                        }
                        let end = (start + chunk_len).min(layer.len());
                        for regions in layer[start..end].chunks(MOVE_BATCH_REGIONS) {
                            moves.clear();
                            for &region in regions {
                                let key = region / 16;
                                let (placement, free_cells) = keys.placement(key);
                                let region = labels.region(free_cells, region % 16);
                                // The group tiles next to the region, each movable into it.
                                let mut movable = neighbors(region) & !free_cells;
                                while movable != 0 {
                                    let from = movable.trailing_zeros();
                                    movable &= movable - 1;
                                    let mut targets = NEIGHBOR_MASKS[from as usize] & region;
                                    while targets != 0 {
                                        let to = targets.trailing_zeros();
                                        targets &= targets - 1;
                                        let next_key = keys.after_move(&placement, key, from, to);
                                        let next_free = free_cells ^ (1 << from) ^ (1 << to);
                                        let id = 16 * next_key + labels.rank(next_free, from);
                                        prefetch(bit_of(id).0.as_ptr().cast());
                                        moves.push((id, next_free));
                                    }
                                }
                            }
                            unsure.clear();
                            for &(id, next_free) in &moves {
                                let Some(first) = reach_first(id, next_free) else {
                                    continue;
                                };
                                next.push(id);
                                if counting {
                                    let next_region = labels.region(next_free, id % 16);
                                    found += u64::from(next_region.count_ones());
                                }
                                // A key's first region reached is its nearest.
                                let value = &atomic_values[id as usize / 16];
                                if first == Some(true) {
                                    value.store(depth + 2, Relaxed);
                                } else {
                                    prefetch(value.as_ptr().cast());
                                    unsure.push(id / 16);
                                }
                            }
                            for &key in &unsure {
                                let value = &atomic_values[key as usize];
                                let old = value.load(Relaxed);
                                if old == 0 {
                                    value.store(depth + 2, Relaxed);
                                } else if depth > old {
                                    far.push(key);
                                }
                            }
                        }
                    }
                    (next, found, far)
                })
            })
            .take(layer_threads)
            .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect()
        });
        let mut next = Vec::with_capacity(layers.iter().map(|layer| layer.0.len()).sum());
        let mut found = 0;
        for (layer_next, layer_found, layer_far) in layers {
            next.extend(layer_next);
            found += layer_found;
            far.extend(layer_far);
        }
        layer_sizes.push(found);
        report(usize::from(depth) + 1, &layer_sizes);
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    (values, far)
}

/// Lowers each placement's value in `values` (by steps of 2, keeping it admissible and its
/// parity) until moving one of its tiles to a neighbouring cell changes it by at most 3: to the
/// least, over every placement, of its value plus 3 for each move away. Keys with the value 0 name
/// no placement, and only the moves from `starts` lower any value at first.
///
/// Lowering a value as far as a neighbour's allows, in any order, reaches the same values, once
/// every placement has been expanded (its neighbours lowered) with its final value. So `starts`
/// are expanded, then the placements lowered, as shortest paths: from the lowest values up (a
/// bucket queue), each once, when final.
fn relax<K>(keys: &K, values: &mut [u8], starts: &[u32])
where
    K: PlacementKeys,
{
    let values = as_atomic(values);
    // Lowers each neighbour of placement `key` above `value + 3` to it, collecting them.
    let expand = |key: u32, value: u8, lowered: &mut Vec<u32>| {
        let (placement, free_cells) = keys.placement(key);
        let mut occupied = !free_cells;
        while occupied != 0 {
            let from = occupied.trailing_zeros();
            occupied &= occupied - 1;
            let mut targets = NEIGHBOR_MASKS[from as usize] & free_cells;
            while targets != 0 {
                let to = targets.trailing_zeros();
                targets &= targets - 1;
                let neighbor = keys.after_move(&placement, key, from, to);
                let slot = &values[neighbor as usize];
                if slot.load(Relaxed) > value + 3 {
                    slot.store(value + 3, Relaxed);
                    lowered.push(neighbor);
                }
            }
        }
    };

    let mut buckets = vec![Vec::new(); 256];
    let mut lowered = Vec::new();
    for &key in starts {
        let value = values[key as usize].load(Relaxed);
        // Higher values lower nothing.
        if value <= u8::MAX - 3 {
            expand(key, value, &mut lowered);
            buckets[usize::from(value + 3)].append(&mut lowered);
        }
    }
    for value in 1..=u8::MAX - 3 {
        for key in std::mem::take(&mut buckets[usize::from(value)]) {
            // Lowered since it was queued: expanded from its lower bucket.
            if values[key as usize].load(Relaxed) == value {
                expand(key, value, &mut lowered);
                buckets[usize::from(value + 3)].append(&mut lowered);
            }
        }
    }
}

/// Each placement of `tiles` (at most 9 of them) with its least additive distance over the
/// blank's cells (see [`additive_values`]), then lowered (by steps of 2, keeping it admissible
/// and its parity) until moving one of `tiles` to a neighbouring cell changes it by at most 3
/// (see [`relax`]). Reports the search's layers to `config`'s callback.
pub(in crate::solver::size4x4::stm) fn relaxed_minimums(
    tiles: &[u8],
    config: &PdbConfig,
) -> Vec<u8> {
    relaxed_minimums_and_mirror(tiles, None, config).0
}

/// [`relaxed_minimums`] of `tiles`, and those carried over to the placements of `mirrored`:
/// `tiles` reflected left to right, each tile `t` becoming `t ^ 3` (a row's labels run one way or
/// the other, so reflecting reverses a cell's label within its row). Reflecting keeps every move,
/// so it keeps the additive distances as long as it keeps the goal: the blank's home region must
/// hold cell 3, cell 0's reflection.
pub(in crate::solver::size4x4::stm) fn mirrored_relaxed_minimums(
    tiles: &[u8],
    mirrored: &[u8],
    config: &PdbConfig,
) -> (Vec<u8>, Vec<u8>) {
    let (minimums, mirror) = relaxed_minimums_and_mirror(tiles, Some(mirrored), config);
    (minimums, mirror.expect("a mirror was asked for"))
}

/// [`relaxed_minimums`], and [`mirrored_relaxed_minimums`]' mirror if `mirrored` is given.
fn relaxed_minimums_and_mirror(
    tiles: &[u8],
    mirrored: Option<&[u8]>,
    config: &PdbConfig,
) -> (Vec<u8>, Option<Vec<u8>>) {
    // Nibble keys are faster, but take 16 bytes and more per placement beyond 6 tiles.
    if tiles.len() <= 6 {
        let keys = NibbleKeys {
            tile_count: tiles.len(),
        };
        relaxed_minimums_with_keys(&keys, tiles, mirrored, config)
    } else {
        relaxed_minimums_with_keys(&IndexKeys::new(tiles), tiles, mirrored, config)
    }
}

/// [`relaxed_minimums_and_mirror`], naming placements with `keys`.
fn relaxed_minimums_with_keys<K>(
    keys: &K,
    tiles: &[u8],
    mirrored: Option<&[u8]>,
    config: &PdbConfig,
) -> (Vec<u8>, Option<Vec<u8>>)
where
    K: PlacementKeys,
{
    let (mut values, starts) = additive_values(keys, tiles, config);
    relax(keys, &mut values, &starts);
    let mirror = mirrored.map(|mirrored| mirror_minimums(keys, tiles, &values, mirrored));
    (keys.minimums(values), mirror)
}

/// The minimums of `values` (one per key of `tiles`) carried over to the placements of `mirrored`
/// (see [`mirrored_relaxed_minimums`]), in index order.
fn mirror_minimums<K>(keys: &K, tiles: &[u8], values: &[u8], mirrored: &[u8]) -> Vec<u8>
where
    K: PlacementKeys,
{
    let home_free_cells = !tiles.iter().fold(0u16, |mask, &tile| mask | (1 << tile));
    assert!(
        (reachable_region(1, home_free_cells) >> 3) & 1 != 0,
        "the blank's home region must be its own reflection"
    );
    // `positions[i]`: the position in `mirrored` of tile `i`'s reflection.
    let positions: Vec<usize> = tiles
        .iter()
        .map(|&tile| {
            mirrored
                .iter()
                .position(|&other| other == tile ^ 3)
                .expect("`mirrored` must be `tiles` reflected")
        })
        .collect();
    assert_eq!(
        tiles.len(),
        mirrored.len(),
        "`mirrored` must be `tiles` reflected"
    );
    in_index_order(mirrored.len(), |cells| {
        let mut reflected = [0; 16];
        for (cell, &position) in reflected.iter_mut().zip(&positions) {
            *cell = cells[position] ^ 3;
        }
        minimum(values[keys.key_of(&reflected[..positions.len()]) as usize])
    })
}

/// The least distance of a placement with value `value` (one more, see [`additive_values`]).
#[inline(always)]
fn minimum(value: u8) -> u8 {
    value.checked_sub(1).expect("every placement is reachable")
}

/// `value` of each placement of `tile_count` tiles (from 2 to 9; given its cells), in index order.
fn in_index_order<F>(tile_count: usize, value: F) -> Vec<u8>
where
    F: Fn(&[u8]) -> u8 + Sync,
{
    /// Places the tiles from `placed` on, on cells not in `used`, in index order (each tile's
    /// cell in increasing order, from tile 0's), writing each placement's value to `out`.
    fn place<'a, F>(
        cells: &mut [u8; 16],
        placed: usize,
        tile_count: usize,
        used: u16,
        out: &mut impl Iterator<Item = &'a mut u8>,
        value: &F,
    ) where
        F: Fn(&[u8]) -> u8,
    {
        if placed == tile_count {
            *out.next().unwrap() = value(&cells[..tile_count]);
            return;
        }
        for cell in 0..16 {
            if (used >> cell) & 1 == 0 {
                cells[placed] = cell;
                place(
                    cells,
                    placed + 1,
                    tile_count,
                    used | (1 << cell),
                    out,
                    value,
                );
            }
        }
    }

    let mut values = vec![0; placement_count(tile_count)];
    // Tiles 0 and 1's cells are an index's leading digits, so each pair of cells starts an equal
    // block, filled by one thread.
    let blocks: Vec<std::sync::Mutex<&mut [u8]>> = values
        .chunks_mut(placement_count(tile_count) / (16 * 15))
        .map(std::sync::Mutex::new)
        .collect();
    let next_block = AtomicUsize::new(0);
    let threads = thread::available_parallelism().map_or(1, usize::from);
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let block = next_block.fetch_add(1, Relaxed);
                    let Some(out) = blocks.get(block) else {
                        break;
                    };
                    let first = block / 15;
                    let second = block % 15 + usize::from(block % 15 >= first);
                    let mut cells = [0; 16];
                    cells[..2].copy_from_slice(&[first as u8, second as u8]);
                    let used = (1 << first) | (1 << second);
                    let mut out = out.lock().unwrap();
                    place(&mut cells, 2, tile_count, used, &mut out.iter_mut(), &value);
                }
            });
        }
    });
    drop(blocks);
    values
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;

    use super::*;

    /// The xxh3 hash of every state's distance, laid out by state (`placement * free_count +
    /// rank`), checking that each state is visited exactly once.
    fn distances_hash(tiles: &[u8], cost: Cost) -> u64 {
        let free_count = 16 - tiles.len();
        let mut distances = vec![u8::MAX; placement_count(tiles.len()) * free_count];
        let atomic_distances = as_atomic(&mut distances);
        breadth_first_search(
            tiles,
            cost,
            &PdbConfig::default(),
            |index, _, rank, distance| {
                let state = index as usize * free_count + rank;
                assert_eq!(atomic_distances[state].swap(distance, Relaxed), u8::MAX);
            },
        );
        assert!(distances.iter().all(|&distance| distance != u8::MAX));
        xxhash_rust::xxh3::xxh3_64(&distances)
    }

    /// `relaxed_minimums` of `tiles` the slow way: the search's distances minimised over each
    /// placement's states, then lowered until no move lowers any further.
    fn relaxed_minimums_slowly(tiles: &[u8]) -> Vec<u8> {
        let mut minimums = vec![u8::MAX; placement_count(tiles.len())];
        let atomic_minimums = as_atomic(&mut minimums);
        breadth_first_search(
            tiles,
            Cost::Additive,
            &PdbConfig::default(),
            |index, _, _, distance| {
                atomic_minimums[index as usize].fetch_min(distance, Relaxed);
            },
        );
        let mut lowered = true;
        while lowered {
            lowered = false;
            for index in 0..minimums.len() {
                let cells = cells_of_index(index as u32, tiles.len());
                let free_cells = !tile_mask(&cells[..tiles.len()]);
                for tile in 0..tiles.len() {
                    let mut targets = NEIGHBOR_MASKS[usize::from(cells[tile])] & free_cells;
                    while targets != 0 {
                        let mut moved = cells;
                        moved[tile] = targets.trailing_zeros() as u8;
                        targets &= targets - 1;
                        let neighbor = index_of_cells(&moved[..tiles.len()]) as usize;
                        let bound = minimums[index].saturating_add(3);
                        if minimums[neighbor] > bound {
                            minimums[neighbor] = bound;
                            lowered = true;
                        }
                    }
                }
            }
        }
        minimums
    }

    #[test]
    fn test_breadth_first_search() {
        for (tiles, cost, expected) in [
            (&[1, 2, 3, 4, 5, 6], Cost::Additive, 0xa3d7_92a5_52c0_b480),
            (&[1, 2, 3, 4, 5, 6], Cost::Full, 0x87ba_0e77_c076_77c2),
            (
                &[10, 11, 12, 13, 14, 15],
                Cost::Additive,
                0xe364_0c2d_b91f_8db9,
            ),
            (&[10, 11, 12, 13, 14, 15], Cost::Full, 0xa4b9_81a1_acb0_e6bb),
        ] {
            assert_eq!(distances_hash(tiles, cost), expected, "{tiles:?} {cost:?}");
        }
    }

    #[test]
    #[ignore = "about a minute"]
    fn test_breadth_first_search_7() {
        let tiles = [1, 2, 3, 4, 5, 6, 7];
        for (cost, expected) in [
            (Cost::Additive, 0x0c71_fcf4_d9b4_27eb),
            (Cost::Full, 0x8e60_fc5a_8fb6_e1d9),
        ] {
            assert_eq!(distances_hash(&tiles, cost), expected, "{cost:?}");
        }
    }

    #[test]
    fn test_relaxed_minimums() {
        let config = PdbConfig::default();
        // The last walls in the blank's home.
        for tiles in [
            &[1, 2, 3, 4, 5][..],
            &[6, 9, 10, 12, 15],
            &[1, 7, 10, 13, 14],
        ] {
            let expected = relaxed_minimums_slowly(tiles);
            let nibble_keys = NibbleKeys {
                tile_count: tiles.len(),
            };
            let index_keys = IndexKeys::new(tiles);
            let (nibble, _) = relaxed_minimums_with_keys(&nibble_keys, tiles, None, &config);
            let (index, _) = relaxed_minimums_with_keys(&index_keys, tiles, None, &config);
            assert!(nibble == expected, "{tiles:?}");
            assert!(index == expected, "{tiles:?}");
        }
    }

    #[test]
    fn test_mirrored_relaxed_minimums() {
        let config = PdbConfig::default();
        for (tiles, mirrored) in [
            (&[6, 7, 8, 9, 14, 15][..], &[5, 4, 11, 10, 13, 12][..]),
            (&[1, 2, 6, 9], &[2, 1, 5, 10]),
        ] {
            let expected = relaxed_minimums(mirrored, &config);
            let (_, nibble) = mirrored_relaxed_minimums(tiles, mirrored, &config);
            let index_keys = IndexKeys::new(tiles);
            let (_, index) =
                relaxed_minimums_with_keys(&index_keys, tiles, Some(mirrored), &config);
            assert!(nibble == expected, "{tiles:?}");
            assert!(index.unwrap() == expected, "{tiles:?}");
        }
    }
}
