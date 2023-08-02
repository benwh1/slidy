//! Defines the [`Tiled`] color scheme.

use palette::rgb::Rgba;

use crate::puzzle::{color_scheme::ColorScheme, size::Size};

/// A [`ColorScheme`] applied to a fixed-size rectangle, and then tiled across the puzzle.
pub struct Tiled<C: ColorScheme> {
    color_scheme: C,
    grid_size: Size,
}

impl<C: ColorScheme> Tiled<C> {
    /// Creates a new [`Tiled`] from a [`ColorScheme`] and a grid size.
    pub fn new(color_scheme: C, grid_size: Size) -> Self {
        Self {
            color_scheme,
            grid_size,
        }
    }

    /// Returns a reference to the inner [`ColorScheme`].
    pub fn inner(&self) -> &C {
        &self.color_scheme
    }
}

impl<C: ColorScheme> ColorScheme for Tiled<C> {
    fn is_valid_size(&self, size: Size) -> bool {
        // Check if the label is valid for all sizes that it will be applied to.
        // There are at most 4 cases to check: the width could be either grid_width or
        // width % grid_width, and similar for height.

        let (width, height) = size.into();
        let (gw, gh) = self.grid_size.into();

        if width >= gw {
            if height >= gh && !self.color_scheme.is_valid_size(self.grid_size) {
                return false;
            }
            if height % gh != 0
                && !Size::new(gw, height % gh)
                    .map(|size| self.color_scheme.is_valid_size(size))
                    .unwrap_or_default()
            {
                return false;
            }
        }
        if width % gw != 0 {
            if height >= gh
                && !Size::new(width % gw, gh)
                    .map(|size| self.color_scheme.is_valid_size(size))
                    .unwrap_or_default()
            {
                return false;
            }
            if height % gh != 0
                && !Size::new(width % gw, height % gh)
                    .map(|size| self.color_scheme.is_valid_size(size))
                    .unwrap_or_default()
            {
                return false;
            }
        }

        true
    }

    fn color(&self, size: Size, (x, y): (usize, usize)) -> Rgba {
        let (width, height) = size.into();
        let (gw, gh) = self.grid_size.into();

        // Coordinates of the piece within a single grid
        let (tx, ty) = (x % gw, y % gh);

        // Coordinates of the grid within the whole puzzle (e.g. top left grid is (0, 0))
        let (gx, gy) = (x / gw, y / gh);

        // Size of the grid containing (x, y)
        let (tile_grid_w, tile_grid_h) = (
            if gx == (width - 1) / gw {
                (width - 1) % gw + 1
            } else {
                gw
            },
            if gy == (height - 1) / gh {
                (height - 1) % gh + 1
            } else {
                gh
            },
        );

        Size::new(tile_grid_w, tile_grid_h)
            .map(|size| self.color_scheme.color(size, (tx, ty)))
            .unwrap_or_default()
    }
}
