//! Defines the [`Tiled`] color scheme.

use palette::rgb::Rgba;

use crate::puzzle::{color_scheme::ColorScheme, size::Size};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// A [`ColorScheme`] applied to a fixed-size rectangle, and then tiled across the puzzle.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    /// The size of the grid that the [`ColorScheme`] is tiled across.
    pub fn grid_size(&self) -> Size {
        self.grid_size
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

/// A [`ColorScheme`] applied to a fixed-size rectangle, and then tiled across the puzzle
/// recursively.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RecursiveTiled<C: ColorScheme> {
    color_scheme: C,
    grid_sizes: Vec<Size>,
}

impl<C: ColorScheme> RecursiveTiled<C> {
    /// Creates a new [`RecursiveTiled`] from a [`ColorScheme`] and a list of grid sizes.
    pub fn new(color_scheme: C, grid_sizes: Vec<Size>) -> Self {
        Self {
            color_scheme,
            grid_sizes,
        }
    }

    /// Returns a reference to the inner [`ColorScheme`].
    pub fn inner(&self) -> &C {
        &self.color_scheme
    }

    /// The sizes of the grids that the [`ColorScheme`] is tiled across.
    pub fn grid_sizes(&self) -> &[Size] {
        &self.grid_sizes
    }

    fn is_valid_size_helper(&self, size: Size, start_idx: usize) -> bool {
        if let Some(&grid_size) = self.grid_sizes.get(start_idx) {
            let (width, height) = size.into();
            let (grid_width, grid_height) = grid_size.into();

            let has_top_left_region = width >= grid_width && height >= grid_height;
            let has_top_right_region = width % grid_width != 0 && height >= grid_height;
            let has_bottom_left_region = width >= grid_width && height % grid_height != 0;
            let has_bottom_right_region = width % grid_width != 0 && height % grid_height != 0;

            if has_top_left_region && !self.is_valid_size_helper(grid_size, start_idx + 1) {
                return false;
            }

            if has_top_right_region
                && !self.is_valid_size_helper(
                    Size::new(width % grid_width, grid_height).unwrap(),
                    start_idx + 1,
                )
            {
                return false;
            }

            if has_bottom_left_region
                && !self.is_valid_size_helper(
                    Size::new(grid_width, height % grid_height).unwrap(),
                    start_idx + 1,
                )
            {
                return false;
            }

            if has_bottom_right_region
                && !self.is_valid_size_helper(
                    Size::new(width % grid_width, height % grid_height).unwrap(),
                    start_idx + 1,
                )
            {
                return false;
            }

            true
        } else if let Some(&last_size) = self.grid_sizes.last() {
            self.color_scheme.is_valid_size(last_size)
        } else {
            false
        }
    }

    fn color_helper(&self, size: Size, (x, y): (usize, usize), start_idx: usize) -> Rgba {
        if let Some(&grid_size) = self.grid_sizes.get(start_idx) {
            let (width, height) = size.into();
            let (grid_width, grid_height) = grid_size.into();

            if x < (width / grid_width) * grid_width {
                if y < (height / grid_height) * grid_height {
                    // Top left region
                    self.color_helper(grid_size, (x % grid_width, y % grid_height), start_idx + 1)
                } else {
                    // Bottom left region
                    self.color_helper(
                        Size::new(grid_width, height % grid_height).unwrap(),
                        (x % grid_width, y % grid_height),
                        start_idx + 1,
                    )
                }
            } else if y < (height / grid_height) * grid_height {
                // Top right region
                self.color_helper(
                    Size::new(width % grid_width, grid_height).unwrap(),
                    (x % grid_width, y % grid_height),
                    start_idx + 1,
                )
            } else {
                // Bottom right region
                self.color_helper(
                    Size::new(width % grid_width, height % grid_height).unwrap(),
                    (x % grid_width, y % grid_height),
                    start_idx + 1,
                )
            }
        } else {
            self.color_scheme.color(size, (x, y))
        }
    }
}

impl<C: ColorScheme> ColorScheme for RecursiveTiled<C> {
    fn is_valid_size(&self, size: Size) -> bool {
        self.is_valid_size_helper(size, 0)
    }

    fn color(&self, size: Size, pos: (usize, usize)) -> Rgba {
        self.color_helper(size, pos, 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::{color_scheme::Scheme, coloring::Rainbow, label::labels::RowGrids};

    use super::*;

    #[test]
    fn test_grid_size() {
        let color_scheme = Scheme::new(RowGrids, Rainbow::default());
        let grid_size = Size::new(5, 3).unwrap();
        let tiled = Tiled::new(color_scheme, grid_size);

        assert_eq!(tiled.grid_size(), grid_size);
    }

    #[test]
    fn test_color() {
        let color_scheme = Scheme::new(RowGrids, Rainbow::default());
        let grid_size = Size::new(5, 3).unwrap();
        let tiled = Tiled::new(color_scheme, grid_size);

        let size = Size::new(14, 14).unwrap();

        for x in [0, 5] {
            for y in [0, 3, 6, 9] {
                for dx in 0..5 {
                    for dy in 0..3 {
                        assert_eq!(
                            tiled.color(size, (x + dx, y + dy)),
                            tiled.color(size, (dx, dy)),
                        );
                    }
                }
            }
        }

        let size2 = Size::new(4, 3).unwrap();
        for y in [0, 3, 6, 9] {
            for dx in 0..4 {
                for dy in 0..3 {
                    assert_eq!(
                        tiled.color(size, (10 + dx, y + dy)),
                        tiled.color(size2, (dx, dy)),
                    );
                }
            }
        }

        let size3 = Size::new(5, 2).unwrap();
        for x in [0, 5] {
            for dx in 0..5 {
                for dy in 0..2 {
                    assert_eq!(
                        tiled.color(size, (x + dx, 12 + dy)),
                        tiled.color(size3, (dx, dy)),
                    );
                }
            }
        }

        let size4 = Size::new(4, 2).unwrap();
        for dx in 0..4 {
            for dy in 0..2 {
                assert_eq!(
                    tiled.color(size, (10 + dx, 12 + dy)),
                    tiled.color(size4, (dx, dy)),
                );
            }
        }
    }
}
