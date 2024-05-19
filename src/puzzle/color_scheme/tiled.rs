//! Defines the [`Tiled`] color scheme.

use palette::rgb::Rgba;

use crate::puzzle::{color_scheme::ColorScheme, label::rect_partition::Rect, size::Size};

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

    fn color(&self, size: Size, (x, y): (u64, u64)) -> Rgba {
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

    fn color_helper(&self, size: Size, (x, y): (u64, u64), start_idx: usize) -> Rgba {
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

    fn grid_containing_position_helper(
        &self,
        (x, y): (u64, u64),
        (width, height): (u64, u64),
        (left, top): (u64, u64),
        start_idx: usize,
    ) -> Rect {
        self.grid_sizes
            .get(start_idx)
            .map(|&grid_size| {
                let (grid_width, grid_height) = grid_size.into();

                let grid_x = x / grid_width;
                let grid_y = y / grid_height;

                let subgrid_width = if x < (width / grid_width) * grid_width {
                    grid_width
                } else {
                    width % grid_width
                };

                let subgrid_height = if y < (height / grid_height) * grid_height {
                    grid_height
                } else {
                    height % grid_height
                };

                let (x, y) = (x % grid_width, y % grid_height);

                let rect = self.grid_containing_position_helper(
                    (x, y),
                    (subgrid_width, subgrid_height),
                    (left + grid_x * grid_width, top + grid_y * grid_height),
                    start_idx + 1,
                );

                rect
            })
            .unwrap_or_else(|| Rect::new((left, top), (left + width, top + height)).unwrap())
    }

    /// Returns the grid containing the piece at position `pos`.
    pub fn grid_containing_position(&self, size: Size, pos: (u64, u64)) -> Rect {
        self.grid_containing_position_helper(pos, size.into(), (0, 0), 0)
    }
}

impl<C: ColorScheme> ColorScheme for RecursiveTiled<C> {
    fn is_valid_size(&self, size: Size) -> bool {
        self.is_valid_size_helper(size, 0)
    }

    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
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

    #[test]
    fn test_grid_containing_position() {
        let color_scheme = Scheme::new(RowGrids, Rainbow::default());
        let size = Size::new(12, 7).unwrap();
        let rec = RecursiveTiled::new(
            color_scheme,
            vec![Size::new(5, 5).unwrap(), Size::new(3, 2).unwrap()],
        );

        let r = |x, y, w, h| Rect::new((x, y), (x + w, y + h)).unwrap();

        let rects = [
            r(0, 0, 3, 2),
            r(3, 0, 2, 2),
            r(5, 0, 3, 2),
            r(8, 0, 2, 2),
            r(10, 0, 2, 2),
            r(0, 2, 3, 2),
            r(3, 2, 2, 2),
            r(5, 2, 3, 2),
            r(8, 2, 2, 2),
            r(10, 2, 2, 2),
            r(0, 4, 3, 1),
            r(3, 4, 2, 1),
            r(5, 4, 3, 1),
            r(8, 4, 2, 1),
            r(10, 4, 2, 1),
            r(0, 5, 3, 2),
            r(3, 5, 2, 2),
            r(5, 5, 3, 2),
            r(8, 5, 2, 2),
            r(10, 5, 2, 2),
        ];

        #[rustfmt::skip]
        let grid = [
             0,  0,  0,  1,  1,  2,  2,  2,  3,  3,  4,  4,
             0,  0,  0,  1,  1,  2,  2,  2,  3,  3,  4,  4,
             5,  5,  5,  6,  6,  7,  7,  7,  8,  8,  9,  9,
             5,  5,  5,  6,  6,  7,  7,  7,  8,  8,  9,  9,
            10, 10, 10, 11, 11, 12, 12, 12, 13, 13, 14, 14,
            15, 15, 15, 16, 16, 17, 17, 17, 18, 18, 19, 19,
            15, 15, 15, 16, 16, 17, 17, 17, 18, 18, 19, 19,
        ];

        for y in 0..7 {
            for x in 0..12 {
                assert_eq!(
                    rec.grid_containing_position(size, (x, y)),
                    rects[grid[(x + y * 12) as usize]].clone(),
                );
            }
        }
    }
}
