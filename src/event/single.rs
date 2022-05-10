use crate::puzzle::solved_state::SolvedState;

#[derive(Clone)]
pub struct Single<'a> {
    pub width: usize,
    pub height: usize,
    pub solved_state: &'a dyn SolvedState,
}

pub trait Event {}

impl<'a> Event for Single<'a> {}
