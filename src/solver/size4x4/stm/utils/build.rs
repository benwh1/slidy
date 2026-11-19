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
        index::{
            cells_of_index, digits_of_index, index_of_cells, index_shift_table, nth_set_bit,
            placement_count, tile_mask,
        },
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

/// Runs `work` on each of `threads` threads, giving each one's result. One thread is the calling
/// thread, without spawning any, so it needs no thread support (as on WebAssembly).
fn on_threads<T, F>(threads: usize, work: F) -> Vec<T>
where
    F: Fn() -> T + Sync,
    T: Send,
{
    if threads <= 1 {
        return vec![work()];
    }
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(threads);
        // Every thread starts before any is joined.
        for _ in 0..threads {
            handles.push(scope.spawn(&work));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    })
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
#[inline(always)]
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
#[inline(always)]
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
    #[inline(always)]
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

/// The bits of `bits` (16 of them) spread to every other bit: bit `i` to bit `2i`.
const fn spread_bits(bits: u32) -> u32 {
    let bits = (bits | (bits << 8)) & 0x00ff_00ff;
    let bits = (bits | (bits << 4)) & 0x0f0f_0f0f;
    let bits = (bits | (bits << 2)) & 0x3333_3333;
    (bits | (bits << 1)) & 0x5555_5555
}

/// Decodes a group's placements in increasing index order, each from the one before: most differ
/// from it only in their last tiles' cells, which are all that is redone.
struct PlacementCursor<'a> {
    tiles: &'a [u8],
    placement: Placement,
    /// The placement's index's digits (see [`digits_of_index`]).
    digits: [u8; 16],
    /// Each tile's cell.
    cells: [u8; 16],
    /// `unused[k]`: the cells without any of the first `k` tiles.
    unused: [u16; 17],
}

impl<'a> PlacementCursor<'a> {
    /// At placement 0.
    fn new(tiles: &'a [u8]) -> Self {
        let placement = Placement::new(0, tiles);
        let cells = cells_of_index(0, tiles.len());
        let mut unused = [u16::MAX; 17];
        for (k, &cell) in cells[..tiles.len()].iter().enumerate() {
            unused[k + 1] = unused[k] & !(1 << cell);
        }
        Self {
            tiles,
            placement,
            digits: [0; 16],
            cells,
            unused,
        }
    }

