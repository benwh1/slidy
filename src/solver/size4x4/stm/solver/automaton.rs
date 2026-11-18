//! Which move sequences the search may take: [`Pruning`] for one optimal solution, [`Backtrack`]
//! for every solution of a length.

mod fsm;

/// Which move sequences the search may take, as an automaton over moves. A node is
/// `(state << 4) | allowed`, `allowed` the mask of directions (slidy's: up 0, left 1, down 2,
/// right 3, a tile's move) the next move may take.
pub trait Automaton: 'static {
    /// The root's node.
    const START: u32;

    /// The node after a move in direction `dir` from `node`.
    fn next(node: u32, dir: u8) -> u32;
}

/// Forbids undoing the previous move and one of each pair of equivalent move sequences of equal
/// length (the generated tables in `fsm`): still reaches every position by a shortest path, so it
/// finds an optimal solution, but not every one.
pub struct Pruning;

impl Automaton for Pruning {
    const START: u32 = fsm::START;

    #[inline(always)]
    fn next(node: u32, dir: u8) -> u32 {
        // SAFETY: a live automaton state and a direction 0..4 index the automaton (16-bit states:
        // half the footprint of packed nodes, -12% L2 misses), whose state's group lies inside
        // the mask table.
        unsafe {
            let next = u32::from(
                *fsm::FSM
                    .get_unchecked((node >> 4) as usize)
                    .get_unchecked(dir as usize),
            );
            let mask = u32::from(*fsm::MASK_OF.get_unchecked((next >> 6) as usize));
            (next << 4) | mask
        }
    }
}

/// Forbids only undoing the previous move, so it finds every solution of a length.
pub struct Backtrack;

impl Automaton for Backtrack {
    const START: u32 = 15;

    #[inline(always)]
    fn next(_node: u32, dir: u8) -> u32 {
        15 & !(1 << ((dir + 2) & 3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The automaton forbids undoing the previous move, and of the
    /// shortest duplicate pair - 1.5 laps around a 2x2 block one way or
    /// the other, which reach the same position - it allows exactly one.
    #[test]
    fn automaton_prunes_backtracks_and_the_six_move_loop() {
        // Slidy directions: up 0, left 1, down 2, right 3.
        let walk = |dirs: &[u8]| {
            let mut node = Pruning::START;
            for &dir in dirs {
                if node & (1 << dir) == 0 {
                    return None;
                }
                node = Pruning::next(node, dir);
            }
            Some(node)
        };
        for dir in 0..4 {
            assert!(walk(&[dir]).is_some());
            assert!(walk(&[dir, (dir + 2) % 4]).is_none());
        }
        let down_left = walk(&[2, 1, 0, 3, 2, 1]);
        let left_down = walk(&[1, 2, 3, 0, 1, 2]);
        assert!(down_left.is_some());
        assert!(left_down.is_none());
    }
}
