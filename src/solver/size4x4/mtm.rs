//! A fast 4x4 [`Mtm`] solver using a ~144MiB pattern database.
//!
//! The heuristic used is the distance from solved when the solved state is relabelled as:
//! ```ignore
//! 1 1 2 2
//! 3 3 2 2
//! 3 3 4 4
//! 3 4 4 0
//! ```
//!
//! This relabelling was discovered by Tomas Rokicki.
//!
//! Double probing is used on the puzzle and its transpose, which massively speeds up the search.
//!
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod base_5_table;
mod consts;
mod indexing;
mod indexing_table;
mod pdb;
mod puzzle;
pub mod solver;
