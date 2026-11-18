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
        NBR,
        index::{lehmer, placements, shift_table, unlehmer},
    },
    statistics::PdbIterationStats,
};

/// Which moves a distance counts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    /// Only moves of the group's tiles.
    Additive,
    /// Every move.
    Full,
}

/// `v` viewed as atomics, for writing from several threads.
pub fn atomic(v: &mut [u8]) -> &[AtomicU8] {
    // SAFETY: `AtomicU8` has the same layout as `u8`, and the borrow is exclusive.
    unsafe { std::slice::from_raw_parts(v.as_mut_ptr().cast::<AtomicU8>(), v.len()) }
}

/// The cells reachable from `seed` through `free` cells.
#[must_use]
pub fn flood(seed: u16, free: u16) -> u16 {
    let mut region = seed;
    loop {
        let mut next = region;
        let mut cells = region;
        while cells != 0 {
            next |= NBR[cells.trailing_zeros() as usize];
            cells &= cells - 1;
        }
        next &= free;
        if next == region {
            return region;
        }
        region = next;
    }
}

/// How many of `free`'s cells are below `cell`.
fn rank(free: u16, cell: u32) -> usize {
    (free & ((1 << cell) - 1)).count_ones() as usize
}

/// One placement's tiles: the board (a group tile's label on its cell, 0 elsewhere), the cells
/// without a group tile, and its index.
struct Placement {
    board: [u8; 16],
    free: u16,
    index: u32,
}

impl Placement {
    fn new(index: u32, tiles: &[u8]) -> Self {
        let cells = unlehmer(index, tiles.len());
        let mut board = [0; 16];
        for (i, &tile) in tiles.iter().enumerate() {
            board[usize::from(cells[i])] = tile;
        }
        let free = (0..16)
            .filter(|&c| board[c] == 0)
            .fold(0, |m, c| m | (1 << c));
        Self { board, free, index }
    }

    /// The index after moving the tile on `from` to `to`, and the cells then free.
    fn moved(&self, table: &[[u64; 16]; 16], from: u32, to: u32) -> (u32, u16) {
        let row = &table[usize::from(self.board[from as usize])];
        let (lo, hi) = (from.min(to), from.max(to));
        let delta = (lo + 1..hi).fold(row[0] as u32, |sum, c| {
            sum + row[usize::from(self.board[c as usize])] as u32
        });
        let index = if to > from {
            self.index + delta
        } else {
            self.index - delta
        };
        (index, self.free ^ (1 << from) ^ (1 << to))
    }
}

/// The searches' states' distances mod 3, 2 bits each; 3 marks an unvisited state.
struct States(Vec<u8>);

impl States {
    fn get(&self, s: usize) -> u8 {
        // SAFETY: the bytes are only written through `claim`, atomically.
        let byte = unsafe { &*(&raw const self.0[s >> 2]).cast::<AtomicU8>() };
        (byte.load(Relaxed) >> (2 * (s & 3))) & 3
    }

    /// Marks unvisited state `s` as at distance `residue` mod 3; false when it was visited.
    fn claim(&self, s: usize, residue: u8) -> bool {
        // SAFETY: as in `get`.
        let byte = unsafe { &*(&raw const self.0[s >> 2]).cast::<AtomicU8>() };
        let shift = 2 * (s & 3);
        byte.try_update(AcqRel, Relaxed, |b| {
            ((b >> shift) & 3 == 3).then_some(b & !((3 ^ residue) << shift))
        })
        .is_ok()
    }
}

/// Searches the states of `tiles` (at most 9 of them, each home on the cell of its label) for
/// `cost`'s distances, calling `found(placement, blank, rank, distance)` once for every state
/// (`rank`: how many cells below the blank's have no group tile), from several threads. Reports
/// each layer to `config`'s callback.
pub fn bfs<F>(tiles: &[u8], cost: Cost, config: &PdbConfig, found: F)
where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    let k = tiles.len();
    let n = placements(k);
    let f = 16 - k;
    let table = shift_table([&[tiles], &[]]);
    let states = States(vec![u8::MAX; (n * f).div_ceil(4)]);
    let threads = thread::available_parallelism().map_or(1, usize::from);

    // The goal: every tile home, and the blank anywhere (additive) or home (full).
    let home = Placement::new(lehmer(tiles), tiles);
    let mut counts = vec![0u64];
    let mut free = home.free;
    while free != 0 {
        let blank = free.trailing_zeros();
        free &= free - 1;
        if cost == Cost::Additive || blank == 0 {
            let r = rank(home.free, blank);
            states.claim(home.index as usize * f + r, 0);
            found(home.index, blank, r, 0);
            counts[0] += 1;
        }
    }
    let report = |depth: usize, counts: &[u64]| {
        if let Some(callback) = &config.end_of_iter_callback {
            callback(PdbIterationStats {
                depth: depth as u64,
                new: counts[depth],
                total: counts.iter().sum(),
            });
        }
    };
    report(0, &counts);

    for depth in 0.. {
        let (here, next) = ((depth % 3) as u8, ((depth + 1) % 3) as u8);
        // Expanding the layer's states (and the earlier ones with the same residue) or checking
        // the unvisited ones, whichever is fewer.
        let push: u64 = counts.iter().rev().step_by(3).sum();
        let pull = (n * f) as u64 - counts.iter().sum::<u64>();
        let pulling = pull < push;
        let new = AtomicU64::new(0);
        let chunk = n.div_ceil(threads * 64).max(1);
        let next_chunk = AtomicUsize::new(0);
        thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    let mut count = 0;
                    loop {
                        let start = next_chunk.fetch_add(chunk, Relaxed);
                        if start >= n {
                            break;
                        }
                        for p in start..(start + chunk).min(n) {
                            let base = p * f;
                            let wanted = if pulling { 3 } else { here };
                            if !(0..f).any(|r| states.get(base + r) == wanted) {
                                continue;
                            }
                            let placement = Placement::new(p as u32, tiles);
                            count += layer(
                                &placement,
                                &table,
                                &states,
                                f,
                                cost,
                                pulling,
                                here,
                                next,
                                depth as u8,
                                &found,
                            );
                        }
                    }
                    new.fetch_add(count, Relaxed);
                });
            }
        });
        counts.push(new.into_inner());
        report(depth + 1, &counts);
        if counts[depth + 1] == 0 {
            return;
        }
    }
}

