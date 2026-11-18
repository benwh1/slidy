//! Optimal 15-puzzle solver.
//!
//! A single-threaded IDA* search ([`solver`]) over pattern databases ([`pdb`]) chosen for each
//! memory budget.

pub mod old;
pub mod pdb;
pub mod solver;
mod utils;
