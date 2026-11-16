//! [`Mtm`]-specific implementation of the projection [`Solver`].

use std::ops::ControlFlow;

use crate::{
    algorithm::metric::Mtm,
    puzzle::{
        label::label::Label, sliding_puzzle::SlidingPuzzle, solvable::Solvable,
        solved_state::SolvedState,
    },
    solver::{
        config::SolverConfig,
        directions::DIRECTIONS,
        projection::{LARGE, SMALL, puzzle::ProjectedPuzzle, solver::Solver},
        residue,
        search::iterative_deepening,
        solver::{Solver as SolverT, SolverError},
    },
};

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    /// Returns the [`Pdb`] entry of the state at `index`, which must be the index of a state.
    ///
    /// [`Pdb`]: crate::solver::projection::pdb::Pdb
    #[inline(always)]
    fn entry(&self, index: u64) -> u64 {
        // SAFETY: `index` is the index of a state, which is within bounds because the PDB has an
        // entry for every state.
        u64::from(unsafe { self.pdb.get_unchecked(index as usize) })
    }

    /// Returns the distance of `state`, which has index `index`, from the projected solved state,
    /// by following moves that get closer to it.
    fn distance<const N: usize>(&self, state: ProjectedPuzzle<N>, index: u64) -> u64 {
        residue::walk_distance(
            (state, index),
            u64::from(self.pdb.get(index as usize)),
            |&(_, index)| index == self.prune_target_solved_index,
            |(state, index), closer| {
                self.closer_neighbor(state, *index, closer)
                    .expect("pattern database has no move closer to solved")
            },
        )
    }

    /// Returns a state one move from `state`, which has index `index`, whose entry is `closer`,
    /// and its index.
    fn closer_neighbor<const N: usize>(
        &self,
        state: &ProjectedPuzzle<N>,
        index: u64,
        closer: u64,
    ) -> Option<(ProjectedPuzzle<N>, u64)> {
        let mut rest = self.moves.possible(state.gap());
        while rest != 0 {
            let d = rest.trailing_zeros();
            rest &= rest - 1;

            let (mut child, mut child_index) = (*state, index);
            while self.moves.possible(child.gap()) & (1 << d) != 0 {
                (child, child_index) = self.moves.apply(&child, child_index, d);
                if u64::from(self.pdb.get(child_index as usize)) == closer {
                    return Some((child, child_index));
                }
            }
        }
        None
    }

    /// Finds solutions of length `depth` of `state`, which has index `index` and distance `h`
    /// from the projected solved state, where `moves` is the set of directions to try. `depth`
    /// must be at least 1.
    fn dfs<const N: usize>(
        &mut self,
        depth: u64,
        state: ProjectedPuzzle<N>,
        index: u64,
        h: u64,
        moves: u8,
    ) -> ControlFlow<()> {
        let mut rest = moves;
        while rest != 0 {
            let d = rest.trailing_zeros();
            rest &= rest - 1;

            // The children can't move along the same axis.
            let axis = 0b0101 << (d & 1);

            // Move one more piece at a time, so that each move of `count` pieces is reached from
            // the move of `count - 1`.
            let (mut state, mut index) = (state, index);
            let mut count = 0;
            while self.moves.possible(state.gap()) & (1 << d) != 0 {
                (state, index) = self.moves.apply(&state, index, d);
                count += 1;
                self.search.stack.push(DIRECTIONS[d as usize]);

                // Children at depth 0 are only checked for being solved, not against the bound.
                if depth == 1 {
                    if index == self.prune_target_solved_index
                        && self.check_solution()
                        && self.search.record_solution().is_break()
                    {
                        return ControlFlow::Break(());
                    }
                } else {
                    let child_h = residue::neighbor_distance(h, self.entry(index));
                    if child_h < depth {
                        let child_moves = self.moves.possible(state.gap()) & !axis;
                        if self
                            .dfs(depth - 1, state, index, child_h, child_moves)
                            .is_break()
                        {
                            return ControlFlow::Break(());
                        }
                    }
                }
            }
            self.search.stack.remove_n(count);
        }

        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if self.size.area() as usize <= SMALL {
            self.solve_impl_n::<SMALL>(puzzle, config)
        } else {
            self.solve_impl_n::<LARGE>(puzzle, config)
        }
    }

    fn solve_impl_n<const N: usize>(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<(), SolverError> {
        if puzzle.size() != self.size {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !self.target.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let projected = self.initial_projected::<N>(puzzle);
        let index = projected.encode(&self.tally);
        let moves = self.moves.possible(projected.gap());
        let hval = self.distance(projected, index);

        iterative_deepening::<Mtm, _, _>(
            self,
            |s| &mut s.search,
            config,
            hval,
            |s, depth| {
                if depth == 0 {
                    if index == s.prune_target_solved_index && s.check_solution() {
                        return s.search.record_solution();
                    }
                    return ControlFlow::Continue(());
                }
                s.dfs::<N>(depth, projected, index, hval, moves)
            },
        );

        Ok(())
    }
}

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    type Context = ();

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        _context: &Self::Context,
    ) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use rand::SeedableRng as _;
    use rand_xoshiro::Xoroshiro128StarStar;

    use super::*;
    use crate::{
        puzzle::{
            label::label::{
                Checkerboard, ConcentricRectangles, Diagonals, Fringe, LastTwoRows, RowGrids, Rows,
                SplitFringe, SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
            },
            puzzle::Puzzle,
            scrambler::{RandomState, Scrambler as _},
            size::Size,
        },
        solver::{
            config::PdbConfig,
            projection::{builder::SolverBuilderError, pdb::Pdb},
            test_util::{self, PerSize},
        },
    };

    type Solver3x3MtmTrivial = Solver<Puzzle, Trivial, Trivial, Mtm>;
    type Solver3x3MtmRows = Solver<Puzzle, Rows, Rows, Mtm>;

    #[test]
    fn test_trivial() {
        let mut solver = Solver3x3MtmTrivial::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 2);
    }

    #[test]
    fn test_rows() {
        let mut solver = Solver3x3MtmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 13);
    }

    #[test]
    fn test_4x4_rows() {
        #[derive(Default)]
        struct Rows211;

        impl Label for Rows211 {
            fn position_label(&self, _: Size, (_, y): (u64, u64)) -> u64 {
                [0, 0, 1, 2][y as usize]
            }

            fn num_labels(&self, _: Size) -> u64 {
                3
            }
        }

        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Rows211, Mtm>::builder()
            .size(size)
            .target(Rows)
            .prune_target(Rows211)
            .pdb_config(PdbConfig {
                end_of_iter_callback: Some(Box::new(|s| {
                    println!("depth {} new {} total {}", s.depth, s.new, s.total);
                })),
            })
            .metric(Mtm)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("15 14 4 8/2 7 9 11/1 12 3 10/6 13 0 5").unwrap();
        let solution = solver.solve(&puzzle).unwrap();

        assert_eq!(solution.len_mtm(), 22);
    }

    #[test]
    fn test_solutions_distinct() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Mtm)
            .size(size)
            .build()
            .unwrap();

        let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
        let mut puzzle = Puzzle::new(size);

        for _ in 0..100 {
            RandomState.scramble_with_rng(&mut puzzle, &mut rng);

            let mut solutions = solver.solve_many(&puzzle, 5).unwrap();
            solutions.sort_by_cached_key(|s| s.to_string());
            solutions.dedup();

            assert_eq!(solutions.len(), 5, "failed on {puzzle}");
        }
    }

    #[test]
    fn test_existing_pdb_wrong_size() {
        let pdb = Pdb::<Mtm>::new(&Trivial, Size::new(2, 2).unwrap(), &PdbConfig::default());
        let result = Solver::<Puzzle, Trivial, Trivial, Mtm>::builder()
            .size(Size::new(3, 3).unwrap())
            .pdb(pdb)
            .build();

        assert!(matches!(result, Err(SolverBuilderError::IncompatiblePdb)));
    }

    #[test]
    fn test_existing_pdb_wrong_label() {
        let size = Size::new(3, 3).unwrap();
        let pdb = Pdb::<Mtm>::new(&Rows, size, &PdbConfig::default());
        let result = Solver::<Puzzle, Trivial, Trivial, Mtm>::builder()
            .size(size)
            .pdb(pdb)
            .build();

        assert!(matches!(result, Err(SolverBuilderError::IncompatiblePdb)));
    }

    #[test]
    fn test_existing_pdb_reused() {
        let size = Size::new(3, 3).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Rows, Mtm>::builder()
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let expected = solver.solve(&puzzle).unwrap().len::<Mtm>();

        let pdb = solver.pdb().clone();
        let mut solver = Solver::<Puzzle, Rows, Rows, Mtm>::builder()
            .size(size)
            .pdb(pdb)
            .build()
            .unwrap();

        assert_eq!(solver.solve(&puzzle).unwrap().len::<Mtm>(), expected);
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = Solver::<Puzzle, RowGrids, Trivial, Mtm>::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = Puzzle::from_str("2 1 3/4 5 6/7 8 0").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(
            solver.solve_collect(&unsolvable, config),
            Err(SolverError::Unsolvable)
        );
    }

    /// Checks the lengths in [`OPTIMAL`] for `target`, which has index `label`, pruning with
    /// `target` and with [`Trivial`].
    ///
    /// [`OPTIMAL`]: test_util::OPTIMAL
    fn check_optimal<T>(target: T, label: usize)
    where
        T: Label + SolvedState + Solvable + Default + Copy,
    {
        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, T, Mtm>::builder()
                .size(size)
                .target(target)
                .prune_target(target)
                .build()
                .unwrap()
        });
        test_util::check_optimal::<Mtm, _>(&mut solvers, &target, label);

        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, Trivial, Mtm>::builder()
                .size(size)
                .target(target)
                .build()
                .unwrap()
        });
        test_util::check_optimal::<Mtm, _>(&mut solvers, &target, label);
    }

    #[test]
    fn test_optimal() {
        check_optimal(Trivial, test_util::TRIVIAL);
        check_optimal(RowGrids, test_util::ROW_GRIDS);
        check_optimal(Rows, test_util::ROWS);
        check_optimal(Fringe, test_util::FRINGE);
        check_optimal(SquareFringe, test_util::SQUARE_FRINGE);
        check_optimal(SplitFringe, test_util::SPLIT_FRINGE);
        check_optimal(SplitSquareFringe, test_util::SPLIT_SQUARE_FRINGE);
        check_optimal(Diagonals, test_util::DIAGONALS);
        check_optimal(LastTwoRows, test_util::LAST_TWO_ROWS);
        check_optimal(SplitLastTwoRows, test_util::SPLIT_LAST_TWO_ROWS);
        check_optimal(ConcentricRectangles, test_util::CONCENTRIC_RECTANGLES);
        check_optimal(Checkerboard, test_util::CHECKERBOARD);
    }

    /// Checks [`test_util::check_configs`] for `target` with the solutions in `table`, pruning
    /// with `target`, and with [`Trivial`] for solutions of length at most 19.
    fn check_configs<T>(target: T, table: test_util::Solutions)
    where
        T: Label + SolvedState + Solvable + Default + Copy,
    {
        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, T, Mtm>::builder()
                .size(size)
                .target(target)
                .prune_target(target)
                .build()
                .unwrap()
        });
        test_util::check_configs::<Mtm>(&mut solvers, table, u64::MAX);

        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, Trivial, Mtm>::builder()
                .size(size)
                .target(target)
                .build()
                .unwrap()
        });
        test_util::check_configs::<Mtm>(&mut solvers, table, 19);
    }

    #[test]
    fn test_configs() {
        check_configs(RowGrids, test_util::ROW_GRIDS_MTM);
        check_configs(Rows, test_util::ROWS_MTM);
        check_configs(Trivial, test_util::TRIVIAL_MTM);
    }
}
