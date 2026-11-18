//! Optimal 15-puzzle solver.
//!
//! A single-threaded IDA* search ([`solver`]) over pattern databases ([`pdb`]) chosen for each
//! memory budget, and the board helpers they share ([`utils`]).

#![allow(missing_docs, reason = "copied from solver3 as is; documented later")]

pub mod old;
pub mod pdb;
pub mod solver;
pub mod utils;
