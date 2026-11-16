//! Defines the [`ManhattanDistance`] heuristic, which is the sum of the manhattan distances of
//! each piece from its solved position.

use std::cmp::Ordering;

use num_traits::{ToPrimitive as _, Zero as _};

use crate::{
    algorithm::metric::Stm,
    puzzle::{
        label::label::{
            Checkerboard, Diagonals, Fringe, Label as _, RowGrids, Rows, SplitFringe,
            SplitSquareFringe, SquareFringe, Trivial,
        },
        size::Size,
        sliding_puzzle::SlidingPuzzle,
        solved_state::SolvedState,
    },
    solver::heuristic::Heuristic,
};

/// Manhattan distance heuristic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManhattanDistance<S>(pub S);

impl ManhattanDistance<Trivial> {
    /// The distance of the gap at `(gx, gy)` from the bottom right corner of `puzzle`.
    fn gap_distance<P>(puzzle: &P, (gx, gy): (u64, u64)) -> u64
    where
        P: SlidingPuzzle,
    {
        let (w, h) = puzzle.size().into();
        w + h - 2 - gx - gy
    }
}

impl<P> Heuristic<P, Trivial, Stm> for ManhattanDistance<Trivial>
where
    P: SlidingPuzzle,
{
    type State = ();

    fn bound(&self, puzzle: &P) -> u64 {
        Self::gap_distance(puzzle, puzzle.gap_position_xy())
    }

    fn init(&self, _: &P) {}

    fn bound_after_move(&self, (): &mut (), puzzle: &P, from: (u64, u64), _: (u64, u64)) -> u64 {
        // The gap is now at `from`, so this saves finding it.
        Self::gap_distance(puzzle, from)
    }
}

/// Defines a function computing the shortest distance of a piece from a solved position.
pub trait Distance {
    /// True if the sum of `self.dist(pos, solved_pos, size)` over all non-gap tiles of a puzzle is
    /// guaranteed to be equal to the length of a solution of the puzzle mod 2.
    const HAS_PARITY_CONSTRAINT: bool;

    /// Suppose the solved position of the piece in position `pos` is `solved_pos`, then this
    /// function returns the minimum Manhattan distance from `pos` to any position where the piece
    /// is considered solved (according to some [`SolvedState`]).
    ///
    /// [`SolvedState`]: crate::puzzle::solved_state::SolvedState
    fn dist(&self, pos: (u64, u64), solved_pos: (u64, u64), size: Size) -> u64;
}

impl Distance for ManhattanDistance<RowGrids> {
    const HAS_PARITY_CONSTRAINT: bool = true;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        x.abs_diff(sx) + y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<Rows> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (_, y): (u64, u64), (_, sy): (u64, u64), _size: Size) -> u64 {
        y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<Fringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), size: Size) -> u64 {
        let fringe = Fringe.position_label(size, (sx, sy));
        fringe.saturating_sub(x)
            + fringe.saturating_sub(y)
            + x.saturating_sub(fringe).min(y.saturating_sub(fringe))
    }
}

impl Distance for ManhattanDistance<SquareFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, pos: (u64, u64), solved_pos: (u64, u64), size: Size) -> u64 {
        square_fringe_dist(&ManhattanDistance(Fringe), pos, solved_pos, size)
    }
}

impl Distance for ManhattanDistance<SplitFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        if sx < sy {
            x.abs_diff(sx) + (sx + 1).saturating_sub(y)
        } else {
            y.abs_diff(sy) + sy.saturating_sub(x)
        }
    }
}

impl Distance for ManhattanDistance<SplitSquareFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, pos: (u64, u64), solved_pos: (u64, u64), size: Size) -> u64 {
        square_fringe_dist(&ManhattanDistance(SplitFringe), pos, solved_pos, size)
    }
}

impl Distance for ManhattanDistance<Diagonals> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        (x + y).abs_diff(sx + sy)
    }
}

impl Distance for ManhattanDistance<Checkerboard> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        u64::from((x + y) % 2 != (sx + sy) % 2)
    }
}

