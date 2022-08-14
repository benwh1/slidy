use super::{
    direction::Direction,
    puzzle_move::{Move, MoveSum},
};
use std::{ops::Add, str::FromStr};
use thiserror::Error;

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Algorithm {
    pub moves: Vec<Move>,
}

impl Algorithm {
    pub fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    pub fn len(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn push(&mut self, m: Move) {
        self.moves.push(m);
    }

    pub fn simplified(&self) -> Self {
        if self.moves.len() < 2 {
            return Self::new(self.moves.clone());
        }

        let mut moves = Vec::new();
        let mut mv = Move {
            direction: Direction::Up,
            amount: 0,
        };
        for i in 0..self.moves.len() {
            match mv + self.moves[i] {
                MoveSum::Ok(m) => {
                    mv = if m.amount == 0 {
                        // Moves completely cancel.
                        // Try and pop a move off `moves` (the next move might cancel with it,
                        // e.g. URLD after the L move).
                        // If there is no move in moves, just restart from an empty move.
                        if let Some(last) = moves.pop() {
                            last
                        } else {
                            Move {
                                direction: Direction::Up,
                                amount: 0,
                            }
                        }
                    } else {
                        // Moves can be added don't fully cancel, keep accumulating into mv.
                        m
                    };
                }
                // If the moves can't be added, there is no more simplification at this point, so
                // push mv and go to the next move
                MoveSum::Invalid => {
                    moves.push(mv);
                    mv = self.moves[i];
                }
            }
        }

        if mv.amount != 0 {
            moves.push(mv);
        }

        Self::new(moves)
    }

    pub fn simplify(&mut self) {
        self.moves = self.simplified().moves;
    }

    pub fn inverse(&self) -> Self {
        Self {
            moves: self.moves.iter().rev().map(|m| m.inverse()).collect(),
        }
    }

    pub fn invert(&mut self) {
        self.moves = self.inverse().moves;
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

impl Add for Algorithm {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut moves = self.moves;
        let mut moves2 = rhs.moves;
        moves.append(&mut moves2);
        Self { moves }
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::algorithm::Algorithm;
    use std::str::FromStr;

    #[test]
    fn test_simplify() {
        let a = Algorithm::from_str("UD2U3DUDDDUUD2").unwrap().simplified();
        assert_eq!(a, Algorithm::from_str("D").unwrap());
    }

    #[test]
    fn test_simplify_2() {
        let a = Algorithm::from_str("UDLRDRLU").unwrap().simplified();
        assert_eq!(a, Algorithm::from_str("").unwrap());
    }

    #[test]
    fn test_simplify_3() {
        let a = Algorithm::from_str(
            "DRLLURLURURURURULLDDDLLUDLLLDDRLURDLRUURULUDLDRDULURRRLUUUUDLRRLLRRULLRRLRDRDUDR
            DDRLLLRULLUDULDDURRLDRURRRDRLUDLDDURRURUUULLDRLLDRDUDLLLLDLURLDRLDLDDULLDLDRRLRD
            LRLUUUDDLUDULLDUDDLRRLUULRRLDLRRULULRDRULRLUDUDLRDURDLLRRDDURLULRRLDDRRRRUDDDULU
            DRURURDULLRDUULLLURRUDUDDURURLUUDULRURLRLLRDRDLDRLRRDURUURLUULRRUURDUDDLRUDLDUUD
            RLRURRRDDULDLLURLULLRLUDUDUDRUDLURURLLURRDRUUDURURLLRLDUUURUULUDLDDLLRRLRDRLRULU
            LDDLDRRULUUDDDDDRRLURRULLDULUDLRUUDDDRULRRURUDLLUDDDDLULLDULLLDDRULURURUDDLRLULU
            LRLDULLLUULLDLUURRRULLUDRRUUUULLRULLRDDLURULUDUULLLRDRLRUUDRRUDRRLRUUURRDLLDULRL
            RLLLDDUUDDULUUDDRUDLLUDLRRLLRDDULLDULLRRURDRULUDLRLDRLRLUDDDUUDURUDULULLDLLDUDLL
            LRRDDLDDLRLLLURUDLUDRDDRLRDLDRRDDDUDRRRDURDLDUDLURDUDLLRUDUUDULLRRDLLLLLDDULDRUD
            LDLRLRLLRRULLDRULRRRULRDDLLRRDRUDDUULRRLRLLRUUUDDRDDLUDLRRULLDLDDLURDRDLRDRUUDUU",
        )
        .unwrap()
        .simplified();
        let b = Algorithm::from_str(
            "DLU2RURURURUL2D3L5DRURU4R2DRD2LUL2ULDRDRUR3DLDR2URU3L2DLDRDL4DLDLDL2DLDR2DLULUL2
            DRDR5U2L3UR3U3RDLDR3U4RU2RDR3DLDL2U2L2URULUR2DRU2RULU2RU2L2D3RULULD2LDR2ULD3RUR2
            UL3DRURULD3LUL5D2RULURURDLULUL4U2L2DLU2R3U5LULD2LURULU2L2DRUR4U3R2DL5DL2DL2URDRU
            L2DRULUL2DL2DLD2LD2L3URD2RDLDR2D3R4DLDLURDLUL5DLDL2DRUR2D2RU2RDL2DLD2LURDRD2RU3",
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_inverse() {
        let a = Algorithm::from_str("ULDR").unwrap();
        let b = Algorithm::from_str("LURD").unwrap();
        assert_eq!(a.inverse(), b);
    }

    #[test]
    fn test_inverse_2() {
        let a = Algorithm::from_str("").unwrap();
        assert_eq!(a.inverse(), a);
    }

    #[test]
    fn test_inverse_3() {
        let a = Algorithm::from_str("DL3ULU3R2DLD2RUL2U").unwrap();
        let b = Algorithm::from_str("DR2DLU2RUL2D3RDR3U").unwrap();
        assert_eq!(a.inverse(), b);
    }

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

    mod add {
        use super::*;

        #[test]
        fn test_add() {
            let a = Algorithm::from_str("ULDR").unwrap();
            let b = Algorithm::from_str("DRUL").unwrap();
            assert_eq!(a + b, Algorithm::from_str("ULDRDRUL").unwrap());
        }
    }
}
