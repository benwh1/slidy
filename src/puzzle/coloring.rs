//! Defines the [`Coloring`] trait and several implementations.

use palette::{rgb::Rgba, Hsl, Hsla, IntoColor};
use thiserror::Error;

/// Provides a function mapping labels to colors.
///
/// See also: [`crate::puzzle::label::label::Label`].
pub trait Coloring {
    /// See also: [`Coloring::color`].
    ///
    /// This function does not check that `label` is within bounds (i.e. `label < num_labels`).
    /// If it is not, the function may panic or return any other color.
    #[must_use]
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba;

    /// Returns a color based on a label and the total number of labels, or `None` if `label` is
    /// out of bounds (i.e. `label >= num_labels`).
    #[must_use]
    fn color(&self, label: usize, num_labels: usize) -> Option<Rgba> {
        if label < num_labels {
            Some(self.color_unchecked(label, num_labels))
        } else {
            None
        }
    }
}

/// Error type for [`ColorList`]
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorListError {
    /// Returned when [`ColorList::new`] is given an empty list.
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

/// A [`Coloring`] that always produces the same color.
#[derive(Clone, Debug, PartialEq)]
pub struct Monochrome {
    color: Rgba,
}

/// A [`Coloring`] that cycles through a given list of colors.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorList {
    colors: Vec<Rgba>,
}

/// A [`Coloring`] that produces rainbow colors.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rainbow;

/// Similar to [`Rainbow`] but produces slightly different colors.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowFull;

/// Similar to [`Rainbow`] but produces brighter, more pastel-like colors.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowBright;

/// Combination of [`RainbowBright`] and [`RainbowFull`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RainbowBrightFull;

/// Given a [`Coloring`] `T`, makes the colors brighter when `label` is even.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlternatingBrightness<'a, T: Coloring>(pub &'a T);

/// Given a [`Coloring`] `C`, adds a fixed constant to the HSL lightness value.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct AddLightness<'a, C: Coloring> {
    coloring: &'a C,
    lightness: f32,
}

impl Monochrome {
    /// Creates a new [`Monochrome`] that always produces `color`.
    #[must_use]
    pub fn new(color: Rgba) -> Self {
        Self { color }
    }
}

impl Coloring for Monochrome {
    fn color_unchecked(&self, _label: usize, _num_labels: usize) -> Rgba {
        self.color
    }
}

impl ColorList {
    /// Create a new [`ColorList`] from a vector of colors.
    pub fn new(colors: Vec<Rgba>) -> Result<Self, ColorListError> {
        if colors.is_empty() {
            Err(ColorListError::EmptyColorList)
        } else {
            Ok(Self { colors })
        }
    }
}

impl Coloring for ColorList {
    fn color_unchecked(&self, label: usize, _num_labels: usize) -> Rgba {
        self.colors[label % self.colors.len()]
    }
}

impl Coloring for Rainbow {
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        let frac = label as f32 / num_labels as f32;
        Hsl::new(330.0 * frac, 1.0, 0.5).into_color()
    }
}

impl Coloring for RainbowFull {
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        if num_labels <= 1 {
            Hsl::new(0.0, 1.0, 0.5).into_color()
        } else {
            Rainbow.color_unchecked(label, num_labels - 1)
        }
    }
}

impl Coloring for RainbowBright {
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        let frac = label as f32 / num_labels as f32;
        let hue = 330.0 * frac;
        let lum = 0.5
            + 0.25 * f32::cos(std::f32::consts::TAU * (0.65 + hue / 720.0))
            + 0.35 * f32::exp(-hue / 100.0);
        Hsl::new(hue, 1.0, lum).into_color()
    }
}

impl Coloring for RainbowBrightFull {
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        if num_labels <= 1 {
            Hsl::new(0.0, 1.0, 0.5).into_color()
        } else {
            RainbowBright.color_unchecked(label, num_labels - 1)
        }
    }
}

impl<'a, T: Coloring> Coloring for AlternatingBrightness<'a, T> {
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        let l = (label / 2) * 2;
        let color = self.0.color_unchecked(l, num_labels);
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

impl<'a, C: Coloring> AddLightness<'a, C> {
    /// Creates a new [`AddLightness`] from `coloring` that adds `lightness` to the lightness value.
    pub fn new(coloring: &'a C, lightness: f32) -> Self {
        Self {
            coloring,
            lightness,
        }
    }
}

impl<'a, C: Coloring> Coloring for AddLightness<'a, C> {
    /// Calls `self.coloring.color` and adds `self.lightness` to the HSL lightness value.
    /// The lightness is clamped to the interval `[0.0, 1.0]`.
    fn color_unchecked(&self, label: usize, num_labels: usize) -> Rgba {
        let color = self.coloring.color_unchecked(label, num_labels);
        let color: Hsla = color.into_color();
        let (h, s, l, a) = color.into_components();
        let l = (l + self.lightness).clamp(0.0, 1.0);

        Hsla::new(h, s, l, a).into_color()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monochrome() {
        let c = Rgba::new(0.2718, 0.3141, 0.6931, 0.4142);
        let a = Monochrome::new(c);
        assert_eq!(a.color(0, 3), Some(c));
        assert_eq!(a.color(1, 3), Some(c));
        assert_eq!(a.color(2, 3), Some(c));
        assert_eq!(a.color(3, 3), None);
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
            assert_eq!(a.color(0, 10), Some(c[0]));
            assert_eq!(a.color(1, 10), Some(c[1]));
            assert_eq!(a.color(2, 10), Some(c[2]));
            assert_eq!(a.color(3, 10), Some(c[0]));
            assert_eq!(a.color(4, 10), Some(c[1]));
            assert_eq!(a.color(5, 10), Some(c[2]));
            assert_eq!(a.color(6, 10), Some(c[0]));
            assert_eq!(a.color(10, 10), None);
        }
    }

    #[test]
    fn test_rainbow() {
        let a = Rainbow;

        assert_eq!(a.color(0, 1), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 1), None);

        assert_eq!(a.color(0, 2), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 2), Some(Hsl::new(165.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(2, 2), None);

        assert_eq!(a.color(0, 4), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 4), Some(Hsl::new(82.5, 1.0, 0.5).into_color()));
        assert_eq!(a.color(2, 4), Some(Hsl::new(165.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(3, 4), Some(Hsl::new(247.5, 1.0, 0.5).into_color()));
        assert_eq!(a.color(4, 4), None);
    }

    #[test]
    fn test_rainbow_full() {
        let a = RainbowFull;

        assert_eq!(a.color(0, 1), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 1), None);

        assert_eq!(a.color(0, 2), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 2), Some(Hsl::new(330.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(2, 2), None);

        assert_eq!(a.color(0, 5), Some(Hsl::new(0.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(1, 5), Some(Hsl::new(82.5, 1.0, 0.5).into_color()));
        assert_eq!(a.color(2, 5), Some(Hsl::new(165.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(3, 5), Some(Hsl::new(247.5, 1.0, 0.5).into_color()));
        assert_eq!(a.color(4, 5), Some(Hsl::new(330.0, 1.0, 0.5).into_color()));
        assert_eq!(a.color(5, 5), None);
    }
}
