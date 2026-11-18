//! Optimal 15-puzzle solver.
//!
//! A single-threaded IDA* search ([`large`]) generic over a set of
//! pattern-database tables ([`large::TableSet`]; the solver's is
//! [`Tables`]), board geometry and solution rendering ([`tables`]), and
//! table loading ([`pdb`]).

#![allow(missing_docs, reason = "copied from solver3 as is; documented later")]

pub mod build;
mod fsm;
pub mod generic;
pub mod lab;
pub mod large;
pub mod max7896;
pub mod old;
pub mod pdb;
pub mod sets;
mod shift;
pub mod tables;
mod walk;
pub mod x7896;
pub mod x9669;

/// The table set the solver uses: the max(9-6, 6-9) set ([`x9669`], 17.0 GB).
pub type Tables = x9669::X9669;
