//! A crate containing various utilities for working with sliding puzzles. The only sliding puzzles
//! supported are arbitrary-sized versions of the
//! [15 puzzle](https://en.wikipedia.org/wiki/15_puzzle), other puzzles such as higher dimensional
//! variants of the 15 puzzle, bandaged sliding puzzles, klotski, sokoban, etc. are not supported.
//!
//! # Examples
//!
//! ## Apply a sequence of moves to a puzzle
//!
//! ```
//! use std::str::FromStr;
//!
//! use slidy::{
//!     algorithm::algorithm::Algorithm,
//!     puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle},
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut puzzle = Puzzle::from_str("8 2 0/4 6 1/3 7 5")?;
//!     let algorithm = Algorithm::from_str("R2U2LDLDRURDLULDRULURDLU")?;
//!     puzzle.apply_alg(&algorithm);
//!
//!     assert!(puzzle.is_solved());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Generate random state scrambles
//!
//! ```
//! use slidy::puzzle::{
//!     puzzle::Puzzle,
//!     scrambler::{RandomState, Scrambler},
//!     size::Size,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut p = Puzzle::new(Size::new(5, 5)?);
//!
//!     for _ in 0..10 {
//!         RandomState.scramble(&mut p);
//!         println!("{p}");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Find an optimal solution
//!
//! ```
//! use std::str::FromStr;
//!
//! use slidy::{
//!     puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle},
//!     solver::{heuristic::ManhattanDistance, solver::Solver},
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut puzzle = Puzzle::from_str("0 10 6 4/1 5 14 15/13 11 8 7/3 2 9 12")?;
//!
//!     let mut solver = Solver::default();
//!     let solution = solver.solve(&puzzle)?;
//!
//!     println!("Solution: {} ({} moves)", solution, solution.len_stm::<u32>());
//!
//!     puzzle.apply_alg(&solution);
//!     assert!(puzzle.is_solved());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Create an SVG image of a puzzle
//!
//! ```
//! use palette::rgb::Rgba;
//! use slidy::puzzle::{
//!     color_scheme::{Scheme, SchemeList},
//!     coloring::{Monochrome, RainbowBrightFull},
//!     label::labels::{SplitFringe, Trivial},
//!     puzzle::Puzzle,
//!     render::{Borders, RendererBuilder, Text},
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let schemes = [Scheme::new(
//!         Trivial,
//!         Monochrome::new(Rgba::new(0.15, 0.15, 0.15, 1.0)),
//!     )];
//!     let scheme_list = SchemeList::new(&schemes)?;
//!
//!     let renderer = RendererBuilder::with_scheme(&scheme_list)
//!         .borders(Borders::with_scheme(Scheme::new(SplitFringe, RainbowBrightFull)).thickness(5.0))
//!         .text(
//!             Text::with_scheme(Scheme::new(
//!                 Trivial,
//!                 Monochrome::new(Rgba::new(1.0, 1.0, 1.0, 1.0)),
//!             ))
//!             .font_size(40.0),
//!         )
//!         .background_color(Rgba::new(0.05, 0.05, 0.05, 1.0))
//!         .tile_size(75.0)
//!         .tile_gap(5.0)
//!         .tile_rounding(10.0)
//!         .padding(10.0)
//!         .build();
//!
//!     let puzzle = Puzzle::default();
//!     let svg = renderer.render(&puzzle)?;
//!
//!     svg::save("out.svg", &svg)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Safe, panicking, and unsafe functions
//!
//! Some functions defined in this crate have variants with names of the form `foo`, `try_foo`, and
//! `foo_unchecked`, with the following behavior:
//!
//! - The functions `foo` may panic, return invalid results, or create invalid states when given
//! invalid arguments.
//! - The functions `try_foo` should return `None` when given invalid arguments, and should never
//! panic. In most cases, the default implementations of these functions call `foo` with the
//! appropriate checks included.
//! - The functions `foo_unchecked` should be considered `unsafe` and are intended for situations
//! where performance is important. The default implementations of these functions do not contain
//! any unsafe code, and most of them are just a call to `foo` or a re-implementation of `foo`
//! using other unchecked functions.

#![feature(if_let_guard)]
#![feature(int_roundings)]
#![feature(iter_intersperse)]
#![feature(let_chains)]
#![feature(slice_group_by)]
#![feature(slice_swap_unchecked)]
#![feature(test)]
#![allow(clippy::module_inception)]
#![warn(clippy::must_use_candidate)]
#![deny(clippy::use_self)]
#![deny(clippy::double_must_use)]
#![deny(clippy::if_not_else)]
#![deny(clippy::inconsistent_struct_constructor)]
#![deny(clippy::iter_not_returning_iterator)]
#![deny(clippy::map_unwrap_or)]
#![deny(clippy::mod_module_files)]
#![deny(clippy::semicolon_if_nothing_returned)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod algorithm;
pub mod puzzle;
pub mod solver;
