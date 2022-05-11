use super::event::Event;
use crate::puzzle::solved_state::SolvedState;
use std::time::Duration;

pub struct FewestMoves<'a> {
    pub width: usize,
    pub height: usize,
    pub solved_state: &'a dyn SolvedState,
    pub time_limit: Option<Duration>,
}

impl<'a> Event for FewestMoves<'a> {}
