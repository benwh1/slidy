use std::{cmp::Ordering, fmt::Display, ops::Add, str::FromStr};

use thiserror::Error;

use crate::algorithm::display::{
    algorithm::{AlgorithmDisplay, DisplaySpaced, DisplayUnspaced},
    puzzle_move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
};

use super::{
    direction::Direction,
    puzzle_move::{Move, MoveSum},
};

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Algorithm {
    pub moves: Vec<Move>,
}

impl Algorithm {
    #[must_use]
    pub fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    #[must_use]
    pub fn len(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn push(&mut self, m: Move) {
        self.moves.push(m);
    }

    pub fn push_combine(&mut self, m: Move) {
        if let Some(other) = self.moves.last_mut() && m.direction == other.direction {
            other.amount += m.amount;
        } else {
            self.moves.push(m);
        }
    }

    pub fn push_simplify(&mut self, m: Move) {
        match self.moves.last_mut() {
            Some(other) if m.direction == other.direction => {
                other.amount += m.amount;
            }
            Some(other) if m.direction == other.direction.inverse() => {
                match m.amount.cmp(&other.amount) {
                    Ordering::Less => other.amount -= m.amount,
                    Ordering::Equal => {
                        self.moves.pop();
                    }
                    Ordering::Greater => {
                        *other = Move {
                            direction: m.direction,
                            amount: m.amount - other.amount,
                        }
                    }
                }
            }
            _ => self.moves.push(m),
        }
    }

    #[must_use]
    pub fn simplified(&self) -> Self {
        if self.moves.len() < 2 {
            return Self::new(self.moves.clone());
        }

        // List of simplified moves
        let mut moves = Vec::new();

        // Current move that we are accumulating into. This will be pushed to `moves` when we
        // reach a move that can't be added to it.
        let mut acc_move = None;

        for &next_mv in self.moves.iter() {
            match acc_move {
                Some(m) => match m + next_mv {
                    MoveSum::Ok(m) => {
                        // Moves completely cancel.
                        acc_move = if m.amount == 0 {
                            // Try and pop a move off `moves`, because the next move might cancel.
                            // e.g. consider URLD where `next_mv` is the L move. We pop the U move
                            // from `moves` so that the following D move can cancel with it.
                            if let Some(last) = moves.pop() {
                                Some(last)
                            } else {
                                None
                            }
                        }
                        // Moves can be added but don't fully cancel, keep accumulating into mv.
                        else {
                            Some(m)
                        };
                    }
                    // Moves can't be added, there is no more simplification at this point.
                    MoveSum::Invalid => {
                        // Push mv and go to the next move.
                        moves.push(m);
                        acc_move = Some(next_mv);
                    }
                },
                None => acc_move = Some(next_mv),
            }
        }

        if let Some(mv) = acc_move && mv.amount != 0 {
            moves.push(mv);
        }

        Self::new(moves)
    }

    pub fn simplify(&mut self) {
        self.moves = self.simplified().moves;
    }

    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            moves: self.moves.iter().rev().map(|m| m.inverse()).collect(),
        }
    }

    pub fn invert(&mut self) {
        self.moves = self.inverse().moves;
    }

    #[must_use]
    pub fn transpose(&self) -> Self {
        Self {
            moves: self.moves.iter().map(|m| m.transpose()).collect(),
        }
    }

    #[must_use]
    pub fn repeat(&self, n: usize) -> Self {
        Self {
            moves: self.moves.repeat(n),
        }
    }

    #[must_use]
    pub fn display_long_spaced(&self) -> DisplaySpaced<DisplayLongSpaced> {
        DisplaySpaced::<DisplayLongSpaced>::new(self)
    }

    #[must_use]
    pub fn display_long_unspaced(&self) -> DisplayUnspaced<DisplayLongUnspaced> {
        DisplayUnspaced::<DisplayLongUnspaced>::new(self)
    }

    #[must_use]
    pub fn display_short_spaced(&self) -> DisplaySpaced<DisplayShort> {
        DisplaySpaced::<DisplayShort>::new(self)
    }

    #[must_use]
    pub fn display_short_unspaced(&self) -> DisplayUnspaced<DisplayShort> {
        DisplayUnspaced::<DisplayShort>::new(self)
    }
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Default formatting is short, unspaced.
        self.display_short_unspaced().fmt(f)
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

        for c in s.chars() {
            // Direction character is the start of a new move
            if let Ok(d) = Direction::try_from(c) {
                // Push the previous move, if there was one
                if let Some(dir) = dir {
                    alg.push(Move::new(dir, amount.unwrap_or(1)));
                }

                // Set the new direction and default amount for the next move
                dir = Some(d);
                amount = None;
            }
            // The number after a move
            else if let Some(d) = c.to_digit(10) {
                // Must have a direction before an amount
                if dir.is_none() {
                    return Err(ParseAlgorithmError::MissingDirection);
                }

                // Append the next digit to the number
                if let Some(a) = amount {
                    amount = Some(10 * a + d);
                } else {
                    amount = Some(d);
                }
            }
            // Any other character is invalid
            else if !c.is_whitespace() {
                return Err(ParseAlgorithmError::InvalidCharacter(c));
            }
        }

        // Push the last move
        if let Some(dir) = dir {
            alg.push(Move::new(dir, amount.unwrap_or(1)));
        }

        Ok(alg)
    }
}

