//! Defines the [`Algorithm`] type as a sequence of moves.

use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, AddAssign},
    str::FromStr,
};

use thiserror::Error;

use crate::algorithm::{
    display::{
        algorithm::{AlgorithmDisplay, DisplaySpaced, DisplayUnspaced},
        r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
    },
    slice::AlgorithmSlice,
};

use super::{
    direction::Direction,
    r#move::r#move::{Move, MoveSum},
};

/// A sequence of moves.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Algorithm {
    moves: Vec<Move>,
}

impl Algorithm {
    /// Create a new empty [`Algorithm`].
    #[must_use]
    pub fn new() -> Self {
        Self { moves: Vec::new() }
    }

    /// Create a new [`Algorithm`] from a list of [`Move`]s.
    #[must_use]
    pub fn from_moves(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    /// The length of the algorithm in single tile moves.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    /// Checks if the algorithm is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Appends a move to the end of the algorithm.
    pub fn push(&mut self, m: Move) {
        self.moves.push(m);
    }

    /// Appends a move to the end of the algorithm.
    ///
    /// If the previous move is in the same direction as the appended move, they are combined into
    /// a single move.
    pub fn push_combine(&mut self, m: Move) {
        if let Some(other) = self.moves.last_mut() && m.direction == other.direction {
            other.amount += m.amount;
        } else {
            self.moves.push(m);
        }
    }

    /// Appends a move to the end of the algorithm.
    ///
    /// If the previous move is in the same or opposite direction as the appended move, they are
    /// combined into a single move (or removed if the two moves cancel completely).
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

    /// Combines all consecutive moves along the same axis into a single move, and removes any moves
    /// that cancel completely.
    #[must_use]
    pub fn simplified(&self) -> Self {
        if self.moves.len() < 2 {
            return Self::from_moves(self.moves.clone());
        }

        // List of simplified moves
        let mut moves = Vec::new();

        // Current move that we are accumulating into. This will be pushed to `moves` when we
        // reach a move that can't be added to it.
        let mut acc_move = None;

        for &next in self.moves.iter() {
            match acc_move {
                Some(sum) => match sum + next {
                    MoveSum::Ok(m) => {
                        // Moves completely cancel.
                        acc_move = if m.amount == 0 {
                            // Try and pop a move off `moves`, because the next move might cancel.
                            // e.g. consider URLD where `next` is the L move. We pop the U move
                            // from `moves` so that the following D move can cancel with it.
                            moves.pop()
                        }
                        // Moves can be added but don't fully cancel, keep accumulating into mv.
                        else {
                            Some(m)
                        };
                    }
                    // Moves can't be added, there is no more simplification at this point.
                    MoveSum::Invalid => {
                        // Push mv and go to the next move.
                        moves.push(sum);
                        acc_move = Some(next);
                    }
                },
                None => acc_move = Some(next),
            }
        }

        if let Some(m) = acc_move && m.amount != 0 {
            moves.push(m);
        }

        Self::from_moves(moves)
    }

    /// Simplifies the algorithm.
    ///
    /// See also: [`Algorithm::simplified`].
    pub fn simplify(&mut self) {
        self.moves = self.simplified().moves;
    }

    /// Returns the algorithm `a` such that concatenating `a` with `self` (in either order), the
    /// result would simplify to the empty algorithm.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            moves: self.moves.iter().rev().map(|m| m.inverse()).collect(),
        }
    }

    /// Inverts the algorithm.
    ///
    /// See also: [`Algorithm::inverse`].
    pub fn invert(&mut self) {
        self.moves = self.inverse().moves;
    }

    /// Returns the algorithm obtained by reflecting the algorithm through the main diagonal.
    #[must_use]
    pub fn transpose(&self) -> Self {
        Self {
            moves: self.moves.iter().map(|m| m.transpose()).collect(),
        }
    }

    /// Returns the algorithm obtained by concatenating `n` copies of `self`.
    #[must_use]
    pub fn repeat(&self, n: usize) -> Self {
        Self {
            moves: self.moves.repeat(n),
        }
    }

    /// Returns an [`AlgorithmSlice`] containing the entire algorithm.
    #[must_use]
    pub fn as_slice(&self) -> AlgorithmSlice {
        AlgorithmSlice {
            first: None,
            middle: &self.moves,
            last: None,
        }
    }

    /// Helper function for creating a [`DisplaySpaced<DisplayLongSpaced>`] around `self`.
    #[must_use]
    pub fn display_long_spaced(&self) -> DisplaySpaced<DisplayLongSpaced> {
        DisplaySpaced::<DisplayLongSpaced>::new(self)
    }

    /// Helper function for creating a [`DisplayUnspaced<DisplayLongUnspaced>`] around `self`.
    #[must_use]
    pub fn display_long_unspaced(&self) -> DisplayUnspaced<DisplayLongUnspaced> {
        DisplayUnspaced::<DisplayLongUnspaced>::new(self)
    }

    /// Helper function for creating a [`DisplaySpaced<DisplayShort>`] around `self`.
    #[must_use]
    pub fn display_short_spaced(&self) -> DisplaySpaced<DisplayShort> {
        DisplaySpaced::<DisplayShort>::new(self)
    }

    /// Helper function for creating a [`DisplayUnspaced<DisplayShort>`] around `self`.
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