/// The distance for a label that is like `inner` in the largest square in the bottom right of
/// the puzzle, and labels each row or column outside the square separately. Used for
/// [`SquareFringe`] (with [`Fringe`]) and [`SplitSquareFringe`] (with [`SplitFringe`]).
fn square_fringe_dist<D>(inner: &D, (x, y): (u64, u64), (sx, sy): (u64, u64), size: Size) -> u64
where
    D: Distance,
{
    let (w, h) = size.into();
    match w.cmp(&h) {
        Ordering::Less => {
            if sy < h.saturating_sub(w) {
                // Solved position is above the square part. Distance is the vertical distance
                // from the current row to the target row.
                y.abs_diff(sy)
            } else {
                // Solved position is within the square part. Distance is the number of rows
                // we need to move down to reach the square part (or 0 if we are already in the
                // square part), plus the distance within the square part.
                let size_diff = h - w;
                let vertical_distance = size_diff.saturating_sub(y);
                let square_distance = inner.dist(
                    (x, y.saturating_sub(size_diff)),
                    (sx, sy - size_diff),
                    size.shrink_to_square(),
                );

                vertical_distance + square_distance
            }
        }
        Ordering::Equal => inner.dist((x, y), (sx, sy), size),
        Ordering::Greater => {
            // Same as above, but for the horizontal direction.
            if sx < w.saturating_sub(h) {
                x.abs_diff(sx)
            } else {
                let size_diff = w - h;
                let horizontal_distance = size_diff.saturating_sub(x);
                let square_distance = inner.dist(
                    (x.saturating_sub(size_diff), y),
                    (sx - size_diff, sy),
                    size.shrink_to_square(),
                );

                horizontal_distance + square_distance
            }
        }
    }
}

/// The [`Heuristic::State`] of [`ManhattanDistance`].
#[derive(Clone, Debug)]
pub struct ManhattanState {
    /// `solved_pos[p]` is the solved position of piece `p`.
    solved_pos: Vec<(u64, u64)>,
    /// The sum of the distances of the pieces.
    sum: u64,
}

impl<S> ManhattanDistance<S>
where
    Self: Distance,
{
    /// Returns the bound given the sum of the distances of the pieces, and the position and
    /// solved position of the gap.
    fn bound_from_sum(sum: u64, (x, y): (u64, u64), (sx, sy): (u64, u64)) -> u64 {
        if Self::HAS_PARITY_CONSTRAINT {
            sum
        } else {
            // Make sure the parity is correct (some positions will give an even bound for a
            // position that takes an odd number of moves, etc.). This uses the actual Manhattan
            // distance of the gap, not `dist`.
            let parity = (x.abs_diff(sx) + y.abs_diff(sy)) % 2;
            if sum % 2 == parity { sum } else { sum + 1 }
        }
    }

    /// Updates the sum of the distances in `state` after the piece at `to` in `puzzle` has moved
    /// there from `from`.
    fn update_sum<P>(
        &self,
        state: &mut ManhattanState,
        puzzle: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) where
        P: SlidingPuzzle,
    {
        let piece = puzzle.piece_at_xy(to).to_usize().unwrap();
        let solved_pos = state.solved_pos[piece];
        let size = puzzle.size();
        state.sum = state.sum + self.dist(to, solved_pos, size) - self.dist(from, solved_pos, size);
    }
}

