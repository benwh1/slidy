use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::metric::Mtm,
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        directions::DIRECTIONS,
        residue,
        search::iterative_deepening,
        small::{coord::Dense, moves::Moves, pdb::Pdb, solver::Solver},
        solver::{Solver as SolverT, SolverError},
    },
};

impl<const W: usize, const H: usize, const N: usize> Default for Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    /// Creates a [`Solver`], building a new pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self::with_pdb(Pdb::<_, _, _, Mtm>::new(config))
    }

    /// Returns `d % 3`, where `d` is the optimal solution length of the state at `index`.
    #[inline(always)]
    fn entry(&self, index: u64) -> u64 {
        // SAFETY: `index` is always the index of a solvable state, computed by `Coord`, so it is
        // less than `N! / 2`.
        u64::from(unsafe { self.pdb.entry_unchecked(index) })
    }

    /// Returns the optimal solution length of the state with the given pieces, gap, and index.
    fn distance(&self, pieces: u64, gap: u8, index: u64) -> u64 {
        residue::walk_distance(
            (pieces, gap, index),
            self.entry(index),
            |&(_, _, index)| index == N as u64 - 1,
            |&state, closer| self.closer_neighbor(state, closer),
        )
    }

    /// Returns a state one move from the state `(pieces, gap, index)` whose entry is `closer`.
    #[inline(always)]
    fn closer_neighbor(&self, (pieces, gap, index): (u64, u8, u64), closer: u64) -> (u64, u8, u64) {
        let mut rest = Moves::<W, H, N>::MOVES[gap as usize].1;
        while rest != 0 {
            let d = rest.trailing_zeros();
            rest &= rest - 1;

            let (mut p, mut g, mut i) = (pieces, gap, index);
            loop {
                let (cells, moves) = Moves::<W, H, N>::MOVES[g as usize];
                if moves & (1 << d) == 0 {
                    break;
                }
                i = Moves::<W, H, N>::child_index(p, g, i, d);
                let cell = cells[d as usize];
                p = Dense::<W, H, N>::move_gap(p, g, cell);
                g = cell;
                if self.entry(i) == closer {
                    return (p, g, i);
                }
            }
        }

        unreachable!("every unsolved state has a neighbor closer to solved");
    }

    /// Finds solutions of length `depth` of the state with the given pieces, gap, index, and
    /// optimal solution length `distance`, where `moves` is the set of directions to try.
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

        let mut rest = moves;
        while rest != 0 {
            let d = rest.trailing_zeros();
            rest &= rest - 1;

            // The children can't move along the same axis.
            let axis = 0b0101 << (d & 1);

            // Move one more piece at a time, each a single move from the current state.
            let (mut p, mut g, mut i) = (pieces, gap, index);
            let mut count = 0;
            loop {
                let (cells, possible) = Moves::<W, H, N>::MOVES[g as usize];
                if possible & (1 << d) == 0 {
                    break;
                }
                i = Moves::<W, H, N>::child_index(p, g, i, d);
                let cell = cells[d as usize];
                p = Dense::<W, H, N>::move_gap(p, g, cell);
                g = cell;
                count += 1;
                self.search.stack.push(DIRECTIONS[d as usize]);

                let child_distance = residue::neighbor_distance(distance, self.entry(i));
                if child_distance > depth - 1 {
                    continue;
                }

                let child_moves = Moves::<W, H, N>::MOVES[g as usize].1 & !axis;
                if self
                    .dfs(depth - 1, child_distance, p, g, i, child_moves)
                    .is_break()
                {
                    return ControlFlow::Break(());
                }
            }

            self.search.stack.remove_n(count);
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

        iterative_deepening::<Mtm, _, _>(
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

impl<P, const W: usize, const H: usize, const N: usize> SolverT<P> for Solver<W, H, N, Mtm>
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

    use rand::SeedableRng as _;
    use rand_xoshiro::Xoshiro256StarStar;
    use xxhash_rust::xxh3::Xxh3;

    use super::*;
    use crate::{
        puzzle::{
            puzzle::Puzzle as GenericPuzzle,
            scrambler::{RandomState, Scrambler as _},
            size::Size,
        },
        solver::{
            Solver2x2Mtm, Solver3x2Mtm, Solver3x3Mtm, Solver4x2Mtm, Solver5x2Mtm,
            config::SolverConfig, solver::SolverError,
        },
    };

    /// Finds every solution up to `extra` moves longer than optimal of `count` random puzzles of
    /// size `w x h`, checking that they solve the puzzle and are no shorter than the first, and
    /// returns the number of solutions and a hash of them, sorted for each puzzle.
    fn solutions<const W: usize, const H: usize, const N: usize>(
        solver: &mut Solver<W, H, N, Mtm>,
        (w, h): (u64, u64),
        count: usize,
        extra: u64,
    ) -> (usize, u64)
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
        Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
    {
        let mut rng = Xoshiro256StarStar::seed_from_u64(w * 16 + h);
        let mut puzzle = GenericPuzzle::new(Size::new(w, h).unwrap());
        let mut hasher = Xxh3::new();
        let mut total = 0;

        for _ in 0..count {
            RandomState.scramble_with_rng(&mut puzzle, &mut rng);
            let config = SolverConfig {
                depth_beyond_optimal: extra,
                num_solutions: u64::MAX,
                ..Default::default()
            };
            let mut found = solver.solve_collect(&puzzle, config).unwrap();
            let optimal = found[0].len_mtm();
            let mut strings = Vec::new();
            for solution in &mut found {
                assert!(solution.len_mtm() >= optimal && solution.len_mtm() <= optimal + extra);
                let mut p = puzzle.clone();
                p.apply_alg(solution);
                assert!(p.is_solved(), "{solution} doesn't solve {puzzle}");
                strings.push(solution.to_string());
            }
            strings.sort();
            for s in strings {
                hasher.update(s.as_bytes());
                hasher.update(b"/");
            }
            hasher.update(b"|");
            total += found.len();
        }

        (total, hasher.digest())
    }

    /// Every solution of many random puzzles of each size up to a few moves longer than
    /// optimal, as found before the solvers used two-bit tables.
    #[test]
    fn test_solutions() {
        let mut solver = Solver2x2Mtm::default();
        assert_eq!(
            solutions(&mut solver, (2, 2), 50, 3),
            (65, 8430651083205791656)
        );

        let mut solver = Solver3x2Mtm::default();
        assert_eq!(
            solutions(&mut solver, (3, 2), 200, 2),
            (550, 17872102192224389719)
        );
        assert_eq!(
            solutions(&mut solver, (2, 3), 200, 2),
            (564, 12972436636240720025)
        );

        let mut solver = Solver4x2Mtm::default();
        assert_eq!(
            solutions(&mut solver, (4, 2), 200, 2),
            (1409, 1686962153956182663)
        );
        assert_eq!(
            solutions(&mut solver, (2, 4), 200, 2),
            (1630, 2342188709095477528)
        );

        let mut solver = Solver3x3Mtm::default();
        assert_eq!(
            solutions(&mut solver, (3, 3), 2000, 0),
            (3731, 549055487852366924)
        );
        assert_eq!(
            solutions(&mut solver, (3, 3), 300, 1),
            (1231, 10108444894142065340)
        );

        let mut solver = Solver5x2Mtm::default();
        assert_eq!(
            solutions(&mut solver, (5, 2), 1000, 0),
            (4918, 11503845386709953739)
        );
        assert_eq!(
            solutions(&mut solver, (2, 5), 1000, 0),
            (5208, 8239117298729703506)
        );
        assert_eq!(
            solutions(&mut solver, (5, 2), 200, 1),
            (2167, 8250669062889589542)
        );
        assert_eq!(
            solutions(&mut solver, (2, 5), 200, 1),
            (2894, 15746142114917349158)
        );
    }

    #[test]
    fn test_solver() {
        let mut solver = Solver3x3Mtm::default();
        let puzzle = GenericPuzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 18);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 18);
    }

    #[test]
    fn test_solver_2() {
        let mut solver = Solver4x2Mtm::default();
        let mut puzzle = GenericPuzzle::from_str("4 6/2 5/0 1/7 3").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(puzzle.is_solved());
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = Solver3x3Mtm::default();
        let puzzle = GenericPuzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = GenericPuzzle::from_str("2 1 3/4 5 6/7 8 0").unwrap();
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
