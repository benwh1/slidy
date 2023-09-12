//! Defines the [`Coloring`] trait and several implementations.

use std::cmp::Ordering;

use blanket::blanket;
use enterpolation::{
    linear::{Linear, LinearError},
    Curve, Identity, Sorted,
};
use palette::{rgb::Rgba, FromColor, Hsl, Hsla, IntoColor, LinSrgba};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Provides a function mapping labels to colors.
///
/// See also: [`crate::puzzle::label::label::Label`].
#[blanket(derive(Ref, Rc, Arc, Mut))]
pub trait Coloring {
    /// Returns a color based on a label and the total number of labels, or `None` if `label` is
    /// out of bounds (i.e. `label >= num_labels`).
    ///
    /// This function does not check that `label` is within bounds (i.e. `label < num_labels`).
    /// If it is not, the function may panic or return any other color.
    #[must_use]
    fn color(&self, label: usize, num_labels: usize) -> Rgba;

    /// See [`Coloring::color`].
    #[must_use]
    fn try_color(&self, label: usize, num_labels: usize) -> Option<Rgba> {
        if label < num_labels {
            Some(self.color(label, num_labels))
        } else {
            None
        }
    }
}

impl<T: Coloring + ?Sized> Coloring for Box<T> {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        (**self).color(label, num_labels)
    }
}

/// Error type for [`ColorList`]
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColorListError {
    /// Returned when [`ColorList::new`] is given an empty list.
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

/// A [`Coloring`] that always produces the same color.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Monochrome {
    color: Rgba,
}

/// A [`Coloring`] that cycles through a given list of colors.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorList {
    colors: Vec<Rgba>,
}

/// A [`Coloring`] that produces rainbow colors.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rainbow;

/// Similar to [`Rainbow`] but produces slightly different colors.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RainbowFull;

/// Similar to [`Rainbow`] but produces brighter, more pastel-like colors.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RainbowBright;

/// Combination of [`RainbowBright`] and [`RainbowFull`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RainbowBrightFull;

/// Given a [`Coloring`] `T`, makes the colors brighter when `label` is even.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AlternatingBrightness<C: Coloring>(pub C);

/// Given a [`Coloring`] `C`, adds a fixed constant to the HSL lightness value.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AddLightness<C: Coloring> {
    coloring: C,
    lightness: f32,
}

/// A [`Coloring`] that produces a gradient effect by interpolating between a given list of colors.
pub struct Gradient<C: Curve<f32, Output = LinSrgba>> {
    gradient: C,
}

impl Monochrome {
    /// Creates a new [`Monochrome`] that always produces `color`.
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
    fn color(&self, label: usize, _num_labels: usize) -> Rgba {
        self.colors[label % self.colors.len()]
    }
}

impl Default for ColorList {
    fn default() -> Self {
        Self {
            colors: vec![Rgba::default()],
        }
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

impl<C: Coloring> Coloring for AlternatingBrightness<C> {
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

impl<C: Coloring> AddLightness<C> {
    /// Creates a new [`AddLightness`] from `coloring` that adds `lightness` to the lightness value.
    pub fn new(coloring: C, lightness: f32) -> Self {
        Self {
            coloring,
            lightness,
        }
    }
}

impl<C: Coloring> Coloring for AddLightness<C> {
    /// Calls `self.coloring.color` and adds `self.lightness` to the HSL lightness value.
    /// The lightness is clamped to the interval `[0.0, 1.0]`.
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let color = self.coloring.color(label, num_labels);
        let color: Hsla = color.into_color();
        let (h, s, l, a) = color.into_components();
        let l = (l + self.lightness).clamp(0.0, 1.0);

        Hsla::new(h, s, l, a).into_color()
    }
}

impl Gradient<Linear<Sorted<Vec<f32>>, Vec<LinSrgba>, Identity>> {
    /// Creates a new [`Gradient`] from a list of `(t, color)` pairs using linear interpolation,
    /// where the `t`s are values from 0 to 1 defining how the colors should be spread out.
    pub fn linear<Color>(mut points: Vec<(f32, Color)>) -> Result<Self, LinearError>
    where
        LinSrgba: FromColor<Color>,
    {
        points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Ordering::Less));

        let knots = points.iter().map(|&(x, _)| x).collect::<Vec<_>>();
        let colors = points
            .into_iter()
            .map(|(_, c)| c)
            .map(LinSrgba::from_color)
            .collect::<Vec<_>>();

        let gradient = Linear::builder().elements(colors).knots(knots).build()?;
        Ok(Self { gradient })
    }
}

