//! The search's interface to a pattern database, in a private module so that only the
//! databases in [`pdb`](super) implement [`Pdb`](super::Pdb).

use crate::solver::size4x4::stm::utils::{Board, Move};

/// Everything the search needs to know about a pattern database. Pairs are
/// `[direct, mirrored]`.
pub trait Search {
    /// A node's table indices in both orientations, and anything else a move updates
    /// incrementally.
    type Indices: Copy;

    /// A node's exact values of its coded tables, which its children's entries decode
    /// against.
    type Values: Copy;

    /// The root's indices, its values and its lower bound.
    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64);

    /// The indices after move `tile_move` from `boards`, which moves `tiles` into the blank.
    /// Prefetches the entries [`Self::evaluate`] will read.
    fn shifted(
        &self,
        parent: &Self::Indices,
        boards: &[Board; 2],
        tile_move: &Move,
        tiles: [u8; 2],
    ) -> Self::Indices;

    /// A child of a node with values `parent`: the child has indices `child`, the blank at
    /// `blanks` and the moved tile `tiles`. `None` when its lower bound reaches `depth` (the
    /// moves left at the parent), else its best-first sort key (smaller first) and its values.
    fn evaluate(
        &self,
        parent: &Self::Values,
        child: &Self::Indices,
        blanks: [u8; 2],
        tiles: [u8; 2],
        depth: u64,
    ) -> Option<(u64, Self::Values)>;
}

/// A successor's best-first sort key (see [`Search::evaluate`]), from its estimates: smaller
/// for a smaller largest estimate, then a smaller total.
#[derive(Default)]
pub struct SortKey {
    best: u64,
    total: u64,
}

impl SortKey {
    /// Adds an estimate of the successor's distance; `None` when it reaches `depth`, the moves
    /// left at its parent, which prunes it.
    #[inline(always)]
    pub fn add(&mut self, estimate: u64, depth: u64) -> Option<()> {
        if estimate >= depth {
            return None;
        }
        self.best = self.best.max(estimate);
        self.total += estimate;
        Some(())
    }

    #[inline(always)]
    pub const fn key(&self) -> u64 {
        (self.best << 10) + self.total
    }
}
