use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::metric::Stm,
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        directions::DIRECTIONS,
        search::iterative_deepening,
        small::{coord::Dense, moves::Moves, pdb::Pdb, solver::Solver},
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

    /// Returns `(d >> 1) & 1`, where `d` is the optimal solution length of the state at `index`.
    #[inline(always)]
    fn bit(&self, index: u64) -> u8 {
        // SAFETY: `index` is always the index of a solvable state, computed by `Coord`, so it is
        // less than `N! / 2`.
        unsafe { self.pdb.bit_unchecked(index) }
    }

    /// Returns the optimal solution length of the state with the given pieces, gap, and index.
    fn distance(&self, mut pieces: u64, mut gap: u8, mut index: u64) -> u64 {
        // The solution length mod 4: the parity is the parity of the distance of the gap from the
        // bottom right corner, and the second bit is in the PDB.
        let (gx, gy) = (gap as usize % W, gap as usize / W);
        let mut d4 = (((W - 1 - gx) + (H - 1 - gy)) % 2) as u64 + 2 * u64::from(self.bit(index));
        let mut distance = 0;

        while index != N as u64 - 1 {
            // Exactly the neighbors closer to solved have this bit.
            let closer = (d4.wrapping_sub(1) >> 1) as u8 & 1;

            let (cells, mut moves) = Moves::<W, H, N>::MOVES[gap as usize];
            loop {
                let d = moves.trailing_zeros();
                moves &= moves - 1;
                let child = Moves::<W, H, N>::child_index(pieces, gap, index, d);
                if self.bit(child) == closer {
                    let cell = cells[d as usize];
                    pieces = Dense::<W, H, N>::move_gap(pieces, gap, cell);
                    gap = cell;
                    index = child;
                    break;
                }
            }

            d4 = (d4 + 3) % 4;
            distance += 1;
        }

        distance
    }

    /// Finds solutions of length `depth` of the state with the given pieces, gap, index, and
    /// optimal solution length `distance`, where `moves` is the set of moves to try.
    fn dfs(
        &mut self,
        depth: u64,
        distance: u64,
        pieces: u64,
        gap: u8,
        index: u64,
        moves: u8,
    ) -> ControlFlow<()> {
        if depth == 0 {
            return self.search.record_solution();
        }

        // Exactly the children closer to solved have this bit.
        let closer = (distance.wrapping_sub(1) >> 1) as u8 & 1;

        let cells = Moves::<W, H, N>::MOVES[gap as usize].0;
        let mut rest = moves;
        while rest != 0 {
            let d = rest.trailing_zeros();
            rest &= rest - 1;

            let child = Moves::<W, H, N>::child_index(pieces, gap, index, d);
            let child_distance = if distance > 0 && self.bit(child) == closer {
                distance - 1
            } else {
                distance + 1
            };
            if child_distance > depth - 1 {
                continue;
            }

            let cell = cells[d as usize];
            let child_pieces = Dense::<W, H, N>::move_gap(pieces, gap, cell);
            let child_moves = Moves::<W, H, N>::MOVES[cell as usize].1 & !(1 << (d ^ 2));

            self.search.stack.push(DIRECTIONS[d as usize]);

            if self
                .dfs(
                    depth - 1,
                    child_distance,
                    child_pieces,
                    cell,
                    child,
                    child_moves,
                )
                .is_break()
            {
                return ControlFlow::Break(());
            }

            self.search.stack.pop();
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
        let (pieces, gap) = (puzzle.pieces(), puzzle.gap());
        if !Dense::<W, H, N>::is_solvable(pieces, gap) {
            return Err(SolverError::Unsolvable);
        }

        let index = Dense::<W, H, N>::index(pieces, gap);
        let distance = self.distance(pieces, gap, index);
        let moves = Moves::<W, H, N>::MOVES[gap as usize].1;

        iterative_deepening::<Stm, _, _>(
            self,
            |s| &mut s.search,
            config,
            distance,
            |s, depth| {
                // `iterative_deepening` starts at `distance` or deeper. `dfs` relies on this, as it
                // records a solution at depth 0 without checking that the puzzle is solved.
                assert!(distance <= depth);
                s.dfs(depth, distance, pieces, gap, index, moves)
            },
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
