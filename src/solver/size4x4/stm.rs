//! Optimal 15-puzzle solver.
//!
//! A single-threaded IDA* search ([`large`]) over a set of pattern-database tables
//! ([`large::TableSet`]), implemented once for any partitions and table formats ([`generic`])
//! with sets chosen for several memory budgets ([`sets`]); board geometry and solution rendering
//! ([`tables`]), table building ([`build`]) and loading ([`pdb`]).

#![allow(missing_docs, reason = "copied from solver3 as is; documented later")]

pub mod build;
mod fsm;
pub mod generic;
pub mod large;
pub mod old;
pub mod pdb;
pub mod sets;
pub mod tables;
