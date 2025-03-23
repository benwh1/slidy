//! Defines the [`Algorithm`] type as a sequence of moves.

use std::{
    cmp::Ordering,
    fmt::Display,
    iter::{self, Sum},
    ops::{Add, AddAssign, Range},
    str::FromStr,
};

use itertools::Itertools as _;
use num_traits::{AsPrimitive, PrimInt};
use thiserror::Error;

use crate::{
    algorithm::{
        as_slice::AsAlgorithmSlice as _,
        direction::Direction,
        display::{
            algorithm::{AlgorithmDisplay as _, DisplaySpaced, DisplayUnspaced},
            r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
        },
        metric::Metric,
        r#move::r#move::Move,
        slice::AlgorithmSlice,
    },
    puzzle::sliding_puzzle::SlidingPuzzle,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`Algorithm::try_slice`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SliceError {
    /// The input range is unordered, e.g. `10..5`.
    #[error("UnorderedRange: range {0:?} is not ordered")]
    UnorderedRange(Range<u64>),

    /// The input range goes beyond the bounds of the [`Algorithm`].
    #[error("OutOfRange: slice is out of range (range is {range:?} but length is {len})")]
    OutOfRange {
        /// The range that was given as input.
        range: Range<u64>,

        /// The length of the [`Algorithm`].
        len: u64,
    },
}

/// A sequence of moves.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Algorithm {
    pub(super) moves: Vec<Move>,
}

impl Algorithm {
    /// Create a new empty [`Algorithm`].
    #[must_use]
    pub fn new() -> Self {
        Self { moves: Vec::new() }
    }

