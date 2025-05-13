//! Defines the [`Grids`] trait.

use crate::puzzle::{label::rect_partition::Rect, size::Size};

/// Defines the grid structure of a [`Label`] or [`ColorScheme`].
///
/// [`Label`]: crate::puzzle::label::label::Label
/// [`ColorScheme`]: crate::puzzle::color_scheme::ColorScheme
pub trait Grids {
    /// Returns the [`Rect`] corresponding to a grid containing the given position on a puzzle of
    /// the given [`Size`].
    fn grid_containing_pos(&self, size: Size, pos: (u64, u64)) -> Rect;
}
