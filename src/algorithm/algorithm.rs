use super::{
    direction::Direction,
    puzzle_move::{Move, MoveError, MoveSum},
};
use std::str::FromStr;
use thiserror::Error;

#[derive(Default)]
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
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseAlgorithmError {
    #[error("MoveError: {0}")]
    MoveError(MoveError),

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
        let mut amount = 1;
        for c in s.chars() {
            match c {
                // New direction
                c if let Ok(d) = Direction::try_from(c) => {
                    if let Some(prev_dir) = dir {
                        // This is not the beginning of the first move in the algorithm.
                        // Push the previous move
                        match Move::new(prev_dir, amount) {
                            Ok(m) => alg.push(m),
                            Err(e) => return Err(ParseAlgorithmError::MoveError(e)),
                        }
                    }

                    // Set the new direction and default amount for the next move
                    dir = Some(d);
                    amount = 1;
                },
                c if let Some(d) = c.to_digit(10) => {
                    // Must have a direction before an amount
                    if dir == None {
                        return Err(ParseAlgorithmError::MissingDirection);
                    }

                    amount = 10 * amount + d;
                }
                c if c.is_whitespace() => continue,
                _ => return Err(ParseAlgorithmError::InvalidCharacter(c)),
            }
        }

        Ok(alg)
    }
}
