use super::{direction::Direction, display::puzzle_move::DisplayMove};
use std::{cmp::Ordering, ops::Add};

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

impl Move {
    pub fn new(direction: Direction, amount: u32) -> Self {
        Self { direction, amount }
    }

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

impl Add for Move {
    type Output = MoveSum;

    fn add(self, rhs: Self) -> Self::Output {
        if self.direction == rhs.direction {
            MoveSum::Ok(Move {
                direction: self.direction,
                amount: self.amount + rhs.amount,
            })
        } else if self.direction == rhs.direction.inverse() {
            match self.amount.cmp(&rhs.amount) {
                Ordering::Less => MoveSum::Ok(Move {
                    direction: rhs.direction,
                    amount: rhs.amount - self.amount,
                }),
                Ordering::Equal | Ordering::Greater => MoveSum::Ok(Move {
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
    use super::Move;
    use crate::algorithm::direction::Direction;

    #[test]
    fn test_inverse() {
        let m = Move {
            direction: Direction::Up,
            amount: 3,
        };
        assert_eq!(
            m.inverse(),
            Move {
                direction: Direction::Down,
                amount: 3
            }
        );
    }

    mod add {
        use super::*;
        use crate::algorithm::puzzle_move::MoveSum;

        #[test]
        fn test_add() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 3,
            };
            let m2 = Move {
                direction: Direction::Up,
                amount: 4,
            };
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
    }
}
