//! [`Stm`]-specific implementation of the projection [`Solver`].

use std::ops::ControlFlow;

use crate::{
    algorithm::metric::Stm,
    puzzle::{
        label::label::Label, sliding_puzzle::SlidingPuzzle, solvable::Solvable,
        solved_state::SolvedState,
    },
    solver::{
        config::SolverConfig,
        projection::{
            LARGE, SMALL,
            puzzle::{DIRECTIONS, ProjectedPuzzle},
            solver::Solver,
        },
        search::iterative_deepening,
        solver::{Solver as SolverT, SolverError},
    },
};

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Stm>
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
        let (mut state, mut index) = (state, index);

        // The PDB entry gives bit 1 of the distance, and the parity of the gap's distance from
        // its solved cell gives bit 0.
        let mut residue =
            2 * u64::from(self.pdb.get(index as usize)) + u64::from(self.moves.parity(state.gap()));
        let mut distance = 0;

        while index != self.prune_target_solved_index {
            // Every move changes the distance by 1, so the neighbours have residues `residue ± 1`
            // mod 4, which have different entries.
            let closer = (residue + 3) % 4;
            let mut rest = self.moves.possible(state.gap());
            loop {
                assert!(rest != 0, "pattern database has no move closer to solved");
                let d = rest.trailing_zeros();
                rest &= rest - 1;

                let (child, child_index) = self.moves.apply(&state, index, d);
                if u64::from(self.pdb.get(child_index as usize)) == closer >> 1 {
                    (state, index) = (child, child_index);
                    break;
                }
            }
            residue = closer;
            distance += 1;
        }

        distance
    }

    /// Finds solutions of length `depth` of `state`, which has index `index` and distance `h`
    /// from the projected solved state, where `moves` is the set of moves to try. `depth` must be
    /// at least 1.
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

            let (child_state, child) = self.moves.apply(&state, index, d);

            // Children at depth 0 are only checked for being solved, not against the bound.
            if depth == 1 {
                if child == self.prune_target_solved_index {
                    self.search.stack.push(DIRECTIONS[d as usize]);
                    if self.check_solution() && self.search.record_solution().is_break() {
                        return ControlFlow::Break(());
                    }
                    self.search.stack.pop();
                }
                continue;
            }

            // The child's distance is `h ± 1`, and the entry is bit 1 of it, which is different
            // for the two.
            let up = h + 1;
            let child_h = up.wrapping_sub(2 * ((self.entry(child) ^ (up >> 1)) & 1));
            if child_h >= depth {
                continue;
            }

            let child_moves = self.moves.possible(child_state.gap()) & !(1 << (d ^ 2));
            self.search.stack.push(DIRECTIONS[d as usize]);
            if self
                .dfs(depth - 1, child_state, child, child_h, child_moves)
                .is_break()
            {
                return ControlFlow::Break(());
            }
            self.search.stack.pop();
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

        iterative_deepening::<Stm, _, _>(
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

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Stm>
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
    use std::{
        ops::ControlFlow,
        str::FromStr as _,
        sync::{
            Arc, Mutex,
            atomic::{AtomicU64, Ordering},
        },
    };

    use rand::SeedableRng as _;
    use rand_xoshiro::Xoroshiro128StarStar;

    use super::*;
    use crate::{
        puzzle::{
            label::{
                label::{
                    Checkerboard, ConcentricRectangles, Diagonals, Fringe, LastTwoRows, RowGrids,
                    Rows, SplitFringe, SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
                },
                scaled::Scaled,
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

    type Solver3x3StmTrivial = Solver<Puzzle, Trivial, Trivial, Stm>;
    type Solver3x3StmRows = Solver<Puzzle, Rows, Rows, Stm>;
    type Solver3x3StmDiff = Solver<Puzzle, Rows, Trivial, Stm>;

    #[test]
    fn test_trivial() {
        let mut solver = Solver3x3StmTrivial::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_rows() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_different_targets() {
        let mut solver = Solver3x3StmDiff::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_solution_validates() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let mut puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(Rows.is_solved(&puzzle));
    }

    #[test]
    fn test_solve_twice() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let s1 = solver.solve(&puzzle).unwrap();
        let s2 = solver.solve(&puzzle).unwrap();
        assert_eq!(s1.len_stm(), s2.len_stm());
    }

    #[test]
    fn test_rows_double_rows_4x4() {
        let size = Size::new(4, 4).unwrap();
        let prune = Scaled::new(Rows, (2, 2)).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Scaled<Rows>, Stm>::builder()
            .size(size)
            .prune_target(prune)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("12 7 9 10/5 6 0 14/11 15 2 8/3 1 4 13").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 45);
    }

    #[test]
    fn test_rows_with_pdb_iteration_callback() {
        let iterations = Arc::new(AtomicU64::new(0));
        let iterations_ref = iterations.clone();
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .pdb_config(PdbConfig {
                end_of_iter_callback: Some(Box::new(move |stats| {
                    assert!(stats.total > 0);
                    iterations_ref.fetch_add(1, Ordering::Relaxed);
                })),
            })
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
        assert!(iterations.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_solutions_distinct() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
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
    fn test_depth_beyond_optimal_bounds_solutions() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();
        let optimal = solver.solve(&puzzle).unwrap().len_stm();

        for (depth_beyond_optimal, max_len) in [(0, optimal), (2, optimal + 2)] {
            let solutions = Arc::new(Mutex::new(Vec::new()));
            let sc = solutions.clone();
            let config = SolverConfig {
                depth_beyond_optimal,
                num_solutions: 1000,
                solution_callback: Some(Box::new(move |s| {
                    sc.lock().unwrap().push(s.len_stm());
                    ControlFlow::Continue(())
                })),
                ..Default::default()
            };

            solver.solve_with_config(&puzzle, config).unwrap();

            let lens = solutions.lock().unwrap();
            assert!(
                !lens.is_empty(),
                "no solutions for depth_beyond_optimal={depth_beyond_optimal:?}"
            );
            for len in lens.iter() {
                assert!(
                    *len <= max_len && *len >= optimal,
                    "len {len} outside [{optimal}, {max_len}] for depth_beyond_optimal={depth_beyond_optimal:?}"
                );
            }
        }
    }

    #[test]
    fn test_solution_callback_break_stops() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();

        let calls = Arc::new(AtomicU64::new(0));
        let cc = calls.clone();
        let config = SolverConfig {
            num_solutions: u64::MAX,
            solution_callback: Some(Box::new(move |_| {
                cc.fetch_add(1, Ordering::Relaxed);
                ControlFlow::Break(())
            })),
            ..Default::default()
        };

        let result = solver.solve_with_config(&puzzle, config);

        assert_eq!(result, Ok(()));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_end_of_iter_callback_break_stops() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();

        let config = SolverConfig {
            num_solutions: u64::MAX,
            end_of_iter_callback: Some(Box::new(|_| ControlFlow::Break(()))),
            ..Default::default()
        };

        let result = solver.solve_with_config(&puzzle, config);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_existing_pdb_wrong_size() {
        let pdb = Pdb::<Stm>::new(&Trivial, Size::new(2, 2).unwrap(), &PdbConfig::default());
        let result = Solver::<Puzzle, Trivial, Trivial, Stm>::builder()
            .size(Size::new(3, 3).unwrap())
            .pdb(pdb)
            .build();

        assert!(matches!(result, Err(SolverBuilderError::IncompatiblePdb)));
    }

    #[test]
    fn test_existing_pdb_wrong_label() {
        let size = Size::new(3, 3).unwrap();
        let pdb = Pdb::<Stm>::new(&Rows, size, &PdbConfig::default());
        let result = Solver::<Puzzle, Trivial, Trivial, Stm>::builder()
            .size(size)
            .pdb(pdb)
            .build();

        assert!(matches!(result, Err(SolverBuilderError::IncompatiblePdb)));
    }

    #[test]
    fn test_existing_pdb_reused() {
        let size = Size::new(3, 3).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Rows, Stm>::builder()
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let expected = solver.solve(&puzzle).unwrap().len::<Stm>();

        let pdb = solver.pdb().clone();
        let mut solver = Solver::<Puzzle, Rows, Rows, Stm>::builder()
            .size(size)
            .pdb(pdb)
            .build()
            .unwrap();

        assert_eq!(solver.solve(&puzzle).unwrap().len::<Stm>(), expected);
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = Solver::<Puzzle, RowGrids, Trivial, Stm>::builder()
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
            Solver::<Puzzle, T, T, Stm>::builder()
                .size(size)
                .target(target)
                .prune_target(target)
                .build()
                .unwrap()
        });
        test_util::check_optimal::<Stm, _>(&mut solvers, &target, label);

        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, Trivial, Stm>::builder()
                .size(size)
                .target(target)
                .build()
                .unwrap()
        });
        test_util::check_optimal::<Stm, _>(&mut solvers, &target, label);
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
    /// with `target`, and with [`Trivial`] for solutions of length at most 21.
    fn check_configs<T>(target: T, table: test_util::Solutions)
    where
        T: Label + SolvedState + Solvable + Default + Copy,
    {
        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, T, Stm>::builder()
                .size(size)
                .target(target)
                .prune_target(target)
                .build()
                .unwrap()
        });
        test_util::check_configs::<Stm>(&mut solvers, table, u64::MAX);

        let mut solvers = PerSize::new(|size| {
            Solver::<Puzzle, T, Trivial, Stm>::builder()
                .size(size)
                .target(target)
                .build()
                .unwrap()
        });
        test_util::check_configs::<Stm>(&mut solvers, table, 21);
    }

    #[test]
    fn test_configs() {
        check_configs(RowGrids, test_util::ROW_GRIDS_STM);
        check_configs(Rows, test_util::ROWS_STM);
        check_configs(Trivial, test_util::TRIVIAL_STM);
    }
}
