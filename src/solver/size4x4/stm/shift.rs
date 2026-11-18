//! Incremental Lehmer-index shifts for a table set with two tile
//! partitions.
//!
//! Each partition splits tiles 1-15 into two groups at a tile (7-8: 1-7
//! and 8-15; 9-6: 1-9 and 10-15), and every move changes the index of
//! exactly one group per partition: the moved tile's. Both partitions'
//! shifts come from one scan of the cells the blank passes, with the two
//! sums packed in one u64.

use std::hint::select_unpredictable;

use crate::solver::size4x4::stm::tables::{Board, group_weight};

/// A table set's two partitions.
pub trait Partitions {
    /// Each partition's split: its first group is tiles `1..=SPLIT[p]`, its
    /// second the rest of tiles 1-15.
    const SPLIT: [u8; 2];
    /// Index scale (bytes per placement) of each group, `[partition][group]`.
    const SCALE: [[u32; 2]; 2];
    /// The packed tables for these partitions.
    const SHIFT: &'static PairShift = &PairShift::new(Self::SPLIT, Self::SCALE);
}

/// Per-tile Lehmer weights for both partitions, scaled to byte offsets in
/// the moved tile's tables and packed with partition 0 in the low 32 bits
/// and partition 1 in the high 32, so a scanned cell costs one add.
pub struct PairShift {
    /// `contrib[piece][tile]`: how a `tile` the moved `piece` passes
    /// shifts its groups' indices.
    pub contrib: [[u64; 16]; 16],
    /// The moved piece's own weight, which every move adds.
    pub wbits: [u64; 16],
}

/// Tiles `lo..=hi` of the group of `piece` in a partition split at `split`,
/// and that group's scale.
const fn group_of(piece: i32, split: u8, scale: [u32; 2]) -> (i32, i32, i32) {
    if piece <= split as i32 {
        (1, split as i32, scale[0] as i32)
    } else {
        (split as i32 + 1, 15, scale[1] as i32)
    }
}

impl PairShift {
    /// The tables for partitions split at `split` with group scales
    /// `scale` (see [`Partitions`]).
    ///
    /// Only the tiles strictly between the two blank positions in index
    /// order change their Lehmer-code contribution: the moved tile itself
    /// shifts one position, and so does every tile in between except
    /// same-group tiles smaller than the moved one; same-group bigger
    /// tiles additionally contribute their weight.
    pub const fn new(split: [u8; 2], scale: [[u32; 2]; 2]) -> Self {
        let mut contrib = [[0u64; 16]; 16];
        let mut wbits = [0u64; 16];
        let mut piece = 0;
        while piece < 16 {
            let mut packed = [0u64; 2];
            let mut own = [0u64; 2];
            let mut cell = 0;
            while cell < 16 {
                let mut p = 0;
                while p < 2 {
                    let (lo, hi, s) = group_of(piece, split[p], scale[p]);
                    let w = group_weight(piece, lo, hi);
                    let inside = cell >= lo && cell <= hi;
                    let c = if inside && cell > piece {
                        group_weight(cell, lo, hi) + w
                    } else if !inside {
                        w
                    } else {
                        0
                    };
                    packed[p] = (c * s) as u32 as u64;
                    own[p] = (w * s) as u32 as u64;
                    p += 1;
                }
                contrib[piece as usize][cell as usize] = packed[0] | (packed[1] << 32);
                cell += 1;
            }
            wbits[piece as usize] = own[0] | (own[1] << 32);
            piece += 1;
        }
        Self { contrib, wbits }
    }
}

/// How one move shifts each group's index: `[partition][group]`, exactly
/// one of each partition's pair nonzero (the moved tile's group).
#[derive(Clone, Copy)]
pub struct IndexDelta {
    pub shift: [[i32; 2]; 2],
    /// How many tiles of each partition's second group lie strictly
    /// between the two blank positions (the cells the scan reads).
    pub passed: [u8; 2],
}

