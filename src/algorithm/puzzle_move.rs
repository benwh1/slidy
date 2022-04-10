use super::direction::Direction;
use std::{fmt::Display, marker::PhantomData};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

struct DisplayLong;
struct DisplayShort;

trait MoveDisplay {}

impl MoveDisplay for DisplayLong {}
impl MoveDisplay for DisplayShort {}

pub struct DisplayMove<'a, T: MoveDisplay> {
    mv: &'a Move,
    phantom: PhantomData<T>,
}

impl Move {
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    pub fn display<T: MoveDisplay>(&self) -> DisplayMove<'_, T> {
        DisplayMove::<T> {
            mv: self,
            phantom: PhantomData,
        }
    }

    pub fn display_long(&self) -> DisplayMove<DisplayLong> {
        self.display::<DisplayLong>()
    }

    pub fn display_short(&self) -> DisplayMove<DisplayShort> {
        self.display::<DisplayShort>()
    }
}

impl Display for DisplayMove<'_, DisplayLong> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.mv
                .direction
                .to_string()
                .repeat(self.mv.amount as usize)
        )
    }
}

impl Display for DisplayMove<'_, DisplayShort> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mv.amount == 1 {
            write!(f, "{}", self.mv.direction)
        } else {
            write!(f, "{}{}", self.mv.direction, self.mv.amount)
        }
    }
}