    /// Placement `index`, at least the current one's.
    #[inline(always)]
    fn seek(&mut self, index: u32) -> &Placement {
        let tile_count = self.tiles.len();
        let digits = digits_of_index(index, tile_count);
        // The first tile whose digit, so cell, changes.
        let changed = u128::from_le_bytes(digits) ^ u128::from_le_bytes(self.digits);
        let first = (changed.trailing_zeros() / 8) as usize;
        let board = &mut self.placement.board;
        for &cell in &self.cells[first.min(tile_count)..tile_count] {
            board[usize::from(cell)] = 0;
        }
        for k in first..tile_count {
            let cell = nth_set_bit(self.unused[k], u32::from(digits[k])) as u8;
            self.cells[k] = cell;
            self.unused[k + 1] = self.unused[k] & !(1 << cell);
            board[usize::from(cell)] = self.tiles[k];
        }
        self.digits = digits;
        self.placement.free_cells = self.unused[tile_count];
        self.placement.index = index;
        &self.placement
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
struct Marks {
    words: Vec<AtomicU64>,
}

impl Marks {
    /// Every one of `states` states unvisited.
    fn new(states: usize) -> Self {
        let words = states.div_ceil(MARKS_PER_WORD);
        Self {
            words: std::iter::repeat_with(|| AtomicU64::new(u64::MAX))
                .take(words)
                .collect(),
        }
    }

    /// Marks every state marked `mark` as [`DONE`], using `threads` threads.
    fn retire(&self, mark: u8, threads: usize) {
        let chunk_len = self.words.len().div_ceil(threads).max(1);
        let next_chunk = AtomicUsize::new(0);
        on_threads(threads, || {
            while let Some(chunk) = self
                .words
                .chunks(chunk_len)
                .nth(next_chunk.fetch_add(1, Relaxed))
            {
                for word in chunk {
                    let marks = word.load(Relaxed);
                    let matching = fields_marked(marks, mark);
                    // Storing only the words that change spares the others' cache lines a
                    // write-back.
                    if matching == 0 {
                        continue;
                    }
                    word.store(
                        (marks & !(matching * 3)) | (matching * u64::from(DONE)),
                        Relaxed,
                    );
                }
            }
        });
    }

    /// The word holding the marks of states `32 * index..32 * (index + 1)`.
    #[inline(always)]
    fn word(&self, index: usize) -> u64 {
        self.words[index].load(Relaxed)
    }

    #[inline(always)]
    fn get(&self, state: usize) -> u8 {
        ((self.word(state / MARKS_PER_WORD) >> (2 * (state % MARKS_PER_WORD))) & 3) as u8
    }

    /// Marks unvisited `state` with `mark`; false when it was visited.
    #[inline(always)]
    fn claim(&self, state: usize, mark: u8) -> bool {
        let shift = 2 * (state % MARKS_PER_WORD);
        let word = &self.words[state / MARKS_PER_WORD];
        // Most states found are already visited, which a plain load shows more cheaply.
        if (word.load(Relaxed) >> shift) & 3 != u64::from(UNVISITED) {
            return false;
        }
        word.try_update(AcqRel, Relaxed, |word| {
            ((word >> shift) & 3 == u64::from(UNVISITED))
                .then_some(word & !(u64::from(UNVISITED ^ mark) << shift))
        })
        .is_ok()
    }
}

/// Whether the CPU has the instructions that the searches' `*_with_bit_instructions` functions
/// are compiled with: `popcnt` (counting a blank's rank among the free cells), and BMI1 and
/// BMI2's (cheaper bit manipulation, and shifts by a variable amount in one instruction).
#[cfg(target_arch = "x86_64")]
fn has_bit_instructions() -> bool {
    is_x86_feature_detected!("popcnt")
        && is_x86_feature_detected!("bmi1")
        && is_x86_feature_detected!("bmi2")
}

/// Whether the CPU has the instructions that the searches' `*_with_bit_instructions` functions
/// are compiled with: they are x86-64 instructions, so no.
#[cfg(not(target_arch = "x86_64"))]
fn has_bit_instructions() -> bool {
    false
}

/// One layer of a search: the states at distance `depth + 1`, found from those at `depth`.
struct Layer {
    /// Whether it is found by checking the unvisited states (else by expanding the layer before).
    pulling: bool,
    /// The mark of the states at `depth`.
    current: u8,
    /// The mark of the states found.
    next: u8,
    depth: u8,
    /// How many placements a thread takes at a time.
    chunk_len: usize,
}

/// The fixed parameters of one search.
struct Search<'a, F> {
    tiles: &'a [u8],
    /// How many placements of `tiles` there are.
    placements: usize,
    /// Divides by `free_count` with a multiplication: see [`Self::placement_of`].
    reciprocal: u128,
    /// The group's [`index_shift_table`].
    shifts: [[u64; 16]; 16],
    marks: Marks,
    /// How many cells have no group tile, so states per placement.
    free_count: usize,
    cost: Cost,
    /// The regions of each set of free cells, for additive searches.
    labels: Option<RegionLabels>,
    visit: &'a F,
}

impl<F> Search<'_, F>
where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    /// The placement of state `state`.
    #[inline(always)]
    fn placement_of(&self, state: usize) -> usize {
        ((state as u128 * self.reciprocal) >> 64) as usize
    }

    /// One thread's part of a layer: the chunks of placements it takes from `next_chunk`, each
    /// placement with a state the layer starts from expanded (see [`Self::expand_placement`]).
    /// Returns how many states it found.
    #[inline(always)]
    fn expand_chunks(&self, layer: &Layer, next_chunk: &AtomicUsize) -> u64 {
        let &Layer {
            pulling,
            current,
            next,
            depth,
            chunk_len,
        } = layer;
        let (placements, free_count) = (self.placements, self.free_count);
        let source = if pulling { UNVISITED } else { current };
        let mut cursor = PlacementCursor::new(self.tiles);
        let mut found = 0;
        loop {
            let start = next_chunk.fetch_add(chunk_len, Relaxed);
            if start >= placements {
                return found;
            }
            let end = (start + chunk_len).min(placements);
            // Finds the chunk's placements with a state marked `source`, a word of marks at a
            // time. No state gains the mark `source` during a layer (the frontier's are fixed,
            // and claimed states lose `UNVISITED`), so a word without one can be skipped.
            let end_state = end * free_count;
            let mut state = start * free_count;
            while state < end_state {
                let word_index = state / MARKS_PER_WORD;
                let word_end = (word_index + 1) * MARKS_PER_WORD;
                let mut matching = fields_marked(self.marks.word(word_index), source)
                    & (u64::MAX << (2 * (state % MARKS_PER_WORD)));
                if end_state < word_end {
                    matching &= (1 << (2 * (end_state % MARKS_PER_WORD))) - 1;
                }
                if matching == 0 {
                    state = word_end;
                    continue;
                }
                let matched = word_index * MARKS_PER_WORD + matching.trailing_zeros() as usize / 2;
                let index = self.placement_of(matched);
                let placement = cursor.seek(index as u32);
                found += self.expand_placement(placement, pulling, current, next, depth);
                state = (index + 1) * free_count;
            }
        }
    }

