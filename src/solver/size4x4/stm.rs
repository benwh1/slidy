//! Optimal 15-puzzle solver.
//!
//! A single-threaded IDA* search ([`large`]) over a set of pattern databases ([`pdb`]); board
//! geometry and solution rendering ([`utils`]).

#![allow(missing_docs, reason = "copied from solver3 as is; documented later")]

mod fsm;
pub mod large;
pub mod old;
pub mod pdb;
pub mod solver;
pub mod utils;
