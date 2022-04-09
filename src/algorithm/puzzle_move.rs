use super::direction::Direction;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisplayType {
    Long,
    Short,
}

pub struct DisplayMove<'a, const T: DisplayType>(&'a Move);

impl Move {
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    pub fn display<const T: DisplayType>(&self) -> DisplayMove<'_, { T }> {
        DisplayMove::<T>(self)
    }

    pub fn display_long(&self) -> DisplayMove<{ DisplayType::Long }> {
        self.display::<{ DisplayType::Long }>()
    }

    pub fn display_short(&self) -> DisplayMove<{ DisplayType::Short }> {
        self.display::<{ DisplayType::Short }>()
    }
}

impl Display for DisplayMove<'_, { DisplayType::Long }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.direction.to_string().repeat(self.0.amount as usize)
        )
    }
}

impl Display for DisplayMove<'_, { DisplayType::Short }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.amount == 1 {
            write!(f, "{}", self.0.direction)
        } else {
            write!(f, "{}{}", self.0.direction, self.0.amount)
        }
    }
}
