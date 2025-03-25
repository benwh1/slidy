//! Defines the [`Coloring`] trait and several implementations.

use std::cmp::Ordering;

use blanket::blanket;
use enterpolation::{
    linear::{Linear, LinearError},
    Curve, Identity, Sorted,
};
use palette::{rgb::Rgba, FromColor, Hsl, Hsla, IntoColor as _, LinSrgba};
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
    fn color(&self, label: u64, num_labels: u64) -> Rgba;

    /// See [`Coloring::color`].
    #[must_use]
    fn try_color(&self, label: u64, num_labels: u64) -> Option<Rgba> {
        (label < num_labels).then(|| self.color(label, num_labels))
    }
}

impl<T: Coloring + ?Sized> Coloring for Box<T> {
    fn color(&self, label: u64, num_labels: u64) -> Rgba {
        (**self).color(label, num_labels)
    }
}

/// Error type for [`ColorList`]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
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
    /// The color that this [`Coloring`] always produces.
    pub color: Rgba,
}

/// A [`Coloring`] that cycles through a given list of colors.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(try_from = "ColorListUnvalidated")
)]
pub struct ColorList {
    colors: Vec<Rgba>,
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
struct ColorListUnvalidated {
    colors: Vec<Rgba>,
}

impl TryFrom<ColorListUnvalidated> for ColorList {
    type Error = ColorListError;

    fn try_from(value: ColorListUnvalidated) -> Result<Self, Self::Error> {
        let ColorListUnvalidated { colors } = value;

        Self::new(colors)
    }
}

/// A [`Coloring`] that produces rainbow colors.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rainbow {
    /// The minimum hue value, in degrees. This will be used to color the first label.
    pub min_hue: f32,

    /// The maximum hue value, in degrees. This will be used to color the last label.
    pub max_hue: f32,

    /// Brightness adjustment. Affects red and blue more than other colors.
    pub brightness: f32,
}

/// Given a [`Coloring`] `T`, makes the colors brighter when `label` is even.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AlternatingBrightness<C: Coloring>(pub C);

/// Given a [`Coloring`] `C`, adds a fixed constant to the HSL lightness value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AddLightness<C: Coloring> {
    coloring: C,
    lightness: f32,
}

/// A [`Coloring`] that produces a gradient effect by interpolating between a given list of colors.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Gradient<C: Curve<f32, Output = LinSrgba>> {
    gradient: C,
}

/// Type alias of a [`Gradient`] that uses linear interpolation.
pub type LinearGradient = Gradient<Linear<Sorted<Vec<f32>>, Vec<LinSrgba>, Identity>>;

impl Monochrome {
    /// Creates a new [`Monochrome`] that always produces `color`.
    #[must_use]
    pub fn new(color: Rgba) -> Self {
        Self { color }
    }
}

impl Coloring for Monochrome {
    fn color(&self, _label: u64, _num_labels: u64) -> Rgba {
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
    fn color(&self, label: u64, _num_labels: u64) -> Rgba {
        self.colors[label as usize % self.colors.len()]
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
    fn color(&self, label: u64, num_labels: u64) -> Rgba {
        if num_labels <= 1 {
            Hsl::new(0.0, 1.0, 0.5).into_color()
        } else {
            // Interpolate between the min and max hues
            let frac = label as f32 / (num_labels - 1) as f32;
            let hue = self.min_hue + (self.max_hue - self.min_hue) * frac;
            let hue = hue % 360.0;
            let lum = 0.5
                + (0.25 * f32::cos(std::f32::consts::TAU * (0.65 + hue / 720.0))
                    + 0.35 * f32::exp(-hue / 100.0))
                    * self.brightness;
            Hsl::new(hue, 1.0, lum).into_color()
        }
    }
}

impl Default for Rainbow {
    fn default() -> Self {
        Self {
            min_hue: 0.0,
            max_hue: 330.0,
            brightness: 1.0,
        }
    }
}

impl<C: Coloring> Coloring for AlternatingBrightness<C> {
    fn color(&self, label: u64, num_labels: u64) -> Rgba {
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
    fn color(&self, label: u64, num_labels: u64) -> Rgba {
        let color = self.coloring.color(label, num_labels);
        let color: Hsla = color.into_color();
        let (h, s, l, a) = color.into_components();
        let l = (l + self.lightness).clamp(0.0, 1.0);

        Hsla::new(h, s, l, a).into_color()
    }
}

impl LinearGradient {
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
    fn color(&self, label: u64, num_labels: u64) -> Rgba {
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
    use super::*;

    #[test]
    fn test_monochrome() {
        use std::f32::consts::{E, LN_2, PI, SQRT_2};

        let c = Rgba::new(E / 10.0, PI / 10.0, LN_2, SQRT_2 - 1.0);
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
        let a = Rainbow {
            min_hue: 0.0,
            max_hue: 330.0,
            brightness: 0.0,
        };

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
        let g = Gradient::linear(vec![
            (0.0, LinSrgba::new(1.0, 0.0, 0.5, 1.0)),
            (0.5, LinSrgba::new(0.5, 0.25, 1.0, 0.5)),
            (0.8, LinSrgba::new(0.5, 0.75, 0.5, 1.0)),
            (1.0, LinSrgba::new(0.0, 0.5, 0.2, 1.0)),
        ])
        .unwrap();

        let expected = vec![
            Some(LinSrgba::new(1.0, 0.0, 0.5, 1.0)),
            Some(LinSrgba::new(0.8, 0.10, 0.7, 0.8)),
            Some(LinSrgba::new(0.6, 0.20, 0.9, 0.6)),
            Some(LinSrgba::new(0.5, 5.0 / 12.0, 5.0 / 6.0, 2.0 / 3.0)),
            Some(LinSrgba::new(0.5, 0.75, 0.5, 1.0)),
            Some(LinSrgba::new(0.0, 0.5, 0.2, 1.0)),
            None,
        ]
        .into_iter()
        .map(|c| {
            c.map(|c| {
                let c: Rgba = c.into_color();
                c.into_components()
            })
        })
        .collect::<Vec<_>>();

        for (i, expected) in expected.iter().enumerate() {
            let color = g.try_color(i as u64, 6).map(|c| c.into_components());

            match (color, expected) {
                (None, None) => {}
                (None, Some(_)) | (Some(_), None) => panic!("One color is None, the other is Some"),
                (Some((cr, cg, cb, ca)), Some((er, eg, eb, ea))) => {
                    assert!(
                        (er - cr).abs() < 1e-6
                            && (eg - cg).abs() < 1e-6
                            && (eb - cb).abs() < 1e-6
                            && (ea - ca).abs() < 1e-6
                    );
                }
            }
        }
    }
}
