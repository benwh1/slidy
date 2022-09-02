use std::fmt::Display;

use num_traits::PrimInt;
use palette::rgb::Rgba;
use svg::{
    node::{
        element::{Group, Rectangle, Style, Text},
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

pub enum Font<'a> {
    Family(&'a str),
    Url { path: &'a str, format: &'a str },
    Base64 { data: &'a str, format: &'a str },
}

pub struct Borders {
    scheme: Box<dyn ColorScheme>,
    thickness: f32,
}

impl Borders {
    pub fn new() -> Self {
        Self {
            scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgba::new(1.0, 1.0, 1.0, 1.0))),
            )),
            thickness: 1.0,
        }
    }

    pub fn scheme(mut self, scheme: Box<dyn ColorScheme>) -> Self {
        self.scheme = scheme;
        self
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl Default for Borders {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Renderer<'a> {
    scheme: Box<dyn ColorScheme>,
    text_scheme: Box<dyn ColorScheme>,
    borders: Option<Borders>,
    font: Font<'a>,
    tile_size: f32,
    tile_rounding: f32,
    tile_gap: f32,
    font_size: f32,
    text_position: (f32, f32),
    padding: f32,
}

impl<'a> Renderer<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgba::new(1.0, 1.0, 1.0, 1.0))),
            )),
            text_scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 1.0))),
            )),
            borders: None,
            font: Font::Family("sans-serif"),
            tile_size: 75.0,
            tile_rounding: 0.0,
            tile_gap: 0.0,
            font_size: 30.0,
            text_position: (0.5, 0.5),
            padding: 0.0,
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
    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = Some(borders);
        self
    }

    #[must_use]
    pub fn font(mut self, font: Font<'a>) -> Self {
        self.font = font;
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

    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
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

        let border_thickness = self
            .borders
            .as_ref()
            .map(|a| a.thickness)
            .unwrap_or_default();

        let (w, h) = (width as f32, height as f32);
        let (image_w, image_h) = (
            w * self.tile_size + (w - 1.0) * self.tile_gap + border_thickness + 2.0 * self.padding,
            h * self.tile_size + (h - 1.0) * self.tile_gap + border_thickness + 2.0 * self.padding,
        );

        let style_str = {
            let font = if let Font::Family(f) = self.font {
                format!("text {{ font-family: {f}; }}")
            } else {
                let src = match self.font {
                    Font::Family(_) => unreachable!(),
                    Font::Url { path, format } => {
                        format!(r#"url({path}) format("{format}")"#)
                    }
                    Font::Base64 { data, format } => {
                        format!(r#"url(data:font/ttf;base64,{data}) format("{format}")"#)
                    }
                };

                format!(
                    "@font-face {{\
                        font-family: f;\
                        src: {src};\
                    }}\
                    text {{\
                        font-family: f;\
                    }}"
                )
            };

            format!(
                "text {{\
                    text-anchor: middle;\
                    dominant-baseline: central;\
                    font-size: {fs}px;\
                }}\
                rect {{\
                    width: {ts}px;\
                    height: {ts}px;\
                    rx: {tr}px;\
                    ry: {tr}px;\
                    stroke-width: {sw}px;\
                }}\
                {font}",
                fs = self.font_size,
                ts = self.tile_size,
                tr = self.tile_rounding,
                sw = border_thickness,
            )
        };

        let mut doc = Document::new()
            .add(Style::new(&style_str))
            .set("width", image_w)
            .set("height", image_h);

        for y in 0..height {
            for x in 0..width {
                let piece = puzzle.piece_at_xy_unchecked(x, y);

                if piece != Piece::zero() {
                    doc = doc.add(self.render_piece(puzzle, x, y));
                }
            }
        }

        Ok(doc)
    }

    fn render_piece<Piece, P>(&self, puzzle: &P, x: usize, y: usize) -> Group
    where
        Piece: PrimInt + Display,
        P: SlidingPuzzle<Piece>,
    {
        let (width, height) = puzzle.size();

        let border_thickness = self
            .borders
            .as_ref()
            .map(|a| a.thickness)
            .unwrap_or_default();

        let piece = puzzle.piece_at_xy_unchecked(x, y);
        let solved_pos = puzzle.solved_pos_xy_unchecked(piece);

        let (x, y) = (x as f32, y as f32);

        let rect_pos = (
            self.padding + border_thickness / 2.0 + (self.tile_size + self.tile_gap) * x,
            self.padding + border_thickness / 2.0 + (self.tile_size + self.tile_gap) * y,
        );

        let rect = {
            let fill = {
                let color: Rgba<_, u8> = self
                    .scheme
                    .color_unchecked(width, height, solved_pos.0, solved_pos.1)
                    .into_format();
                format!("#{color:x}")
            };

            let mut r = Rectangle::new()
                .set("x", rect_pos.0)
                .set("y", rect_pos.1)
                .set("fill", fill);

            if let Some(s) = &self.borders {
                let stroke = {
                    let color: Rgba<_, u8> = s
                        .scheme
                        .color_unchecked(width, height, solved_pos.0, solved_pos.1)
                        .into_format();
                    format!("#{color:x}")
                };

                r = r.set("stroke", stroke)
            }

            r
        };

        let text = {
            let fill = {
                let color: Rgba<_, u8> = self
                    .text_scheme
                    .color_unchecked(width, height, solved_pos.0, solved_pos.1)
                    .into_format();
                format!("#{color:x}")
            };

            let (tx, ty) = self.text_position;

            Text::new()
                .set("x", rect_pos.0 + self.tile_size * tx)
                .set("y", rect_pos.1 + self.tile_size * ty)
                .set("fill", fill)
                .add(TextNode::new(piece.to_string()))
        };

        Group::new().add(rect).add(text)
    }
}

impl Default for Renderer<'_> {
    fn default() -> Self {
        Self::new()
    }
}
