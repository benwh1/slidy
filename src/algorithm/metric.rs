//! Defines the [`Metric`] trait and the [`Stm`] and [`Mtm`] metrics.

use crate::algorithm::r#move::r#move::Move;

/// Defines a length function on [`Move`]s.
pub trait Metric {
    /// The length of a [`Move`].
    fn len(mv: Move) -> u32;
}

/// Single tile move metric, where moves like U5 have length 5, etc.
pub struct Stm;

/// Multi tile move metric, where all moves have length 1.
pub struct Mtm;

impl Metric for Stm {
    #[inline]
    fn len(mv: Move) -> u32 {
        mv.amount()
    }
}

impl Metric for Mtm {
    #[inline]
    fn len(_: Move) -> u32 {
        1
    }
}
