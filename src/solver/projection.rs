//! Solvers that prune using a pattern database built from states projected onto a choice of
//! [`Label`].
//!
//! [`Label`]: crate::puzzle::label::label::Label

pub mod builder;
mod encoding;
pub mod mtm;
pub mod pdb;
mod puzzle;
pub mod solver;
mod stm;

const SMALL: usize = 16;
const LARGE: usize = 32;