    /// [`Self::expand_chunks`], compiled with instructions that make the bit manipulation
    /// cheaper (see [`has_bit_instructions`]).
    ///
    /// # Safety
    ///
    /// The CPU must support the `popcnt`, `bmi1`, and `bmi2` target features, as
    /// [`has_bit_instructions`] checks (only on x86-64; elsewhere this is never called).
    #[cfg_attr(target_arch = "x86_64", target_feature(enable = "popcnt,bmi1,bmi2"))]
    unsafe fn expand_chunks_with_bit_instructions(
        &self,
        layer: &Layer,
        next_chunk: &AtomicUsize,
    ) -> u64 {
        self.expand_chunks(layer, next_chunk)
    }

    /// The state of placement `index` with the blank on `blank`, given the free cells.
    #[inline(always)]
    fn state(&self, index: u32, free_cells: u16, blank: u32) -> usize {
        index as usize * self.free_count + blank_rank(free_cells, blank)
    }

    /// Claims the state of placement `index` with the blank on `blank`, if unvisited, marking it
    /// `next` and visiting it at distance `depth + 1`. Returns how many it claimed.
    #[inline(always)]
    fn claim_state(&self, index: u32, free_cells: u16, blank: u32, next: u8, depth: u8) -> u64 {
        let rank = blank_rank(free_cells, blank);
        let claimed = self
            .marks
            .claim(index as usize * self.free_count + rank, next);
        if claimed {
            (self.visit)(index, blank, rank, depth + 1);
        }
        u64::from(claimed)
    }

    /// [`Self::claim_cells`] for the cells of the region of `cell`, whose states are always
    /// claimed together, with `regions` the regions of `free_cells`.
    #[inline(always)]
    fn claim_region(
        &self,
        index: u32,
        free_cells: u16,
        regions: Regions,
        cell: u32,
        next: u8,
        depth: u8,
    ) -> u64 {
        // A region with a claimed state needs nothing more.
        if self.marks.get(self.state(index, free_cells, cell)) != UNVISITED {
            return 0;
        }
        let region = regions.region(regions.rank(cell));
        self.claim_cells(index, free_cells, region, next, depth)
    }

    /// The marks of placement `index`'s states, a 2-bit field each, in rank order.
    #[inline(always)]
    fn placement_marks(&self, index: u32) -> u64 {
        // A placement's states are consecutive, so they span at most two words.
        let first = index as usize * self.free_count;
        let word = first / MARKS_PER_WORD;
        let shift = 2 * (first % MARKS_PER_WORD);
        let mut marks = self.marks.word(word) >> shift;
        if shift + 2 * self.free_count > 64 {
            marks |= self.marks.word(word + 1) << (64 - shift);
        }
        marks & ((1 << (2 * self.free_count)) - 1)
    }

    /// Claims the unvisited states of placement `index` with the blank on a cell of `cells` (all
    /// in `free_cells`), marking them `next` and visiting them at distance `depth + 1`, with one
    /// update of each word of marks they are in. Returns how many it claimed.
    #[inline(always)]
    fn claim_cells(&self, index: u32, free_cells: u16, cells: u16, next: u8, depth: u8) -> u64 {
        // A placement's states are consecutive, so they span at most two words: the cells'
        // fields' low bits, from the first state's word's on.
        let first = index as usize * self.free_count;
        let mut ranks = 0;
        let mut remaining = cells;
        while remaining != 0 {
            ranks |= 1 << blank_rank(free_cells, remaining.trailing_zeros());
            remaining &= remaining - 1;
        }
        let shift = 2 * (first % MARKS_PER_WORD);
        let fields = u128::from(spread_bits(ranks)) << shift;
        let mut claimed = 0;
        for (offset, fields) in [(0, fields as u64), (1, (fields >> 64) as u64)] {
            if fields == 0 {
                continue;
            }
            let index = first / MARKS_PER_WORD + offset;
            if let Ok(old) = self.marks.words[index].try_update(AcqRel, Relaxed, |word| {
                let unvisited = fields_marked(word, UNVISITED) & fields;
                (unvisited != 0).then_some(word & !(unvisited * u64::from(UNVISITED ^ next)))
            }) {
                claimed |= u128::from(fields_marked(old, UNVISITED) & fields) << (64 * offset);
            }
        }
        let claimed = claimed >> shift;
        let mut found = 0;
        let mut remaining = cells;
        while remaining != 0 {
            let blank = remaining.trailing_zeros();
            remaining &= remaining - 1;
            let rank = blank_rank(free_cells, blank);
            if (claimed >> (2 * rank)) & 1 != 0 {
                (self.visit)(index, blank, rank, depth + 1);
                found += 1;
            }
        }
        found
    }

