//! Defines the [`GridTiled`] color scheme.

use palette::rgb::Rgba;

use crate::puzzle::{color_scheme::ColorScheme, grids::Grids, label::grid::Grid, size::Size};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A [`ColorScheme`] tiled across the puzzle in the pattern specified by a [`Grid`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GridTiled<C: ColorScheme> {
    grid: Grid,
    scheme: C,
}

impl<C: ColorScheme> GridTiled<C> {
    /// Creates a new [`GridTiled`] instance.
    pub fn new(grid: Grid, scheme: C) -> Self {
        Self { grid, scheme }
    }
}

impl<C: ColorScheme> ColorScheme for GridTiled<C> {
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        let grid = self.grid.grid_containing_pos(size, pos);
        let grid_size = {
            let (sx, sy) = grid.size();
            Size::new(sx, sy).unwrap()
        };
        let grid_pos = (pos.0 - grid.left(), pos.1 - grid.top());
        self.scheme.color(grid_size, grid_pos)
    }
}