/// Error type for [`Algorithm::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseAlgorithmError {
    /// Found a character that can not appear in an algorithm, e.g. "U2 R3 a D"
    #[error("InvalidCharacter: character {0} is invalid")]
    InvalidCharacter(char),

    /// Read a number with no direction, e.g. "U2 R3 5 D"
    #[error("MissingDirection: a number must be preceded by a direction")]
    MissingDirection,

    /// Overflow when reading the number after a direction
    #[error("Overflow: integer overflow occurred when reading the number after a direction")]
    Overflow,
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
                    amount = Some(
                        a.checked_mul(10)
                            .ok_or(ParseAlgorithmError::Overflow)?
                            .checked_add(d)
                            .ok_or(ParseAlgorithmError::Overflow)?,
                    );
                } else {
                    amount = Some(d);
                }
            }
            // A whitespace character signals the end of a move
            else if c.is_whitespace() {
                // Push the previous move, if there was one
                if let Some(dir) = dir {
                    alg.push(Move::new(dir, amount.unwrap_or(1)));
                }

                // Direction and amount for the next move are unknown
                dir = None;
                amount = None;
            }
            // Any other character is invalid
            else {
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

    /// Concatenates the algorithms.
    #[must_use]
    fn add(self, rhs: Self) -> Self::Output {
        let mut moves = self.moves;
        let mut moves2 = rhs.moves;
        moves.append(&mut moves2);
        Self { moves }
    }
}

impl AddAssign for Algorithm {
    /// Appends `rhs` to `self`.
    fn add_assign(&mut self, mut rhs: Self) {
        self.moves.append(&mut rhs.moves);
    }
}

