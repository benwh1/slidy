//! Known answers for testing solvers, shared by the tests of the different solvers.

use std::{
    ops::ControlFlow,
    str::FromStr as _,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{
    algorithm::{
        algorithm::Algorithm,
        metric::{Mtm, Stm},
    },
    puzzle::{
        puzzle::Puzzle, size::Size, sliding_puzzle::SlidingPuzzle as _, solved_state::SolvedState,
    },
    solver::{config::SolverConfig, solver::Solver},
};

/// Indices of solved states in [`OPTIMAL`].
pub const TRIVIAL: usize = 0;
pub const ROW_GRIDS: usize = 1;
pub const ROWS: usize = 2;
pub const FRINGE: usize = 3;
pub const SQUARE_FRINGE: usize = 4;
pub const SPLIT_FRINGE: usize = 5;
pub const SPLIT_SQUARE_FRINGE: usize = 6;
pub const DIAGONALS: usize = 7;
pub const LAST_TWO_ROWS: usize = 8;
pub const SPLIT_LAST_TWO_ROWS: usize = 9;
pub const CONCENTRIC_RECTANGLES: usize = 10;
pub const CHECKERBOARD: usize = 11;

/// Optimal solution lengths of random puzzles, in [`Stm`] and then [`Mtm`], for each solved
/// state in the order of the indices above.
///
/// The puzzles were generated with `slidy generate -s WxH -n N --seed 1` for sizes 4x1, 1x4,
/// 2x2, 3x2, 2x3, 4x2, 2x4 and 3x3, and the lengths were found by a brute-force breadth-first
/// search from all solved states.
#[rustfmt::skip]
pub const OPTIMAL: &[(&str, [[u64; 12]; 2])] = &[
    ("0 1 2 3",           [[ 3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
    ("1 0 2 3",           [[ 2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
    ("1 2 3 0",           [[ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0], [ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0]]),
    ("0/1/2/3",           [[ 3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
    ("1/0/2/3",           [[ 2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
    ("1/2/3/0",           [[ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0], [ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0]]),
    ("3 0/2 1",           [[ 1,  5,  5,  1,  1,  5,  5,  5,  5,  5,  1,  5], [ 1,  5,  5,  1,  1,  5,  5,  5,  5,  5,  1,  5]]),
    ("3 1/0 2",           [[ 1,  3,  3,  1,  1,  3,  3,  3,  3,  3,  1,  3], [ 1,  3,  3,  1,  1,  3,  3,  3,  3,  3,  1,  3]]),
    ("1 0/3 2",           [[ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
    ("5 4 0/2 1 3",       [[ 1, 13,  9,  7, 13,  9, 13, 13, 13, 13,  1,  1], [ 1, 11,  9,  6, 10,  9, 10, 11, 11, 11,  1,  1]]),
    ("3 5 4/2 1 0",       [[ 0, 12, 10,  4, 12, 10, 12, 12, 12, 12,  0,  4], [ 0,  8,  8,  4,  8,  8,  8,  8,  8,  8,  0,  4]]),
    ("5 3 2/4 1 0",       [[ 0, 18,  6,  8,  6, 14,  8, 12,  6,  6,  0,  4], [ 0, 16,  4,  7,  4, 12,  7,  8,  4,  4,  0,  4]]),
    ("1 0 4/3 5 2",       [[ 2, 14,  8,  2,  8,  8,  8, 14, 12, 12,  2, 10], [ 2, 13,  7,  2,  6,  7,  7, 11, 10, 10,  2, 10]]),
    ("2 5 3/4 1 0",       [[ 0, 14,  4,  4,  6,  4, 10,  8,  8,  8,  0,  6], [ 0, 12,  4,  4,  4,  4,  8,  8,  7,  7,  0,  5]]),
    ("0 4 2/1 3 5",       [[ 3, 17,  7,  3,  3,  7,  3,  9,  7,  7,  3,  7], [ 2, 14,  6,  2,  2,  6,  3,  7,  6,  6,  2,  6]]),
    ("5 0/3 4/1 2",       [[ 2, 20, 14,  4, 14, 16, 14, 10, 16,  4,  2, 10], [ 1, 20,  9,  3,  9, 12,  9,  7, 12,  3,  1,  7]]),
    ("3 5/4 2/1 0",       [[ 0, 16, 16,  4, 12, 12, 16, 10, 12,  4,  0,  6], [ 0, 16, 12,  4,  8,  8, 12,  8,  8,  4,  0,  4]]),
    ("5 3/2 0/1 4",       [[ 1, 15, 15,  1, 11, 15, 15, 13, 15,  1,  1,  1], [ 1, 12, 12,  1,  8, 12, 12, 12, 12,  1,  1,  1]]),
    ("0 1/2 4/5 3",       [[ 3, 17,  3,  3,  3,  3,  3,  9,  3,  3,  3,  9], [ 2, 14,  3,  2,  2,  2,  3,  7,  2,  2,  2,  6]]),
    ("2 5/3 0/1 4",       [[ 1, 13, 13,  1,  9, 13, 13, 11, 13,  1,  1,  5], [ 1, 11, 11,  1,  7, 11, 11,  9, 11,  1,  1,  4]]),
    ("4 2/0 5/3 1",       [[ 2, 16,  8,  6,  8, 10,  8, 14, 10,  6,  2,  2], [ 2, 14,  6,  6,  6,  9,  6, 10,  9,  6,  2,  2]]),
    ("0 6 1 3/5 7 4 2",   [[ 4, 24, 10, 12, 16, 12, 16, 24, 16, 16,  4,  8], [ 2, 18,  6,  9, 11,  9, 11, 15, 11, 11,  2,  6]]),
    ("4 2 3 7/6 1 0 5",   [[ 1, 25,  7, 11, 15, 17, 15, 19, 19, 19,  1,  7], [ 1, 16,  5,  8,  9, 12,  9, 16, 13, 13,  1,  5]]),
    ("1 2 0 5/7 4 6 3",   [[ 2, 22,  8,  8, 20, 16, 22, 22, 20, 20,  2,  8], [ 2, 14,  7,  6, 14, 11, 14, 14, 14, 14,  2,  7]]),
    ("6 2 5 4/0 1 7 3",   [[ 3, 23,  7,  9, 11, 15, 13, 13, 11, 11,  3, 13], [ 1, 17,  7,  7,  9, 12,  9, 11,  9,  9,  1,  8]]),
    ("0 5 6 1/4 7 3 2",   [[ 4, 26, 10,  8, 22, 14, 22, 24, 26, 26,  4, 14], [ 2, 18,  9,  6, 14, 11, 15, 17, 18, 18,  2, 10]]),
    ("6 1/0 3/5 2/4 7",   [[ 3, 21, 17, 11, 17, 19, 17, 11, 17, 13,  3,  9], [ 2, 17, 14, 10, 13, 13, 14, 10, 13, 10,  2,  6]]),
    ("4 2/3 7/6 1/5 0",   [[ 0, 22, 16, 10, 12, 16, 16, 20, 16, 10,  0, 10], [ 0, 19, 12,  8,  8, 12, 12, 12, 12,  8,  0,  8]]),
    ("1 2/5 0/7 3/6 4",   [[ 2,  8,  8,  8,  8,  8,  8,  8,  8,  8,  2,  8], [ 1,  5,  5,  5,  5,  5,  5,  5,  5,  5,  1,  5]]),
    ("6 2/5 4/1 7/0 3",   [[ 1, 23, 21, 11, 21, 19, 21, 23, 21,  9,  1,  9], [ 1, 14, 14,  9, 14, 14, 14, 14, 14,  7,  1,  6]]),
    ("5 6/0 1/4 2/3 7",   [[ 3, 23, 19, 11, 19, 17, 19, 23, 21, 11,  3,  7], [ 2, 15, 15,  9, 14, 14, 15, 15, 15,  9,  2,  6]]),
    ("7 0 1/2 5 3/6 8 4", [[ 3, 21, 13, 11, 11, 21, 21, 19, 19, 15,  3, 11], [ 2, 18, 10,  6,  6, 15, 15, 14, 14, 10,  2,  9]]),
    ("1 2 8/5 4 7/3 6 0", [[ 0, 24, 16, 10, 10, 18, 18, 16, 14, 12,  6, 10], [ 0, 16, 12,  8,  8, 12, 12, 12, 12,  8,  4,  8]]),
    ("5 1 8/7 6 3/4 0 2", [[ 1, 25, 17, 15, 15, 19, 19, 17, 17, 15,  9,  7], [ 1, 19, 12, 11, 11, 13, 13, 13, 13, 11,  7,  6]]),
    ("2 6 4/8 0 7/5 1 3", [[ 2, 22, 18, 14, 14, 22, 22, 18, 22, 16,  6, 16], [ 2, 19, 14, 10, 10, 16, 16, 16, 16, 13,  6, 12]]),
    ("0 5 6/1 4 8/2 7 3", [[ 4, 24, 18, 16, 16, 18, 18, 10, 18, 16,  6, 10], [ 2, 18, 13, 10, 10, 13, 13,  6, 13, 10,  5,  6]]),
    ("4 5 2/8 7 0/6 1 3", [[ 1, 23, 17, 15, 15, 17, 17, 19, 17, 15,  5, 15], [ 1, 16, 11, 11, 11, 11, 11, 14, 11, 11,  4,  9]]),
];

/// A metric to test the solver in.
pub trait TestMetric {
    /// The index of the metric's lengths in [`OPTIMAL`].
    const INDEX: usize;

    /// The difference between consecutive depths searched by the solver.
    const STEP: u64;

    /// Returns the length of `alg` in the metric.
    fn len(alg: &Algorithm) -> u64;
}

impl TestMetric for Stm {
    const INDEX: usize = 0;
    const STEP: u64 = 2;

    fn len(alg: &Algorithm) -> u64 {
        alg.len_stm()
    }
}

impl TestMetric for Mtm {
    const INDEX: usize = 1;
    const STEP: u64 = 1;

    fn len(alg: &Algorithm) -> u64 {
        alg.len_mtm()
    }
}

/// Solutions of puzzles from [`OPTIMAL`] for one solved state and metric, for each length from
/// the optimal length upwards, in the order the solver finds them. Lengths that the solver
/// doesn't search because of [`Stm`] parity have no solutions.
///
/// The solutions were found by a brute-force depth-first search that tries moves in the same
/// order as the solver.
pub type Solutions = &'static [(&'static str, &'static [&'static [&'static str]])];

pub const ROW_GRIDS_STM: Solutions = &[
    (
        "3 0/2 1",
        &[
            &["URDLU"],
            &[],
            &["RULDRUL"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["URDLURDLURDLURDLU"],
        ],
    ),
    (
        "3 1/0 2",
        &[
            &["DLU"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["LDRULDRUL"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["DLURDLURDLURDLU"],
        ],
    ),
    (
        "5 4 0/2 1 3",
        &[
            &["R2ULDLURDRUL2"],
            &[],
            &["UR2DLULDRURDL2U", "R2UL2DRULDR2UL2"],
            &[],
        ],
    ),
    ("3 5 4/2 1 0", &[&["DR2UL2DR2UL2"], &[], &[], &[]]),
    (
        "5 3 2/4 1 0",
        &[
            &[
                "DRULDR2ULDLUR2DLUL",
                "RDLURDRULDLUR2DLUL",
                "RDRUL2DRURDLULDRUL",
                "RDRUL2DRURDL2URDLU",
            ],
            &[],
            &["RDLUR2DLURDL2UR2DLUL", "RDRUL2DR2ULDRUL2DRUL"],
            &[],
        ],
    ),
    (
        "1 0 4/3 5 2",
        &[
            &["URDL2URDRULDLU"],
            &[],
            &["ULDRURDLULDR2UL2", "URDL2UR2DLURDL2U"],
            &[],
        ],
    ),
    (
        "7 0 1/2 5 3/6 8 4",
        &[
            &[
                "LU2RDRDLULUR2DLULDRUL",
                "LU2RDRDLULUR2DL2URDLU",
                "LURULDR2DLU2RDLULDRUL",
                "LURULDR2DLU2RDL2URDLU",
                "LUR2DLU2LDRURDLULDRUL",
                "LUR2DLU2LDRURDL2URDLU",
            ],
            &[],
            &[
                "LU2RDLURDRDLU2RDLULDRUL",
                "LU2RDLURDRDLU2RDL2URDLU",
                "LURULDR2DLURULDRUL2DRUL",
                "LUR2DLU2LDR2ULDRUL2DRUL",
                "LUR2DLULURDLUR2DLULDRUL",
                "LUR2DLULURDLUR2DL2URDLU",
            ],
        ],
    ),
    (
        "5 1 8/7 6 3/4 0 2",
        &[
            &[
                "DLDRULURD2LUR2DL2U2RDRUL2",
                "DRULDLUR2D2LULDRU2LDR2UL2",
                "DRDLULDRULURDLDRULURDRUL2",
                "DRDL2URULDRDLURULD2RURUL2",
                "DRDL2URDLU2RDLDRULURDRUL2",
            ],
            &[],
            &[
                "DLURDRDL2URULDRDLURULDR2UL2",
                "DLUR2D2L2URURDLDRU2LDRDL2U2",
                "D2LURDLU2RD2LUR2DL2U2RDRUL2",
                "DRULDLUR2D2L2URDLURULDR2UL2",
                "DRDLULDRU2LDRULD2RULURDRUL2",
                "DRDLULDRULURD2LURDLU2RDRUL2",
                "DRDL2U2RDLURD2LURULD2RURUL2",
                "DRDL2U2RDLDRURDL2URDRU2LDLU",
                "DRDL2U2RD2RU2LDLUR2D2LURUL2",
                "DRDL2U2RDRUL2DRDLU2RDLDRULU",
                "DRDL2URULD2RULDRU2LD2RURUL2",
                "DRDL2URDLU2RD2LURDLU2RDRUL2",
                "RDLURDL2UR2D2LULDRU2LDR2UL2",
                "RD2LULDRU2LD2R2ULURD2LURUL2",
            ],
        ],
    ),
];

pub const ROW_GRIDS_MTM: Solutions = &[
    (
        "3 0/2 1",
        &[
            &["URDLU"],
            &[],
            &["RULDRUL"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["URDLURDLURDLURDLU"],
        ],
    ),
    (
        "3 1/0 2",
        &[
            &["DLU"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["LDRULDRUL"],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["DLURDLURDLURDLU"],
        ],
    ),
    (
        "5 4 0/2 1 3",
        &[
            &["R2ULDLURDRUL2", "R2UL2DRULDR2UL2"],
            &[],
            &["UR2DLULDRURDL2U", "UR2DL2URDLUR2DL2U"],
            &[],
        ],
    ),
    ("3 5 4/2 1 0", &[&["DR2UL2DR2UL2"], &[], &[], &[]]),
    (
        "5 3 2/4 1 0",
        &[
            &["DRULDR2ULDLUR2DLUL", "RDRUL2DRURDL2URDLU"],
            &[
                "RDLURDRULDLUR2DLUL",
                "RDLUR2DLURDL2UR2DLUL",
                "RDRUL2DRURDLULDRUL",
                "RDRUL2DR2ULDRUL2DRUL",
            ],
            &[],
            &[],
        ],
    ),
    (
        "1 0 4/3 5 2",
        &[
            &["URDL2URDRULDLU", "URDL2UR2DLURDL2U"],
            &["ULDRURDLULDR2UL2", "ULDR2ULDRUL2DR2UL2"],
            &["LURDLUR2DLULDR2UL2"],
            &[],
        ],
    ),
    (
        "7 0 1/2 5 3/6 8 4",
        &[
            &[
                "LURULDR2DLU2RDL2URDLU",
                "LUR2DLU2LDRURDL2URDLU",
                "LUR2DLU2RD2L2U2RDLDR2UL2U",
                "LU2RDRDLULUR2DL2URDLU",
            ],
            &[
                "LURULDR2DLU2RDLULDRUL",
                "LUR2DLU2LDRURDLULDRUL",
                "LUR2DLU2LDR2ULDRUL2DRUL",
                "LU2RDRDLULUR2DLULDRUL",
                "LU2R2DL2DRU2RD2L2URURD2LU2L",
            ],
            &[
                "U2RD2L2URULDR2ULDRDLURUL2",
                "U2RD2L2URULDR2ULD2RULDRU2L2",
                "U2RD2L2U2RDLUR2DLURD2LURUL2",
                "LUR2ULDRDL2URULD2RU2RDL2U",
                "LUR2UL2DR2ULDLUR2D2LU2RDL2U",
                "LUR2DLULURDLUR2DL2URDLU",
                "LUR2DLU2LD2RU2RDLDLU2RDLU",
                "LUR2DLU2LD2R2ULURD2L2U2R2DL2U",
                "LU2RDLURDRDLU2RDL2URDLU",
                "LU2RD2RULDRUL2UR2DL2URDLU",
                "LU2R2DLULDR2UL2DR2DLU2RDL2U",
                "RU2LDRDL2URULDR2DLU2RDL2U",
                "RU2LDRDL2UR2DLU2LDRURDL2U",
                "RU2LDRDL2U2RDRDLULUR2DL2U",
            ],
        ],
    ),
    (
        "5 1 8/7 6 3/4 0 2",
        &[
            &[
                "D2LU2R2DL2DRU2LD2R2UL2URD2LU2",
                "D2RUL2UR2D2L2UR2ULD2RU2LD2LU2",
            ],
            &[
                "DLDRULURD2LUR2DL2U2RDRUL2",
                "DRULDLUR2D2LULDRU2LDR2UL2",
                "DRDL2U2RD2RU2LDLUR2D2LURUL2",
                "D2LURDLU2RD2LUR2DL2U2RDRUL2",
            ],
            &[
                "LDR2UL2DRURD2LU2LD2RU2LDR2UL2",
                "DLUR2D2L2URURDLDRU2LDRDL2U2",
                "D2LUR2DL2U2RD2LUR2DLURULD2LU2",
                "D2LUR2DL2U2RD2LUR2DL2UR2ULDLU",
                "D2RUL2DR2U2LD2RUL2UR2D2L2U2RD2LU2",
                "D2RU2L2D2RU2LDR2DL2U2R2DL2URD2LU2",
                "RDLURDL2UR2D2LULDRU2LDR2UL2",
                "RD2LULDRU2LD2R2ULURD2LURUL2",
                "RD2LU2RDL2DRU2LD2R2ULURD2LU2L",
            ],
        ],
    ),
];

pub const ROWS_STM: Solutions = &[
    (
        "5 4 0/2 1 3",
        &[
            &["URDRULDLU"],
            &[],
            &[
                "URDLUR2DLUL",
                "UR2DLURDL2U",
                "RULDRURDLUL",
                "RURDLULDRUL",
                "RURDL2URDLU",
            ],
            &[],
        ],
    ),
    (
        "3 5 4/2 1 0",
        &[
            &["DRULDR2UL2", "RDLURDRUL2", "RDRUL2DRUL"],
            &[],
            &["DR2UL2DR2UL2", "RDRULDLURDLU"],
            &[],
        ],
    ),
    ("5 3 2/4 1 0", &[&["R2DL2U"], &[], &["DRURDL2U"], &[]]),
    (
        "1 0 4/3 5 2",
        &[&["LURDRUL2", "RUL2DRUL"], &[], &["RULDLURDLU"], &[]],
    ),
    (
        "7 0 1/2 5 3/6 8 4",
        &[
            &["LUR2DLURULDLU", "RU2LD2LU2RDLU", "RULDLU2RDRUL2"],
            &[],
            &[
                "URDLULUR2D2LU2L",
                "URDLURULDRDL2U2",
                "URDL2U2R2D2LU2L",
                "LU2RDRDLU2RDLUL",
                "LURDRULDRU2LDLU",
                "LUR2DLULUR2DLUL",
                "RU2LD2LURULDRUL",
                "RULDLUR2UL2DRUL",
                "RULDRU2LDRDL2U2",
            ],
        ],
    ),
    (
        "5 1 8/7 6 3/4 0 2",
        &[
            &[
                "DLDRU2LDRURD2LU2L",
                "RDL2DRULURDRDL2U2",
                "RD2L2U2RD2LU2RDLU",
            ],
            &[],
            &[
                "DLUR2D2LULDRU2LDRUL",
                "DLUR2D2LULDRULURDLU",
                "DLUR2D2L2URDLU2RDLU",
                "DLDRU2RDL2URDRDL2U2",
                "DLDRU2RD2LULURD2LU2",
                "DLDRULURDLUR2D2LU2L",
                "DLDRULURDRULDRDL2U2",
                "DLDRULUR2DLURD2L2U2",
                "DLDRULUR2D2L2U2RDLU",
                "DRDL2U2RD2LU2RDRUL2",
                "RDL2DRURDLULURD2LU2",
                "RDLDLURDLU2RDRDL2U2",
                "RD2LULDRULURDLDRULU",
                "RD2L2U2RD2LURULDRUL",
                "RD2L2URDLU2RDLDRULU",
            ],
        ],
    ),
];

pub const ROWS_MTM: Solutions = &[
    (
        "5 4 0/2 1 3",
        &[
            &["URDRULDLU", "UR2DLURDL2U", "UR2DL2UR2DL2U"],
            &["URDLUR2DLUL", "RURDL2URDLU"],
            &[
                "RULDRURDLUL",
                "RULDR2ULDRUL2",
                "RURDLULDRUL",
                "R2ULDLURDRUL2",
                "R2ULDRUL2DRUL",
                "R2UL2DRULDR2UL2",
                "R2UL2DR2UL2DR2UL2",
            ],
            &[],
        ],
    ),
    (
        "3 5 4/2 1 0",
        &[
            &["DRULDR2UL2", "DR2UL2DR2UL2"],
            &["RDLURDRUL2", "RDRUL2DRUL"],
            &[],
            &[],
        ],
    ),
    ("5 3 2/4 1 0", &[&["R2DL2U"], &[], &[], &["DRURDL2U"]]),
    (
        "1 0 4/3 5 2",
        &[
            &["LURDRUL2", "RUL2DRUL"],
            &[],
            &[],
            &["ULDRULDR2UL2", "ULDR2UL2DR2UL2", "RULDLURDLU"],
        ],
    ),
    (
        "7 0 1/2 5 3/6 8 4",
        &[
            &["URDL2U2R2D2LU2L", "RU2LD2LU2RDLU"],
            &["RULDLU2RDRUL2"],
            &[
                "URDLULUR2D2LU2L",
                "LUR2DLURULDLU",
                "RULDRU2LDRDL2U2",
                "RU2LD2RULURD2L2U2",
            ],
        ],
    ),
    (
        "5 1 8/7 6 3/4 0 2",
        &[
            &["RD2L2U2RD2LU2RDLU"],
            &[],
            &[
                "DLDRU2LDRURD2LU2L",
                "DRDL2U2RD2LU2RDRUL2",
                "RDL2DRULURDRDL2U2",
                "RD2L2URDRUL2UR2D2L2U2",
                "RD2L2U2RD2RULURD2L2U2",
            ],
        ],
    ),
];

pub const TRIVIAL_STM: Solutions = &[
    ("5 4 0/2 1 3", &[&["U"], &[], &["RUL"], &[]]),
    ("3 5 4/2 1 0", &[&[""], &[], &[], &[]]),
    ("5 3 2/4 1 0", &[&[""], &[], &[], &[]]),
    ("1 0 4/3 5 2", &[&["UL", "LU"], &[], &["RUL2"], &[]]),
    (
        "7 0 1/2 5 3/6 8 4",
        &[
            &["U2L", "ULU", "LU2"],
            &[],
            &["URUL2", "LURUL", "RU2L2", "RULUL", "RUL2U"],
        ],
    ),
    ("5 1 8/7 6 3/4 0 2", &[&["L"], &[], &["DLU"]]),
];

pub const TRIVIAL_MTM: Solutions = &[
    ("5 4 0/2 1 3", &[&["U"], &[], &["RUL", "R2UL2"], &[]]),
    ("3 5 4/2 1 0", &[&[""], &[], &[], &[]]),
    ("5 3 2/4 1 0", &[&[""], &[], &[], &[]]),
    ("1 0 4/3 5 2", &[&["UL", "LU"], &["RUL2"], &[], &["URDL2U"]]),
    (
        "7 0 1/2 5 3/6 8 4",
        &[&["U2L", "LU2"], &["ULU", "RU2L2"], &["URUL2", "RUL2U"]],
    ),
    ("5 1 8/7 6 3/4 0 2", &[&["L"], &[], &["DLU", "D2LU2"]]),
];

/// Returns the solutions that `solver` finds for `puzzle` with `config`, as strings.
pub fn collect<T>(solver: &mut T, puzzle: &Puzzle, config: SolverConfig) -> Vec<String>
where
    T: Solver<Puzzle, Context = ()>,
{
    let solutions = solver.solve_collect(puzzle, config).unwrap();
    solutions.iter().map(ToString::to_string).collect()
}

/// Returns a [`SolverConfig`] with the given fields and no callbacks.
pub fn config(min: u64, max: u64, depth_beyond_optimal: u64, num_solutions: u64) -> SolverConfig {
    SolverConfig {
        min,
        max,
        depth_beyond_optimal,
        num_solutions,
        ..Default::default()
    }
}

/// Solvers for puzzles of any size.
pub trait Solvers {
    /// The type of the solvers.
    type Solver: Solver<Puzzle, Context = ()>;

    /// Returns a solver for puzzles of size `size`.
    fn get(&mut self, size: Size) -> &mut Self::Solver;
}

/// Solvers that each work for only one size, made by a function of the size when needed.
pub struct PerSize<T, F> {
    make: F,
    current: Option<(Size, T)>,
}

impl<T, F> PerSize<T, F>
where
    F: FnMut(Size) -> T,
{
    /// Creates the [`PerSize`], where `make(size)` makes a solver for puzzles of size `size`.
    pub const fn new(make: F) -> Self {
        Self {
            make,
            current: None,
        }
    }
}

impl<T, F> Solvers for PerSize<T, F>
where
    T: Solver<Puzzle, Context = ()>,
    F: FnMut(Size) -> T,
{
    type Solver = T;

    fn get(&mut self, size: Size) -> &mut T {
        if self.current.as_ref().is_none_or(|(s, _)| *s != size) {
            self.current = Some((size, (self.make)(size)));
        }
        &mut self.current.as_mut().unwrap().1
    }
}

/// Checks that `solvers` find the solutions in `table` with various [`SolverConfig`]s,
/// skipping puzzles with solutions in the table longer than `max_length`.
pub fn check_configs<M>(solvers: &mut impl Solvers, table: Solutions, max_length: u64)
where
    M: TestMetric,
{
    const ALL: u64 = u64::MAX;
    const NONE: [&str; 0] = [];

    for &(state, by_depth) in table {
        let puzzle = Puzzle::from_str(state).unwrap();
        let solver = solvers.get(puzzle.size());
        let mut run = |config| collect(solver, &puzzle, config);

        let opt = M::len(&Algorithm::from_str(by_depth[0][0]).unwrap());
        let extra = by_depth.len() - 1;
        // The longest solutions in the table.
        let last = opt + extra as u64;
        // The solutions of length at most `opt + i`.
        let up_to = |i: usize| by_depth[..=i].concat();
        let all = up_to(extra);
        if last > max_length {
            continue;
        }

        assert_eq!(run(SolverConfig::default()), &all[..1], "state {state}");
        assert_eq!(run(config(0, ALL, 0, ALL)), by_depth[0], "state {state}");

        for (i, &solutions) in by_depth.iter().enumerate() {
            let depth = opt + i as u64;
            let msg = format!("state {state}, depth {depth}");

            assert_eq!(run(config(0, ALL, i as u64, ALL)), up_to(i), "{msg}");
            assert_eq!(run(config(0, depth, ALL, ALL)), up_to(i), "{msg}");
            assert_eq!(run(config(depth, depth, ALL, ALL)), solutions, "{msg}");

            // The shortest solutions of length at least `depth`.
            let shortest = by_depth[i..].iter().find(|s| !s.is_empty());
            let shortest = shortest.copied().unwrap_or_default();
            assert_eq!(run(config(depth, last, 0, ALL)), shortest, "{msg}");
        }

        for n in 0..=all.len() + 1 {
            let expected = &all[..n.min(all.len())];
            assert_eq!(
                run(config(0, last, ALL, n as u64)),
                expected,
                "state {state}"
            );
        }

        if opt > 0 {
            assert_eq!(run(config(0, opt - 1, ALL, ALL)), NONE, "state {state}");
        }
        assert_eq!(run(config(opt + 1, opt, ALL, ALL)), NONE, "state {state}");

        // Stop after `k` solutions using the solution callback.
        for k in 1..=all.len() {
            let count = AtomicUsize::new(0);
            let config = SolverConfig {
                solution_callback: Some(Box::new(move |_| {
                    if count.fetch_add(1, Ordering::Relaxed) + 1 == k {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                })),
                ..config(0, last, ALL, ALL)
            };
            assert_eq!(run(config), &all[..k], "state {state}, {k} solutions");
        }

        // Record the depths searched using the end of iteration callback, starting from the
        // optimal length and from the heuristic's bound.
        for min in [opt, 0] {
            let depths = Arc::new(Mutex::new(Vec::new()));
            let depths_clone = Arc::clone(&depths);
            let config = SolverConfig {
                end_of_iter_callback: Some(Box::new(move |stats| {
                    depths_clone.lock().unwrap().push(stats.depth);
                    ControlFlow::Continue(())
                })),
                ..config(min, last, ALL, ALL)
            };
            assert_eq!(run(config), all, "state {state}, min {min}");

            let depths = depths.lock().unwrap();
            let msg = format!("state {state}, min {min}, depths {depths:?}");
            assert!(depths[0] <= opt, "{msg}");
            assert!(depths.windows(2).all(|d| d[1] == d[0] + M::STEP), "{msg}");
            assert_eq!(
                *depths.last().unwrap(),
                last - extra as u64 % M::STEP,
                "{msg}"
            );
            if min == opt {
                assert_eq!(depths[0], opt, "{msg}");
            }
        }

        // Stop after the search at length `opt + i` using the end of iteration callback.
        for i in (0..=extra).step_by(M::STEP as usize) {
            let depth = opt + i as u64;
            let config = SolverConfig {
                end_of_iter_callback: Some(Box::new(move |stats| {
                    if stats.depth >= depth {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                })),
                ..config(0, last, ALL, ALL)
            };
            assert_eq!(run(config), up_to(i), "state {state}, depth {depth}");
        }
    }
}

/// Checks that `solvers` find solutions of the lengths in [`OPTIMAL`] in the metric `M` for
/// `solved_state`, which has index `label`.
pub fn check_optimal<M, S>(solvers: &mut impl Solvers, solved_state: &S, label: usize)
where
    M: TestMetric,
    S: SolvedState,
{
    for (state, lengths) in OPTIMAL {
        let expected = lengths[M::INDEX][label];
        let mut puzzle = Puzzle::from_str(state).unwrap();
        let solution = solvers.get(puzzle.size()).solve(&puzzle).unwrap();
        assert_eq!(M::len(&solution), expected, "state {state}");
        puzzle.apply_alg(&solution);
        assert!(solved_state.is_solved(&puzzle), "state {state}");
    }
}
