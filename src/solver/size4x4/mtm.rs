//! A fast 4x4 [`Mtm`] solver using a ~62 MiB pattern database and ~3 MiB of indexing tables.
//!
//! The heuristic used is the distance from solved when the solved state is relabeled as:
//! ```text
//! 1 1 2 2
//! 3 2 2 2
//! 4 2 2 2
//! 5 6 7 0
//! ```
//!
//! This relabeling was found by a search over relabelings with at most 2^28 states, scored by the
//! number of nodes the solver searches on random puzzles. The database stores each state's
//! distance mod 3 in 2 bits, which is enough to recover the distances during the search.
//!
//! Double probing is used on the puzzle and its transpose, which massively speeds up the search.
//!
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod consts;
mod indexing_table;
pub mod pdb;
mod puzzle;
pub mod solver;
