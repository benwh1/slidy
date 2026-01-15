//! A fast, low-memory, 4x4-specific solver using pattern databases of 3 and 4 pieces, based on
//! the [ida15] solver by Robert Hilchie.
//!
//! The pattern databases are defined by splitting the puzzle into four quadrants, each containing
//! 3 or 4 pieces. Only two are stored in memory, because the three quadrants with 4 pieces are all
//! equivalent by symmetry.
//!
//! During the search, the puzzle is represented using four coordinates and moves are applied using
//! a transposition table.
//!
//! [ida15]: https://web.ncf.ca/aa576/

mod pattern;
mod pdb;
mod puzzle;
pub mod solver;
