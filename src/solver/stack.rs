use crate::algorithm::{algorithm::Algorithm, direction::Direction};

pub(super) struct Stack<const N: usize> {
    stack: [Direction; N],
    idx: usize,
}

impl<const N: usize> Stack<N> {
    pub(super) fn new() -> Self {
        Self {
            stack: [Direction::Up; N],
            idx: 0,
        }
    }

    pub(super) fn push(&mut self, d: Direction) {
        self.stack[self.idx] = d;
        self.idx += 1;
    }

    pub(super) fn pop(&mut self) {
        self.remove_n(1);
    }

    pub(super) fn remove_n(&mut self, n: usize) {
        self.idx -= n;
    }

    pub(super) fn clear(&mut self) {
        self.idx = 0;
    }

    pub(super) fn to_alg(&self) -> Algorithm {
        self.iter().collect::<Algorithm>().simplified()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = Direction> + use<'_, N> {
        self.stack[..self.idx].iter().copied()
    }
}
