use super::direction::Direction;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

pub struct DisplayLong<'a>(&'a Move);
pub struct DisplayShort<'a>(&'a Move);

impl Move {
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    pub fn display_long(&self) -> DisplayLong {
        DisplayLong(self)
    }

    pub fn display_short(&self) -> DisplayShort {
        DisplayShort(self)
    }
}

impl Display for DisplayLong<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.direction.to_string().repeat(self.0.amount as usize)
        )
    }
}

impl Display for DisplayShort<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.amount == 1 {
            write!(f, "{}", self.0.direction)
        } else {
            write!(f, "{}{}", self.0.direction, self.0.amount)
        }
    }
}