    /// One placement's part of a layer: when pushing, finds the unvisited neighbours of its
    /// states marked `current` (the layer at distance `depth`); when pulling, finds its unvisited
    /// states with a neighbour there. Either way they are at distance `depth + 1`, and are marked
    /// `next`. Returns how many states it found.
    #[inline(always)]
    fn expand_placement(
        &self,
        placement: &Placement,
        pulling: bool,
        current: u8,
        next: u8,
        depth: u8,
    ) -> u64 {
        // The states this layer starts from: unvisited ones when pulling, else the layer's.
        let source = if pulling { UNVISITED } else { current };
        let (index, free_cells) = (placement.index, placement.free_cells);
        let mut found = 0;
        match self.cost {
            Cost::Full => {
                // Moves onto a free cell keep the placement, so their states are claimed (or
                // checked) together from a snapshot of the placement's marks; only moves of a
                // group tile need a state each. States unvisited in the snapshot may have been
                // claimed since, which the claim sees; the others stay visited.
                let [currents, unvisited] = cells_marked(
                    free_cells,
                    self.placement_marks(index),
                    [current, UNVISITED],
                );
                let sources = if pulling { unvisited } else { currents };
                if sources == 0 {
                    return 0;
                }
                let within = neighbors(currents) & unvisited;
                if within != 0 {
                    found += self.claim_cells(index, free_cells, within, next, depth);
                }
                // When pulling, the states found within need no other move.
                let mut blanks = if pulling { sources & !within } else { sources };
                while blanks != 0 {
                    let blank = blanks.trailing_zeros();
                    blanks &= blanks - 1;
                    // The blank moves to `neighbor`, moving the group tile there to `blank`.
                    let mut tiles = NEIGHBOR_MASKS[blank as usize] & !free_cells;
                    while tiles != 0 {
                        let neighbor = tiles.trailing_zeros();
                        tiles &= tiles - 1;
                        let (next_index, next_free) =
                            placement.after_move(&self.shifts, neighbor, blank);
                        if !pulling {
                            found += self.claim_state(next_index, next_free, neighbor, next, depth);
                        } else if self.marks.get(self.state(next_index, next_free, neighbor))
                            == current
                        {
                            found += self.claim_state(index, free_cells, blank, next, depth);
                            break;
                        }
                    }
                }
            }
            Cost::Additive => {
                // The blank's region (its cells reachable without moving a group tile) shares
                // one distance, so each region is handled at once, its mark read at its lowest
                // cell.
                let labels = self.labels.as_ref().expect("additive searches have labels");
                let regions = labels.regions(free_cells);
                for rank in 0..regions.count(free_cells) {
                    let lowest = regions.lowest(rank);
                    if self.marks.get(self.state(index, free_cells, lowest)) != source {
                        continue;
                    }
                    let region = regions.region(rank);
                    // Moves of a group tile next to the region into it.
                    let mut movable = neighbors(region) & !free_cells;
                    'moves: while movable != 0 {
                        let from = movable.trailing_zeros();
                        movable &= movable - 1;
                        let mut tos = NEIGHBOR_MASKS[from as usize] & region;
                        while tos != 0 {
                            let to = tos.trailing_zeros();
                            tos &= tos - 1;
                            let (next_index, next_free) =
                                placement.after_move(&self.shifts, from, to);
                            if !pulling {
                                found += self.claim_region(
                                    next_index,
                                    next_free,
                                    labels.regions(next_free),
                                    from,
                                    next,
                                    depth,
                                );
                            } else if self.marks.get(self.state(next_index, next_free, from))
                                == current
                            {
                                found += self.claim_cells(index, free_cells, region, next, depth);
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

/// For each of `wanted`, the cells of `free_cells` whose marks in `marks` (a 2-bit field each, in
/// the order of the cells) are it.
#[inline(always)]
fn cells_marked<const N: usize>(free_cells: u16, marks: u64, wanted: [u8; N]) -> [u16; N] {
    let (mut cells, mut remaining, mut marks) = ([0; N], free_cells, marks);
    // Without branches, which the marks would make unpredictable.
    while remaining != 0 {
        let cell = remaining.isolate_lowest_one();
        remaining &= remaining - 1;
        for (cells, &mark) in cells.iter_mut().zip(&wanted) {
            *cells |= cell & 0u16.wrapping_sub(u16::from(marks & 3 == u64::from(mark)));
        }
        marks >>= 2;
    }
    cells
}

/// Searches the states of `tiles` (at most 9 of them, each home on the cell of its label) for
/// `cost`'s distances, calling `visit(placement, blank, rank, distance)` once for every state
/// (`rank`: how many cells below the blank's have no group tile), from `threads` threads.
/// Reports each layer to `config`'s callback.
pub(in crate::solver::size4x4::stm) fn breadth_first_search<F>(
    tiles: &[u8],
    cost: Cost,
    config: &PdbConfig,
    threads: usize,
    visit: F,
) where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    let placements = placement_count(tiles.len());
    let free_count = 16 - tiles.len();
    let states = placements * free_count;
    let bit_instructions = has_bit_instructions();
    let search = Search {
        tiles,
        placements,
        // `state / free_count`, as a multiplication: with `free_count * reciprocal` = 2^64 + e
        // for some e < `free_count` (at most 16), the error is below the quotient's fractional
        // headroom for every state below 2^60.
        reciprocal: u128::from(u64::MAX / free_count as u64 + 1),
        shifts: index_shift_table([&[tiles], &[]]),
        marks: Marks::new(states),
        free_count,
        cost,
        labels: (cost == Cost::Additive).then(|| RegionLabels::new(free_count)),
        visit: &visit,
    };
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
        let layer = Layer {
            pulling,
            current,
            next,
            depth: depth as u8,
            chunk_len: placements.div_ceil(threads * 64).max(1),
        };
        let next_chunk = AtomicUsize::new(0);
        let found = on_threads(threads, || {
            if bit_instructions {
                // SAFETY: The target features are available.
                unsafe { search.expand_chunks_with_bit_instructions(&layer, &next_chunk) }
            } else {
                search.expand_chunks(&layer, &next_chunk)
            }
        });
        layer_sizes.push(found.into_iter().sum());
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

    /// The regions of `free_cells`.
    #[inline(always)]
    fn regions(&self, free_cells: u16) -> Regions {
        Regions(self.0[usize::from(free_cells)])
    }
}

/// The regions of one set of free cells: its [`RegionLabels`] entry.
#[derive(Clone, Copy)]
struct Regions(u64);

impl Regions {
    /// How many regions there are, given the free cells (not every cell).
    #[inline(always)]
    fn count(self, free_cells: u16) -> u32 {
        self.rank((!free_cells).trailing_zeros())
    }

    /// The rank of free cell `cell`'s region.
    #[inline(always)]
    fn rank(self, cell: u32) -> u32 {
        ((self.0 >> (4 * cell)) & 15) as u32
    }

    /// The lowest bit of each nibble labelled `rank`.
    #[inline(always)]
    fn labelled(self, rank: u32) -> u64 {
        // A nibble is zero exactly where the label is `rank`.
        let difference = self.0 ^ (u64::from(rank) * RegionLabels::NIBBLES);
        !(difference | (difference >> 1) | (difference >> 2) | (difference >> 3))
            & RegionLabels::NIBBLES
    }

    /// The lowest cell of the region of rank `rank`.
    #[inline(always)]
    fn lowest(self, rank: u32) -> u32 {
        self.labelled(rank).trailing_zeros() / 4
    }

    /// The region of rank `rank`.
    #[inline(always)]
    fn region(self, rank: u32) -> u16 {
        let mut matching = self.labelled(rank);
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
    /// placement, using `threads` threads.
    fn minimums(&self, values: Vec<u8>, threads: usize) -> Vec<u8>;
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

    fn minimums(&self, mut values: Vec<u8>, _: usize) -> Vec<u8> {
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

    fn minimums(&self, values: Vec<u8>, threads: usize) -> Vec<u8> {
        in_index_order(self.tile_count, threads, |cells| {
            minimum(values[self.key_of(cells) as usize])
        })
    }
}

/// How many regions [`additive_values`] expands before checking their moves.
const MOVE_BATCH_REGIONS: usize = 16;

/// The state of [`additive_values`]' search. A region is `16 * key + its rank`.
struct RegionSearch<'a, K> {
    keys: &'a K,
    labels: RegionLabels,
    /// Each key's value, as [`additive_values`] gives it.
    values: &'a [AtomicU8],
    /// A bit per region, set once it is reached. Nearly every placement has at most 4 regions:
    /// their bits come first, 4 per key, so that the search's lookups mostly stay in the cache,
    /// then 16 per key for the rest.
    bits: Box<[AtomicU64]>,
    /// Where the bits of regions of rank 4 and up start.
    first_bits: usize,
    /// Whether to count each layer's cells (only the callback needs them).
    counting: bool,
}

/// What [`RegionSearch::reach_first`] found of a region.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reached {
    /// It was reached before.
    Before,
    /// It is new, and the first of its key's regions reached.
    First,
    /// It is new, and may not be the first of its key's.
    New,
}

/// One layer of a [`RegionSearch`].
struct RegionLayer<'a> {
    /// The regions at distance `depth`.
    frontier: &'a [u32],
    depth: u8,
    /// The start of the next chunk of `frontier` to be expanded.
    next_chunk: AtomicUsize,
    /// How many regions of `frontier` a thread takes at a time.
    chunk_len: usize,
}

impl<K> RegionSearch<'_, K>
where
    K: PlacementKeys,
{
    /// A region's bit: its word and the bit's mask.
    #[inline(always)]
    fn bit_of(&self, region: u32) -> (&AtomicU64, u64) {
        let position = if region % 16 < 4 {
            4 * (region as usize / 16) + region as usize % 16
        } else {
            self.first_bits + region as usize
        };
        (&self.bits[position / 64], 1u64 << (position % 64))
    }

    /// Reaches `region`, giving whether it is new.
    fn reach(&self, region: u32) -> bool {
        let (word, bit) = self.bit_of(region);
        word.load(Relaxed) & bit == 0 && word.fetch_or(bit, Relaxed) & bit == 0
    }

    /// Reaches `region` (with `free_cells`), if it is new, telling whether its key's other
    /// regions are all unreached when that is known without reading its value: when they all have
    /// bits in its word. Read before setting the bit, that misses only other regions of the key
    /// reached in the same layer, whose values are the same.
    #[inline(always)]
    fn reach_first(&self, region: u32, free_cells: u16) -> Reached {
        let (word, bit) = self.bit_of(region);
        let old = word.load(Relaxed);
        if old & bit != 0 || word.fetch_or(bit, Relaxed) & bit != 0 {
            return Reached::Before;
        }
        let key_bits = 15 << (4 * (region / 16 % 16));
        if self.labels.regions(free_cells).count(free_cells) <= 4 && old & key_bits == 0 {
            Reached::First
        } else {
            Reached::New
        }
    }

    /// One thread's part of `layer`: the chunks of its frontier it takes, their regions'
    /// unreached neighbours (the next layer's regions) reached and their keys' values set. Returns
    /// the regions it reached, how many cells they have (if counting), and the keys with a region
    /// more than 2 further than their value.
    #[inline(always)]
    fn expand(&self, layer: &RegionLayer) -> (Vec<u32>, u64, Vec<u32>) {
        let (keys, labels, depth) = (self.keys, &self.labels, layer.depth);
        let (mut next, mut found, mut far) = (Vec::new(), 0, Vec::new());
        // A batch of moves' regions, with their free cells, whose bits are fetched before any is
        // checked.
        let mut moves: Vec<(u32, u16)> = Vec::with_capacity(1024);
        // The new regions whose keys' values must be read, fetched first too.
        let mut unsure: Vec<u32> = Vec::with_capacity(1024);
        loop {
            let start = layer.next_chunk.fetch_add(layer.chunk_len, Relaxed);
            if start >= layer.frontier.len() {
                return (next, found, far);
            }
            let end = (start + layer.chunk_len).min(layer.frontier.len());
            for regions in layer.frontier[start..end].chunks(MOVE_BATCH_REGIONS) {
                moves.clear();
                for &region in regions {
                    let key = region / 16;
                    let (placement, free_cells) = keys.placement(key);
                    let region = labels.regions(free_cells).region(region % 16);
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
                            let id = 16 * next_key + labels.regions(next_free).rank(from);
                            prefetch(self.bit_of(id).0.as_ptr().cast());
                            moves.push((id, next_free));
                        }
                    }
                }
                unsure.clear();
                for &(id, next_free) in &moves {
                    let reached = self.reach_first(id, next_free);
                    if reached == Reached::Before {
                        continue;
                    }
                    next.push(id);
                    if self.counting {
                        let next_region = labels.regions(next_free).region(id % 16);
                        found += u64::from(next_region.count_ones());
                    }
                    // A key's first region reached is its nearest.
                    let value = &self.values[id as usize / 16];
                    if reached == Reached::First {
                        value.store(depth + 2, Relaxed);
                    } else {
                        prefetch(value.as_ptr().cast());
                        unsure.push(id / 16);
                    }
                }
                for &key in &unsure {
                    let value = &self.values[key as usize];
                    let old = value.load(Relaxed);
                    if old == 0 {
                        value.store(depth + 2, Relaxed);
                    } else if depth > old {
                        far.push(key);
                    }
                }
            }
        }
    }

    /// [`Self::expand`], compiled with instructions that make the bit manipulation cheaper (see
    /// [`has_bit_instructions`]).
    ///
    /// # Safety
    ///
    /// The CPU must support the `popcnt`, `bmi1`, and `bmi2` target features, as
    /// [`has_bit_instructions`] checks (only on x86-64; elsewhere this is never called).
    #[cfg_attr(target_arch = "x86_64", target_feature(enable = "popcnt,bmi1,bmi2"))]
    unsafe fn expand_with_bit_instructions(
        &self,
        layer: &RegionLayer,
    ) -> (Vec<u32>, u64, Vec<u32>) {
        self.expand(layer)
    }
}

/// Each key's least additive distance over the blank's cells, plus 1 (0 for keys naming no
/// placement): what [`breadth_first_search`] finds for [`Cost::Additive`], minimised over each
/// placement's states, but searching the blank's regions (its cells reachable without moving a
/// group tile, which share a distance) instead of every cell. Reports each layer (counting every
/// cell of its regions, as [`breadth_first_search`] does) to `config`'s callback.
///
/// Also gives the keys with a region more than 2 further than their value: the only placements
/// that moving a tile from can lower another's value in [`relax`] (a move into a region leads at
/// most 1 further), so they are all it needs to start from.
fn additive_values<K>(
    keys: &K,
    tiles: &[u8],
    config: &PdbConfig,
    threads: usize,
) -> (Vec<u8>, Vec<u32>)
where
    K: PlacementKeys,
{
    let bit_instructions = has_bit_instructions();
    let mut values = vec![0; keys.len()];
    let search = RegionSearch {
        keys,
        labels: RegionLabels::new(16 - tiles.len()),
        values: as_atomic(&mut values),
        bits: zeroed_words((20 * keys.len()).div_ceil(64)),
        first_bits: 4 * keys.len(),
        counting: config.end_of_iter_callback.is_some(),
    };

    // The goal: every tile home, and the blank in its home's region, which holds cell 0 (so has
    // rank 0).
    let home = keys.key_of(tiles);
    let mut frontier = vec![16 * home];
    let mut far = Vec::new();
    search.reach(16 * home);
    search.values[home as usize].store(1, Relaxed);
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

    for depth in 0u8.. {
        let layer = RegionLayer {
            frontier: &frontier,
            depth,
            next_chunk: AtomicUsize::new(0),
            chunk_len: frontier.len().div_ceil(threads * 64).max(1),
        };
        // Small layers are not worth starting every thread for.
        let layer_threads = frontier.len().div_ceil(1 << 14).clamp(1, threads);
        let layers = on_threads(layer_threads, || {
            if bit_instructions {
                // SAFETY: The target features are available.
                unsafe { search.expand_with_bit_instructions(&layer) }
            } else {
                search.expand(&layer)
            }
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
/// no placement, and only the moves from `starts` lower any value at first. Uses `threads`
/// threads.
///
/// Lowering a value as far as a neighbour's allows, in any order, reaches the same values, once
/// every placement has been expanded (its neighbours lowered) with its final value. So `starts`
/// are expanded, then the placements lowered, as shortest paths: from the lowest values up (a
/// bucket queue), each once, when final. Expanding a bucket lowers values only into a higher one,
/// so its placements are expanded in parallel.
fn relax<K>(keys: &K, values: &mut [u8], starts: &[u32], threads: usize)
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
                // Only the thread that lowers it collects it.
                if slot.load(Relaxed) > value + 3 && slot.fetch_min(value + 3, Relaxed) > value + 3
                {
                    lowered.push(neighbor);
                }
            }
        }
    };
    // Expands each of `queue` whose value `value_of` gives (none: skipped), on the threads,
    // queueing the placements lowered in `buckets` (by value).
    let expand_all = |queue: &[u32],
                      value_of: &(dyn Fn(u32) -> Option<u8> + Sync),
                      buckets: &mut Vec<Vec<u32>>| {
        let next_chunk = AtomicUsize::new(0);
        let chunk_len = queue.len().div_ceil(threads * 64).max(1);
        // Small queues are not worth starting every thread for.
        let queue_threads = queue.len().div_ceil(1 << 12).clamp(1, threads);
        let lowered_by_thread = on_threads(queue_threads, || {
            let mut lowered_by_value = vec![Vec::new(); 256];
            let mut lowered = Vec::new();
            loop {
                let start = next_chunk.fetch_add(chunk_len, Relaxed);
                if start >= queue.len() {
                    return lowered_by_value;
                }
                for &key in &queue[start..(start + chunk_len).min(queue.len())] {
                    if let Some(value) = value_of(key) {
                        expand(key, value, &mut lowered);
                        lowered_by_value[usize::from(value + 3)].append(&mut lowered);
                    }
                }
            }
        });
        for lowered_by_value in lowered_by_thread {
            for (bucket, lowered) in buckets.iter_mut().zip(lowered_by_value) {
                bucket.extend(lowered);
            }
        }
    };

    let mut buckets = vec![Vec::new(); 256];
    expand_all(
        starts,
        &|key| {
            let value = values[key as usize].load(Relaxed);
            // Higher values lower nothing. A start lowered meanwhile is queued again.
            (value <= u8::MAX - 3).then_some(value)
        },
        &mut buckets,
    );
    for value in 1..=u8::MAX - 3 {
        let queue = std::mem::take(&mut buckets[usize::from(value)]);
        // Those lowered since they were queued are expanded from their lower bucket.
        expand_all(
            &queue,
            &|key| (values[key as usize].load(Relaxed) == value).then_some(value),
            &mut buckets,
        );
    }
}

/// Each placement of `tiles` (at most 9 of them) with its least additive distance over the
/// blank's cells (see [`additive_values`]), then lowered (by steps of 2, keeping it admissible
/// and its parity) until moving one of `tiles` to a neighbouring cell changes it by at most 3
/// (see [`relax`]). Reports the search's layers to `config`'s callback, using `threads` threads.
pub(in crate::solver::size4x4::stm) fn relaxed_minimums(
    tiles: &[u8],
    config: &PdbConfig,
    threads: usize,
) -> Vec<u8> {
    relaxed_minimums_and_mirror(tiles, None, config, threads).0
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
    threads: usize,
) -> (Vec<u8>, Vec<u8>) {
    let (minimums, mirror) = relaxed_minimums_and_mirror(tiles, Some(mirrored), config, threads);
    (minimums, mirror.expect("a mirror was asked for"))
}

/// [`relaxed_minimums`], and [`mirrored_relaxed_minimums`]' mirror if `mirrored` is given.
fn relaxed_minimums_and_mirror(
    tiles: &[u8],
    mirrored: Option<&[u8]>,
    config: &PdbConfig,
    threads: usize,
) -> (Vec<u8>, Option<Vec<u8>>) {
    // Nibble keys are faster, but take 16 bytes and more per placement beyond 6 tiles.
    if tiles.len() <= 6 {
        let keys = NibbleKeys {
            tile_count: tiles.len(),
        };
        relaxed_minimums_with_keys(&keys, tiles, mirrored, config, threads)
    } else {
        relaxed_minimums_with_keys(&IndexKeys::new(tiles), tiles, mirrored, config, threads)
    }
}

/// [`relaxed_minimums_and_mirror`], naming placements with `keys`.
fn relaxed_minimums_with_keys<K>(
    keys: &K,
    tiles: &[u8],
    mirrored: Option<&[u8]>,
    config: &PdbConfig,
    threads: usize,
) -> (Vec<u8>, Option<Vec<u8>>)
where
    K: PlacementKeys,
{
    let (mut values, starts) = additive_values(keys, tiles, config, threads);
    relax(keys, &mut values, &starts, threads);
    let mirror = mirrored.map(|mirrored| mirror_minimums(keys, tiles, &values, mirrored, threads));
    (keys.minimums(values, threads), mirror)
}

/// The minimums of `values` (one per key of `tiles`) carried over to the placements of `mirrored`
/// (see [`mirrored_relaxed_minimums`]), in index order.
fn mirror_minimums<K>(
    keys: &K,
    tiles: &[u8],
    values: &[u8],
    mirrored: &[u8],
    threads: usize,
) -> Vec<u8>
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
    in_index_order(mirrored.len(), threads, |cells| {
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

/// `value` of each placement of `tile_count` tiles (from 2 to 9; given its cells), in index order,
/// using `threads` threads.
fn in_index_order<F>(tile_count: usize, threads: usize, value: F) -> Vec<u8>
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
    on_threads(threads, || {
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
    drop(blocks);
    values
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;

    use super::*;

    /// The threads the tests build with.
    const THREADS: usize = 4;

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
            THREADS,
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
            THREADS,
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
    #[ignore = "about 15 seconds"]
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
            let (nibble, _) =
                relaxed_minimums_with_keys(&nibble_keys, tiles, None, &config, THREADS);
            let (index, _) = relaxed_minimums_with_keys(&index_keys, tiles, None, &config, THREADS);
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
            let expected = relaxed_minimums(mirrored, &config, THREADS);
            let (_, nibble) = mirrored_relaxed_minimums(tiles, mirrored, &config, THREADS);
            let index_keys = IndexKeys::new(tiles);
            let (_, index) =
                relaxed_minimums_with_keys(&index_keys, tiles, Some(mirrored), &config, THREADS);
            assert!(nibble == expected, "{tiles:?}");
            assert!(index.unwrap() == expected, "{tiles:?}");
        }
    }
}
