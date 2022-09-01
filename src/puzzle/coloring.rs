pub mod coloring;
pub mod lightness;

pub mod colorings {
    pub use super::{
        coloring::{
            AlternatingBrightness, ColorList, ColorListError, Coloring, Monochrome, Rainbow,
            RainbowFull,
        },
        lightness::AddLightness,
    };
}
