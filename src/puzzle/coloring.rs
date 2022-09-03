pub mod coloring;
pub mod lightness;

pub mod colorings {
    pub use super::{
        coloring::{
            AlternatingBrightness, ColorList, ColorListError, Coloring, Monochrome, Rainbow,
            RainbowBright, RainbowBrightFull, RainbowFull,
        },
        lightness::AddLightness,
    };
}
