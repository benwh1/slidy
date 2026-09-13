//! Contains the main definition of a sliding puzzle as the [`SlidingPuzzle`] trait, and defines
//! many properties of sliding puzzles.
//!
//! [`SlidingPuzzle`]: sliding_puzzle/trait.SlidingPuzzle.html

#[cfg(feature = "palette")]
pub mod color_scheme;
#[cfg(feature = "palette")]
pub mod coloring;
pub mod display;
pub mod grids;
pub mod label;
pub mod puzzle;
#[cfg(feature = "palette")]
pub mod render;
pub mod scrambler;
pub mod size;
pub mod sliding_puzzle;
pub mod small;
pub mod solvable;
pub mod solved_state;
