//! Defines the [`BalancedSplitScheme`] color scheme, which splits the puzzle into smaller grids
//! until reaching a minimum size.

use palette::rgb::Rgba;

use crate::puzzle::{
    color_scheme::{
        multi_layer::{Layer, MultiLayerColorScheme},
        ColorScheme,
    },
    coloring::Coloring,
    grids::Grids,
    label::rect_partition::Rect,
    size::Size,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Ways of splitting the puzzle.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Splitting {
    /// Prioritizes splitting the puzzle into the top and bottom halves.
    UpDown {
        /// The minimum `height / width` aspect ratio for splitting the puzzle. If the ratio is
        /// smaller than the cutoff fraction, the puzzle is split into left and right halves
        /// instead.
        cutoff_fraction: f32,
    },
    /// Prioritizes splitting the puzzle into the left and right halves.
    LeftRight {
        /// The minimum `width / height` aspect ratio for splitting the puzzle. If the ratio is
        /// smaller than the cutoff fraction, the puzzle is split into top and bottom halves
        /// instead.
        cutoff_fraction: f32,
    },
    /// Splits the puzzle into four quadrants. If one dimension is smaller than the minimum
    /// splitting size, the puzzle is split into two halves instead.
    Quarters,
}

/// A [`MultiLayerColorScheme`] that repeatedly splits the puzzle until reaching the minimum
/// splitting size.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BalancedSplitScheme<S, C> {
    small_scheme: S,
    grid_coloring: C,
    minimum_splitting_size: (u64, u64),
    splitting: Splitting,
}

impl<S: ColorScheme, C: Coloring> BalancedSplitScheme<S, C> {
    /// Creates a new [`BalancedSplitScheme`].
    pub fn new(
        small_scheme: S,
        grid_coloring: C,
        minimum_splitting_size: (u64, u64),
        splitting: Splitting,
    ) -> Self {
        Self {
            small_scheme,
            grid_coloring,
            minimum_splitting_size,
            splitting,
        }
    }

    /// The [`ColorScheme`] used for small grids.
    pub fn small_scheme(&self) -> &S {
        &self.small_scheme
    }

    /// The [`Coloring`] used for grids that aren't fully split.
    pub fn grid_coloring(&self) -> &C {
        &self.grid_coloring
    }

    /// The minimum width and height of a grid that can be split.
    pub fn minimum_splitting_size(&self) -> (u64, u64) {
        self.minimum_splitting_size
    }

    /// The way that grids are split.
    pub fn splitting(&self) -> Splitting {
        self.splitting
    }
}

impl<S: ColorScheme, C: Coloring> MultiLayerColorScheme for BalancedSplitScheme<S, C> {
    fn num_layers(&self, size: Size) -> u32 {
        let (mut width, mut height) = size.into();
        let mut w_layers = 1;
        let mut h_layers = 1;

        while width >= self.minimum_splitting_size.0 {
            width = width.div_ceil(2);
            w_layers += 1;
        }

        while height >= self.minimum_splitting_size.1 {
            height = height.div_ceil(2);
            h_layers += 1;
        }

        match self.splitting {
            Splitting::UpDown { .. } | Splitting::LeftRight { .. } => w_layers + h_layers - 1,
            Splitting::Quarters => w_layers.max(h_layers),
        }
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        let (mut width, mut height) = size.into();
        let (mut x, mut y) = pos;
        let (min_split_width, min_split_height) = self.minimum_splitting_size;
        let mut split_width;
        let mut split_height;
        let mut label = 0;

        for _ in 0..layer + 1 {
            split_width = false;
            split_height = false;

            match self.splitting {
                Splitting::UpDown { cutoff_fraction } => {
                    let aspect_ratio = height as f32 / width as f32;

                    if aspect_ratio >= cutoff_fraction && height >= min_split_height {
                        split_height = true;
                    } else {
                        split_width = true;
                    }
                }
                Splitting::LeftRight { cutoff_fraction } => {
                    let aspect_ratio = width as f32 / height as f32;

                    if aspect_ratio >= cutoff_fraction && width >= min_split_width {
                        split_width = true;
                    } else {
                        split_height = true;
                    }
                }
                Splitting::Quarters => {
                    if width >= min_split_width {
                        split_width = true;
                    }

                    if height >= min_split_height {
                        split_height = true;
                    }
                }
            }

            if width < min_split_width {
                split_width = false;
            }

            if height < min_split_height {
                split_height = false;
            }

            let mut top = 1;
            let mut left = 1;

            if split_width {
                let half = width.div_ceil(2);
                if x < half {
                    width = half;
                    left = 0;
                } else {
                    width -= half;
                    x -= half;
                }
            }

            if split_height {
                let half = height.div_ceil(2);
                if y < half {
                    height = half;
                    top = 0;
                } else {
                    height -= half;
                    y -= half;
                }
            }

            label = match (split_width, split_height) {
                (true, true) => left + 2 * top,
                (true, false) => left,
                (false, true) => top,
                (false, false) => {
                    let small_size = Size::new(width, height).unwrap();
                    return self.small_scheme.color(small_size, (x, y));
                }
            }
        }

        let num_labels = match self.splitting {
            Splitting::UpDown { .. } => 2,
            Splitting::LeftRight { .. } => 2,
            Splitting::Quarters => 4,
        };

        self.grid_coloring.color(label, num_labels)
    }
}

