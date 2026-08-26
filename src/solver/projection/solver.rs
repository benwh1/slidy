use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

use num_traits::AsPrimitive;

use super::pdb::Pdb;
use crate::{
    algorithm::{
        axis::Axis,
        direction::Direction,
        metric::{Mtm, Stm},
    },
    puzzle::{
        label::label::Label,
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::{
        projection::puzzle::{project_puzzle, ProjectedPuzzle},
        solver::{Solver as SolverT, SolverConfig, SolverError},
        stack::Stack,
        statistics::{PdbIterationStats, SolverIterationStats},
    },
};

pub struct Solver<const W: usize, const H: usize, const N: usize, Target, PruneTarget, MetricTag> {
    pdb: Pdb,
    stack: Stack<128>,
    puzzle: Cell<Puzzle<W, H>>,
    solutions_found: Cell<u64>,
    config: RefCell<Option<SolverConfig>>,
    target: Target,
    prune_target: PruneTarget,
    _metric: PhantomData<MetricTag>,
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget> Default
    for Solver<W, H, N, Target, PruneTarget, Stm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget, MetricTag>
    Solver<W, H, N, Target, PruneTarget, MetricTag>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
    }

    fn with_pdb(pdb: Pdb, target: Target, prune_target: PruneTarget) -> Self {
        Self {
            pdb,
            stack: Stack::default(),
            puzzle: Cell::new(Puzzle::<W, H>::new()),
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
            target,
            prune_target,
            _metric: PhantomData,
        }
    }

    fn initial_projected(&self) -> ProjectedPuzzle<N> {
        project_puzzle::<W, H, N, Puzzle<W, H>, PruneTarget>(&self.puzzle.get(), &self.prune_target)
    }

    fn solved_state_arr(&self) -> [u8; N] {
        let mut arr = [0u8; N];
        arr.copy_from_slice(self.pdb.solved_state());
        arr
    }

    fn check_solution(&self) -> bool {
        let mut p = self.puzzle.get();
        for dir in self.stack.iter() {
            p.try_move_dir(dir);
        }
        self.target.is_solved(&p)
    }
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    Solver<W, H, N, Target, PruneTarget, Stm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let prune_target = PruneTarget::default();
        let target = Target::default();
        let pdb = Pdb::new_stm::<W, H, N, PruneTarget>(&prune_target, pdb_iteration_callback);
        Self::with_pdb(pdb, target, prune_target)
    }

    #[must_use]
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    pub fn with_prune_target(prune_target: PruneTarget) -> Self {
        let target = Target::default();
        let pdb = Pdb::new_stm::<W, H, N, PruneTarget>(&prune_target, None);
        Self::with_pdb(pdb, target, prune_target)
    }

    pub fn with_pdb_iteration_callback(callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(callback))
    }

    fn dfs(&self, depth: u8, last_dir: Option<Direction>, projected: ProjectedPuzzle<N>) -> bool {
        let solved = self.solved_state_arr();
        if projected.is_solved(&solved) && self.check_solution() {
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                f(self.stack.to_alg());
            }
            return self.cfg().num_solutions == self.solutions_found.get();
        }

        let idx = self.pdb.encode(&projected);
        let heuristic = self.pdb.get(idx);
        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            return false;
        }

        let original = projected;

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_dir == Some(dir.inverse()) {
                continue;
            }

            let mut proj = original;
            if proj.do_move::<W, H>(dir) {
                self.stack.push(dir);
                if self.dfs(depth - 1, Some(dir), proj) {
                    return true;
                }
                self.stack.pop();
            }
        }

        false
    }

    fn solve_impl<P>(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if !p.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;

        self.stack.clear();
        self.puzzle.set(p);
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let projected = self.initial_projected();
        let start_idx = self.pdb.encode(&projected);
        let mut depth = self.pdb.get(start_idx).max(min);

        while depth <= max {
            if self.dfs(depth, None, projected) {
                return Ok(());
            }

            if let Some(f) = &self.cfg().end_of_iter_callback {
                f(SolverIterationStats { depth });
            }

            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };
        }

        Err(SolverError::NoSolutionFound)
    }
}

