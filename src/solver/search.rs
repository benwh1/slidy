//! The iterative deepening search shared by all of the solvers.

use std::ops::ControlFlow;

use crate::{
    algorithm::metric::Metric,
    solver::{config::SolverConfig, stack::Stack, statistics::SolverIterationStats},
};

/// The state of a search that is shared by all of the solvers: the moves made so far, the number
/// of solutions found, and the configuration of the current search.
pub(super) struct Search<const N: usize> {
    pub(super) stack: Stack<N>,
    solutions_found: u64,
    config: SolverConfig,
}

impl<const N: usize> Search<N> {
    pub(super) fn new() -> Self {
        Self {
            stack: Stack::new(),
            solutions_found: 0,
            config: SolverConfig::default(),
        }
    }

    /// Records that the moves on the stack are a solution, and passes it to the solution
    /// callback.
    ///
    /// Returns [`ControlFlow::Break`] if the search should stop, either because the callback
    /// returned [`ControlFlow::Break`] or because enough solutions have been found.
    pub(super) fn record_solution(&mut self) -> ControlFlow<()> {
        self.solutions_found += 1;

        if let Some(f) = &self.config.solution_callback
            && f(self.stack.to_alg()).is_break()
        {
            return ControlFlow::Break(());
        }

        if self.config.num_solutions == self.solutions_found {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }
}

/// Runs an iterative deepening search using `config`.
///
/// `search` returns the [`Search`] that is part of `solver`. `lower_bound` is a lower bound on the
/// length of a solution in the metric `M`. `dfs(solver, depth)` searches for solutions of length
/// `depth`, calling [`Search::record_solution`] for each one, and returns
/// [`ControlFlow::Break`] if the search should stop.
pub(super) fn iterative_deepening<M, S, const N: usize>(
    solver: &mut S,
    search: impl Fn(&mut S) -> &mut Search<N>,
    config: SolverConfig,
    lower_bound: u64,
    mut dfs: impl FnMut(&mut S, u64) -> ControlFlow<()>,
) where
    M: Metric,
{
    if config.num_solutions == 0 {
        return;
    }

    let min = config.min;
    let max = config.max;
    let depth_beyond_optimal = config.depth_beyond_optimal;

    // Reset state
    let state = search(solver);
    state.stack.clear();
    state.solutions_found = 0;
    state.config = config;

    // In a metric with movecount parity, every solution has the same parity as `lower_bound`, so
    // only depths with that parity need to be searched.
    let (min, step) = if M::HAS_MOVECOUNT_PARITY {
        let min = if lower_bound % 2 == min % 2 {
            min
        } else {
            min + 1
        };
        (min, 2)
    } else {
        (min, 1)
    };

    let mut depth = lower_bound.max(min);

    if depth > max {
        return;
    }

    let mut first_solution_depth = None;

    loop {
        // Run DFS. This checks against `num_solutions` and the return value of the solution
        // callback.
        if dfs(solver, depth).is_break() {
            break;
        }

        let state = search(solver);

        // Set first solution depth.
        if first_solution_depth.is_none() && state.solutions_found > 0 {
            first_solution_depth = Some(depth);
        }

        // Run end of iteration callback and check return value.
        if let Some(f) = &state.config.end_of_iter_callback
            && f(SolverIterationStats { depth }).is_break()
        {
            break;
        }

        // Go to next depth.
        depth = match depth.checked_add(step) {
            Some(d) => d,
            None => break,
        };

        // Check against `max`.
        if depth > max {
            break;
        }

        // Check against `depth_beyond_optimal`.
        if first_solution_depth.is_some_and(|first| depth - first > depth_beyond_optimal) {
            break;
        }
    }
}
