use palette::rgb::Rgb;
use thiserror::Error;

use crate::puzzle::{
    coloring::Coloring,
    label::{label::Label, rect_partition::RectPartition},
};

pub trait ColorScheme {
    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgb;

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

impl ColorScheme for Scheme {
    fn color_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> Rgb {
        let label = self.label.position_label_unchecked(width, height, x, y);
        let num_labels = self.label.num_labels_unchecked(width, height);
        self.coloring.color(label, num_labels)
    }
}
