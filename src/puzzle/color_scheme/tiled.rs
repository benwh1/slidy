//! Defines the [`Tiled`] color scheme.

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::color_scheme::ColorScheme;

/// A [`ColorScheme`] applied to a fixed-size rectangle, and then tiled across the puzzle.
pub struct Tiled<C: ColorScheme> {
    color_scheme: C,
    grid_size: (usize, usize),
}

/// Error type for [`Tiled`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TiledError {
    /// Returned from [`Tiled::new`] if the grid width or height are zero.
    #[error("InvalidGridSize: grid width and height must be positive")]
    InvalidGridSize,
}

impl<C: ColorScheme> Tiled<C> {
    /// Creates a new [`Tiled`] from a [`ColorScheme`] and a grid size.
    pub fn new(color_scheme: C, grid_size: (usize, usize)) -> Result<Self, TiledError> {
        if grid_size.0 == 0 || grid_size.1 == 0 {
            Err(TiledError::InvalidGridSize)
        } else {
            Ok(Self {
                color_scheme,
                grid_size,
            })
        }
    }

    /// Returns a reference to the inner [`ColorScheme`].
    pub fn inner(&self) -> &C {
        &self.color_scheme
    }
}

impl<C: ColorScheme> ColorScheme for Tiled<C> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        // Check if the label is valid for all sizes that it will be applied to.
        // There are at most 4 cases to check: the width could be either grid_width or
        // width % grid_width, and similar for height.

        let (gw, gh) = self.grid_size;

        let mut b = true;
        if width >= gw {
            if height >= gh {
                b = b && self.color_scheme.is_valid_size(gw, gh);
            }
            if height % gh != 0 {
                b = b && self.color_scheme.is_valid_size(gw, height % gh);
            }
        }
        if width % gw != 0 {
            if height >= gh {
                b = b && self.color_scheme.is_valid_size(width % gw, gh);
            }
            if height % gh != 0 {
                b = b && self.color_scheme.is_valid_size(width % gw, height % gh);
            }
        }

        b
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        let (gw, gh) = self.grid_size;

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

        self.color_scheme.color(tile_grid_w, tile_grid_h, tx, ty)
    }
}
