use crate::algorithm::puzzle_move::Move;
use std::{fmt::Display, marker::PhantomData};

pub trait MoveDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

pub struct DisplayMove<'a, T> {
    mv: &'a Move,
    phantom: PhantomData<T>,
}

impl<'a, T> DisplayMove<'a, T> {
    pub fn new(mv: &'a Move) -> Self {
        Self {
            mv,
            phantom: PhantomData,
        }
    }
}

impl<'a, T> Display for DisplayMove<'a, T>
where
    DisplayMove<'a, T>: MoveDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MoveDisplay::fmt(self, f)
    }
}

pub struct DisplayLongSpaced;
pub struct DisplayLongUnspaced;
pub struct DisplayShort;

impl MoveDisplay for DisplayMove<'_, DisplayLongSpaced> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = {
            let mut a = self.mv.direction.to_string();
            a.push(' ');
            a = a.repeat(self.mv.amount as usize);
            a.pop();
            a
        };

        write!(f, "{}", str)
    }
}

impl MoveDisplay for DisplayMove<'_, DisplayLongUnspaced> {
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

impl MoveDisplay for DisplayMove<'_, DisplayShort> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mv.amount == 1 {
            write!(f, "{}", self.mv.direction)
        } else {
            write!(f, "{}{}", self.mv.direction, self.mv.amount)
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
        assert_eq!(m.display::<DisplayLongSpaced>().to_string(), "U");
        assert_eq!(m.display::<DisplayLongUnspaced>().to_string(), "U");
        assert_eq!(m.display::<DisplayShort>().to_string(), "U");
    }

    #[test]
    fn test_display_2() {
        let m = Move {
            direction: Direction::Up,
            amount: 3,
        };
        assert_eq!(m.display::<DisplayLongSpaced>().to_string(), "U U U");
        assert_eq!(m.display::<DisplayLongUnspaced>().to_string(), "UUU");
        assert_eq!(m.display::<DisplayShort>().to_string(), "U3");
    }
}