impl Add for Algorithm {
    type Output = Self;

    #[must_use]
    fn add(self, rhs: Self) -> Self::Output {
        let mut moves = self.moves;
        let mut moves2 = rhs.moves;
        moves.append(&mut moves2);
        Self { moves }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len() {
        let a = Algorithm::from_str("ULDR").unwrap();
        assert_eq!(a.len(), 4);
    }

    #[test]
    fn test_len_2() {
        let a = Algorithm::from_str("U3L6D2R20").unwrap();
        assert_eq!(a.len(), 31);
    }

    #[test]
    fn test_len_3() {
        let a = Algorithm::from_str("UUU3").unwrap();
        assert_eq!(a.len(), 5);
    }

    #[test]
    fn test_push_combine() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_combine(Move::from(Direction::Right));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Right, 2)));
    }

    #[test]
    fn test_push_combine_2() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_combine(Move::from(Direction::Left));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Left, 1)));
    }

    #[test]
    fn test_push_simplify() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::from(Direction::Right));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Right, 2)));
    }

    #[test]
    fn test_push_simplify_2() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::new(Direction::Left, 3));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Left, 2)));
    }

    #[test]
    fn test_push_simplify_3() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::from(Direction::Left));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Down, 1)));
    }

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

    #[test]
    fn test_transpose() {
        let a = Algorithm::from_str("D2RUR2D2L3URU").unwrap();
        let b = Algorithm::from_str("R2DLD2R2U3LDL").unwrap();
        assert_eq!(a.transpose(), b);
    }

    #[test]
    fn test_repeat() {
        let a = Algorithm::from_str("U2LD3R").unwrap();
        let b = a.repeat(3);
        let expected = Algorithm::from_str(&"U2LD3R".repeat(3)).unwrap();
        assert_eq!(b, expected);
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

        #[test]
        fn test_from_str_to_string() {
            let algs = [
                "",
                "ULDRULDRULDR",
                "UD2U3DUDD2DUUD2",
                "UDLRLDULUDRLURDLURRULRUDRRUDLLDDUDURLURLRLUDURLUDR",
                "DLULD2RU2LDLDRDRU2LULD3RU3R2D3LU2RDLU2LDLDR2ULURDLUL",
            ];
            for a in algs {
                assert_eq!(Algorithm::from_str(a).unwrap().to_string(), a);
            }
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

#[cfg(test)]
mod benchmarks {
    extern crate test;

    use std::str::FromStr;

    use test::Bencher;

    use super::*;

    #[bench]
    fn bench_from_str(b: &mut Bencher) {
        b.iter(|| {
            Algorithm::from_str(
                "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
            )
        });
    }
}
