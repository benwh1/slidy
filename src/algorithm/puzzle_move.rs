use super::{direction::Direction, display::puzzle_move::DisplayMove};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

impl Move {
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    pub fn display<T>(&self) -> DisplayMove<'_, T> {
        DisplayMove::<T>::new(self)
    }
}
