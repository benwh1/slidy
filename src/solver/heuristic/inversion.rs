//! Defines the [`InversionCount`] heuristic, which is a lower bound computed from the inversions
//! of the pieces in reading order and in column order.

use std::cmp::Ordering;

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::RowGrids, sliding_puzzle::SlidingPuzzle},
    solver::heuristic::Heuristic,
};

/// Inversion count heuristic, also known as inversion distance. Only applies to the [`RowGrids`]
/// solved state.
///
/// Reading the pieces of a `w x h` puzzle in reading order (ignoring the gap), a horizontal move
/// doesn't change the order, and a vertical move moves one piece past `w - 1` others, which
/// changes the number of inversions by at most `w - 1`. So at least `ceil(v / (w - 1))` vertical
/// moves are needed, where `v` is the number of inversions in reading order.
///
/// Similarly, reading the pieces in column order (top to bottom, then left to right) and
/// comparing them by their solved positions in column order, at least `ceil(c / (h - 1))`
/// horizontal moves are needed, where `c` is the number of inversions in column order.
///
/// The bound is the sum of these two numbers of moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InversionCount;

/// The number of inversions `n` in one direction, where each move in that direction changes `n`
/// by at most `d`. Stored as `ceil(n / d)` and `ceil(n / d) * d - n`, so that it can be updated
/// without dividing.
#[derive(Clone, Debug)]
struct Inversions {
    /// The lower bound on the number of moves in this direction, `ceil(n / d)`.
    moves: u64,
    /// `ceil(n / d) * d - n`, which is less than `d`.
    slack: u64,
}

impl Inversions {
    /// Creates [`Inversions`] for `n` inversions, where each move changes the number of
    /// inversions by at most `d`. If `d` is 0, the bound is 0.
    fn new(n: u64, d: u64) -> Self {
        if d == 0 {
            Self { moves: 0, slack: 0 }
        } else {
            let moves = n.div_ceil(d);
            Self {
                moves,
                slack: moves * d - n,
            }
        }
    }

    /// Adds `delta` to the number of inversions, where `|delta| <= d`.
    fn add(&mut self, delta: i64, d: u64) {
        let change = delta.unsigned_abs();
        match delta.cmp(&0) {
            Ordering::Greater => {
                if self.slack >= change {
                    self.slack -= change;
                } else {
                    self.slack = self.slack + d - change;
                    self.moves += 1;
                }
            }
            Ordering::Less => {
                self.slack += change;
                if self.slack >= d {
                    self.slack -= d;
                    self.moves -= 1;
                }
            }
            Ordering::Equal => {}
        }
    }
}

/// The [`Heuristic::State`] of [`InversionCount`].
#[derive(Clone, Debug)]
pub struct InversionState {
    width: u64,
    height: u64,
    /// `column_rank[p]` is the index of the solved position of piece `p` in column order.
    column_rank: Vec<u64>,
    /// The inversions in reading order.
    vertical: Inversions,
    /// The inversions in column order.
    horizontal: Inversions,
}

impl InversionState {
    fn bound(&self) -> u64 {
        self.vertical.moves + self.horizontal.moves
    }
}

/// Returns the number of pairs `i < j` with `values[i] > values[j]`.
fn count_inversions(values: &[u64]) -> u64 {
    values
        .iter()
        .enumerate()
        .map(|(i, &a)| values[i + 1..].iter().filter(|&&b| a > b).count() as u64)
        .sum()
}

