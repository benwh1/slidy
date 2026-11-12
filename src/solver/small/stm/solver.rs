use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{direction::Direction, metric::Stm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        search::iterative_deepening,
        small::{indexing, pdb::Pdb, solver::Solver},
        solver::{Solver as SolverT, SolverError},
    },
};

impl<const W: usize, const H: usize, const N: usize> Default for Solver<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    /// Creates a [`Solver`], building a new pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self::with_pdb(Pdb::<_, _, _, Stm>::new(config))
    }

    fn dfs(
        &mut self,
        depth: u64,
        inverse_last_move: Option<Direction>,
        mut puzzle: Puzzle<W, H>,
    ) -> ControlFlow<()> {
        let coord = indexing::encode(puzzle.piece_array());

        // SAFETY: `encode` produces integers from 0 to k-1 where k is the size of the PDB, so the
        // index is always in bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(coord as usize) } as u64;
        if heuristic > depth {
            return ControlFlow::Continue(());
        }

        if depth == 0 {
            return self.search.record_solution();
        }

        let original_puzzle = puzzle;

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if inverse_last_move == Some(dir) {
                continue;
            }

            puzzle = original_puzzle;

            if puzzle.try_move_dir(dir) {
                self.search.stack.push(dir);

                if self.dfs(depth - 1, Some(dir.inverse()), puzzle).is_break() {
                    return ControlFlow::Break(());
                }

                self.search.stack.pop();
            }
        }

        ControlFlow::Continue(())
    }

    fn solve_impl<P>(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if p.try_set_state(puzzle) {
            return self.solve_small_puzzle_impl(p, config);
        }

        let mut p = Puzzle::<H, W>::new();
        if p.try_set_state(puzzle) {
            let solution_callback = config.solution_callback;
            let transpose_config = SolverConfig {
                solution_callback: Some(Box::new(move |s| {
                    solution_callback
                        .as_ref()
                        .map_or(ControlFlow::Continue(()), |f| f(s.transpose()))
                })),
                ..config
            };

            return self.solve_small_puzzle_impl(p.conjugate_with_transpose(), transpose_config);
        }

        Err(SolverError::IncompatiblePuzzleSize)
    }

    fn solve_small_puzzle_impl(
        &mut self,
        puzzle: Puzzle<W, H>,
        config: SolverConfig,
    ) -> Result<(), SolverError> {
        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let coord = indexing::encode(puzzle.piece_array());
        let hval = self.pdb.get(coord as usize) as u64;

        iterative_deepening::<Stm, _, _>(
            self,
            |s| &mut s.search,
            config,
            hval,
            |s, depth| s.dfs(depth, None, puzzle),
        );

        Ok(())
    }
}

impl<P, const W: usize, const H: usize, const N: usize> SolverT<P> for Solver<W, H, N, Stm>
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
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

    use crate::{
        puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
        solver::{
            Solver2x2Stm, Solver3x3Stm, Solver4x2Stm,
            config::SolverConfig,
            solver::{Solver as _, SolverError},
        },
    };

    #[test]
    fn test_solver() {
        let mut solver = Solver3x3Stm::default();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 25);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 25);
    }

    #[test]
    fn test_solver_2() {
        let mut solver = Solver4x2Stm::default();
        let mut puzzle = Puzzle::from_str("4 6/2 5/0 1/7 3").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(puzzle.is_solved());
    }

    #[test]
    fn test_min_deeper_than_initial_stack_capacity() {
        let mut solver = Solver2x2Stm::default();
        let mut puzzle = Puzzle::from_str("1 2/0 3").unwrap();
        let config = SolverConfig {
            min: 200,
            ..Default::default()
        };

        let solutions = solver.solve_collect(&puzzle, config).unwrap();
        assert_eq!(solutions.len(), 1);

        // The shortest solutions are `L` and `URDLURDLURD`, and repeating the 12 move cycle
        // around the 2x2 puzzle gives solutions of length `1 + 12k` and `11 + 12k`.
        let solution = &solutions[0];
        assert_eq!(solution.len_stm(), 203);
        puzzle.apply_alg(solution);
        assert!(puzzle.is_solved());
    }

    #[test]
    fn test_max_min_with_other_parity() {
        // The solution has even length, but `u64::MAX` is odd, so there are no depths to search.
        let mut solver = Solver2x2Stm::default();
        let puzzle = Puzzle::from_str("1 2/3 0").unwrap();
        let config = SolverConfig {
            min: u64::MAX,
            ..Default::default()
        };
        let solutions = solver.solve_collect(&puzzle, config).unwrap();
        assert!(solutions.is_empty());
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = Solver3x3Stm::default();
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
}
