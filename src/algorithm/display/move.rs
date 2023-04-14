//! Defines ways in which a [`Move`] can be displayed.

use std::fmt::Display;

use crate::algorithm::r#move::r#move::Move;

/// Marker trait for structs that are used to display moves
pub trait MoveDisplay {
    /// Create a new [`MoveDisplay`] for displaying `mv`.
    #[must_use]
    fn new(mv: Move) -> Self;
}

macro_rules! define_display {
    ($($(#[$annot:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$annot])*
            #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(Move);

            impl MoveDisplay for $name {
                fn new(mv: Move) -> Self {
                    Self(mv)
                }
            }
        )*
    };
}

define_display!(
    /// Formats the move using one character per tile move, with spaces between them.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::r#move::{DisplayLongSpaced, MoveDisplay},
    /// #     r#move::r#move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayLongSpaced::new(a).to_string(), "U U U U U");
    /// ```
    DisplayLongSpaced,
    /// Formats the move using one character per tile move.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::r#move::{DisplayLongUnspaced, MoveDisplay},
    /// #     r#move::r#move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayLongUnspaced::new(a).to_string(), "UUUUU");
    /// ```
    DisplayLongUnspaced,
    /// Formats the move as a direction followed by a number.
    ///
    /// # Example
    ///
    /// ```
    /// # use slidy::algorithm::{
    /// #     direction::Direction,
    /// #     display::r#move::{DisplayShort, MoveDisplay},
    /// #     r#move::r#move::Move,
    /// # };
    /// # use std::str::FromStr;
    /// let a = Move::new(Direction::Up, 5);
    /// assert_eq!(&DisplayShort::new(a).to_string(), "U5");
    /// ```
    DisplayShort
);

impl Display for DisplayLongSpaced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.0.direction.to_string();
        s.push(' ');
        s = s.repeat(self.0.amount as usize);
        s.pop();

        f.write_str(&s)
    }
}

impl Display for DisplayLongUnspaced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.direction.to_string().repeat(self.0.amount as usize))
    }
}

impl Display for DisplayShort {
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
        display::r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
        r#move::r#move::Move,
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
