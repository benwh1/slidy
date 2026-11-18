//! Building a [`generic`](super::generic) set's tables in memory: breadth-first searches over a
//! group's placements, then encoding into the group's [`Format`].

use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

use crate::solver::size4x4::stm::{
    generic::{Format, Group},
    tables::NEIGHBORS,
};

/// `NBR[cell]`: the cells next to `cell`.
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

/// The connected part of `free` around `seed`.
fn flood(seed: u16, free: u16) -> u16 {
    let mut region = seed;
    loop {
        let mut next = region;
        let mut r = region;
        while r != 0 {
            next |= NBR[r.trailing_zeros() as usize];
            r &= r - 1;
        }
        next &= free;
        if next == region {
            return region;
        }
        region = next;
    }
}

/// Lehmer index of `cells` (in group order).
#[inline]
pub(crate) fn lehmer(cells: &[u8]) -> u32 {
    let mut used = 0u16;
    let mut idx = 0u32;
    for (i, &p) in cells.iter().enumerate() {
        let digit = u32::from(p) - (used & ((1u16 << p) - 1)).count_ones();
        idx = idx * (16 - i as u32) + digit;
        used |= 1 << p;
    }
    idx
}

/// The cells of placement `idx` of `k` tiles.
fn unlehmer(mut idx: u32, k: usize) -> [u8; 16] {
    let mut digits = [0u8; 16];
    for i in (0..k).rev() {
        let radix = 16 - i as u32;
        digits[i] = (idx % radix) as u8;
        idx /= radix;
    }
    let mut out = [0u8; 16];
    let mut used = 0u16;
    for i in 0..k {
        let mut free = !used;
        for _ in 0..digits[i] {
            free &= free - 1;
        }
        let c = free.trailing_zeros() as u8;
        out[i] = c;
        used |= 1 << c;
    }
    out
}

fn occupied(cells: &[u8]) -> u16 {
    cells.iter().fold(0, |m, &c| m | (1 << c))
}

fn atomic(v: &mut [u8]) -> &[AtomicU8] {
    // SAFETY: AtomicU8 has u8's layout, and the borrow is exclusive.
    unsafe { std::slice::from_raw_parts(v.as_mut_ptr().cast::<AtomicU8>(), v.len()) }
}

