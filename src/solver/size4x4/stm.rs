//! An optimal solver for 4x4 puzzles in [`Stm`]: a single-threaded IDA* search ([`solver`]) with
//! a choice of pattern databases ([`pdb`]), from 5.9 MB to 17 GB, trading memory for speed.
//!
//! [`Stm`]: crate::algorithm::metric::Stm

pub mod pdb;
pub mod solver;
mod utils;
