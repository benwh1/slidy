//! Defines the [`InversionCount`] heuristic, which is a lower bound computed from the inversions
//! of the pieces in reading order and in column order.

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
/// Each vertical move moves the gap up or down by one row, so the number of vertical moves is
/// also at least the distance `dy` of the gap from its solved row, and has the same parity as
/// `dy`. So the number of vertical moves is at least the smallest number that is at least
/// `ceil(v / (w - 1))` and `dy` and has the same parity as `dy`. The same applies to horizontal
/// moves, using the distance of the gap from its solved column.
///
/// The bound is the sum of these lower bounds on the numbers of vertical and horizontal moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InversionCount;

/// The number of inversions `n` in one direction, where each move in that direction changes `n`
/// by at most `d`. Stored as `ceil(n / d)` and `ceil(n / d) * d - n`, so that it can be updated
/// without dividing.
#[derive(Clone, Copy, Debug)]
struct Inversions {
    /// The lower bound on the number of moves in this direction, `ceil(n / d)`.
    moves: u64,
    /// `ceil(n / d) * d - n`, which is in `0..d`.
    slack: i64,
    /// The maximum change in `n` from one move, `d`.
    divisor: i64,
}

impl Inversions {
    /// Creates [`Inversions`] for `n` inversions, where each move changes the number of
    /// inversions by at most `d`.
    ///
    /// If `d` is 0, moves never change the number of inversions, so `n` must be 0 if the puzzle is
    /// solvable. In that case, `d` is replaced by 1, which gives a bound of 0 as long as `n` is 0.
    fn new(n: u64, d: u64) -> Self {
        let d = d.max(1);
        let moves = n.div_ceil(d);
        Self {
            moves,
            slack: (moves * d - n) as i64,
            divisor: d as i64,
        }
    }

    /// Adds `delta` to the number of inversions, where `|delta| <= d`.
    #[inline]
    fn add(&mut self, delta: i64) {
        // The new slack is in `-d..2 * d`, so at most one carry is needed to get it into `0..d`.
        let slack = self.slack - delta;
        let carry = i64::from(slack < 0) - i64::from(slack >= self.divisor);
        self.moves = self.moves.wrapping_add_signed(carry);
        self.slack = slack + carry * self.divisor;
    }
}

/// The [`Heuristic::State`] of [`InversionCount`].
#[derive(Clone, Debug)]
pub struct InversionState {
    width: u64,
    height: u64,
    /// The solved positions in reading order of the pieces in each cell, in reading order. This
    /// includes the gap, but the gap is never compared with anything.
    rows: Vec<u64>,
    /// The solved positions in column order of the pieces in each cell, in column order.
    columns: Vec<u64>,
    /// The inversions in reading order, then the inversions in column order.
    inversions: [Inversions; 2],
    /// The values of `inversions` before each move that hasn't been undone yet.
    history: Vec<[Inversions; 2]>,
}

impl InversionState {
    /// Returns the bound when the gap is at `(gx, gy)`.
    #[inline]
    fn bound(&self, (gx, gy): (u64, u64)) -> u64 {
        /// The smallest number that is at least `moves` and `gap_distance`, and has the same
        /// parity as `gap_distance`.
        fn moves_with_gap_distance(moves: u64, gap_distance: u64) -> u64 {
            let moves = moves.max(gap_distance);
            moves + ((moves ^ gap_distance) & 1)
        }

        // The gap is solved in the bottom right corner.
        let vertical = moves_with_gap_distance(self.inversions[0].moves, self.height - 1 - gy);
        let horizontal = moves_with_gap_distance(self.inversions[1].moves, self.width - 1 - gx);
        vertical + horizontal
    }

    /// Swaps the cells at `a` and `b` in `rows` and `columns`, and returns the indices of `a` and
    /// `b` in reading order and in column order.
    #[inline]
    fn swap(&mut self, a: (u64, u64), b: (u64, u64)) -> ((usize, usize), (usize, usize)) {
        let (w, h) = (self.width, self.height);
        let rows = ((a.0 + a.1 * w) as usize, (b.0 + b.1 * w) as usize);
        let columns = ((a.0 * h + a.1) as usize, (b.0 * h + b.1) as usize);
        self.rows.swap(rows.0, rows.1);
        self.columns.swap(columns.0, columns.1);
        (rows, columns)
    }

