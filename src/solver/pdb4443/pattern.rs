pub(super) struct Pattern {
    pub(super) pieces: &'static [u8],
}

impl Pattern {
    pub(super) fn new(pieces: &'static [u8]) -> Self {
        Self { pieces }
    }

    pub(super) fn pdb_size(&self) -> usize {
        (0..self.pieces.len()).map(|i| 16 - i).product()
    }
}
