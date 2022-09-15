use std::fmt::Display;

use crate::algorithm::puzzle_move::Move;

/// Marker trait for structs that are used to display moves
pub trait MoveDisplay {
    #[must_use]
    fn new(mv: Move) -> Self;
}

macro_rules! define_display {
    ($name:ident) => {
        pub struct $name(Move);

        impl MoveDisplay for $name {
            fn new(mv: Move) -> Self {
                Self(mv)
            }
        }
    };
}

define_display!(DisplayLongSpaced);
define_display!(DisplayLongUnspaced);
define_display!(DisplayShort);

impl Display for DisplayLongSpaced {
    /// Formats the move using one character per tile move, with spaces between them.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::puzzle_move::{DisplayLongSpaced, MoveDisplay},
    /// #     puzzle_move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayLongSpaced::new(a).to_string(), "U U U U U");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.0.direction.to_string();
        s.push(' ');
        s = s.repeat(self.0.amount as usize);
        s.pop();

        write!(f, "{}", s)
    }
}

impl Display for DisplayLongUnspaced {
    /// Formats the move using one character per tile move.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::puzzle_move::{DisplayLongUnspaced, MoveDisplay},
    /// #     puzzle_move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayLongUnspaced::new(a).to_string(), "UUUUU");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.direction.to_string().repeat(self.0.amount as usize)
        )
    }
}

impl Display for DisplayShort {
    /// Formats the move as a direction followed by a number.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::puzzle_move::{DisplayShort, MoveDisplay},
    /// #     puzzle_move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayShort::new(a).to_string(), "U5");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.amount == 1 {
            write!(f, "{}", self.0.direction)
        } else {
            write!(f, "{}{}", self.0.direction, self.0.amount)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::{
        direction::Direction,
        display::puzzle_move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
        puzzle_move::Move,
    };

    #[test]
    fn test_display() {
        let m = Move {
            direction: Direction::Up,
            amount: 1,
        };
        assert_eq!(DisplayLongSpaced(m).to_string(), "U");
        assert_eq!(DisplayLongUnspaced(m).to_string(), "U");
        assert_eq!(DisplayShort(m).to_string(), "U");
    }

    #[test]
    fn test_display_2() {
        let m = Move {
            direction: Direction::Up,
            amount: 3,
        };
        assert_eq!(DisplayLongSpaced(m).to_string(), "U U U");
        assert_eq!(DisplayLongUnspaced(m).to_string(), "UUU");
        assert_eq!(DisplayShort(m).to_string(), "U3");
    }
}
