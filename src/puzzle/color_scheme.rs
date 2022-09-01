use palette::rgb::Rgb;
use thiserror::Error;

use crate::puzzle::{
    coloring::coloring::Coloring,
    label::{label::Label, rect_partition::RectPartition},
};

pub trait ColorScheme {
    #[must_use]
    fn is_valid_size(&self, width: usize, height: usize) -> bool;

    #[must_use]
    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgb;

    #[must_use]
    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Option<Rgb> {
        if x < width && y < height {
            Some(self.color_unchecked(width, height, x, y))
        } else {
            None
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

    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgb {
        let label = self.label.position_label_unchecked(width, height, x, y);
        let num_labels = self.label.num_labels_unchecked(width, height);
        self.coloring.color(label, num_labels)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecursiveSchemeError {
    #[error("IncompatiblePartitionAndSubschemes: partition has {num_rects} rects, but {num_subschemes} subschemes were given")]
    IncompatiblePartitionAndSubschemes {
        num_rects: usize,
        num_subschemes: usize,
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
        if partition.num_rects() == subschemes.len() {
            Ok(Self {
                scheme,
                partition: Some(partition),
                subschemes,
            })
        } else {
            Err(RecursiveSchemeError::IncompatiblePartitionAndSubschemes {
                num_rects: partition.num_rects(),
                num_subschemes: subschemes.len(),
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
        if self.subschemes.is_empty() {
            1
        } else {
            1 + self.subschemes[0].height()
        }
    }
}

impl RecursiveScheme {
    #[must_use]
    pub fn color_at_layer(
        &self,
        layer: u32,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Rgb {
        if layer == 0 || self.partition.is_none() {
            self.scheme.color_unchecked(width, height, x, y)
        } else {
            let (pos, rect) = self
                .partition
                .as_ref()
                .unwrap()
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
        }
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

    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgb {
        self.scheme.color_at_layer(self.index, width, height, x, y)
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
