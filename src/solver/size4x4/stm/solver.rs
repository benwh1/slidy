//! The optimal 4x4 STM solver: slidy's [`Solver`](SolverT) interface over the search
//! ([`large`]) with any of the table sets ([`sets`](super::sets)).

use std::{error::Error, num::NonZero, path::Path};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::metric::Stm,
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::{
        config::{PdbConfig, SolverConfig},
        search::{Search, iterative_deepening},
        size4x4::stm::{
            large::{Backtrack, Engine, Pruning, Root, TableSet},
            pdb::{
                build,
                region::Region,
                tables::{Gen, Spec},
            },
            utils::{self, Board},
        },
        solver::{Solver as SolverT, SolverError},
    },
};

/// An optimal solver for 4x4 puzzles in [`Stm`], with table set `H` (one of the
/// [`sets`](super::sets), which trade memory for speed).
pub struct Solver<H: TableSet> {
    region: Region,
    engine: Engine<H>,
    search: Search<128>,
}

impl<S, const N: usize> Solver<Gen<S, N>>
where
    S: Spec,
{
    /// Creates a new [`Solver`], building its tables in memory on every available thread.
    ///
    /// The small sets build in seconds; a set with 7-tile groups takes a minute or more, and the
    /// larger ones much longer: load those with [`Self::load`] instead.
    #[must_use]
    pub fn new(_config: &PdbConfig) -> Self {
        let threads = std::thread::available_parallelism().map_or(1, NonZero::get);
        let region = Region::build(&S::LAYOUT, |part, out| {
            let group = S::GROUPS[part];
            build::encode(group, &build::values(group, threads), out, threads);
        })
        .expect("allocating the tables");
        Self::with_region(region)
    }
}