/// One placement's part of a layer: when pushing, finds the unvisited neighbours of its states
/// at residue `here`; when pulling, finds its unvisited states with a neighbour there. Either
/// way they are at distance `depth + 1`. Returns how many states it found.
#[expect(clippy::too_many_arguments, reason = "the search's state, passed down")]
fn layer<F>(
    placement: &Placement,
    table: &[[u64; 16]; 16],
    states: &States,
    f: usize,
    cost: Cost,
    pulling: bool,
    here: u8,
    next: u8,
    depth: u8,
    found: &F,
) -> u64
where
    F: Fn(u32, u32, usize, u8) + Sync,
{
    let state = |index: u32, free: u16, blank: u32| index as usize * f + rank(free, blank);
    let mut count = 0;
    let mut claim = |index: u32, free: u16, blank: u32| {
        let r = rank(free, blank);
        if states.claim(index as usize * f + r, next) {
            found(index, blank, r, depth + 1);
            count += 1;
        }
    };
    let p = placement;
    match cost {
        Cost::Full => {
            let mut free = p.free;
            while free != 0 {
                let blank = free.trailing_zeros();
                free &= free - 1;
                let own = states.get(state(p.index, p.free, blank));
                if own != if pulling { 3 } else { here } {
                    continue;
                }
                let mut nbrs = NBR[blank as usize];
                while nbrs != 0 {
                    let nb = nbrs.trailing_zeros();
                    nbrs &= nbrs - 1;
                    // The blank moves to `nb`, moving the tile there (if any) to `blank`.
                    let (q, qfree) = if p.board[nb as usize] == 0 {
                        (p.index, p.free)
                    } else {
                        p.moved(table, nb, blank)
                    };
                    if !pulling {
                        claim(q, qfree, nb);
                    } else if states.get(state(q, qfree, nb)) == here {
                        claim(p.index, p.free, blank);
                        break;
                    }
                }
            }
        }
        Cost::Additive => {
            // The blank's region (its cells reachable without moving a group tile) shares one
            // distance, so each region is handled once, from its lowest cell.
            let mut todo = p.free;
            while todo != 0 {
                let region = flood(todo.isolate_lowest_one(), p.free);
                todo &= !region;
                let lowest = region.trailing_zeros();
                if states.get(state(p.index, p.free, lowest)) != if pulling { 3 } else { here } {
                    continue;
                }
                // Moves of a group tile next to the region into it.
                let mut tiles = (0..16)
                    .filter(|&c| NBR[c] & region != 0)
                    .fold(0u16, |m, c| m | (1 << c))
                    & !p.free;
                'moves: while tiles != 0 {
                    let from = tiles.trailing_zeros();
                    tiles &= tiles - 1;
                    let mut into = NBR[from as usize] & region;
                    while into != 0 {
                        let to = into.trailing_zeros();
                        into &= into - 1;
                        let (q, qfree) = p.moved(table, from, to);
                        if !pulling {
                            let mut cells = flood(1 << from, qfree);
                            while cells != 0 {
                                claim(q, qfree, cells.trailing_zeros());
                                cells &= cells - 1;
                            }
                        } else if states.get(state(q, qfree, from)) == here {
                            let mut cells = region;
                            while cells != 0 {
                                claim(p.index, p.free, cells.trailing_zeros());
                                cells &= cells - 1;
                            }
                            break 'moves;
                        }
                    }
                }
            }
        }
    }
    count
}

/// Lowers each placement's value in `min` (by steps of 2, keeping it admissible and its parity)
/// until moving one of `tiles` to a neighbouring cell changes it by at most 3.
pub fn relax(tiles: &[u8], min: &mut [u8]) {
    let table = shift_table([&[tiles], &[]]);
    let min = atomic(min);
    let threads = thread::available_parallelism().map_or(1, usize::from);
    let chunk = min.len().div_ceil(threads);
    // Values only fall, and a stale neighbour only lowers less, so racing passes are sound;
    // they repeat until one changes nothing.
    loop {
        let changed = thread::scope(|s| {
            #[expect(
                clippy::needless_collect,
                reason = "every thread starts before any is joined"
            )]
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let table = &table;
                    s.spawn(move || {
                        let mut changed = false;
                        for p in t * chunk..((t + 1) * chunk).min(min.len()) {
                            let placement = Placement::new(p as u32, tiles);
                            let current = min[p].load(Relaxed);
                            let mut lowest = current;
                            let mut cells = !placement.free;
                            while cells != 0 {
                                let from = cells.trailing_zeros();
                                cells &= cells - 1;
                                let mut into = NBR[from as usize] & placement.free;
                                while into != 0 {
                                    let to = into.trailing_zeros();
                                    into &= into - 1;
                                    let (q, _) = placement.moved(table, from, to);
                                    lowest = lowest.min(min[q as usize].load(Relaxed) + 3);
                                }
                            }
                            if lowest < current {
                                min[p].store(lowest, Relaxed);
                                changed = true;
                            }
                        }
                        changed
                    })
                })
                .collect();
            handles
                .into_iter()
                .fold(false, |c, h| h.join().unwrap() | c)
        });
        if !changed {
            return;
        }
    }
}
