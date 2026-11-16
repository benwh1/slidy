//! The order in which the solvers try directions, and the moves of the gap in each direction.

use crate::algorithm::direction::Direction;

/// The directions to search, in order. Bit `d` of a set of moves is `DIRECTIONS[d]`, the inverse
/// of `DIRECTIONS[d]` is `DIRECTIONS[d ^ 2]`, and the moves along the same axis as `DIRECTIONS[d]`
/// are the bits of `0b0101 << (d & 1)`.
pub(super) const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

/// Returns the cells that the gap at cell `gap` of a `w x h` puzzle moves to in each of the
/// [`DIRECTIONS`] (0 for moves that aren't possible), and the set of possible moves.
pub(super) const fn gap_moves(w: usize, h: usize, gap: usize) -> ([u8; 4], u8) {
    let (x, y) = (gap % w, gap / w);
    // The gap moves in the opposite direction to the piece.
    let possible = [y + 1 < h, x + 1 < w, y > 0, x > 0];
    let cells = [gap + w, gap + 1, gap.wrapping_sub(w), gap.wrapping_sub(1)];

    let mut out = ([0; 4], 0);
    let mut d = 0;
    while d < 4 {
        if possible[d] {
            out.0[d] = cells[d] as u8;
            out.1 |= 1 << d;
        }
        d += 1;
    }
    out
}
