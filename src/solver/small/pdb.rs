//! Defines the [`Pdb`] struct, which is a pattern database containing the optimal solution length
//! of every state of a small `WxH` puzzle.
//!
//! This is used by [`Solver`] to efficiently find optimal solutions.
//!
//! [`Solver`]: crate::solver::small::solver::Solver

use std::marker::PhantomData;

use crate::algorithm::metric::{Mtm, Stm};

/// A pattern database for a small `WxH` puzzle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pdb<const W: usize, const H: usize, const N: usize, Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

/// [`Pdb`] specialized to the 2x2 size and [`Stm`] metric.
pub type Pdb2x2Stm = Pdb<2, 2, 4, Stm>;
/// [`Pdb`] specialized to the 2x2 size and [`Mtm`] metric.
pub type Pdb2x2Mtm = Pdb<2, 2, 4, Mtm>;
/// [`Pdb`] specialized to the 3x2 size and [`Stm`] metric.
pub type Pdb3x2Stm = Pdb<3, 2, 6, Stm>;
/// [`Pdb`] specialized to the 3x2 size and [`Mtm`] metric.
pub type Pdb3x2Mtm = Pdb<3, 2, 6, Mtm>;
/// [`Pdb`] specialized to the 3x3 size and [`Stm`] metric.
pub type Pdb3x3Stm = Pdb<3, 3, 9, Stm>;
/// [`Pdb`] specialized to the 3x3 size and [`Mtm`] metric.
pub type Pdb3x3Mtm = Pdb<3, 3, 9, Mtm>;
/// [`Pdb`] specialized to the 4x2 size and [`Stm`] metric.
pub type Pdb4x2Stm = Pdb<4, 2, 8, Stm>;
/// [`Pdb`] specialized to the 4x2 size and [`Mtm`] metric.
pub type Pdb4x2Mtm = Pdb<4, 2, 8, Mtm>;
/// [`Pdb`] specialized to the 4x3 size and [`Stm`] metric.
pub type Pdb4x3Stm = Pdb<4, 3, 12, Stm>;
/// [`Pdb`] specialized to the 4x3 size and [`Mtm`] metric.
pub type Pdb4x3Mtm = Pdb<4, 3, 12, Mtm>;
/// [`Pdb`] specialized to the 5x2 size and [`Stm`] metric.
pub type Pdb5x2Stm = Pdb<5, 2, 10, Stm>;
/// [`Pdb`] specialized to the 5x2 size and [`Mtm`] metric.
pub type Pdb5x2Mtm = Pdb<5, 2, 10, Mtm>;
/// [`Pdb`] specialized to the 6x2 size and [`Stm`] metric.
pub type Pdb6x2Stm = Pdb<6, 2, 12, Stm>;
/// [`Pdb`] specialized to the 6x2 size and [`Mtm`] metric.
pub type Pdb6x2Mtm = Pdb<6, 2, 12, Mtm>;

impl<const W: usize, const H: usize, const N: usize, Metric> Pdb<W, H, N, Metric> {
    /// See [`Self::try_from_bytes`].
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            pdb: bytes,
            phantom_metric: PhantomData,
        }
    }

    /// Returns the entry for the state at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    /// Returns the entry for the state at `index`, without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be a valid index into the PDB.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        *self.pdb.get_unchecked(index)
    }
}

impl<const W: usize, const H: usize, const N: usize, Metric> AsRef<[u8]> for Pdb<W, H, N, Metric> {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}
