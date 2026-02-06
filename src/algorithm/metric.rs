//! Defines the [`Metric`] trait and the [`Stm`] and [`Mtm`] metrics.

use num_traits::{AsPrimitive, PrimInt};

use crate::algorithm::r#move::r#move::Move;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Defines a length function on [`Move`]s.
pub trait Metric {
    /// The length of a [`Move`].
    fn len<T: PrimInt + 'static>(mv: Move) -> T
    where
        u64: AsPrimitive<T>;
}

/// Single tile move metric, where moves like U5 have length 5, etc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Stm;

/// Multi tile move metric, where all moves have length 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mtm;

impl Metric for Stm {
    fn len<T: PrimInt + 'static>(mv: Move) -> T
    where
        u64: AsPrimitive<T>,
    {
        mv.amount().as_()
    }
}

impl Metric for Mtm {
    fn len<T: PrimInt + 'static>(_mv: Move) -> T
    where
        u64: AsPrimitive<T>,
    {
        T::one()
    }
}