impl<P, S> Heuristic<P, S, Stm> for ManhattanDistance<S>
where
    P: SlidingPuzzle,
    S: SolvedState,
    Self: Distance,
{
    type State = ManhattanState;

    fn bound(&self, puzzle: &P) -> u64 {
        let size = puzzle.size();
        let (w, h) = size.into();
        let mut sum = 0;
        for y in 0..h {
            for x in 0..w {
                let piece = puzzle.piece_at_xy((x, y));
                if piece != P::Piece::zero() {
                    sum += self.dist((x, y), puzzle.solved_pos_xy(piece), size);
                }
            }
        }

        let solved_gap = puzzle.solved_pos_xy(P::Piece::zero());
        Self::bound_from_sum(sum, puzzle.gap_position_xy(), solved_gap)
    }

    fn init(&self, puzzle: &P) -> ManhattanState {
        let size = puzzle.size();
        let (w, h) = size.into();
        let mut solved_pos = vec![(0, 0); size.area() as usize];
        let mut sum = 0;
        for y in 0..h {
            for x in 0..w {
                let piece = puzzle.piece_at_xy((x, y));
                let pos = puzzle.solved_pos_xy(piece);
                solved_pos[piece.to_usize().unwrap()] = pos;
                if piece != P::Piece::zero() {
                    sum += self.dist((x, y), pos, size);
                }
            }
        }

        ManhattanState { solved_pos, sum }
    }

    fn bound_after_move(
        &self,
        state: &mut ManhattanState,
        puzzle: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) -> u64 {
        self.update_sum(state, puzzle, from, to);
        Self::bound_from_sum(state.sum, from, state.solved_pos[0])
    }

    fn undo_move(&self, state: &mut ManhattanState, puzzle: &P, from: (u64, u64), to: (u64, u64)) {
        // The piece is back at `from`, having moved there from `to`.
        self.update_sum(state, puzzle, to, from);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::puzzle::puzzle::Puzzle;

    #[test]
    fn test_trivial_stm_bound() {
        let cases = [("1 2 3/4 5 6/7 8 0", 0), ("1 0 3/4 2 6/7 5 8", 3)];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            let bound = ManhattanDistance(Trivial).bound(&puzzle);
            assert_eq!(bound, expected, "state {state}");
        }
    }

    macro_rules! test_manhattan_distance {
        ($label:ty, $($w:literal x $h:literal, $solved_pos:literal : $dists:expr),+ $(,)?) => {
            paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::size::Size;

                    use super::*;

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h _ $solved_pos >] () {
                        let size = Size::new($w, $h).unwrap();
                        let md = ManhattanDistance($label);

                        #[allow(clippy::identity_op)]
                        let solved_pos_xy = ($solved_pos % $w, $solved_pos / $w);

                        let dists = (0..size.area())
                            .map(|i| md.dist((i % $w, i / $w), solved_pos_xy, size))
                            .collect::<Vec<_>>();
                        assert_eq!(dists, $dists);
                    })*
                }
            }
        };
    }

    test_manhattan_distance!(
        RowGrids,
        4 x 4, 2 : [
            2, 1, 0, 1,
            3, 2, 1, 2,
            4, 3, 2, 3,
            5, 4, 3, 4,
        ],
    );

    test_manhattan_distance!(
        Rows,
        4 x 4, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
        ],
    );

    test_manhattan_distance!(
        Fringe,
        4 x 4, 9 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            1, 0, 1, 1,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            4, 3, 2, 2, 2, 2,
            3, 2, 1, 1, 1, 1,
            2, 1, 0, 0, 0, 0,
            2, 1, 0, 1, 1, 1,
        ],
    );

    test_manhattan_distance!(
        SquareFringe,
        4 x 4, 9 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            1, 0, 1, 1,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            6, 5, 4, 3, 2, 2,
            5, 4, 3, 2, 1, 1,
            4, 3, 2, 1, 0, 0,
            4, 3, 2, 1, 0, 1,
        ],
        4 x 6, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
        ],
    );

    test_manhattan_distance!(
        SplitFringe,
        4 x 4, 5 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            2, 1, 1, 1,
            3, 2, 2, 2,
        ],
        4 x 4, 9 : [
            3, 2, 3, 4,
            2, 1, 2, 3,
            1, 0, 1, 2,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            4, 3, 2, 2, 2, 2,
            3, 2, 1, 1, 1, 1,
            2, 1, 0, 0, 0, 0,
            3, 2, 1, 1, 1, 1,
        ],
        6 x 4, 20 : [
            5, 4, 3, 4, 5, 6,
            4, 3, 2, 3, 4, 5,
            3, 2, 1, 2, 3, 4,
            2, 1, 0, 1, 2, 3,
        ],
    );

    test_manhattan_distance!(
        SplitSquareFringe,
        4 x 4, 9 : [
            3, 2, 3, 4,
            2, 1, 2, 3,
            1, 0, 1, 2,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            6, 5, 4, 3, 2, 2,
            5, 4, 3, 2, 1, 1,
            4, 3, 2, 1, 0, 0,
            5, 4, 3, 2, 1, 1,
        ],
        4 x 6, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
        ],
    );
}
