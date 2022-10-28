use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    coloring::Coloring,
    label::{label::Label, rect_partition::RectPartition},
};

/// Error type for [`ColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorSchemeError {
    /// Returned from [`ColorScheme::color`] when [`ColorScheme::is_valid_size`] returns false.
    #[error("InvalidSize: {width}x{height} is not a valid size")]
    InvalidSize { width: usize, height: usize },

    /// Returned from [`ColorScheme::color`] when [`ColorScheme::is_valid_size`] returns false.
    #[error(
        "PositionOutOfBounds: position ({x}, {y}) is out of bounds on a {width}x{height} puzzle."
    )]
    PositionOutOfBounds {
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    },
}

pub trait ColorScheme {
    #[must_use]
    fn is_valid_size(&self, width: usize, height: usize) -> bool;

    #[must_use]
    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba;

    fn color(
        &self,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Result<Rgba, ColorSchemeError> {
        if !self.is_valid_size(width, height) {
            Err(ColorSchemeError::InvalidSize { width, height })
        } else if x >= width || y >= height {
            Err(ColorSchemeError::PositionOutOfBounds {
                width,
                height,
                x,
                y,
            })
        } else {
            Ok(self.color_unchecked(width, height, x, y))
        }
    }
}

pub struct Scheme {
    label: Box<dyn Label>,
    coloring: Box<dyn Coloring>,
}

impl Scheme {
    #[must_use]
    pub fn new(label: Box<dyn Label>, coloring: Box<dyn Coloring>) -> Self {
        Self { label, coloring }
    }
}

impl ColorScheme for Scheme {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.label.is_valid_size(width, height)
    }

    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        let label = self.label.position_label_unchecked(width, height, x, y);
        let num_labels = self.label.num_labels_unchecked(width, height);
        self.coloring.color_unchecked(label, num_labels)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecursiveSchemeError {
    #[error("IncompatiblePartitionAndSubschemes: partition has {num_rects} rects, but {num_subschemes} subschemes were given")]
    IncompatiblePartitionAndSubschemes {
        num_rects: usize,
        num_subschemes: usize,
    },

    #[error("InvalidSubschemeSize: puzzle size {w}x{h} is not valid for subscheme at index {subscheme_idx}",
        w = rect_size.0,
        h = rect_size.1
    )]
    InvalidSubschemeSize {
        subscheme_idx: usize,
        rect_size: (u32, u32),
    },
}

pub struct RecursiveScheme {
    scheme: Scheme,
    partition: Option<RectPartition>,
    subschemes: Vec<Self>,
}

impl RecursiveScheme {
    pub fn new(
        scheme: Scheme,
        partition: RectPartition,
        subschemes: Vec<Self>,
    ) -> Result<Self, RecursiveSchemeError> {
        if partition.num_rects() != subschemes.len() {
            Err(RecursiveSchemeError::IncompatiblePartitionAndSubschemes {
                num_rects: partition.num_rects(),
                num_subschemes: subschemes.len(),
            })
        } else if let Some(idx) = subschemes
            .iter()
            .zip(partition.rects.iter())
            .position(|(s, r)| !s.is_valid_size(r.width() as usize, r.height() as usize))
        {
            Err(RecursiveSchemeError::InvalidSubschemeSize {
                subscheme_idx: idx,
                rect_size: partition.rects[idx].size(),
            })
        } else {
            Ok(Self {
                scheme,
                partition: Some(partition),
                subschemes,
            })
        }
    }

    #[must_use]
    pub fn new_leaf(scheme: Scheme) -> Self {
        Self {
            scheme,
            partition: None,
            subschemes: Vec::new(),
        }
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        1 + self
            .subschemes
            .iter()
            .map(|s| s.height())
            .max()
            .unwrap_or_default()
    }

    #[must_use]
    pub fn color_at_layer(
        &self,
        layer: u32,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Option<Rgba> {
        if layer == 0 {
            Some(self.scheme.color_unchecked(width, height, x, y))
        } else if let Some(partition) = &self.partition {
            let (pos, rect) = partition
                .rects
                .iter()
                .enumerate()
                .find(|(_, rect)| rect.contains(x as u32, y as u32))
                .unwrap();
            let subscheme = &self.subschemes[pos];

            // Map the coordinates to the subscheme (so top left of the rect goes to (0, 0), etc.)
            let (width, height) = (rect.width() as usize, rect.height() as usize);
            let (left, top) = rect.top_left();
            let (x, y) = (x - left as usize, y - top as usize);

            subscheme.color_at_layer(layer - 1, width, height, x, y)
        } else {
            None
        }
    }
}

impl ColorScheme for RecursiveScheme {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.scheme.is_valid_size(width, height)
    }

    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        self.scheme.color_unchecked(width, height, x, y)
    }
}

pub struct IndexedRecursiveScheme {
    scheme: RecursiveScheme,
    index: u32,
}

impl IndexedRecursiveScheme {
    #[must_use]
    pub fn new(scheme: RecursiveScheme) -> Self {
        Self { scheme, index: 0 }
    }

    pub fn ascend(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn descend(&mut self) {
        if self.index + 1 < self.scheme.height() {
            self.index += 1;
        }
    }

    #[must_use]
    pub fn subscheme_color(&self, width: usize, height: usize, x: usize, y: usize) -> Option<Rgba> {
        self.scheme
            .color_at_layer(self.index + 1, width, height, x, y)
    }
}

impl ColorScheme for IndexedRecursiveScheme {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        let partition_valid = if let Some(p) = &self.scheme.partition {
            p.is_valid_size(width, height)
        } else {
            true
        };
        let scheme_valid = self.scheme.scheme.is_valid_size(width, height);

        partition_valid && scheme_valid
    }

    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        self.scheme
            .color_at_layer(self.index, width, height, x, y)
            .unwrap()
    }
}

impl From<Scheme> for RecursiveScheme {
    fn from(scheme: Scheme) -> Self {
        Self::new_leaf(scheme)
    }
}

impl From<RecursiveScheme> for IndexedRecursiveScheme {
    fn from(scheme: RecursiveScheme) -> Self {
        Self::new(scheme)
    }
}

impl From<Scheme> for IndexedRecursiveScheme {
    fn from(scheme: Scheme) -> Self {
        Self::from(RecursiveScheme::from(scheme))
    }
}