impl<S: ColorScheme, C: Coloring> Grids for Layer<BalancedSplitScheme<S, C>> {
    fn grid_containing_pos(&self, size: Size, pos: (u64, u64)) -> Rect {
        let (width, height) = size.into();
        let (x, y) = pos;
        let (min_split_width, min_split_height) = self.scheme().minimum_splitting_size();

        let (mut left, mut top, mut right, mut bottom) = (0, 0, width, height);

        let mut split_width;
        let mut split_height;

        for _ in 0..self.layer() + 1 {
            split_width = false;
            split_height = false;

            match self.scheme().splitting() {
                Splitting::UpDown { cutoff_fraction } => {
                    let aspect_ratio = height as f32 / width as f32;

                    if aspect_ratio >= cutoff_fraction && height >= min_split_height {
                        split_height = true;
                    } else {
                        split_width = true;
                    }
                }
                Splitting::LeftRight { cutoff_fraction } => {
                    let aspect_ratio = width as f32 / height as f32;

                    if aspect_ratio >= cutoff_fraction && width >= min_split_width {
                        split_width = true;
                    } else {
                        split_height = true;
                    }
                }
                Splitting::Quarters => {
                    if width >= min_split_width {
                        split_width = true;
                    }

                    if height >= min_split_height {
                        split_height = true;
                    }
                }
            }

            if width < min_split_width {
                split_width = false;
            }

            if height < min_split_height {
                split_height = false;
            }

            if split_width {
                let half = width.div_ceil(2);
                if x < half {
                    right = half;
                } else {
                    left = half;
                }
            }

            if split_height {
                let half = height.div_ceil(2);
                if y < half {
                    bottom = half;
                } else {
                    top = half;
                }
            }
        }

        Rect::new((left, top), (right, bottom)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::{color_scheme::Scheme, coloring::ColorList, label::label::Rows};

    use super::*;

    fn scheme(splitting: Splitting) -> BalancedSplitScheme<Scheme<Rows, ColorList>, ColorList> {
        let small_colors = (0..16)
            .map(|i| Rgba::new(i as f32 / 16.0, 0.0, 0.0, 1.0))
            .collect();
        let small_scheme = Scheme::new(Rows, ColorList::new(small_colors).unwrap());

        let grid_colors = (0..4)
            .map(|i| Rgba::new(i as f32 / 4.0, 0.0, 0.0, 1.0))
            .collect();
        let grid_coloring = ColorList::new(grid_colors).unwrap();

        BalancedSplitScheme::new(small_scheme, grid_coloring, (8, 8), splitting)
    }

    fn scheme1() -> BalancedSplitScheme<Scheme<Rows, ColorList>, ColorList> {
        scheme(Splitting::UpDown {
            cutoff_fraction: 0.75,
        })
    }

    fn scheme2() -> BalancedSplitScheme<Scheme<Rows, ColorList>, ColorList> {
        scheme(Splitting::Quarters)
    }

    fn s(w: u64, h: u64) -> Size {
        Size::new(w, h).unwrap()
    }

    #[test]
    fn test_num_layers_1() {
        let scheme = scheme1();

        assert_eq!(scheme.num_layers(s(7, 7)), 1);
        assert_eq!(scheme.num_layers(s(7, 8)), 2);
        assert_eq!(scheme.num_layers(s(8, 7)), 2);
        assert_eq!(scheme.num_layers(s(8, 8)), 3);
        assert_eq!(scheme.num_layers(s(14, 14)), 3);
        assert_eq!(scheme.num_layers(s(15, 15)), 5);
        assert_eq!(scheme.num_layers(s(16, 16)), 5);
        assert_eq!(scheme.num_layers(s(49, 49)), 7);
        assert_eq!(scheme.num_layers(s(173, 121)), 11);
        assert_eq!(scheme.num_layers(s(1000, 10)), 10);
    }

    #[test]
    fn test_num_layers_2() {
        let scheme = scheme2();

        assert_eq!(scheme.num_layers(s(7, 7)), 1);
        assert_eq!(scheme.num_layers(s(7, 8)), 2);
        assert_eq!(scheme.num_layers(s(8, 7)), 2);
        assert_eq!(scheme.num_layers(s(8, 8)), 2);
        assert_eq!(scheme.num_layers(s(14, 14)), 2);
        assert_eq!(scheme.num_layers(s(15, 15)), 3);
        assert_eq!(scheme.num_layers(s(16, 16)), 3);
        assert_eq!(scheme.num_layers(s(49, 49)), 4);
        assert_eq!(scheme.num_layers(s(173, 121)), 6);
        assert_eq!(scheme.num_layers(s(1000, 10)), 9);
    }

    #[test]
    fn test_color_1() {
        let scheme = scheme1();
        let size = s(49, 15);

        // Layer 0
        for y in 0..15 {
            for x in 0..25 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..15 {
            for x in 25..49 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 1
        for y in 0..15 {
            for x in 0..13 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..15 {
            for x in 13..25 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 2
        for y in 0..8 {
            for x in 0..13 {
                let color = scheme.color(size, (x, y), 2);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 8..15 {
            for x in 0..13 {
                let color = scheme.color(size, (x, y), 2);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 3
        for y in 0..8 {
            for x in 0..7 {
                let color = scheme.color(size, (x, y), 3);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..8 {
            for x in 7..13 {
                let color = scheme.color(size, (x, y), 3);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 4
        for y in 0..4 {
            for x in 0..7 {
                let color = scheme.color(size, (x, y), 4);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 4..8 {
            for x in 0..7 {
                let color = scheme.color(size, (x, y), 4);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 5
        for y in 0..4 {
            for x in 0..7 {
                let color = scheme.color(size, (x, y), 5);
                assert_eq!(color, Rgba::new(y as f32 / 16.0, 0.0, 0.0, 1.0));
            }
        }
    }

    #[test]
    fn test_color_2() {
        let scheme = scheme2();
        let size = s(49, 15);

        // Layer 0
        for y in 0..8 {
            for x in 0..25 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..8 {
            for x in 25..49 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 8..15 {
            for x in 0..25 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(2.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 8..15 {
            for x in 25..49 {
                let color = scheme.color(size, (x, y), 0);
                assert_eq!(color, Rgba::new(3.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 1
        for y in 0..4 {
            for x in 0..13 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..4 {
            for x in 13..25 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 4..8 {
            for x in 0..13 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(2.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 4..8 {
            for x in 13..25 {
                let color = scheme.color(size, (x, y), 1);
                assert_eq!(color, Rgba::new(3.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }

        // Layer 2
        for y in 0..4 {
            for x in 0..7 {
                let color = scheme.color(size, (x, y), 2);
                assert_eq!(color, Rgba::new(0.0, 0.0, 0.0, 1.0));
            }
        }
        for y in 0..4 {
            for x in 7..13 {
                let color = scheme.color(size, (x, y), 2);
                assert_eq!(color, Rgba::new(1.0 / 4.0, 0.0, 0.0, 1.0));
            }
        }
    }
}
