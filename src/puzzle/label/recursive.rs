use thiserror::Error;

use super::{label::Label, rect_partition::RectPartition};

pub struct RecursiveLabel {
    label: Box<dyn Label>,
    partition: Option<RectPartition>,
    sublabels: Vec<RecursiveLabel>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecursiveLabelError {
    #[error(
        "InvalidSublabels: {num_labels} sublabels were given, but the partition has {num_rects} pieces"
    )]
    InvalidSublabels { num_labels: usize, num_rects: usize },

    #[error(
        "InvalidPartitionRect: top left corner of partition rect is ({left}, {top}), expected (0, 0)",
        left = top_left.0,
        top = top_left.1,
    )]
    InvalidPartitionRect { top_left: (u32, u32) },
}

impl RecursiveLabel {
    fn new<L: Label + 'static>(
        label: L,
        partition: RectPartition,
        sublabels: Vec<Self>,
    ) -> Result<Self, RecursiveLabelError> {
        if partition.num_rects() != sublabels.len() {
            Err(RecursiveLabelError::InvalidSublabels {
                num_labels: sublabels.len(),
                num_rects: partition.num_rects(),
            })
        } else if partition.rect().top_left() != (0, 0) {
            Err(RecursiveLabelError::InvalidPartitionRect {
                top_left: partition.rect().top_left(),
            })
        } else {
            Ok(Self {
                label: Box::new(label),
                partition: Some(partition),
                sublabels,
            })
        }
    }
}

impl<L: Label + 'static> From<L> for RecursiveLabel {
    fn from(label: L) -> Self {
        Self {
            label: Box::new(label),
            partition: None,
            sublabels: Vec::new(),
        }
    }
}