    /// Updates the state after the piece at `to` has moved there from `from`.
    #[inline]
    fn make_move(&mut self, from: (u64, u64), to: (u64, u64)) {
        self.history.push(self.inversions);
        let ((row_from, row_to), (col_from, col_to)) = self.swap(from, to);

        // A vertical move moves the piece past the `w - 1` pieces between `from` and `to` in
        // reading order, and doesn't change the column order. Similarly for horizontal moves.
        let (order, from, to, axis) = if from.0 == to.0 {
            (&self.rows, row_from, row_to, 0)
        } else {
            (&self.columns, col_from, col_to, 1)
        };

        let moved = order[to];
        let passed = &order[from.min(to) + 1..from.max(to)];
        let smaller = passed.iter().filter(|&&other| other < moved).count();

        // If the piece moved earlier in the order, it is now before the pieces it moved past, so
        // there is one more inversion for each smaller piece and one fewer for each larger piece.
        let delta = 2 * smaller as i64 - passed.len() as i64;
        self.inversions[axis].add(if to < from { delta } else { -delta });
    }

    /// Undoes [`InversionState::make_move`] with the same `from` and `to`.
    #[inline]
    fn undo_move(&mut self, from: (u64, u64), to: (u64, u64)) {
        self.swap(from, to);
        self.inversions = self.history.pop().unwrap();
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

impl<P> Heuristic<P, RowGrids, Stm> for InversionCount
where
    P: SlidingPuzzle,
{
    type State = InversionState;

    fn bound(&self, puzzle: &P) -> u64 {
        Heuristic::<P, RowGrids, Stm>::init(self, puzzle).bound(puzzle.gap_position_xy())
    }

    fn init(&self, puzzle: &P) -> InversionState {
        let (w, h) = puzzle.size().into();
        let solved = |(x, y)| puzzle.solved_pos_xy(puzzle.piece_at_xy((x, y)));
        let rows = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|pos| {
                let (sx, sy) = solved(pos);
                sx + sy * w
            })
            .collect::<Vec<_>>();
        let columns = (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .map(|pos| {
                let (sx, sy) = solved(pos);
                sx * h + sy
            })
            .collect::<Vec<_>>();

        // The solved position of the gap is the last cell in both orders.
        let gap = puzzle.num_pieces();
        let without_gap = |order: &[u64]| {
            order
                .iter()
                .copied()
                .filter(|&p| p != gap)
                .collect::<Vec<_>>()
        };
        let inversions = [
            Inversions::new(count_inversions(&without_gap(&rows)), w - 1),
            Inversions::new(count_inversions(&without_gap(&columns)), h - 1),
        ];

        InversionState {
            width: w,
            height: h,
            rows,
            columns,
            inversions,
            history: Vec::new(),
        }
    }

    fn bound_after_move(
        &self,
        state: &mut InversionState,
        _: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) -> u64 {
        state.make_move(from, to);
        state.bound(from)
    }

    fn undo_move(&self, state: &mut InversionState, _: &P, from: (u64, u64), to: (u64, u64)) {
        state.undo_move(from, to);
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
                    inversions.add(delta);
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
            // 0 inversions in reading order, but the gap needs 2 vertical moves. 12 inversions in
            // column order.
            ("0 1 2/3 4 5/6 7 8", 8),
            ("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0", 0),
            // 0 inversions in reading order, but the gap needs 3 vertical moves. 36 inversions in
            // column order, which need 12 horizontal moves, but the gap needs an odd number.
            ("0 1 2 3/4 5 6 7/8 9 10 11/12 13 14 15", 16),
            // 3 inversions in reading order, which need 1 vertical move, but the gap needs an even
            // number. 18 inversions in column order, which need 6 horizontal moves, but the gap
            // needs an odd number.
            ("1 2 3 4/5 6 7 8/9 10 11 12/0 15 14 13", 9),
        ];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            assert_eq!(bound(&puzzle), expected, "state {state}");
        }
    }

    #[test]
    fn test_rotated_square() {
        // Rotating the solved state of an `n x n` puzzle by 180 degrees reverses the pieces, so
        // there are `(n^2 - 1)(n^2 - 2) / 2` inversions in each direction, which need at least
        // `(n + 1)(n^2 - 2) / 2` moves in each direction. The gap needs to move `n - 1` rows and
        // columns, so the number of moves in each direction has the same parity as `n - 1`. This
        // adds 1 in each direction when `n % 4 == 1`.
        for n in 2..=10 {
            let size = Size::new(n, n).unwrap();
            let pieces = (0..n * n).rev().map(|i| (i + 1) % (n * n)).collect();
            let puzzle = Puzzle::with_pieces(pieces, size).unwrap();
            assert_eq!(puzzle.gap_position_xy(), (0, 0));

            let base = n * n * n + n * n - 2 * n - 2;
            let expected = if n % 4 == 1 { base + 2 } else { base };
            assert_eq!(bound(&puzzle), expected, "n = {n}");
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
