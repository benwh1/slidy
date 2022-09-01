use std::fmt::Display;

use num_traits::PrimInt;
use palette::rgb::Rgb;
use svg::{
    node::{
        element::{Rectangle, Style, Text},
        Text as TextNode,
    },
    Document,
};
use thiserror::Error;

use super::{
    color_scheme::{ColorScheme, Scheme},
    coloring::coloring::Monochrome,
    label::label::Trivial,
    sliding_puzzle::SlidingPuzzle,
};

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RendererError {
    #[error(
        "IncompatibleLabel: puzzle size ({width}x{height}) can not be used with the given label"
    )]
    IncompatibleLabel { width: usize, height: usize },
}

pub struct Renderer {
    scheme: Box<dyn ColorScheme>,
    text_scheme: Box<dyn ColorScheme>,
    draw_borders: bool,
    tile_size: f32,
    tile_rounding: f32,
    tile_gap: f32,
    font_size: f32,
    text_position: (f32, f32),
}

impl Renderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgb::new(1.0, 1.0, 1.0))),
            )),
            text_scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgb::new(0.0, 0.0, 0.0))),
            )),
            draw_borders: false,
            tile_size: 75.0,
            tile_rounding: 0.0,
            tile_gap: 0.0,
            font_size: 30.0,
            text_position: (0.5, 0.5),
        }
    }

    #[must_use]
    pub fn scheme(mut self, scheme: Box<dyn ColorScheme>) -> Self {
        self.scheme = scheme;
        self
    }

    #[must_use]
    pub fn text_scheme(mut self, text_scheme: Box<dyn ColorScheme>) -> Self {
        self.text_scheme = text_scheme;
        self
    }

    #[must_use]
    pub fn borders(mut self, draw: bool) -> Self {
        self.draw_borders = draw;
        self
    }

    #[must_use]
    pub fn tile_size(mut self, size: f32) -> Self {
        self.tile_size = size.max(0.0);
        self
    }

    #[must_use]
    pub fn tile_rounding(mut self, rounding: f32) -> Self {
        self.tile_rounding = rounding.max(0.0);
        self
    }

    #[must_use]
    pub fn tile_gap(mut self, gap: f32) -> Self {
        self.tile_gap = gap.max(0.0);
        self
    }

    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(0.0);
        self
    }

    #[must_use]
    pub fn text_position(mut self, pos: (f32, f32)) -> Self {
        self.text_position = pos;
        self
    }

    pub fn svg<Piece, P>(&self, puzzle: &P) -> Result<Document, RendererError>
    where
        Piece: PrimInt + Display,
        P: SlidingPuzzle<Piece>,
    {
        let (width, height) = puzzle.size();

        if !self.scheme.is_valid_size(width, height) {
            return Err(RendererError::IncompatibleLabel { width, height });
        }

        let tile_size = self.tile_size as f32;
        let tile_gap = self.tile_gap as f32;
        let borders = if self.draw_borders { 1.0 } else { 0.0 };

        let (w, h) = (width as f32, height as f32);
        let (image_w, image_h) = (
            w * tile_size + (w - 1.0) * tile_gap + borders,
            h * tile_size + (h - 1.0) * tile_gap + borders,
        );

        let mut doc = Document::new()
            .add(Style::new(format!(
                "text {{\
                    text-anchor: middle;\
                    dominant-baseline: central;\
                    font-size: {fs}px;\
                    font-family: 'DejaVu Sans';\
                }}\
                rect {{\
                    width: {ts}px;\
                    height: {ts}px;\
                    rx: {tr}px;\
                    ry: {tr}px;\
                    {stroke}\
                }}",
                fs = self.font_size,
                ts = self.tile_size,
                tr = self.tile_rounding,
                stroke = if self.draw_borders {
                    "stroke: black;"
                } else {
                    ""
                }
            )))
            .set("width", image_w)
            .set("height", image_h);

        for y in 0..height {
            for x in 0..width {
                let piece = puzzle.piece_at_xy_unchecked(x, y);

                if piece != Piece::zero() {
                    let solved_pos = puzzle.solved_pos_xy_unchecked(piece);

                    let rect_pos = (
                        (tile_size + tile_gap) * x as f32,
                        (tile_size + tile_gap) * y as f32,
                    );

                    let rect = {
                        let color: Rgb<_, u8> = self
                            .scheme
                            .color_unchecked(w as usize, h as usize, solved_pos.0, solved_pos.1)
                            .into_format();
                        let color_str = format!("#{color:x}");

                        Rectangle::new()
                            .set("x", borders / 2.0 + rect_pos.0)
                            .set("y", borders / 2.0 + rect_pos.1)
                            .set("fill", color_str)
                    };

                    let text = {
                        let color: Rgb<_, u8> = self
                            .text_scheme
                            .color_unchecked(w as usize, h as usize, solved_pos.0, solved_pos.1)
                            .into_format();
                        let color_str = format!("#{color:x}");

                        let (tx, ty) = self.text_position;

                        Text::new()
                            .set("x", (borders / 2.0 + rect_pos.0) + tile_size * tx)
                            .set("y", (borders / 2.0 + rect_pos.1) + tile_size * ty)
                            .set("fill", color_str)
                            .add(TextNode::new(piece.to_string()))
                    };

                    doc = doc.add(rect).add(text);
                }
            }
        }

        Ok(doc)
    }
}