/// Returns 1, 0, or -1 if `a` is greater than, equal to, or less than `b`.
fn sign<T>(a: T, b: T) -> i64
where
    T: Ord + Copy,
{
    match a.cmp(&b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

impl InversionCount {
    /// Updates `state` after the piece at `to` in `puzzle` has moved there from `from`.
    fn update<P>(state: &mut InversionState, puzzle: &P, from: (u64, u64), to: (u64, u64))
    where
        P: SlidingPuzzle,
    {
        let (w, h) = (state.width, state.height);
        let piece = puzzle.piece_at_xy(to);

        if from.0 == to.0 {
            // Vertical move. In reading order, the piece moved past the `w - 1` pieces between
            // `from` and `to`, which are to the right of the upper cell and to the left of the
            // lower cell.
            let x = to.0;
            let (upper, lower) = (from.1.min(to.1), from.1.max(to.1));
            let mut delta = 0;
            for bx in x + 1..w {
                delta += sign(piece, puzzle.piece_at_xy((bx, upper)));
            }
            for bx in 0..x {
                delta += sign(piece, puzzle.piece_at_xy((bx, lower)));
            }

            // `delta` is the change if the piece moved up, so it is now before the pieces it
            // moved past.
            if to.1 > from.1 {
                delta = -delta;
            }
            state.vertical.add(delta, w - 1);
        } else {
            // Horizontal move. Same as above, but in column order.
            let y = to.1;
            let (left, right) = (from.0.min(to.0), from.0.max(to.0));
            let rank = |p: P::Piece| state.column_rank[p.to_usize().unwrap()];
            let piece_rank = rank(piece);
            let mut delta = 0;
            for by in y + 1..h {
                delta += sign(piece_rank, rank(puzzle.piece_at_xy((left, by))));
            }
            for by in 0..y {
                delta += sign(piece_rank, rank(puzzle.piece_at_xy((right, by))));
            }

            if to.0 > from.0 {
                delta = -delta;
            }
            state.horizontal.add(delta, h - 1);
        }
    }
}

impl<P> Heuristic<P, RowGrids, Stm> for InversionCount
where
    P: SlidingPuzzle,
{
    type State = InversionState;

    fn bound(&self, puzzle: &P) -> u64 {
        Heuristic::<P, RowGrids, Stm>::init(self, puzzle).bound()
    }

    fn init(&self, puzzle: &P) -> InversionState {
        let (w, h) = puzzle.size().into();

        let mut column_rank = vec![0; puzzle.area() as usize];
        for p in 0..puzzle.area() {
            let piece = puzzle.piece_at(p);
            let (sx, sy) = puzzle.solved_pos_xy(piece);
            column_rank[piece.to_usize().unwrap()] = sx * h + sy;
        }

        let gap = puzzle.gap_position_xy();
        let reading_order = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&pos| pos != gap)
            .map(|pos| puzzle.piece_at_xy(pos).to_u64().unwrap())
            .collect::<Vec<_>>();
        let column_order = (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&pos| pos != gap)
            .map(|pos| column_rank[puzzle.piece_at_xy(pos).to_usize().unwrap()])
            .collect::<Vec<_>>();

        InversionState {
            width: w,
            height: h,
            column_rank,
            vertical: Inversions::new(count_inversions(&reading_order), w - 1),
            horizontal: Inversions::new(count_inversions(&column_order), h - 1),
        }
    }

    fn bound_after_move(
        &self,
        state: &mut InversionState,
        puzzle: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) -> u64 {
        Self::update(state, puzzle, from, to);
        state.bound()
    }

    fn undo_move(&self, state: &mut InversionState, puzzle: &P, from: (u64, u64), to: (u64, u64)) {
        // The piece is back at `from`, having moved there from `to`.
        Self::update(state, puzzle, to, from);
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr as _};

    use super::*;
    use crate::{
        algorithm::direction::Direction,
        puzzle::{puzzle::Puzzle, size::Size},
    };

    fn bound(puzzle: &Puzzle) -> u64 {
        Heuristic::<Puzzle, RowGrids, Stm>::bound(&InversionCount, puzzle)
    }

    #[test]
    fn test_inversions_add() {
        for d in 1..6 {
            for n in 0..40_u64 {
                for delta in -(d as i64)..=d as i64 {
                    let Some(m) = n.checked_add_signed(delta) else {
                        continue;
                    };
                    let mut inversions = Inversions::new(n, d);
                    inversions.add(delta, d);
                    let expected = Inversions::new(m, d);
                    assert_eq!(
                        (inversions.moves, inversions.slack),
                        (expected.moves, expected.slack),
                        "n = {n}, d = {d}, delta = {delta}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_bound() {
        let cases = [
            ("1 2 3/4 5 6/7 8 0", 0),
            ("1 2 3/4 5 6/7 0 8", 1),
            ("1 2 3/4 5 0/7 8 6", 1),
            ("1 2 3/4 5 6/0 7 8", 2),
            ("1 2 3/0 5 6/4 7 8", 3),
            // 24 inversions in reading order and 22 in column order.
            ("8 6 7/2 5 4/3 0 1", 23),
            ("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0", 0),
        ];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            assert_eq!(bound(&puzzle), expected, "state {state}");
        }
    }

    #[test]
    fn test_rotated_square() {
        // Rotating the solved state of an `n x n` puzzle by 180 degrees reverses the pieces, so
        // there are `(n^2 - 1)(n^2 - 2) / 2` inversions in each direction.
        for n in 2..=10 {
            let size = Size::new(n, n).unwrap();
            let pieces = (0..n * n).rev().map(|i| (i + 1) % (n * n)).collect();
            let puzzle = Puzzle::with_pieces(pieces, size).unwrap();
            assert_eq!(puzzle.gap_position_xy(), (0, 0));
            assert_eq!(bound(&puzzle), n * n * n + n * n - 2 * n - 2, "n = {n}");
        }
    }

    /// Checks that the bound is at most the optimal solution length, for every solvable state of
    /// a puzzle of size `w x h`.
    fn check_admissible(w: u64, h: u64) {
        let dirs = [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ];

        let mut distances = HashMap::new();
        let mut layer = vec![Puzzle::new(Size::new(w, h).unwrap())];
        distances.insert(layer[0].pieces().to_vec(), 0);
        let mut depth = 0;
        while !layer.is_empty() {
            let mut next = Vec::new();
            for puzzle in layer {
                assert!(bound(&puzzle) <= depth, "{puzzle} at depth {depth}");
                for dir in dirs {
                    let mut child = puzzle.clone();
                    if child.try_move_dir(dir) && !distances.contains_key(child.pieces()) {
                        distances.insert(child.pieces().to_vec(), depth + 1);
                        next.push(child);
                    }
                }
            }
            layer = next;
            depth += 1;
        }
    }

    #[test]
    fn test_admissible() {
        for (w, h) in [(3, 3), (4, 2), (2, 4), (3, 2), (2, 3), (4, 1), (1, 4)] {
            check_admissible(w, h);
        }
    }
}
