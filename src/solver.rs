//! Contains everything related to finding solutions to sliding puzzles.
//!
//! For convenience, we provide type aliases for the fastest solvers for each small puzzle size and
//! metric.

pub mod heuristic;
pub mod size4x4;
pub mod small;
pub mod solver;
pub mod statistics;

use crate::{
    algorithm::metric::{Mtm, Stm},
    solver::{
        size4x4::{mtm::solver::Solver as Solver44M, stm::solver::Solver as Solver44S},
        small::solver::Solver as SmallSolver,
    },
};

/// A solver for 2x2 puzzles in [`Stm`].
pub type Solver2x2Stm = SmallSolver<2, 2, 4, Stm>;
/// A solver for 2x2 puzzles in [`Mtm`].
pub type Solver2x2Mtm = SmallSolver<2, 2, 4, Mtm>;
/// A solver for 3x2 and 2x3 puzzles in [`Stm`].
pub type Solver3x2Stm = SmallSolver<3, 2, 6, Stm>;
/// A solver for 3x2 and 2x3 puzzles in [`Mtm`].
pub type Solver3x2Mtm = SmallSolver<3, 2, 6, Mtm>;
/// A solver for 3x3 puzzles in [`Stm`].
pub type Solver3x3Stm = SmallSolver<3, 3, 9, Stm>;
/// A solver for 3x3 puzzles in [`Mtm`].
pub type Solver3x3Mtm = SmallSolver<3, 3, 9, Mtm>;
/// A solver for 4x2 and 2x4 puzzles in [`Stm`].
pub type Solver4x2Stm = SmallSolver<4, 2, 8, Stm>;
/// A solver for 4x2 and 2x4 puzzles in [`Mtm`].
pub type Solver4x2Mtm = SmallSolver<4, 2, 8, Mtm>;
/// A solver for 4x3 and 3x4puzzles in [`Stm`].
pub type Solver4x3Stm = SmallSolver<4, 3, 12, Stm>;
/// A solver for 4x3 and 3x4 puzzles in [`Mtm`].
pub type Solver4x3Mtm = SmallSolver<4, 3, 12, Mtm>;
/// A solver for 4x4 puzzles in [`Stm`].
pub type Solver4x4Stm = Solver44S;
/// A solver for 4x4 puzzles in [`Mtm`].
pub type Solver4x4Mtm = Solver44M;
/// A solver for 5x2 and 2x5 puzzles in [`Stm`].
pub type Solver5x2Stm = SmallSolver<5, 2, 10, Stm>;
/// A solver for 5x2 and 2x5 puzzles in [`Mtm`].
pub type Solver5x2Mtm = SmallSolver<5, 2, 10, Mtm>;
/// A solver for 6x2 and 2x6 puzzles in [`Stm`].
pub type Solver6x2Stm = SmallSolver<6, 2, 12, Stm>;
/// A solver for 6x2 and 2x6 puzzles in [`Mtm`].
pub type Solver6x2Mtm = SmallSolver<6, 2, 12, Mtm>;
