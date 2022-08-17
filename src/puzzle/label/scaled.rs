use thiserror::Error;

use super::label::Label;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scaled<'a, L: Label> {
    label: &'a L,
    horizontal: u32,
    vertical: u32,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScaledError {
    #[error("ZeroScale: horizontal and vertical scale factors must be positive")]
    ZeroScale,
}

impl<'a, L: Label> Scaled<'a, L> {
    pub fn new(label: &'a L, horizontal: u32, vertical: u32) -> Result<Self, ScaledError> {
        if horizontal == 0 || vertical == 0 {
            Err(ScaledError::ZeroScale)
        } else {
            Ok(Self {
                label,
                horizontal,
                vertical,
            })
        }
    }
}

impl<'a, L: Label> Label for Scaled<'a, L> {
    fn position_label(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let width = width.div_ceil(self.horizontal as usize);
        let height = height.div_ceil(self.vertical as usize);
        let x = x.div_ceil(self.horizontal as usize);
        let y = y.div_ceil(self.vertical as usize);
        self.label.position_label(width, height, x, y)
    }

    fn num_labels(&self, width: usize, height: usize) -> usize {
        let width = width.div_ceil(self.horizontal as usize);
        let height = height.div_ceil(self.vertical as usize);
        self.label.num_labels(width, height)
    }
}
