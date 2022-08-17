mod rect_partition;

use palette::{rgb::Rgb, Gradient, Hsl, IntoColor};
use thiserror::Error;

pub trait ColorScheme {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}

#[derive(Error, Debug)]
pub enum ColorSchemeError {
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

pub struct ColorList {
    colors: Vec<Rgb>,
}

pub struct Rainbow;

impl ColorList {
    pub fn new(colors: Vec<Rgb>) -> Result<Self, ColorSchemeError> {
        if colors.is_empty() {
            Err(ColorSchemeError::EmptyColorList)
        } else {
            Ok(Self { colors })
        }
    }
}

impl ColorScheme for ColorList {
    #[must_use]
    fn color(&self, label: usize, _num_labels: usize) -> Rgb {
        self.colors[label % self.colors.len()]
    }
}

impl ColorScheme for Rainbow {
    #[must_use]
    fn color(&self, label: usize, num_labels: usize) -> Rgb {
        let colors = [
            Hsl::new(0.0, 1.0, 0.5),
            Hsl::new(165.0, 1.0, 0.5),
            Hsl::new(330.0, 1.0, 0.5),
        ];
        let g = Gradient::new(colors);
        g.get(label as f32 / num_labels as f32).into_color()
    }
}
