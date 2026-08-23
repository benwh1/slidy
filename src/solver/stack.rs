use std::cell::Cell;

use crate::algorithm::{algorithm::Algorithm, direction::Direction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stack<const N: usize> {
    stack: [Cell<Direction>; N],
    idx: Cell<usize>,
}

impl<const N: usize> Stack<N> {
    pub(super) fn push(&self, d: Direction) {
        self.stack[self.idx.get()].set(d);
        self.idx.update(|n| n + 1);
    }

    pub(super) fn pop(&self) {
        self.remove_n(1);
    }

    pub(super) fn remove_n(&self, n: usize) {
        self.idx.update(|i| i - n);
    }

    pub(super) fn clear(&self) {
        self.idx.set(0)
    }

    pub(super) fn to_alg(&self) -> Algorithm {
        self.iter().collect::<Algorithm>().simplified()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = Direction> + use<'_, N> {
        self.stack[..self.idx.get()].iter().map(Cell::get)
    }
}

impl<const N: usize> Default for Stack<N> {
    fn default() -> Self {
        Self {
            stack: [const { Cell::new(Direction::Up) }; N],
            idx: Cell::new(0),
        }
    }
}
