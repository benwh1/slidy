//! Defines the [`Metric`] trait and the [`Stm`] and [`Mtm`] metrics.

use num_traits::{AsPrimitive, PrimInt};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::algorithm::r#move::r#move::Move;

/// Defines a length function on [`Move`]s.
pub trait Metric {
    /// Whether the metric preserves move-count parity. `true` for [`Stm`], `false` for [`Mtm`].
    const HAS_MOVECOUNT_PARITY: bool;

    /// The length of a [`Move`].
    fn len<T>(mv: Move) -> T
    where
        T: PrimInt + 'static,
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
    const HAS_MOVECOUNT_PARITY: bool = true;

    fn len<T>(mv: Move) -> T
    where
        T: PrimInt + 'static,
        u64: AsPrimitive<T>,
    {
        mv.amount().as_()
    }
}

impl Metric for Mtm {
    const HAS_MOVECOUNT_PARITY: bool = false;

    fn len<T>(_mv: Move) -> T
    where
        T: PrimInt + 'static,
        u64: AsPrimitive<T>,
    {
        T::one()
    }
}