impl<P, const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    SolverT<P, u8, Target, (), Stm> for Solver<W, H, N, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget> Default
    for Solver<W, H, N, Target, PruneTarget, Mtm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    Solver<W, H, N, Target, PruneTarget, Mtm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let prune_target = PruneTarget::default();
        let target = Target::default();
        let pdb = Pdb::new_mtm::<W, H, N, PruneTarget>(&prune_target, pdb_iteration_callback);
        Self::with_pdb(pdb, target, prune_target)
    }

    #[must_use]
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    pub fn with_pdb_iteration_callback(callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(callback))
    }

    fn dfs(&self, depth: u8, last_axis: Option<Axis>, projected: ProjectedPuzzle<N>) -> bool {
        let solved = self.solved_state_arr();
        if projected.is_solved(&solved) && self.check_solution() {
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                f(self.stack.to_alg());
            }
            return self.cfg().num_solutions == self.solutions_found.get();
        }

        let idx = self.pdb.encode(&projected);
        let heuristic = self.pdb.get(idx);
        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            return false;
        }

        let original = projected;

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_axis == Some(dir.into()) {
                continue;
            }

            let mut proj = original;
            let mut count = 0;
            while proj.do_move::<W, H>(dir) {
                count += 1;
                self.stack.push(dir);
                if self.dfs(depth - 1, Some(dir.into()), proj) {
                    return true;
                }
            }
            self.stack.remove_n(count);
        }

        false
    }

    fn solve_impl<P>(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if !p.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;

        self.stack.clear();
        self.puzzle.set(p);
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let projected = self.initial_projected();
        let start_idx = self.pdb.encode(&projected);
        let mut depth = self.pdb.get(start_idx).max(min);

        while depth <= max {
            if self.dfs(depth, None, projected) {
                return Ok(());
            }

            if let Some(f) = &self.cfg().end_of_iter_callback {
                f(SolverIterationStats { depth });
            }

            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };
        }

        Err(SolverError::NoSolutionFound)
    }
}

impl<P, const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    SolverT<P, u8, Target, (), Mtm> for Solver<W, H, N, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        algorithm::metric::{Mtm, Stm},
        puzzle::{
            label::{
                label::{Rows, Trivial},
                scaled::Scaled,
            },
            puzzle::Puzzle,
        },
    };

    type Solver3x3StmTrivial = Solver<3, 3, 9, Trivial, Trivial, Stm>;
    type Solver3x3MtmTrivial = Solver<3, 3, 9, Trivial, Trivial, Mtm>;
    type Solver3x3StmRows = Solver<3, 3, 9, Rows, Rows, Stm>;
    type Solver3x3MtmRows = Solver<3, 3, 9, Rows, Rows, Mtm>;
    type Solver3x3StmDiff = Solver<3, 3, 9, Rows, Trivial, Stm>;

    #[test]
    fn test_stm_trivial() {
        let solver = Solver3x3StmTrivial::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_mtm_trivial() {
        let solver = Solver3x3MtmTrivial::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_mtm::<u64>() > 0);
    }

    #[test]
    fn test_stm_rows() {
        let solver = Solver3x3StmRows::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_mtm_rows() {
        let solver = Solver3x3MtmRows::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_mtm::<u64>() > 0);
    }

    #[test]
    fn test_stm_different_targets() {
        let solver = Solver3x3StmDiff::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_solution_validates() {
        let solver = Solver3x3StmRows::new();
        let mut puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(Rows.is_solved(&puzzle));
    }

    #[test]
    fn test_solve_twice() {
        let solver = Solver3x3StmRows::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let s1 = solver.solve(&puzzle).unwrap();
        let s2 = solver.solve(&puzzle).unwrap();
        assert_eq!(s1.len_stm::<u64>(), s2.len_stm::<u64>());
    }

    #[test]
    fn test_stm_rows_double_rows_4x4() {
        let prune = Scaled::new(Rows, (2, 2)).unwrap();
        let solver = Solver::<4, 4, 16, Rows, Scaled<Rows>, Stm>::with_prune_target(prune);
        let puzzle = Puzzle::from_str("12 7 9 10/5 6 0 14/11 15 2 8/3 1 4 13").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 45);
    }
}
