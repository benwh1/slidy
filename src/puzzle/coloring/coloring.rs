use palette::{rgb::Rgb, Gradient, Hsl, IntoColor};
use thiserror::Error;

pub trait Coloring {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}

#[derive(Error, Debug)]
pub enum ColoringError {
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

pub struct Monochrome(Rgb);

pub struct ColorList {
    colors: Vec<Rgb>,
}

pub struct Rainbow;

pub struct AlternatingBrightness<'a, T: Coloring>(pub &'a T);

impl Coloring for Monochrome {
    fn color(&self, _label: usize, _num_labels: usize) -> Rgb {
        self.0
    }
}

impl ColorList {
    pub fn new(colors: Vec<Rgb>) -> Result<Self, ColoringError> {
        if colors.is_empty() {
            Err(ColoringError::EmptyColorList)
        } else {
            Ok(Self { colors })
        }
    }
}

impl Coloring for ColorList {
    #[must_use]
    fn color(&self, label: usize, _num_labels: usize) -> Rgb {
        self.colors[label % self.colors.len()]
    }
}

impl Coloring for Rainbow {
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

impl<'a, T: Coloring> Coloring for AlternatingBrightness<'a, T> {
    fn color(&self, label: usize, num_labels: usize) -> Rgb {
        let l = (label / 2) * 2;
        let color = self.0.color(l, num_labels);
        if label == l {
            let color: Hsl = color.into_color();
            let (h, s, l) = color.into_components();
            let l = 1.0 - (1.0 - l) / 2.0;
            Hsl::new(h, s, l).into_color()
        } else {
            color
        }
    }
}
