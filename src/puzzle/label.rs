//! Defines the [`Label`] trait and all implementations.
//!
//! [`Label`]: label/trait.Label.html

pub mod label;
pub mod rect_partition;
pub mod scaled;
pub mod symmetry;

pub mod labels {
    //! Re-exports the [`Label`] trait and all implementations.

    pub use super::{
        label::{
            BijectiveLabel, Checkerboard, ConcentricRectangles, Diagonals, Fringe, FringeGrids,
            Label, LabelError, LastTwoRows, RowGrids, Rows, Spiral, SpiralGrids, SplitFringe,
            SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
        },
        rect_partition::{RectPartition, RectPartitionError},
        scaled::{Scaled, ScaledError},
        symmetry::{
            Id, ReflectAntidiagonal, ReflectDiagonal, ReflectHorizontal, ReflectVertical,
            RotateCcw, RotateCw, RotateHalf,
        },
    };
}
