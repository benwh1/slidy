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
//! use std::str::FromStr as _;
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
//! # #[cfg(feature = "thread_rng")]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut p = Puzzle::new(Size::new(5, 5)?);
//!
//!     for _ in 0..10 {
//!         // Requires the `thread_rng` feature to be enabled.
//!         // Otherwise, `scramble_with_rng` can be used with a custom `Rng`.
//!         RandomState.scramble(&mut p);
//!         println!("{p}");
//!     }
//!
//!     Ok(())
//! }
//!
//! # #[cfg(not(feature = "thread_rng"))]
//! # fn main() {}
//! ```
//!
//! ## Find an optimal solution
//!
//! ```
//! use std::str::FromStr as _;
//!
//! use slidy::{
//!     puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle},
//!     solver::solver::Solver,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut puzzle = Puzzle::from_str("0 10 6 4/1 5 14 15/13 11 8 7/3 2 9 12")?;
//!
//!     let mut solver = Solver::default();
//!     let solution = solver.solve(&puzzle)?;
//!
//!     println!("Solution: {} ({} moves)", solution, solution.len_stm::<u64>());
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
//!     color_scheme::{ColorScheme, Scheme},
//!     coloring::{Monochrome, Rainbow},
//!     label::label::{SplitFringe, Trivial},
//!     puzzle::Puzzle,
//!     render::{Borders, Renderer, RendererBuilder, Text},
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let scheme = Scheme::new(
//!         Trivial,
//!         Monochrome::new(Rgba::new(0.15, 0.15, 0.15, 1.0)),
//!     );
//!     let border_scheme = Scheme::new(SplitFringe, Rainbow::default());
//!     let text_scheme = Scheme::new(Trivial, Monochrome::new(Rgba::new(1.0, 1.0, 1.0, 1.0)));
//!
//!     let renderer = RendererBuilder::with_dyn_scheme(&scheme)
//!         .borders(Borders::with_scheme(&border_scheme as &dyn ColorScheme).thickness(5.0))
//!         .text(Text::with_scheme(&text_scheme as &dyn ColorScheme).font_size(40.0))
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
//!   invalid arguments.
//! - The functions `try_foo` should return `None` when given invalid arguments, and should never
//!   panic. In most cases, the default implementations of these functions call `foo` with the
//!   appropriate checks included.
//! - The functions `foo_unchecked` should be considered `unsafe` and are intended for situations
//!   where performance is important. The default implementations of these functions do not contain
//!   any unsafe code, and most of them are just a call to `foo` or a re-implementation of `foo`
//!   using other unchecked functions.

#![cfg_attr(feature = "nightly", feature(test))]
#![allow(clippy::module_inception)]
#![deny(clippy::allow_attributes)]
#![deny(clippy::as_pointer_underscore)]
#![deny(clippy::as_ptr_cast_mut)]
#![deny(clippy::assertions_on_result_states)]
#![deny(clippy::bool_to_int_with_if)]
#![deny(clippy::borrow_as_ptr)]
#![deny(clippy::branches_sharing_code)]
#![deny(clippy::checked_conversions)]
#![deny(clippy::clear_with_drain)]
#![deny(clippy::cloned_instead_of_copied)]
#![deny(clippy::collection_is_never_read)]
#![deny(clippy::deref_by_slicing)]
#![deny(clippy::derive_partial_eq_without_eq)]
#![deny(clippy::doc_markdown)]
#![deny(clippy::double_must_use)]
#![deny(clippy::elidable_lifetime_names)]
#![deny(clippy::empty_drop)]
#![deny(clippy::empty_enum)]
#![deny(clippy::empty_enum_variants_with_brackets)]
#![deny(clippy::empty_structs_with_brackets)]
#![deny(clippy::enum_glob_use)]
#![deny(clippy::equatable_if_let)]
#![deny(clippy::error_impl_error)]
#![deny(clippy::explicit_deref_methods)]
#![deny(clippy::explicit_into_iter_loop)]
#![deny(clippy::explicit_iter_loop)]
#![deny(clippy::expl_impl_clone_on_copy)]
#![deny(clippy::fallible_impl_from)]
#![deny(clippy::filter_map_next)]
#![deny(clippy::flat_map_option)]
#![deny(clippy::float_cmp)]
#![deny(clippy::fn_params_excessive_bools)]
#![deny(clippy::fn_to_numeric_cast_any)]
#![deny(clippy::format_collect)]
#![deny(clippy::format_push_string)]
#![deny(clippy::from_iter_instead_of_collect)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::if_not_else)]
#![deny(clippy::if_then_some_else_none)]
#![deny(clippy::ignored_unit_patterns)]
#![deny(clippy::implicit_clone)]
#![deny(clippy::implicit_hasher)]
#![deny(clippy::imprecise_flops)]
#![deny(clippy::inconsistent_struct_constructor)]
#![deny(clippy::index_refutable_slice)]
#![deny(clippy::inefficient_to_string)]
#![deny(clippy::infinite_loop)]
#![deny(clippy::inline_always)]
#![deny(clippy::into_iter_without_iter)]
#![deny(clippy::invalid_upcast_comparisons)]
#![deny(clippy::items_after_statements)]
#![deny(clippy::iter_filter_is_ok)]
#![deny(clippy::iter_filter_is_some)]
#![deny(clippy::iter_not_returning_iterator)]
#![deny(clippy::iter_on_empty_collections)]
#![deny(clippy::iter_on_single_items)]
#![deny(clippy::iter_with_drain)]
#![deny(clippy::iter_without_into_iter)]
#![deny(clippy::large_stack_arrays)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::large_types_passed_by_value)]
#![deny(clippy::literal_string_with_formatting_args)]
#![deny(clippy::lossy_float_literal)]
#![deny(clippy::manual_assert)]
#![deny(clippy::manual_instant_elapsed)]
#![deny(clippy::manual_is_power_of_two)]
#![deny(clippy::manual_is_variant_and)]
#![deny(clippy::manual_let_else)]
#![deny(clippy::manual_midpoint)]
#![deny(clippy::manual_string_new)]
#![deny(clippy::map_err_ignore)]
#![deny(clippy::map_unwrap_or)]
#![deny(clippy::map_with_unused_argument_over_ranges)]
#![deny(clippy::match_bool)]
#![deny(clippy::match_wildcard_for_single_variants)]
#![deny(clippy::mismatching_type_param_order)]
#![deny(clippy::missing_asserts_for_indexing)]
#![deny(clippy::mod_module_files)]
#![deny(clippy::multiple_inherent_impl)]
#![deny(clippy::mutex_atomic)]
#![deny(clippy::mutex_integer)]
#![deny(clippy::mut_mut)]
#![deny(clippy::needless_bitwise_bool)]
#![deny(clippy::needless_collect)]
#![deny(clippy::needless_continue)]
#![deny(clippy::needless_for_each)]
#![deny(clippy::needless_pass_by_ref_mut)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::needless_raw_string_hashes)]
#![deny(clippy::needless_raw_strings)]
#![deny(clippy::negative_feature_names)]
#![deny(clippy::no_effect_underscore_binding)]
#![deny(clippy::non_send_fields_in_send_ty)]
#![deny(clippy::nonstandard_macro_braces)]
#![deny(clippy::non_std_lazy_statics)]
#![deny(clippy::non_zero_suggestions)]
#![deny(clippy::option_as_ref_cloned)]
#![deny(clippy::option_if_let_else)]
#![deny(clippy::option_option)]
#![deny(clippy::or_fun_call)]
#![deny(clippy::partialeq_to_none)]
#![deny(clippy::path_buf_push_overwrite)]
#![deny(clippy::ptr_cast_constness)]
#![deny(clippy::pub_underscore_fields)]
#![deny(clippy::rc_buffer)]
#![deny(clippy::rc_mutex)]
#![deny(clippy::read_zero_byte_vec)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::redundant_else)]
#![deny(clippy::redundant_feature_names)]
#![deny(clippy::redundant_pub_crate)]
#![deny(clippy::redundant_type_annotations)]
#![deny(clippy::ref_as_ptr)]
#![deny(clippy::ref_binding_to_reference)]
#![deny(clippy::ref_option)]
#![deny(clippy::ref_option_ref)]
#![deny(clippy::renamed_function_params)]
#![deny(clippy::rest_pat_in_fully_bound_structs)]
#![deny(clippy::return_and_then)]
#![deny(clippy::return_self_not_must_use)]
#![deny(clippy::same_functions_in_if_condition)]
#![deny(clippy::same_name_method)]
#![deny(clippy::semicolon_if_nothing_returned)]
#![deny(clippy::semicolon_inside_block)]
#![deny(clippy::set_contains_or_insert)]
#![deny(clippy::significant_drop_in_scrutinee)]
#![deny(clippy::significant_drop_tightening)]
#![deny(clippy::single_option_map)]
#![deny(clippy::stable_sort_primitive)]
#![deny(clippy::string_lit_as_bytes)]
#![deny(clippy::string_lit_chars_any)]
#![deny(clippy::string_to_string)]
#![deny(clippy::str_split_at_newline)]
#![deny(clippy::struct_field_names)]
#![deny(clippy::suspicious_operation_groupings)]
#![deny(clippy::tests_outside_test_module)]
#![deny(clippy::trait_duplication_in_bounds)]
#![deny(clippy::transmute_ptr_to_ptr)]
#![deny(clippy::transmute_undefined_repr)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::trivial_regex)]
#![deny(clippy::tuple_array_conversions)]
#![deny(clippy::type_repetition_in_bounds)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::uninlined_format_args)]
#![deny(clippy::unnecessary_box_returns)]
#![deny(clippy::unnecessary_debug_formatting)]
#![deny(clippy::unnecessary_join)]
#![deny(clippy::unnecessary_literal_bound)]
#![deny(clippy::unnecessary_safety_comment)]
#![deny(clippy::unnecessary_safety_doc)]
#![deny(clippy::unnecessary_self_imports)]
#![deny(clippy::unnecessary_semicolon)]
#![deny(clippy::unnecessary_struct_initialization)]
#![deny(clippy::unnecessary_wraps)]
#![deny(clippy::unneeded_field_pattern)]
#![deny(clippy::unnested_or_patterns)]
#![deny(clippy::unsafe_derive_deserialize)]
#![deny(clippy::unused_async)]
#![deny(clippy::unused_peekable)]
#![deny(clippy::unused_result_ok)]
#![deny(clippy::unused_rounding)]
#![deny(clippy::unused_self)]
#![deny(clippy::unused_trait_names)]
#![deny(clippy::use_debug)]
#![deny(clippy::used_underscore_binding)]
#![deny(clippy::used_underscore_items)]
#![deny(clippy::useless_let_if_seq)]
#![deny(clippy::use_self)]
#![deny(clippy::verbose_file_reads)]
#![deny(clippy::while_float)]
#![deny(clippy::wildcard_dependencies)]
#![deny(clippy::wildcard_enum_match_arm)]
#![deny(clippy::wildcard_imports)]
#![deny(clippy::zero_sized_map_values)]
#![warn(clippy::multiple_crate_versions)]
#![warn(clippy::must_use_candidate)]
#![warn(clippy::single_call_fn)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod algorithm;
pub mod puzzle;
pub mod solver;
