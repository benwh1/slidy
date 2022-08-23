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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monochrome() {
        let c = Rgb::new(0.2718, 0.3141, 0.6931);
        let a = Monochrome(c);
        assert_eq!(a.color(1, 3), c);
    }

    mod color_list {
        use super::*;

        #[test]
        fn test_new() {
            let a = ColorList::new(Vec::new());
            assert_eq!(a, Err(ColoringError::EmptyColorList));
        }

        #[test]
        fn test_new_2() {
            let a = ColorList::new(vec![
                Rgb::new(0.1, 0.2, 0.3),
                Rgb::new(0.1, 0.3, 0.6),
                Rgb::new(0.6, 0.3, 0.4),
            ]);
            assert!(a.is_ok());
        }

        #[test]
        fn test_color_list() {
            let c = vec![
                Rgb::new(0.1, 0.2, 0.3),
                Rgb::new(0.1, 0.3, 0.6),
                Rgb::new(0.6, 0.3, 0.4),
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
}
