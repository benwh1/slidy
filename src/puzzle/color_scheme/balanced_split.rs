use palette::rgb::Rgba;

use crate::puzzle::{
    color_scheme::{
        multi_layer::{Layer, MultiLayerColorScheme},
        ColorScheme,
    },
    coloring::Coloring,
    size::Size,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Splitting {
    UpDown { cutoff_fraction: f32 },
    LeftRight { cutoff_fraction: f32 },
    Quarters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplittingType {
    UpDown,
    LeftRight,
    Quarters,
}

impl From<Splitting> for SplittingType {
    fn from(value: Splitting) -> Self {
        match value {
            Splitting::UpDown { .. } => Self::UpDown,
            Splitting::LeftRight { .. } => Self::LeftRight,
            Splitting::Quarters => Self::Quarters,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BalancedSplitScheme<S, C> {
    small_scheme: S,
    grid_coloring: C,
    minimum_splitting_size: (u64, u64),
    splitting: Splitting,
}

impl<S: ColorScheme, C: Coloring> BalancedSplitScheme<S, C> {
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

    pub fn small_scheme(&self) -> &S {
        &self.small_scheme
    }

    pub fn grid_coloring(&self) -> &C {
        &self.grid_coloring
    }

    pub fn minimum_splitting_size(&self) -> (u64, u64) {
        self.minimum_splitting_size
    }

    pub fn splitting(&self) -> Splitting {
        self.splitting
    }

    pub fn layer(&self, layer: u32) -> Layer<&Self> {
        Layer::new(self, layer)
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
            Splitting::UpDown { .. } | Splitting::LeftRight { .. } => w_layers + h_layers,
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
                    };

                    if height >= min_split_height {
                        split_height = true;
                    };
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
