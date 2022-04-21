use super::{
    direction::Direction,
    puzzle_move::{Move, MoveSum},
};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Algorithm {
    pub moves: Vec<Move>,
}

impl Algorithm {
    pub fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    pub fn length(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    pub fn push(&mut self, m: Move) {
        self.moves.push(m)
    }

    pub fn simplified(&self) -> Self {
        if self.moves.len() < 2 {
            return Algorithm::new(self.moves.clone());
        }

        let mut moves = Vec::new();
        let mut mv = self.moves[0];
        for i in 1..self.moves.len() {
            if let MoveSum::Ok(m) = mv + self.moves[i] {
                mv = m;
            } else {
                moves.push(mv);
                mv = self.moves[i];
            }
        }
        moves.push(mv);

        Algorithm::new(moves)
    }

    pub fn simplify(&mut self) {
        self.moves = self.simplified().moves
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseAlgorithmError {
    #[error("InvalidCharacter: character {0} is invalid")]
    InvalidCharacter(char),

    #[error("MissingDirection: a number must be preceded by a direction")]
    MissingDirection,
}

impl FromStr for Algorithm {
    type Err = ParseAlgorithmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut alg = Self::default();

        let mut dir = None;
        let mut amount = None;

        // Useful macro to try and push the last move that was read
        macro_rules! try_push {
            () => {
                if let Some(prev_dir) = dir {
                    // This is not the first move in the algorithm, so push the previous move
                    let real_amount = if let Some(a) = amount {
                        // No number after the previous move means the amount is actually 1
                        a
                    } else {
                        1
                    };

                    alg.push(Move {
                        amount: real_amount,
                        direction: prev_dir,
                    });
                }
            };
        }

        for c in s.chars() {
            match c {
                // New direction
                c if let Ok(d) = Direction::try_from(c) => {
                    try_push!();

                    // Set the new direction and default amount for the next move
                    dir = Some(d);
                    amount = None;
                },
                c if let Some(d) = c.to_digit(10) => {
                    // Must have a direction before an amount
                    if dir == None {
                        return Err(ParseAlgorithmError::MissingDirection);
                    }

                    if let Some(a) = amount {
                        amount = Some(10 * a + d);
                    }
                    else {
                        amount = Some(d);
                    }
                }
                c if c.is_whitespace() => continue,
                _ => return Err(ParseAlgorithmError::InvalidCharacter(c)),
            }
        }

        // Push the last move
        try_push!();

        Ok(alg)
    }
}

#[cfg(test)]
mod tests {
    mod from_str {
        use crate::algorithm::{
            algorithm::{Algorithm, ParseAlgorithmError},
            direction::Direction,
            puzzle_move::Move,
        };
        use std::str::FromStr;

        #[test]
        fn test_from_str() {
            let a = Algorithm::from_str("U2L3DR4");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![
                        Move {
                            direction: Direction::Up,
                            amount: 2,
                        },
                        Move {
                            direction: Direction::Left,
                            amount: 3,
                        },
                        Move {
                            direction: Direction::Down,
                            amount: 1,
                        },
                        Move {
                            direction: Direction::Right,
                            amount: 4,
                        },
                    ],
                })
            );
        }

        #[test]
        fn test_from_str_2() {
            let a = Algorithm::from_str("");
            assert_eq!(a, Ok(Algorithm { moves: vec![] }));
        }

        #[test]
        fn test_from_str_3() {
            let a = Algorithm::from_str("U");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![Move {
                        direction: Direction::Up,
                        amount: 1
                    }]
                })
            );
        }

        #[test]
        fn test_from_str_4() {
            let a = Algorithm::from_str("L1234567890");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![Move {
                        direction: Direction::Left,
                        amount: 1234567890
                    }]
                })
            );
        }

        #[test]
        fn test_from_str_5() {
            let a = Algorithm::from_str("ULDR");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![
                        Move {
                            direction: Direction::Up,
                            amount: 1
                        },
                        Move {
                            direction: Direction::Left,
                            amount: 1
                        },
                        Move {
                            direction: Direction::Down,
                            amount: 1
                        },
                        Move {
                            direction: Direction::Right,
                            amount: 1
                        }
                    ]
                })
            );
        }

        #[test]
        fn test_from_str_6() {
            let a = Algorithm::from_str("D3RU2RD2aRU3L3");
            assert_eq!(a, Err(ParseAlgorithmError::InvalidCharacter('a')));
        }

        #[test]
        fn test_from_str_7() {
            let a = Algorithm::from_str("3R4DL2");
            assert_eq!(a, Err(ParseAlgorithmError::MissingDirection));
        }

        #[test]
        fn test_from_str_8() {
            let a = Algorithm::from_str("R3L0U2");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![
                        Move {
                            direction: Direction::Right,
                            amount: 3
                        },
                        Move {
                            direction: Direction::Left,
                            amount: 0
                        },
                        Move {
                            direction: Direction::Up,
                            amount: 2
                        },
                    ]
                })
            );
        }
    }
}
