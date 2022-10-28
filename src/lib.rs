#![feature(if_let_guard)]
#![feature(int_log)]
#![feature(int_roundings)]
#![feature(iter_intersperse)]
#![feature(let_chains)]
#![feature(slice_group_by)]
#![feature(test)]
#![allow(clippy::module_inception)]
#![warn(clippy::must_use_candidate)]
#![deny(clippy::use_self)]
#![deny(clippy::double_must_use)]
#![deny(clippy::if_not_else)]
#![deny(clippy::iter_not_returning_iterator)]
#![deny(clippy::mod_module_files)]

pub mod algorithm;
pub mod puzzle;
pub mod solver;