impl<H> Solver<H>
where
    H: TableSet,
{
    fn with_region(region: Region) -> Self {
        Self {
            region,
            engine: Engine::default(),
            search: Search::new(),
        }
    }

    /// Creates a [`Solver`] from tables saved in `dir`: the set's combined file (see
    /// [`Self::save`]), or else its separate table files, read into memory.
    ///
    /// Only the files' sizes are checked: other tables of the right sizes give wrong solutions
    /// (or a panic), though never undefined behaviour.
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self::with_region(Region::preload(dir, &H::LAYOUT)?))
    }

    /// Creates a [`Solver`] that maps the set's combined file in `dir` read-only, so that
    /// processes using the same file share its memory. See [`Self::load`].
    pub fn map(dir: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self::with_region(Region::map(dir, &H::LAYOUT)?))
    }

    /// Saves the tables to `dir` as the set's combined file, for [`Self::load`].
    pub fn save(&self, dir: &Path) -> Result<(), Box<dyn Error>> {
        self.region.save(dir)
    }

    fn solve_impl<P>(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
    {
        if puzzle.size() != Size::new(4, 4).unwrap() {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let board: Board = std::array::from_fn(|i| puzzle.piece_at(i as u64).to_u8().unwrap());
        let root = Root::<H>::new(&utils::to_internal(&board), self.region.as_dbs());
        let lower_bound = root.lower_bound();

        // The duplicate-pruning automaton finds an optimal solution but not every solution, so
        // it serves only searches for one solution that start at or below the optimal length.
        let pruning = config.num_solutions == 1 && config.min <= lower_bound;

        iterative_deepening::<Stm, _, _>(
            self,
            |s| &mut s.search,
            config,
            lower_bound,
            |s, depth| {
                let Self {
                    region,
                    engine,
                    search,
                } = s;
                let dbs = region.as_dbs();
                let mut found = |cells: &[u8]| {
                    search.stack.clear();
                    // The blank's cells after each move, first move first.
                    let trail = cells[..cells.len() - 1].iter().rev();
                    for d in utils::directions(root.blank(), trail.map(|&c| usize::from(c))) {
                        search.stack.push(d);
                    }
                    search.record_solution()
                };
                if pruning {
                    engine.search::<Pruning>(dbs, &root, depth, &mut found)
                } else {
                    engine.search::<Backtrack>(dbs, &root, depth, &mut found)
                }
            },
        );

        Ok(())
    }
}

impl<P, H> SolverT<P> for Solver<H>
where
    P: SlidingPuzzle,
    H: TableSet,
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
    use std::{str::FromStr as _, sync::LazyLock};

    use super::*;
    use crate::{
        algorithm::algorithm::Algorithm, puzzle::puzzle::Puzzle,
        solver::size4x4::stm::pdb::sets::Tiny,
    };

    /// The tiny set's solver, built once and shared by the tests.
    static SOLVER: LazyLock<std::sync::Mutex<Solver<Tiny>>> =
        LazyLock::new(|| std::sync::Mutex::new(Solver::new(&PdbConfig::default())));

    fn solver() -> std::sync::MutexGuard<'static, Solver<Tiny>> {
        SOLVER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn test_solver() {
        let puzzle = Puzzle::from_str("12 15 5 1/11 9 2 13/0 10 8 6/14 7 4 3").unwrap();
        let mut solver = solver();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 58);
        let mut solved = puzzle.clone();
        solved.apply_alg(&solution);
        assert!(solved.is_solved());

        let solution = solver.solve(&puzzle).unwrap();
        drop(solver);
        assert_eq!(solution.len_stm(), 58);
    }

    #[test]
    fn test_solved() {
        let puzzle = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0").unwrap();
        assert_eq!(solver().solve(&puzzle), Ok(Algorithm::new()));
    }

    #[test]
    fn test_solve_all_optimal() {
        let puzzle = Puzzle::from_str("1 11 14 15/0 9 4 12/3 10 7 8/13 5 6 2").unwrap();
        let mut solutions = solver()
            .solve_all_optimal(&puzzle)
            .unwrap()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        solutions.sort();
        let expected = [
            "L2ULDRU2LDR2ULD3LU2RD2RU2RDLU2LD3RU3LD2LDR2UL2U2",
            "L2ULDRU2LDRD2LU2R2ULD3RU2RDLU2LD3RU3LD2LDR2UL2U2",
            "L2ULDRU2LDRD2LU2RD2RU3LD2RURDLU2LD3RU2LDLDR2UL2U2",
            "L2URUL2DRULD2R2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
            "L2URULDLDR2U2L2D2RDLU2RURDRDLULD2RU3LD3RU3LD2LU2",
            "L2URULDLDR2URDLU2L2D2RDLU2RURDLD2RU3LD3RU3LD2LU2",
            "L2URULDLURDLDR2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
            "LU2L2DRULD2R2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
            "LU2LDLURD2LURD2LU3RD3LU3RD2RURDLU2LD2RDLURULDLU2",
            "LU2LDLURDLDR2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
        ];
        assert_eq!(solutions, expected);
    }

    /// Every solution of a length, and a minimum above the optimum, match the old solver's.
    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the solver is used until the end"
    )]
    fn test_matches_old_solver() {
        let puzzle = Puzzle::from_str("5 1 2 3/9 6 7 4/0 10 11 8/13 14 15 12").unwrap();
        let mut old = crate::solver::size4x4::stm::old::solver::Solver::default();
        let mut solver = solver();
        let optimal = solver.solve(&puzzle).unwrap().len_stm();
        assert_eq!(optimal, old.solve(&puzzle).unwrap().len_stm());

        let lengths = |solutions: Vec<Algorithm>| {
            let mut lengths = solutions.iter().map(Algorithm::len_stm).collect::<Vec<_>>();
            lengths.sort_unstable();
            lengths
        };
        let above = || SolverConfig {
            min: optimal + 1,
            num_solutions: 3,
            ..Default::default()
        };
        let ours = solver.solve_collect(&puzzle, above()).unwrap();
        for solution in &ours {
            let mut solved = puzzle.clone();
            solved.apply_alg(solution);
            assert!(solved.is_solved());
        }
        assert_eq!(
            lengths(ours),
            lengths(old.solve_collect(&puzzle, above()).unwrap())
        );

        let all = |length| SolverConfig {
            min: length,
            max: length,
            num_solutions: u64::MAX,
            ..Default::default()
        };
        for length in [optimal, optimal + 8] {
            let sorted = |solutions: Vec<Algorithm>| {
                let mut out = solutions
                    .iter()
                    .map(Algorithm::to_string)
                    .collect::<Vec<_>>();
                out.sort();
                out
            };
            let ours = sorted(solver.solve_collect(&puzzle, all(length)).unwrap());
            assert!(!ours.is_empty());
            assert_eq!(
                ours,
                sorted(old.solve_collect(&puzzle, all(length)).unwrap())
            );
        }
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = solver();
        let puzzle = Puzzle::from_str("6 12 0 3/11 1 10 4/9 7 8 13/15 14 2 5").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = Puzzle::from_str("2 1 3 4/5 6 7 8/9 10 11 12/13 14 15 0").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(
            solver.solve_collect(&unsolvable, config),
            Err(SolverError::Unsolvable)
        );
    }

    #[test]
    fn test_wrong_size() {
        let puzzle = Puzzle::new(Size::new(3, 3).unwrap());
        assert_eq!(
            solver().solve(&puzzle),
            Err(SolverError::IncompatiblePuzzleSize)
        );
    }
}
