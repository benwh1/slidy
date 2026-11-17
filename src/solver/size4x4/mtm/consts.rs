pub(super) const SIZE: usize = 259459200;
/// The number of pieces with each label: the gap, then labels 1 to 7 of
/// [`ReducedFourBitPuzzle::SOLVED`].
///
/// [`ReducedFourBitPuzzle::SOLVED`]: super::puzzle::ReducedFourBitPuzzle::SOLVED
pub(super) const TALLY: [u8; 8] = [1, 2, 8, 1, 1, 1, 1, 1];