impl From<AlgorithmSlice<'_>> for Algorithm {
    fn from(value: AlgorithmSlice<'_>) -> Self {
        let mut alg = Self::new();

        if let Some(m) = value.first {
            alg.push(m);
        }

        alg += Self::from_moves(value.middle.to_vec());

        if let Some(m) = value.last {
            alg.push(m);
        }

        alg
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
    fn test_is_empty() {
        let a = Algorithm::from_str("ULDR").unwrap();
        assert!(!a.is_empty());
    }

    #[test]
    fn test_is_empty_2() {
        let a = Algorithm::new();
        assert!(a.is_empty());
    }

    #[test]
    fn test_push_combine() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_combine(Move::from(Direction::Right));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Right, 2)));
        assert_eq!(a.to_string(), "ULDR2");
    }

    #[test]
    fn test_push_combine_2() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_combine(Move::from(Direction::Left));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Left, 1)));
        assert_eq!(a.to_string(), "ULDRL");
    }

    #[test]
    fn test_push_simplify() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::from(Direction::Right));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Right, 2)));
        assert_eq!(a.to_string(), "ULDR2");
    }

    #[test]
    fn test_push_simplify_2() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::new(Direction::Left, 3));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Left, 2)));
        assert_eq!(a.to_string(), "ULDL2");
    }

    #[test]
    fn test_push_simplify_3() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::from(Direction::Left));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Down, 1)));
        assert_eq!(a.to_string(), "ULD");
    }

    #[test]
    fn test_push_simplify_4() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        a.push_simplify(Move::from(Direction::Up));
        assert_eq!(a.moves.last(), Some(&Move::from(Direction::Up)));
        assert_eq!(a.to_string(), "ULDRU");
    }

    #[test]
    fn test_push_simplify_5() {
        let mut a = Algorithm::from_str("ULDR5").unwrap();
        a.push_simplify(Move::new(Direction::Left, 3));
        assert_eq!(a.moves.last(), Some(&Move::new(Direction::Right, 2)));
        assert_eq!(a.to_string(), "ULDR2");
    }

    #[test]
    fn test_simplify() {
        let mut a = Algorithm::from_str("UD2U3DUDDDUUD2").unwrap();
        a.simplify();
        assert_eq!(a, Algorithm::from_str("D").unwrap());
    }

    #[test]
    fn test_simplify_2() {
        let mut a = Algorithm::from_str("UDLRDRLU").unwrap();
        a.simplify();
        assert_eq!(a, Algorithm::from_str("").unwrap());
    }

    #[test]
    fn test_simplify_3() {
        let mut a = Algorithm::from_str(
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
        .unwrap();
        let b = Algorithm::from_str(
            "DLU2RURURURUL2D3L5DRURU4R2DRD2LUL2ULDRDRUR3DLDR2URU3L2DLDRDL4DLDLDL2DLDR2DLULUL2
            DRDR5U2L3UR3U3RDLDR3U4RU2RDR3DLDL2U2L2URULUR2DRU2RULU2RU2L2D3RULULD2LDR2ULD3RUR2
            UL3DRURULD3LUL5D2RULURURDLULUL4U2L2DLU2R3U5LULD2LURULU2L2DRUR4U3R2DL5DL2DL2URDRU
            L2DRULUL2DL2DLD2LD2L3URD2RDLDR2D3R4DLDLURDLUL5DLDL2DRUR2D2RU2RDL2DLD2LURDRD2RU3",
        )
        .unwrap();
        a.simplify();
        assert_eq!(a, b);
    }

    #[test]
    fn test_simplify_4() {
        let mut a = Algorithm::from_str("LRL").unwrap();
        a.simplify();
        assert_eq!(a, Algorithm::from_str("L").unwrap());
    }

    #[test]
    fn test_simplify_5() {
        let mut a = Algorithm::from_str("U10").unwrap();
        let b = a.clone();
        a.simplify();
        assert_eq!(a, b);
    }

    #[test]
    fn test_simplify_6() {
        let mut a = Algorithm::new();
        let b = a.clone();
        a.simplify();
        assert_eq!(a, b);
    }

    #[test]
    fn test_invert() {
        let mut a = Algorithm::from_str("ULDR").unwrap();
        let b = Algorithm::from_str("LURD").unwrap();
        a.invert();
        assert_eq!(a, b);
    }

    #[test]
    fn test_invert_2() {
        let mut a = Algorithm::from_str("").unwrap();
        let b = a.clone();
        a.invert();
        assert_eq!(a, b);
    }

    #[test]
    fn test_invert_3() {
        let mut a = Algorithm::from_str("DL3ULU3R2DLD2RUL2U").unwrap();
        let b = Algorithm::from_str("DR2DLU2RUL2D3RDR3U").unwrap();
        a.invert();
        assert_eq!(a, b);
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

    #[test]
    fn test_as_slice() {
        let a = Algorithm::from_str("U2LD3R").unwrap();
        let b = a.as_slice();
        assert_eq!(
            b,
            AlgorithmSlice {
                first: None,
                middle: &a.moves,
                last: None
            }
        );
    }

    mod from_str {
        use crate::algorithm::{
            algorithm::{Algorithm, ParseAlgorithmError},
            direction::Direction,
            r#move::r#move::Move,
        };
        use std::str::FromStr;

        #[test]
        fn test_from_str() {
            let a = Algorithm::from_str("U2L3DR4");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![
                        Move::new(Direction::Up, 2),
                        Move::new(Direction::Left, 3),
                        Move::new(Direction::Down, 1),
                        Move::new(Direction::Right, 4)
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
                    moves: vec![Move::new(Direction::Up, 1)]
                })
            );
        }

        #[test]
        fn test_from_str_4() {
            let a = Algorithm::from_str("L1234567890");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![Move::new(Direction::Left, 1234567890)]
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
                        Move::new(Direction::Up, 1),
                        Move::new(Direction::Left, 1),
                        Move::new(Direction::Down, 1),
                        Move::new(Direction::Right, 1)
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
                        Move::new(Direction::Right, 3),
                        Move::new(Direction::Left, 0),
                        Move::new(Direction::Up, 2),
                    ]
                })
            );
        }

        #[test]
        fn test_from_str_9() {
            let a = Algorithm::from_str("L0");
            assert_eq!(
                a,
                Ok(Algorithm {
                    moves: vec![Move::new(Direction::Left, 0),]
                })
            );
        }

        #[test]
        fn test_from_str_10() {
            let a = Algorithm::from_str(" U L D R ");
            let b = Algorithm::from_str("ULDR");
            assert_eq!(a, b);
        }

        #[test]
        fn test_from_str_11() {
            let a = Algorithm::from_str(" U2 L3  D4\t R1");
            let b = Algorithm::from_str("U2L3D4R");
            assert_eq!(a, b);
        }

        #[test]
        fn test_from_str_12() {
            let a = Algorithm::from_str("D3 R U2 R D2 R U3 L3 a");
            assert_eq!(a, Err(ParseAlgorithmError::InvalidCharacter('a')));
        }

        #[test]
        fn test_from_str_13() {
            let a = Algorithm::from_str("D 3L");
            assert_eq!(a, Err(ParseAlgorithmError::MissingDirection));
        }

        #[test]
        fn test_from_str_14() {
            let a = Algorithm::from_str("U10000000000");
            assert_eq!(a, Err(ParseAlgorithmError::Overflow));
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

    mod add_assign {
        use super::*;

        #[test]
        fn test_add_assign() {
            let mut a = Algorithm::from_str("ULDRU").unwrap();
            let b = Algorithm::from_str("DRUL").unwrap();
            a += b;
            assert_eq!(a, Algorithm::from_str("ULDRUDRUL").unwrap());
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
