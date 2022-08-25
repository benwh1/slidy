use std::{cmp::Ordering, fmt::Display, ops::Add};

use thiserror::Error;

use crate::algorithm::{
    direction::Direction,
    display::puzzle_move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort, MoveDisplay},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveSum {
    Ok(Move),
    Invalid,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveError {
    #[error("ZeroAmount: move amount must be greater than 0")]
    ZeroAmount,
}

impl Move {
    #[must_use]
    pub fn new(direction: Direction, amount: u32) -> Self {
        Self { direction, amount }
    }

    pub fn new_nonzero(direction: Direction, amount: u32) -> Result<Self, MoveError> {
        if amount == 0 {
            Err(MoveError::ZeroAmount)
        } else {
            Ok(Self { direction, amount })
        }
    }

    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    #[must_use]
    pub fn transpose(&self) -> Self {
        Self {
            direction: self.direction.transpose(),
            amount: self.amount,
        }
    }

    #[must_use]
    pub fn display_long_spaced(&self) -> DisplayLongSpaced {
        DisplayLongSpaced::new(*self)
    }

    #[must_use]
    pub fn display_long_unspaced(&self) -> DisplayLongUnspaced {
        DisplayLongUnspaced::new(*self)
    }

    #[must_use]
    pub fn display_short(&self) -> DisplayShort {
        DisplayShort::new(*self)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Default formatting is short.
        self.display_short().fmt(f)
    }
}

impl From<Direction> for Move {
    #[must_use]
    fn from(direction: Direction) -> Self {
        Self {
            direction,
            amount: 1,
        }
    }
}

impl Add for Move {
    type Output = MoveSum;

    #[must_use]
    fn add(self, rhs: Self) -> Self::Output {
        if self.direction == rhs.direction {
            MoveSum::Ok(Self {
                direction: self.direction,
                amount: self.amount + rhs.amount,
            })
        } else if self.direction == rhs.direction.inverse() {
            match self.amount.cmp(&rhs.amount) {
                Ordering::Less => MoveSum::Ok(Self {
                    direction: rhs.direction,
                    amount: rhs.amount - self.amount,
                }),
                Ordering::Equal | Ordering::Greater => MoveSum::Ok(Self {
                    direction: self.direction,
                    amount: self.amount - rhs.amount,
                }),
            }
        }
        // Even if the directions are not on the same axis, we can still add the moves if one of
        // them has amount == 0 (in which case the sum is just the other move).
        // Put the check for rhs.amount == 0 first so that if they are both 0, we return self
        // instead of rhs.
        else if rhs.amount == 0 {
            MoveSum::Ok(self)
        } else if self.amount == 0 {
            MoveSum::Ok(rhs)
        } else {
            MoveSum::Invalid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let a = Move::new(Direction::Up, 2);
        assert_eq!(
            a,
            Move {
                direction: Direction::Up,
                amount: 2
            }
        );
    }

    #[test]
    fn test_new_2() {
        let a = Move::new(Direction::Up, 0);
        assert_eq!(
            a,
            Move {
                direction: Direction::Up,
                amount: 0
            }
        );
    }

    #[test]
    fn test_new_nonzero() {
        let a = Move::new_nonzero(Direction::Up, 2);
        assert_eq!(
            a,
            Ok(Move {
                direction: Direction::Up,
                amount: 2
            })
        );
    }

    #[test]
    fn test_new_nonzero_2() {
        let a = Move::new_nonzero(Direction::Up, 0);
        assert_eq!(a, Err(MoveError::ZeroAmount));
    }

    #[test]
    fn test_inverse() {
        let a = Move::new(Direction::Up, 3);
        let b = Move::new(Direction::Down, 3);
        assert_eq!(a.inverse(), b);
    }

    #[test]
    fn test_transpose() {
        let a = Move::new(Direction::Up, 3);
        let b = Move::new(Direction::Left, 3);
        assert_eq!(a.transpose(), b);
    }

    mod from_direction {
        use super::*;

        #[test]
        fn test_from_direction() {
            assert_eq!(Move::from(Direction::Up), Move::new(Direction::Up, 1));
        }
    }

    mod add {
        use super::*;

        #[test]
        fn test_add() {
            let m1 = Move::new(Direction::Up, 3);
            let m2 = Move::new(Direction::Up, 4);
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Up,
                    amount: 7
                })
            );
        }

        #[test]
        fn test_add_2() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 3,
            };
            let m2 = Move {
                direction: Direction::Down,
                amount: 4,
            };
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Down,
                    amount: 1
                })
            );
        }

        #[test]
        fn test_add_3() {
            let m1 = Move {
                direction: Direction::Left,
                amount: 5,
            };
            let m2 = Move {
                direction: Direction::Right,
                amount: 5,
            };
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Left,
                    amount: 0
                })
            );
        }

        #[test]
        fn test_add_4() {
            let m1 = Move {
                direction: Direction::Left,
                amount: 2,
            };
            let m2 = Move {
                direction: Direction::Up,
                amount: 1,
            };
            assert_eq!(m1 + m2, MoveSum::Invalid);
        }

        #[test]
        fn test_add_5() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Down,
                amount: 2,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m2));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }

        #[test]
        fn test_add_6() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Left,
                amount: 2,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m2));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }

        #[test]
        fn test_add_7() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Left,
                amount: 0,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m1));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }
    }
}
