use num_traits::Zero as _;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, size::Size, sliding_puzzle::SlidingPuzzle},
    solver::projection::{
        LARGE,
        encoding::{self, vertical_delta},
    },
};

/// The directions to search, in order. Bit `d` of a set of moves is `DIRECTIONS[d]`, and the
/// inverse of `DIRECTIONS[d]` is `DIRECTIONS[d ^ 2]`.
pub(super) const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

/// A projected puzzle state. `N` is the length of the `pieces` buffer (16 for projections of up
/// to 16 cells, 32 otherwise).
#[derive(Clone, Copy, Debug)]
pub(super) struct ProjectedPuzzle<const N: usize> {
    /// The value in each cell, with 0 for the gap and the cells after the last.
    pieces: [u8; N],
    /// The cell of the gap.
    gap: u8,
}

impl<const N: usize> ProjectedPuzzle<N> {
    pub(super) fn new(pieces: &[u8], gap: u8) -> Self {
        let len = pieces.len();
        assert!(len <= N);
        let mut buf = [0u8; N];
        buf[..len].copy_from_slice(pieces);
        Self { pieces: buf, gap }
    }

    /// Returns the cell of the gap.
    pub(super) const fn gap(&self) -> u8 {
        self.gap
    }

    /// Ranks the projected state to the multiset index used for the PDB arrays.
    pub(super) fn encode(&self, tally: &[u8]) -> u64 {
        encoding::encode::<N>(&self.pieces, tally)
    }
}

/// The moves of the gap on a puzzle of one size, and how they change the index of a projected
/// state.
pub(super) struct Moves {
    /// For each cell of the gap, the cells that it moves to in each of the [`DIRECTIONS`] (0 for
    /// moves that aren't possible), and the set of possible moves.
    table: Box<[([u8; 4], u8)]>,
    /// The [`encoding::weights`] of the tally.
    weights: [u64; LARGE],
    width: usize,
}

impl Moves {
    /// Creates the [`Moves`] for puzzles of size `size` with the given tally of values.
    pub(super) fn new(size: Size, tally: &[u8]) -> Self {
        let (w, h) = (size.width() as usize, size.height() as usize);
        let table = (0..w * h)
            .map(|gap| {
                let (x, y) = (gap % w, gap / w);
                let possible = [y + 1 < h, x + 1 < w, y > 0, x > 0];
                let cells = [gap + w, gap + 1, gap.wrapping_sub(w), gap.wrapping_sub(1)];
                let mut out = ([0; 4], 0);
                for d in 0..4 {
                    if possible[d] {
                        out.0[d] = cells[d] as u8;
                        out.1 |= 1 << d;
                    }
                }
                out
            })
            .collect();

        Self {
            table,
            weights: encoding::weights(tally),
            width: w,
        }
    }

    /// Returns the set of possible moves with the gap at `gap`.
    #[inline(always)]
    pub(super) fn possible(&self, gap: u8) -> u8 {
        self.table[gap as usize].1
    }

    /// Returns the state reached by moving in direction `DIRECTIONS[d]` from `state`, which has
    /// index `index`, and its index. The move must be possible.
    #[inline(always)]
    pub(super) fn apply<const N: usize>(
        &self,
        state: &ProjectedPuzzle<N>,
        index: u64,
        d: u32,
    ) -> (ProjectedPuzzle<N>, u64) {
        let gap = state.gap;
        let cell = self.table[gap as usize].0[d as usize];
        let mut child = *state;
        child.pieces.swap(gap as usize, cell as usize);
        child.gap = cell;

        let child_index = match d {
            0 => index.wrapping_add(vertical_delta(
                &state.pieces,
                gap as usize,
                self.width,
                &self.weights,
            )),
            1 => index + self.weights[0],
            2 => index.wrapping_sub(vertical_delta(
                &child.pieces,
                cell as usize,
                self.width,
                &self.weights,
            )),
            _ => index - self.weights[0],
        };

        (child, child_index)
    }
}

pub(super) fn project_puzzle<const N: usize, P, L>(puzzle: &P, label: &L) -> ProjectedPuzzle<N>
where
    P: SlidingPuzzle,
    L: Label,
{
    let size = puzzle.size();
    let mut pieces = Vec::with_capacity(size.area() as usize);
    for i in 0..size.area() {
        let piece = puzzle.piece_at(i);
        if piece.is_zero() {
            pieces.push(0);
        } else {
            let solved_pos = puzzle.solved_pos_xy(piece);
            pieces.push(label.position_label(size, solved_pos) as u8 + 1);
        }
    }
    ProjectedPuzzle::new(&pieces, puzzle.gap_position() as u8)
}
