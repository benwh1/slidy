use super::{controls::Controls, display_type::DisplayType};
use crate::puzzle::solved_state::SolvedState;

#[derive(Clone)]
pub struct Single<'a> {
    pub width: usize,
    pub height: usize,
    pub display_type: &'a dyn DisplayType,
    pub solved_state: &'a dyn SolvedState,
    pub controls: Controls,
}