    /// Create a new [`Algorithm`] from a list of [`Move`]s.
    #[must_use]
    pub fn with_moves(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    /// The length of the algorithm in the [`Metric`] `M`.
    #[must_use]
    pub fn len<M: Metric, T: PrimInt + Sum + 'static>(&self) -> T
    where
        u64: AsPrimitive<T>,
    {
        self.as_slice().len::<M, T>()
    }

    /// The length of the algorithm in the [`Stm`] [`Metric`].
    ///
    /// [`Stm`]: ../metric.html
    #[must_use]
    pub fn len_stm<T: PrimInt + Sum + 'static>(&self) -> T
    where
        u64: AsPrimitive<T>,
    {
        self.as_slice().len_stm()
    }

    /// The length of the algorithm in the [`Mtm`] [`Metric`].
    ///
    /// [`Mtm`]: ../metric.html
    #[must_use]
    pub fn len_mtm<T: PrimInt + Sum + 'static>(&self) -> T
    where
        u64: AsPrimitive<T>,
    {
        self.as_slice().len_mtm()
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
        if let Some(other) = self
            .moves
            .last_mut()
            .filter(|other| m.direction == other.direction)
        {
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
        self.as_slice().simplified()
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

    /// Returns an [`AlgorithmSlice`] containing the (single-tile) moves in the range `range`.
    pub fn try_slice(&self, range: Range<u64>) -> Result<AlgorithmSlice, SliceError> {
        if range.start > range.end {
            return Err(SliceError::UnorderedRange(range));
        }

        let len = self.len_stm();
        if range.start > len || range.end > len {
            return Err(SliceError::OutOfRange { range, len });
        }

        let iter = iter::once(0).chain(self.moves.iter().scan(0, |a, b| {
            *a += b.amount;
            Some(*a)
        }));

        // Find the first move where all previous moves have a combined length >= range.start
        let (start_idx, start_total) = iter
            .clone()
            .find_position(|&i| i >= range.start)
            .unwrap_or((self.moves.len() + 1, self.len_stm()));

        // Find the last move where all moves up to and including this one have a combined length
        // <= range.end
        let (end_idx, end_total) = iter
            .clone()
            .tuple_windows()
            .find_position(|&(_, j)| j > range.end)
            .map_or((self.moves.len(), self.len_stm()), |(idx, (i, _))| (idx, i));

        if start_idx > end_idx {
            // The beginning and the end of the slice are both within a single move, e.g. U9[3..7].
            // Return a slice containing a single move with direction = the direction of the move
            // that we sliced through, and amount = length of the range.
            Ok(AlgorithmSlice {
                first: self
                    .moves
                    .get(start_idx - 1)
                    .and_then(|mv| Move::new_nonzero(mv.direction, range.end - range.start).ok()),
                middle: &[],
                last: None,
            })
        } else {
            // The middle section of the slice (everything except maybe the first and last moves)
            // is given by the slice of `self.moves` from `start_idx` to `end_idx`. The first and
            // last moves (if needed) are created by indexing into `self.moves` and creating moves
            // with the relevant direction and amount.
            Ok(AlgorithmSlice {
                first: start_idx
                    .checked_sub(1)
                    .and_then(|idx| self.moves.get(idx))
                    .and_then(|mv| Move::new_nonzero(mv.direction, start_total - range.start).ok()),
                middle: &self.moves[start_idx..end_idx],
                last: self
                    .moves
                    .get(end_idx)
                    .and_then(|mv| Move::new_nonzero(mv.direction, range.end - end_total).ok()),
            })
        }
    }

    /// Checks if `self` is a solution of `puzzle`.
    #[must_use]
    pub fn is_solution_of<Puzzle: SlidingPuzzle + Clone>(&self, mut puzzle: Puzzle) -> bool {
        let b = puzzle.try_apply_alg(self);
        b && puzzle.is_solved()
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
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
                let d = d as u64;

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

#[cfg(feature = "serde")]
impl serde::Serialize for Algorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.to_string();
        serializer.serialize_str(&str)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Algorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let alg_str = String::deserialize(deserializer)?;
        Algorithm::from_str(&alg_str).map_err(serde::de::Error::custom)
    }
}

impl Add for Algorithm {
    type Output = Self;

    /// Concatenates the algorithms.
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

        alg += Self::with_moves(value.middle.to_vec());

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
        assert_eq!(a.len_stm::<u64>(), 4);
        assert_eq!(a.len_mtm::<u64>(), 4);
    }

    #[test]
    fn test_len_2() {
        let a = Algorithm::from_str("U3L6D2R20").unwrap();
        assert_eq!(a.len_stm::<u64>(), 31);
        assert_eq!(a.len_mtm::<u64>(), 4);
    }

    #[test]
    fn test_len_3() {
        let a = Algorithm::from_str("UUU3").unwrap();
        assert_eq!(a.len_stm::<u64>(), 5);
        assert_eq!(a.len_mtm::<u64>(), 3);
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

    mod slice {
        use super::*;

        macro_rules! slice {
            ($first:literal, $middle:literal, $last:literal) => {{
                let first = if $first.is_empty() {
                    None
                } else {
                    Some(Move::from_str($first).unwrap())
                };

                let mut middle = Algorithm::from_str($middle).unwrap();

                let last = if $last.is_empty() {
                    None
                } else {
                    Some(Move::from_str($last).unwrap())
                };

                Ok(AlgorithmSlice {
                    first,
                    middle: &std::mem::take(&mut middle.moves),
                    last,
                })
            }};
        }

        #[test]
        fn test_slice() {
            let alg = Algorithm::from_str("R2DLU10RUR2D2D3D5L5U2L").unwrap();

            // Empty slices
            assert_eq!(alg.try_slice(0..0), slice!("", "", ""));
            assert_eq!(alg.try_slice(1..1), slice!("", "", ""));
            assert_eq!(alg.try_slice(36..36), slice!("", "", ""));

            // Slices on move boundaries
            assert_eq!(
                alg.try_slice(0..36),
                slice!("", "R2DLU10RUR2D2D3D5L5U2L", "")
            );
            assert_eq!(alg.try_slice(0..28), slice!("", "R2DLU10RUR2D2D3D5", ""));
            assert_eq!(alg.try_slice(4..36), slice!("", "U10RUR2D2D3D5L5U2L", ""));
            assert_eq!(alg.try_slice(4..28), slice!("", "U10RUR2D2D3D5", ""));

            // Slices not on move boundaries
            assert_eq!(alg.try_slice(0..30), slice!("", "R2DLU10RUR2D2D3D5", "L2"));
            assert_eq!(alg.try_slice(11..36), slice!("U3", "RUR2D2D3D5L5U2L", ""));
            assert_eq!(alg.try_slice(11..30), slice!("U3", "RUR2D2D3D5", "L2"));

            // Small slices
            assert_eq!(alg.try_slice(3..4), slice!("", "L", ""));
            assert_eq!(alg.try_slice(5..7), slice!("U2", "", ""));
            assert_eq!(alg.try_slice(16..19), slice!("", "R2", "D"));
            assert_eq!(alg.try_slice(17..19), slice!("R", "", "D"));
        }

        #[test]
        fn test_slice_2() {
            let alg = Algorithm::from_str("R2DLU10RUR2D2D3D5L5U2L").unwrap();

            #[allow(clippy::reversed_empty_ranges)]
            let empty_range = 10..5;

            assert_eq!(
                alg.try_slice(empty_range.clone()),
                Err(SliceError::UnorderedRange(empty_range))
            );
            assert_eq!(
                alg.try_slice(0..37),
                Err(SliceError::OutOfRange {
                    range: 0..37,
                    len: 36
                })
            );
            assert_eq!(
                alg.try_slice(37..37),
                Err(SliceError::OutOfRange {
                    range: 37..37,
                    len: 36
                })
            );
        }
    }

    mod from_str {
        use std::str::FromStr as _;

        use crate::algorithm::{
            algorithm::{Algorithm, ParseAlgorithmError},
            direction::Direction,
            r#move::r#move::Move,
        };

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
            let a = Algorithm::from_str("U100000000000000000000");
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

    mod from_algorithm_slice {
        use super::*;

        #[test]
        fn test_from_algorithm_slice() {
            let slice = AlgorithmSlice {
                first: None,
                middle: &[],
                last: None,
            };
            assert_eq!(Algorithm::from(slice), Algorithm::new());
        }

        #[test]
        fn test_from_algorithm_slice_2() {
            let slice = AlgorithmSlice {
                first: None,
                middle: &[Move::new(Direction::Up, 2)],
                last: None,
            };
            assert_eq!(Algorithm::from(slice), Algorithm::from_str("U2").unwrap());
        }

        #[test]
        fn test_from_algorithm_slice_3() {
            let slice = AlgorithmSlice {
                first: Some(Move::new(Direction::Left, 1)),
                middle: &[Move::new(Direction::Up, 2), Move::new(Direction::Right, 3)],
                last: Some(Move::new(Direction::Down, 4)),
            };
            assert_eq!(
                Algorithm::from(slice),
                Algorithm::from_str("LU2R3D4").unwrap()
            );
        }

        #[test]
        fn test_from_algorithm_slice_4() {
            let slice = AlgorithmSlice {
                first: Some(Move::new(Direction::Left, 1)),
                middle: &[],
                last: None,
            };
            assert_eq!(Algorithm::from(slice), Algorithm::from_str("L").unwrap());
        }

        #[test]
        fn test_from_algorithm_slice_5() {
            let slice = AlgorithmSlice {
                first: None,
                middle: &[],
                last: Some(Move::new(Direction::Left, 1)),
            };
            assert_eq!(Algorithm::from(slice), Algorithm::from_str("L").unwrap());
        }

        #[test]
        fn test_from_algorithm_slice_6() {
            let slice = AlgorithmSlice {
                first: Some(Move::new(Direction::Left, 1)),
                middle: &[],
                last: Some(Move::new(Direction::Up, 2)),
            };
            assert_eq!(Algorithm::from(slice), Algorithm::from_str("LU2").unwrap());
        }
    }
}

#[cfg(all(feature = "nightly", test))]
mod benchmarks {
    extern crate test;

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
