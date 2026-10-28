use crate::algorithm::{algorithm::Algorithm, direction::Direction};

/// The moves on the current path of a depth-first search. `N` is the initial capacity; the stack
/// grows if a search goes deeper than that (e.g. when `SolverConfig::min` is large).
pub(super) struct Stack<const N: usize> {
    stack: Vec<Direction>,
}

impl<const N: usize> Stack<N> {
    pub(super) fn new() -> Self {
        Self {
            stack: Vec::with_capacity(N),
        }
    }

    pub(super) fn push(&mut self, d: Direction) {
        self.stack.push(d);
    }

    pub(super) fn pop(&mut self) {
        self.remove_n(1);
    }

    pub(super) fn remove_n(&mut self, n: usize) {
        self.stack.truncate(self.stack.len() - n);
    }

    pub(super) fn clear(&mut self) {
        self.stack.clear();
    }

    pub(super) fn to_alg(&self) -> Algorithm {
        self.iter().collect::<Algorithm>().simplified()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = Direction> + use<'_, N> {
        self.stack.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grows_beyond_initial_capacity() {
        let mut stack = Stack::<2>::new();
        for _ in 0..10 {
            stack.push(Direction::Up);
            stack.push(Direction::Left);
        }
        assert_eq!(stack.iter().count(), 20);

        stack.remove_n(5);
        assert_eq!(stack.iter().count(), 15);
        assert_eq!(stack.iter().last(), Some(Direction::Up));

        stack.pop();
        assert_eq!(stack.iter().last(), Some(Direction::Left));

        stack.clear();
        assert_eq!(stack.iter().count(), 0);
    }
}