/// One breadth-first layer: `f(p, push)` for every placement of `cur` on `threads` threads,
/// collecting the pushed placements (deduplicated by `mark` for layer `d`).
fn layer(
    cur: &[u32],
    mark: &[AtomicU8],
    d: u8,
    threads: usize,
    f: &(dyn Fn(usize, &mut dyn FnMut(usize)) + Sync),
) -> Vec<u32> {
    let chunk = cur.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = cur
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    let mut out = Vec::new();
                    for &p in c {
                        f(p as usize, &mut |q| {
                            if mark[q].load(Relaxed) != d {
                                mark[q].store(d, Relaxed);
                                out.push(q as u32);
                            }
                        });
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

/// A group's exact values at `placement * 16 + blank cell` (255 where the blank would sit on a
/// group tile): the additive one, and the full one (`None` unless asked for).
pub struct Values {
    pub add: Vec<u8>,
    pub full: Option<Vec<u8>>,
}

/// Build `group`'s values on `threads` threads.
pub fn values(group: Group, threads: usize) -> Values {
    let (tiles, k) = group.tile_list();
    let home = &tiles[..k];
    let n = group.placements() as usize;
    let goal = lehmer(home) as usize;
    let mut markv = vec![u8::MAX; n];
    // Additive: layers of placements; a blank region shares its value.
    let mut addv = vec![255u8; n * 16];
    {
        let mark = atomic(&mut markv);
        let add = atomic(&mut addv);
        let free = !occupied(home);
        for b in 0..16 {
            if free & (1 << b) != 0 {
                add[goal * 16 + b].store(0, Relaxed);
            }
        }
        let mut cur = vec![goal as u32];
        let mut d = 0u8;
        while !cur.is_empty() {
            cur = layer(&cur, mark, d, threads, &|p, push| {
                let cells = unlehmer(p as u32, k);
                let free = !occupied(&cells[..k]);
                let mut todo = 0u16;
                for b in 0..16 {
                    if add[p * 16 + b].load(Relaxed) == d {
                        todo |= 1 << b;
                    }
                }
                while todo != 0 {
                    let region = flood(1 << todo.trailing_zeros(), free);
                    todo &= !region;
                    for j in 0..k {
                        let from = cells[j] as usize;
                        let mut into = NBR[from] & region;
                        while into != 0 {
                            let to = into.trailing_zeros() as u8;
                            into &= into - 1;
                            let mut moved = cells;
                            moved[j] = to;
                            let q = lehmer(&moved[..k]) as usize;
                            if add[q * 16 + from].load(Relaxed) != 255 {
                                continue;
                            }
                            let qfree = (free | (1 << from)) & !(1u16 << to);
                            let mut r = flood(1 << from, qfree);
                            while r != 0 {
                                add[q * 16 + r.trailing_zeros() as usize].store(d + 1, Relaxed);
                                r &= r - 1;
                            }
                            push(q);
                        }
                    }
                }
            });
            d += 1;
        }
    }
    let full = group.format.has_full().then(|| {
        markv.fill(u8::MAX);
        let mark = atomic(&mut markv);
        let mut fullv = vec![255u8; n * 16];
        let full = atomic(&mut fullv);
        full[goal * 16].store(0, Relaxed);
        let mut cur = vec![goal as u32];
        let mut d = 0u8;
        while !cur.is_empty() {
            cur = layer(&cur, mark, d, threads, &|p, push| {
                let cells = unlehmer(p as u32, k);
                let occ = occupied(&cells[..k]);
                for b in 0..16usize {
                    if full[p * 16 + b].load(Relaxed) != d {
                        continue;
                    }
                    let mut nbs = NBR[b];
                    while nbs != 0 {
                        let nb = nbs.trailing_zeros() as usize;
                        nbs &= nbs - 1;
                        let q = if occ & (1 << nb) != 0 {
                            let j = cells[..k].iter().position(|&c| c as usize == nb).unwrap();
                            let mut moved = cells;
                            moved[j] = b as u8;
                            lehmer(&moved[..k]) as usize
                        } else {
                            p
                        };
                        let e = &full[q * 16 + nb];
                        if e.load(Relaxed) == 255 {
                            e.store(d + 1, Relaxed);
                            push(q);
                        }
                    }
                }
            });
            d += 1;
        }
        drop(markv);
        fullv
    });
    Values { add: addv, full }
}

/// Each placement's additive value minimised over the blank's cells, then lowered (by steps of
/// 2, keeping it admissible and its parity) until a move changes it by at most 3, on `threads`
/// threads.
pub fn relaxed_min(group: Group, add: &[u8], threads: usize) -> Vec<u8> {
    let k = group.len() as usize;
    let mut minv: Vec<u8> = add.chunks(16).map(|c| *c.iter().min().unwrap()).collect();
    let min = atomic(&mut minv);
    let chunk = min.len().div_ceil(threads);
    // Values only fall, and a stale neighbour only lowers less, so racing passes are sound;
    // they repeat until one changes nothing.
    loop {
        let changed = std::thread::scope(|s| {
            let handles: Vec<_> = (0..min.len())
                .step_by(chunk)
                .map(|lo| {
                    s.spawn(move || {
                        let mut changed = false;
                        for p in lo..(lo + chunk).min(min.len()) {
                            let cells = unlehmer(p as u32, k);
                            let free = !occupied(&cells[..k]);
                            let cur = min[p].load(Relaxed);
                            let mut lowest = cur;
                            for j in 0..k {
                                let mut into = NBR[cells[j] as usize] & free;
                                while into != 0 {
                                    let mut moved = cells;
                                    moved[j] = into.trailing_zeros() as u8;
                                    into &= into - 1;
                                    let q = lehmer(&moved[..k]) as usize;
                                    lowest = lowest.min(min[q].load(Relaxed) + 3);
                                }
                            }
                            if lowest < cur {
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
            return minv;
        }
    }
}

/// Encode `group`'s table from its values into `out` (zeroed, [`Group::bytes`] long), on
/// `threads` threads.
pub fn encode(group: Group, values: &Values, out: &mut [u8], threads: usize) {
    let add = &values.add;
    let full = || values.full.as_ref().unwrap();
    let n = group.placements() as usize;
    let bit = |v: u8| u32::from(v >> 1) & 1;
    match group.format {
        Format::Min2 => {
            let min = relaxed_min(group, add, threads);
            for (p, &v) in min.iter().enumerate() {
                out[p >> 2] |= ((v >> 1) & 3) << ((p & 3) * 2);
            }
        }
        Format::Cells => {
            for p in 0..n {
                let w = (0..16).fold(0u16, |w, c| w | ((bit(add[p * 16 + c]) as u16) << c));
                out[2 * p..2 * p + 2].copy_from_slice(&w.to_le_bytes());
            }
        }
        Format::Pair => {
            let full = full();
            for p in 0..n {
                let w = (0..16).fold(0u32, |w, c| {
                    w | (bit(add[p * 16 + c]) << (2 * c)) | (bit(full[p * 16 + c]) << (2 * c + 1))
                });
                out[4 * p..4 * p + 4].copy_from_slice(&w.to_le_bytes());
            }
        }
        Format::Pair8 => {
            let full = full();
            let k = group.len() as usize;
            for p in 0..n {
                let free = !occupied(&unlehmer(p as u32, k)[..k]);
                let (mut w, mut r, mut f) = (0u16, 0, free);
                while f != 0 {
                    let c = f.trailing_zeros() as usize;
                    f &= f - 1;
                    w |= (bit(add[p * 16 + c]) as u16) << r;
                    w |= (bit(full[p * 16 + c]) as u16) << (8 + r);
                    r += 1;
                }
                out[2 * p..2 * p + 2].copy_from_slice(&w.to_le_bytes());
            }
        }
        Format::Bytes16 => {
            let full = full();
            for i in 0..n * 16 {
                // Unreachable entries (the blank on a tile) are never read.
                out[2 * i] = add[i];
                out[2 * i + 1] = full[i];
            }
        }
        Format::Bytes8 => out.copy_from_slice(&add[..n * 16]),
    }
}

/// Convert `group`'s table from [`Format::Pair`] (`pair`) to [`Format::Pair8`] (`out`), on
/// `threads` threads.
pub fn pair_to_pair8(group: Group, pair: &[u8], out: &mut [u8], threads: usize) {
    let k = group.len() as usize;
    let n = group.placements() as usize;
    assert!(k >= 8 && pair.len() == 4 * n && out.len() == 2 * n);
    let chunk = n.div_ceil(threads);
    std::thread::scope(|s| {
        for (t, out) in out.chunks_mut(2 * chunk).enumerate() {
            s.spawn(move || {
                for (i, o) in out.chunks_exact_mut(2).enumerate() {
                    let p = t * chunk + i;
                    let word = u32::from_le_bytes(pair[4 * p..4 * p + 4].try_into().unwrap());
                    let mut free = !occupied(&unlehmer(p as u32, k)[..k]);
                    let (mut w, mut r) = (0u16, 0);
                    while free != 0 {
                        let c = free.trailing_zeros();
                        free &= free - 1;
                        w |= ((word >> (2 * c)) & 1) as u16 * (1 << r);
                        w |= ((word >> (2 * c + 1)) & 1) as u16 * (1 << (8 + r));
                        r += 1;
                    }
                    o.copy_from_slice(&w.to_le_bytes());
                }
            });
        }
    });
}
