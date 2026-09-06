//! Solvers that prune using a pattern database built from states projected onto a choice of
//! [`Label`].

pub mod builder;
mod encoding;
pub mod mtm;
mod pdb;
mod puzzle;
pub mod solver;
mod stm;
