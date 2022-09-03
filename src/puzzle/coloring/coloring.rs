use palette::{rgb::Rgba, Hsl, Hsla, IntoColor};
use thiserror::Error;

pub trait Coloring {
    #[must_use]
    fn color(&self, label: usize, num_labels: usize) -> Rgba;
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorListError {
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Monochrome {
    color: Rgba,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorList {
    colors: Vec<Rgba>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rainbow;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowFull;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowBright;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowBrightFull;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlternatingBrightness<'a, T: Coloring>(pub &'a T);

impl Monochrome {
    #[must_use]
    pub fn new(color: Rgba) -> Self {
        Self { color }
    }
}

impl Coloring for Monochrome {
    fn color(&self, _label: usize, _num_labels: usize) -> Rgba {
        self.color
    }
}

impl ColorList {
    pub fn new(colors: Vec<Rgba>) -> Result<Self, ColorListError> {
        if colors.is_empty() {
            Err(ColorListError::EmptyColorList)
        } else {
            Ok(Self { colors })
        }
    }
}

impl Coloring for ColorList {
    fn color(&self, label: usize, _num_labels: usize) -> Rgba {
        self.colors[label % self.colors.len()]
    }
}

impl Coloring for Rainbow {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let frac = label as f32 / num_labels as f32;
        Hsl::new(330.0 * frac, 1.0, 0.5).into_color()
    }
}

impl Coloring for RainbowFull {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        if num_labels <= 1 {
            Hsl::new(0.0, 1.0, 0.5).into_color()
        } else {
            Rainbow.color(label, num_labels - 1)
        }
    }
}

impl Coloring for RainbowBright {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let frac = label as f32 / num_labels as f32;
        let hue = 330.0 * frac;
        let lum = 0.5
            + 0.25 * f32::cos(std::f32::consts::TAU * (0.65 + hue / 720.0))
            + 0.35 * f32::exp(-hue / 100.0);
        Hsl::new(hue, 1.0, lum).into_color()
    }
}

impl Coloring for RainbowBrightFull {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        if num_labels <= 1 {
            Hsl::new(0.0, 1.0, 0.5).into_color()
        } else {
            RainbowBright.color(label, num_labels - 1)
        }
    }
}

impl<'a, T: Coloring> Coloring for AlternatingBrightness<'a, T> {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let l = (label / 2) * 2;
        let color = self.0.color(l, num_labels);
        if label == l {
            let color: Hsla = color.into_color();
            let (h, s, l, a) = color.into_components();
            let l = 1.0 - (1.0 - l) / 2.0;
            Hsla::new(h, s, l, a).into_color()
        } else {
            color
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monochrome() {
        let c = Rgba::new(0.2718, 0.3141, 0.6931, 0.4142);
        let a = Monochrome::new(c);
        assert_eq!(a.color(1, 3), c);
    }

    mod color_list {
        use super::*;

        #[test]
        fn test_new() {
            let a = ColorList::new(Vec::new());
            assert_eq!(a, Err(ColorListError::EmptyColorList));
        }

        #[test]
        fn test_new_2() {
            let a = ColorList::new(vec![
                Rgba::new(0.1, 0.2, 0.3, 1.0),
                Rgba::new(0.1, 0.3, 0.6, 1.0),
                Rgba::new(0.6, 0.3, 0.4, 1.0),
            ]);
            assert!(a.is_ok());
        }

        #[test]
        fn test_color_list() {
            let c = vec![
                Rgba::new(0.1, 0.2, 0.3, 1.0),
                Rgba::new(0.1, 0.3, 0.6, 1.0),
                Rgba::new(0.6, 0.3, 0.4, 1.0),
            ];
            let a = ColorList::new(c.clone()).unwrap();
            assert_eq!(a.color(0, 10), c[0]);
            assert_eq!(a.color(1, 10), c[1]);
            assert_eq!(a.color(2, 10), c[2]);
            assert_eq!(a.color(3, 10), c[0]);
            assert_eq!(a.color(4, 10), c[1]);
            assert_eq!(a.color(5, 10), c[2]);
            assert_eq!(a.color(6, 10), c[0]);
        }
    }

    #[test]
    fn test_rainbow() {
        assert_eq!(Rainbow.color(0, 1), Hsl::new(0.0, 1.0, 0.5).into_color());

        assert_eq!(Rainbow.color(0, 2), Hsl::new(0.0, 1.0, 0.5).into_color());
        assert_eq!(Rainbow.color(1, 2), Hsl::new(165.0, 1.0, 0.5).into_color());

        assert_eq!(Rainbow.color(0, 4), Hsl::new(0.0, 1.0, 0.5).into_color());
        assert_eq!(Rainbow.color(1, 4), Hsl::new(82.5, 1.0, 0.5).into_color());
        assert_eq!(Rainbow.color(2, 4), Hsl::new(165.0, 1.0, 0.5).into_color());
        assert_eq!(Rainbow.color(3, 4), Hsl::new(247.5, 1.0, 0.5).into_color());
    }

    #[test]
    fn test_rainbow_full() {
        use RainbowFull as RF;

        assert_eq!(RF.color(0, 1), Hsl::new(0.0, 1.0, 0.5).into_color());

        assert_eq!(RF.color(0, 2), Hsl::new(0.0, 1.0, 0.5).into_color());
        assert_eq!(RF.color(1, 2), Hsl::new(330.0, 1.0, 0.5).into_color());

        assert_eq!(RF.color(0, 5), Hsl::new(0.0, 1.0, 0.5).into_color());
        assert_eq!(RF.color(1, 5), Hsl::new(82.5, 1.0, 0.5).into_color());
        assert_eq!(RF.color(2, 5), Hsl::new(165.0, 1.0, 0.5).into_color());
        assert_eq!(RF.color(3, 5), Hsl::new(247.5, 1.0, 0.5).into_color());
        assert_eq!(RF.color(4, 5), Hsl::new(330.0, 1.0, 0.5).into_color());
    }
}
