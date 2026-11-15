//! Projection [`Solver`] and its metric-specific implementations.
//!
//! The [`Solver`] struct and the implementation shared by all metrics live here; the
//! metric-specific implementations live in the `stm` and `mtm` submodules.

use std::marker::PhantomData;

use crate::{
    algorithm::direction::Direction,
    puzzle::{
        label::label::Label, size::Size, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState,
    },
    solver::{
        projection::{
            LARGE,
            builder::SolverBuilder,
            encoding::{self, vertical_delta},
            pdb::{Pdb, compute_solved_state, compute_tally},
            puzzle::{ProjectedPuzzle, project_puzzle},
        },
        search::Search,
    },
};

/// The directions to search, in order. Bit `d` of a set of moves is `DIRECTIONS[d]`, and the
/// inverse of `DIRECTIONS[d]` is `DIRECTIONS[d ^ 2]`.
pub(super) const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

/// Returns, for each cell of the gap on a puzzle of size `size`, the cells that it moves to in
/// each of the [`DIRECTIONS`] (0 for moves that aren't possible), and the set of possible moves.
pub(super) fn moves_table(size: Size) -> Box<[([u8; 4], u8)]> {
    let (w, h) = (size.width() as usize, size.height() as usize);
    (0..w * h)
        .map(|gap| {
            let (x, y) = (gap % w, gap / w);
            let possible = [y + 1 < h, x + 1 < w, y > 0, x > 0];
            let cells = [gap + w, gap + 1, gap.wrapping_sub(w), gap.wrapping_sub(1)];
            let mut out = ([0; 4], 0);
            for d in 0..4 {
                if possible[d] {
                    out.0[d] = cells[d] as u8;
                    out.1 |= 1 << d;
                }
            }
            out
        })
        .collect()
}

/// An iterative deepening solver that uses a pattern database of projected puzzle states as its
/// heuristic.
///
/// The solver is generic over the puzzle type `P`, which can be any [`SlidingPuzzle`]; the size
/// of the puzzle must match the [`Size`] provided to the [`builder`](Solver::builder).
pub struct Solver<P, Target, PruneTarget, Metric> {
    pub(super) pdb: Pdb<Metric>,
    pub(super) tally: Box<[u8]>,
    pub(super) search: Search<128>,
    pub(super) size: Size,
    pub(super) target: Target,
    prune_target: PruneTarget,
    pub(super) prune_target_solved_index: u64,
    /// The [`encoding::weights`] of `tally`.
    pub(super) weights: [u64; LARGE],
    /// The [`moves_table`] for `size`.
    pub(super) moves: Box<[([u8; 4], u8)]>,
    pub(super) width: usize,
    phantom_p: PhantomData<P>,
    phantom_metric: PhantomData<Metric>,
}

impl<P, Target, PruneTarget, Metric> Solver<P, Target, PruneTarget, Metric>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    #[must_use]
    /// Creates a [`SolverBuilder`] for constructing a [`Solver`].
    pub fn builder() -> SolverBuilder<P, Target, PruneTarget, Metric> {
        SolverBuilder::new()
    }

    pub(super) fn with_pdb(
        pdb: Pdb<Metric>,
        target: Target,
        prune_target: PruneTarget,
        size: Size,
    ) -> Self {
        let prune_target_solved_state = compute_solved_state(&prune_target, size);
        let tally = compute_tally(&prune_target_solved_state).into_boxed_slice();
        let solved_projected = ProjectedPuzzle::<LARGE>::new(
            &prune_target_solved_state,
            size.num_pieces() as u8,
            size.width() as u8,
        );
        let prune_target_solved_index = solved_projected.encode(&tally);

        Self {
            pdb,
            weights: encoding::weights(&tally),
            tally,
            search: Search::new(),
            size,
            prune_target_solved_index,
            moves: moves_table(size),
            width: size.width() as usize,
            target,
            prune_target,
            phantom_p: PhantomData,
            phantom_metric: PhantomData,
        }
    }

    /// Returns a reference to the inner [`Pdb`].
    pub fn pdb(&self) -> &Pdb<Metric> {
        &self.pdb
    }

    pub(super) fn initial_projected<const N: usize>(&self, puzzle: &P) -> ProjectedPuzzle<N> {
        project_puzzle::<N, P, PruneTarget>(puzzle, &self.prune_target)
    }

    /// Returns the index of the state reached from the state with the given pieces, gap, and
    /// index by moving the gap to `cell` in direction `DIRECTIONS[d]`, where `child_pieces` are
    /// the pieces after the move.
    #[inline(always)]
    pub(super) fn child_index<const N: usize>(
        &self,
        pieces: &[u8; N],
        child_pieces: &[u8; N],
        gap: u8,
        cell: u8,
        index: u64,
        d: u32,
    ) -> u64 {
        match d {
            0 => index.wrapping_add(vertical_delta(
                pieces,
                gap as usize,
                self.width,
                &self.weights,
            )),
            1 => index + self.weights[0],
            2 => index.wrapping_sub(vertical_delta(
                child_pieces,
                cell as usize,
                self.width,
                &self.weights,
            )),
            _ => index - self.weights[0],
        }
    }

    /// Returns the [`Pdb`] entry of the state at `index`, which must be the index of a state.
    #[inline(always)]
    pub(super) fn bound(&self, index: u64) -> u64 {
        // SAFETY: `index` is the index of a state, which is within bounds because the PDB has an
        // entry for every state.
        u64::from(unsafe { self.pdb.get_unchecked(index as usize) })
    }

    pub(super) fn check_solution(&self, puzzle: &P) -> bool {
        let mut p = puzzle.clone();
        for dir in self.search.stack.iter() {
            p.try_move_dir(dir);
        }
        self.target.is_solved(&p)
    }
}