/// All four index shifts for one blank move from `lo` across `gap` (the
/// blank moving up when `sign`), `piece` the moved tile: both partitions
/// in a single gap scan.
///
/// The gap between the two blank positions is 1, 3, 5 or 7 cells, so the
/// scan is unrolled per gap size and the loop disappears; the sign is
/// applied with a mask rather than a branch, and the two sums ride in
/// one packed accumulator (partition 0 in the low 32 bits, partition 1
/// in the high 32).
#[inline(always)]
pub fn move_deltas<P>(board: &Board, lo: usize, gap: usize, sign: bool, piece: usize) -> IndexDelta
where
    P: Partitions,
{
    // SAFETY: piece is a tile value (0..15), and so is every tile read.
    let row = unsafe { P::SHIFT.contrib.get_unchecked(piece) };
    // Seeded with the moved piece's own packed weights.
    // SAFETY: as above.
    let mut acc = unsafe { *P::SHIFT.wbits.get_unchecked(piece) };
    let mut passed = [0u8; 2];
    macro_rules! cell {
        ($k:literal) => {{
            // SAFETY: a cell strictly between the two blank positions, and
            // a tile value.
            let (other, packed) = unsafe {
                let other = *board.get_unchecked(lo + $k) as usize;
                (other, *row.get_unchecked(other))
            };
            acc = acc.wrapping_add(packed);
            passed[0] += u8::from(other > P::SPLIT[0] as usize);
            passed[1] += u8::from(other > P::SPLIT[1] as usize);
        }};
    }
    // The gap is 1, 3, 5 or 7, i.e. 0, 2, 4 or 6 cells in between.
    match gap {
        1 => {}
        3 => {
            cell!(1);
            cell!(2);
        }
        5 => {
            cell!(1);
            cell!(2);
            cell!(3);
            cell!(4);
        }
        _ => {
            cell!(1);
            cell!(2);
            cell!(3);
            cell!(4);
            cell!(5);
            cell!(6);
        }
    }
    // Moving the other way walks the same tiles, so the deltas negate:
    // (x ^ mask) - mask, the two's complement negation for mask = all ones.
    let mask = 0u64.wrapping_sub(u64::from(sign));
    let d0 = ((acc & 0xFFFF_FFFF) ^ mask).wrapping_sub(mask) as i32;
    let d1 = ((acc >> 32) ^ mask).wrapping_sub(mask) as i32;
    // Each delta goes to the moved piece's group by a select, in
    // registers: stored at a computed index into a stack array and read
    // back wider (the index sums vectorise), it failed store forwarding.
    let split = |d: i32, second: bool| {
        [
            select_unpredictable(second, 0, d),
            select_unpredictable(second, d, 0),
        ]
    };
    IndexDelta {
        shift: [
            split(d0, piece > P::SPLIT[0] as usize),
            split(d1, piece > P::SPLIT[1] as usize),
        ],
        passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::size4x4::stm::tables::permutations;

    /// The 7-8 and 9-6 partitions with their tables' scales.
    struct Test;

    impl Partitions for Test {
        const SPLIT: [u8; 2] = [7, 9];
        const SCALE: [[u32; 2]; 2] = [[4, 4], [1, 32]];
    }

    #[test]
    fn contribution_table_matches_group_arithmetic() {
        // The packed table must equal the branchy cell semantics it
        // replaces: per piece and cell, the 7-8 and 9-6 contributions.
        let groups = |piece: i32| {
            let g78 = if piece <= 7 { (1, 7) } else { (8, 15) };
            let g96 = if piece <= 9 { (1, 9) } else { (10, 15) };
            (g78, g96)
        };
        // Scaled to byte offsets in the moved piece's tables.
        let scales = |piece: i32| (4, if piece >= 10 { 32 } else { 1 });
        for piece in 0..16i32 {
            let ((lo78, hi78), (lo96, hi96)) = groups(piece);
            let w78 = group_weight(piece, lo78, hi78);
            let w96 = group_weight(piece, lo96, hi96);
            for cell in 0..16i32 {
                let in78 = cell >= lo78 && cell <= hi78;
                let c78 = if in78 && cell > piece {
                    group_weight(cell, lo78, hi78) + w78
                } else if !in78 {
                    w78
                } else {
                    0
                };
                let in96 = cell >= lo96 && cell <= hi96;
                let c96 = if in96 && cell > piece {
                    group_weight(cell, lo96, hi96) + w96
                } else if !in96 {
                    w96
                } else {
                    0
                };
                let (s78, s96) = scales(piece);
                let packed = Test::SHIFT.contrib[piece as usize][cell as usize];
                assert_eq!(packed & 0xFFFF_FFFF, (c78 * s78) as u32 as u64);
                assert_eq!(packed >> 32, (c96 * s96) as u32 as u64);
            }
            // The moved tile's own weights, packed the same way.
            let (s78, s96) = scales(piece);
            let wbits = Test::SHIFT.wbits[piece as usize];
            assert_eq!(wbits & 0xFFFF_FFFF, (w78 * s78) as u32 as u64);
            assert_eq!(wbits >> 32, (w96 * s96) as u32 as u64);
        }
        // Spot-check the permutation counts behind the weights.
        assert_eq!(group_weight(1, 1, 7), permutations(16 - 1, 7 - 1));
        assert_eq!(group_weight(1, 1, 9), 259_459_200);
        assert_eq!(group_weight(10, 10, 15), 360_360);
        assert_eq!(group_weight(15, 10, 15), 1);
        assert_eq!(group_weight(0, 1, 7), 0);
    }
}
