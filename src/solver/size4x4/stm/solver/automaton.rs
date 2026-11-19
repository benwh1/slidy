//! Which move sequences the search may take: [`Pruning`] for one optimal solution, [`Backtrack`]
//! for every solution of a length.

mod fsm;

/// Which move sequences the search may take, as an automaton over moves. A node is
/// `(state << 4) | allowed`, `allowed` the mask of directions the blank's next move may take
/// (slidy's order: up 0, left 1, down 2, right 3).
pub(super) trait Automaton: 'static {
    /// The root's node.
    const START: u32;

    /// The node after a move in direction `direction` from `node`.
    fn next(node: u32, direction: u8) -> u32;
}

/// Forbids undoing the previous move and one of each pair of equivalent move sequences of equal
/// length (the generated tables in `fsm`): still reaches every position by a shortest path, so it
/// finds an optimal solution, but not every one.
pub(super) struct Pruning;

impl Automaton for Pruning {
    const START: u32 = fsm::START;

    #[inline(always)]
    fn next(node: u32, direction: u8) -> u32 {
        // SAFETY: a live automaton state and a direction 0..4 index the automaton (16-bit states:
        // half the footprint of packed nodes, -12% L2 misses), whose state's group lies inside
        // the mask table.
        unsafe {
            let next_state = u32::from(
                *fsm::TRANSITIONS
                    .get_unchecked((node >> 4) as usize)
                    .get_unchecked(direction as usize),
            );
            let mask =
                u32::from(*fsm::ALLOWED_DIRECTIONS.get_unchecked((next_state >> 6) as usize));
            (next_state << 4) | mask
        }
    }
}

/// Forbids only undoing the previous move, so it finds every solution of a length.
pub(super) struct Backtrack;

impl Automaton for Backtrack {
    const START: u32 = 15;

    #[inline(always)]
    fn next(_node: u32, direction: u8) -> u32 {
        15 & !(1 << ((direction + 2) & 3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pruning() {
        // Slidy directions: up 0, left 1, down 2, right 3.
        let walk = |directions: &[u8]| {
            let mut node = Pruning::START;
            for &direction in directions {
                if node & (1 << direction) == 0 {
                    return None;
                }
                node = Pruning::next(node, direction);
            }
            Some(node)
        };
        // Undoing the previous move is pruned.
        for direction in 0..4 {
            assert!(walk(&[direction]).is_some());
            assert!(walk(&[direction, (direction + 2) % 4]).is_none());
        }
        // Of 1.5 laps around a 2x2 block one way or the other, which reach the same position,
        // exactly one is allowed.
        let down_left = walk(&[2, 1, 0, 3, 2, 1]);
        let left_down = walk(&[1, 2, 3, 0, 1, 2]);
        assert!(down_left.is_some());
        assert!(left_down.is_none());
    }
}
