use palette::{rgb::Rgba, Hsla, IntoColor};

use super::coloring::Coloring;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct AddLightness<'a, C: Coloring> {
    coloring: &'a C,
    lightness: f32,
}

impl<'a, C: Coloring> AddLightness<'a, C> {
    pub fn new(coloring: &'a C, lightness: f32) -> Self {
        Self {
            coloring,
            lightness,
        }
    }
}

impl<'a, C: Coloring> Coloring for AddLightness<'a, C> {
    fn color(&self, label: usize, num_labels: usize) -> Rgba {
        let color = self.coloring.color(label, num_labels);
        let color: Hsla = color.into_color();
        let (h, s, l, a) = color.into_components();
        let l = (l + self.lightness).clamp(0.0, 1.0);

        Hsla::new(h, s, l, a).into_color()
    }
}
