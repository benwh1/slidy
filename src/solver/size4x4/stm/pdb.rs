//! The pattern databases: the tables' storage ([`region`]), building ([`build`]), the generic
//! lookups over any tile groups ([`tables`]) and the sets chosen for each memory budget
//! ([`sets`]).

pub mod build;
pub mod region;
pub mod sets;
pub mod tables;