impl<C: Curve<f32, Output = LinSrgba>> Coloring for Gradient<C> {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let point = if num_labels < 2 {
            0.0
        } else {
            label as f32 / (num_labels - 1) as f32
        };

        self.gradient.gen(point).into_color()
    }
}

#[cfg(test)]
mod tests {
    use palette::LinSrgba;

    use super::*;

    #[test]
    fn test_monochrome() {
        let c = Rgba::new(0.2718, 0.3141, 0.6931, 0.4142);
        let a = Monochrome::new(c);
        assert_eq!(a.try_color(0, 3), Some(c));
        assert_eq!(a.try_color(1, 3), Some(c));
        assert_eq!(a.try_color(2, 3), Some(c));
        assert_eq!(a.try_color(3, 3), None);
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
            assert_eq!(a.try_color(0, 10), Some(c[0]));
            assert_eq!(a.try_color(1, 10), Some(c[1]));
            assert_eq!(a.try_color(2, 10), Some(c[2]));
            assert_eq!(a.try_color(3, 10), Some(c[0]));
            assert_eq!(a.try_color(4, 10), Some(c[1]));
            assert_eq!(a.try_color(5, 10), Some(c[2]));
            assert_eq!(a.try_color(6, 10), Some(c[0]));
            assert_eq!(a.try_color(10, 10), None);
        }
    }

    #[test]
    fn test_rainbow() {
        let a = Rainbow;

        assert_eq!(
            a.try_color(0, 1),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(1, 1), None);

        assert_eq!(
            a.try_color(0, 2),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(1, 2),
            Some(Hsl::new(165.0, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(2, 2), None);

        assert_eq!(
            a.try_color(0, 4),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(1, 4),
            Some(Hsl::new(82.5, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(2, 4),
            Some(Hsl::new(165.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(3, 4),
            Some(Hsl::new(247.5, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(4, 4), None);
    }

    #[test]
    fn test_rainbow_full() {
        let a = RainbowFull;

        assert_eq!(
            a.try_color(0, 1),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(1, 1), None);

        assert_eq!(
            a.try_color(0, 2),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(1, 2),
            Some(Hsl::new(330.0, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(2, 2), None);

        assert_eq!(
            a.try_color(0, 5),
            Some(Hsl::new(0.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(1, 5),
            Some(Hsl::new(82.5, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(2, 5),
            Some(Hsl::new(165.0, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(3, 5),
            Some(Hsl::new(247.5, 1.0, 0.5).into_color())
        );
        assert_eq!(
            a.try_color(4, 5),
            Some(Hsl::new(330.0, 1.0, 0.5).into_color())
        );
        assert_eq!(a.try_color(5, 5), None);
    }

    #[test]
    fn test_gradient() {
        let g = Gradient::with_domain(vec![
            (0.0, LinSrgba::new(1.0, 0.0, 0.5, 1.0)),
            (0.5, LinSrgba::new(0.5, 0.25, 1.0, 0.5)),
            (0.8, LinSrgba::new(0.5, 0.75, 0.5, 1.0)),
            (1.0, LinSrgba::new(0.0, 0.5, 0.2, 1.0)),
        ]);

        let expected = vec![
            Some(LinSrgba::new(1.0, 0.0, 0.5, 1.0).into_color()),
            Some(LinSrgba::new(0.8, 0.10, 0.7, 0.8).into_color()),
            Some(LinSrgba::new(0.6, 0.20, 0.9, 0.6).into_color()),
            Some(LinSrgba::new(0.5, 5.0 / 12.0, 5.0 / 6.0, 2.0 / 3.0).into_color()),
            Some(LinSrgba::new(0.5, 0.75, 0.5, 1.0).into_color()),
            Some(LinSrgba::new(0.0, 0.5, 0.2, 1.0).into_color()),
            None,
        ];

        for i in 0..7 {
            assert_eq!(g.try_color(i, 6), expected[i]);
        }
    }
}
