use std::fmt::Display;

use ab_glyph::{point, Font, FontRef, ScaleFont};
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use itertools::Itertools;
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

#[must_use]
fn convert_rgb(c: Rgb) -> Rgba<u8> {
    let (r, g, b) = c.into_format::<u8>().into_components();
    Rgba([r, g, b, 255])
}

fn draw_centered_text(
    image: &mut RgbaImage,
    font: &FontRef,
    size: f32,
    pos: (f32, f32),
    text: &str,
    text_color: Rgba<u8>,
) {
    // Calculate the bounding box of the text if we were to draw it with the bottom left corner
    // at the point (0, 0)
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (0.0f32, 0.0f32);

    // Loop over pairs of consecutive characters so we can compute kerning between them,
    // and store the start point of each character (relative to (0.0, 0.0))
    let mut character_offsets = Vec::new();
    for (c1, c2) in text.chars().tuple_windows() {
        character_offsets.push(max_x);

        // Place the glyph directly to the right of the bounding rect of all previous glyphs
        let glyph = font
            .glyph_id(c1)
            .with_scale_and_position(size, point(max_x, 0.0));

        // Update the bounding rect to include the new glyph
        let Some(outline) = font.outline_glyph(glyph) else { continue };
        let rect = outline.px_bounds();
        let kerning = font
            .as_scaled(size)
            .kern(font.glyph_id(c1), font.glyph_id(c2));
        (min_x, min_y) = (min_x.min(rect.min.x), min_y.min(rect.min.y));
        (max_x, max_y) = (max_x.max(rect.max.x - kerning), max_y.max(rect.max.y));
    }

    // Do the same thing with the last character (no kerning involved because there is no next
    // character)
    if let Some(c) = text.chars().last() {
        character_offsets.push(max_x);

        let glyph = font
            .glyph_id(c)
            .with_scale_and_position(size, point(max_x, 0.0));

        if let Some(outline) = font.outline_glyph(glyph) {
            let rect = outline.px_bounds();
            (min_x, min_y) = (min_x.min(rect.min.x), min_y.min(rect.min.y));
            (max_x, max_y) = (max_x.max(rect.max.x), max_y.max(rect.max.y));
        }
    }

    // Calculate the position that we should start drawing from (instead of (0.0, 0.0)) so that
    // the text ends up centered at `pos`
    let (dx, dy) = ((max_x + min_x) / 2.0, (max_y + min_y) / 2.0);
    let (pos_x, pos_y) = (pos.0 - dx, pos.1 - dy);

    // Render the characters to the image
    for (c, offset) in text.chars().zip(character_offsets.iter()) {
        let glyph = font
            .glyph_id(c)
            .with_scale_and_position(size, point(pos_x + offset, pos_y));
        let Some(outline) = font.outline_glyph(glyph) else { continue };

        let rect = outline.px_bounds();
        let top_left = (rect.min.x as u32, rect.min.y as u32);
        outline.draw(|x, y, c| {
            let (px, py) = (top_left.0 + x, top_left.1 + y);
            if let Some(pixel) = image.get_pixel_mut_checked(px, py) {
                let channels = text_color.channels();
                let base_color = Rgba([
                    channels[0],
                    channels[1],
                    channels[2],
                    (c * 255.0).floor() as u8,
                ]);
                pixel.blend(&base_color);
            }
        });
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RendererError {
    #[error(
        "IncompatibleLabel: puzzle size ({width}x{height}) can not be used with the given label"
    )]
    IncompatibleLabel { width: usize, height: usize },
}

pub struct Renderer<'a, 'b> {
    scheme: Box<dyn ColorScheme>,
    text_scheme: Box<dyn ColorScheme>,
    font: &'a FontRef<'b>,
    draw_borders: bool,
    tile_size: u32,
    tile_rounding: u32,
    tile_gap: u32,
    font_size: f32,
}

impl<'a, 'b> Renderer<'a, 'b> {
    #[must_use]
    pub fn with_scheme_and_font(scheme: Box<dyn ColorScheme>, font: &'a FontRef<'b>) -> Self {
        Self {
            scheme,
            text_scheme: Box::new(Scheme::new(
                Box::new(Trivial),
                Box::new(Monochrome::new(Rgb::new(0.0, 0.0, 0.0))),
            )),
            font,
            draw_borders: false,
            tile_size: 75,
            tile_rounding: 0,
            tile_gap: 0,
            font_size: 30.0,
        }
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
    pub fn tile_size(mut self, size: u32) -> Self {
        self.tile_size = size;
        self
    }

    #[must_use]
    pub fn tile_rounding(mut self, rounding: u32) -> Self {
        self.tile_rounding = rounding;
        self
    }

    #[must_use]
    pub fn tile_gap(mut self, gap: u32) -> Self {
        self.tile_gap = gap;
        self
    }

    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn render<Piece, P>(
        &self,
        puzzle: &P,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, RendererError>
    where
        Piece: PrimInt + Display,
        P: SlidingPuzzle<Piece>,
    {
        let (w, h) = puzzle.size();

        if !self.scheme.is_valid_size(w, h) {
            return Err(RendererError::IncompatibleLabel {
                width: w,
                height: h,
            });
        }

        let tile_size = self.tile_size;

        let (w, h) = (w as u32, h as u32);
        let (image_w, image_h) = {
            let a = if self.draw_borders { 1 } else { 0 };
            (w * tile_size + a, h * tile_size + a)
        };

        let mut img = RgbaImage::new(image_w as u32, image_h as u32);

        for y in 0..h {
            for x in 0..w {
                let piece = puzzle.piece_at_xy_unchecked(x as usize, y as usize);

                if piece != Piece::zero() {
                    let solved_pos = puzzle.solved_pos_xy_unchecked(piece);

                    let color = self.scheme.color_unchecked(
                        w as usize,
                        h as usize,
                        solved_pos.0,
                        solved_pos.1,
                    );
                    let color = convert_rgb(color);

                    let text_color = self.text_scheme.color_unchecked(
                        w as usize,
                        h as usize,
                        solved_pos.0,
                        solved_pos.1,
                    );
                    let text_color = convert_rgb(text_color);

                    let (rect_x, rect_y) = ((tile_size * x) as i32, (tile_size * y) as i32);
                    let rect = Rect::at(rect_x, rect_y).of_size(tile_size, tile_size);

                    drawing::draw_filled_rect_mut(&mut img, rect, color);
                    if self.draw_borders {
                        drawing::draw_hollow_rect_mut(
                            &mut img,
                            Rect::at(rect_x, rect_y).of_size(tile_size + 1, tile_size + 1),
                            Rgba([0u8, 0u8, 0u8, 255u8]),
                        );
                    }

                    let text = piece.to_string();
                    let (x, y) = (x as f32, y as f32);
                    draw_centered_text(
                        &mut img,
                        self.font,
                        self.font_size,
                        (tile_size as f32 * (x + 0.5), tile_size as f32 * (y + 0.5)),
                        &text,
                        text_color,
                    );
                }
            }
        }

        Ok(img)
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
            .add(Style::new("text { font-family: 'DejaVu Sans'; }"))
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

                        let mut r = Rectangle::new()
                            .set("x", borders / 2.0 + rect_pos.0)
                            .set("y", borders / 2.0 + rect_pos.1)
                            .set("rx", self.tile_rounding)
                            .set("ry", self.tile_rounding)
                            .set("width", tile_size)
                            .set("height", tile_size)
                            .set("fill", color_str);

                        if self.draw_borders {
                            r = r.set("stroke", "black");
                        }

                        r
                    };

                    let text = {
                        let color: Rgb<_, u8> = self
                            .text_scheme
                            .color_unchecked(w as usize, h as usize, solved_pos.0, solved_pos.1)
                            .into_format();
                        let color_str = format!("#{color:x}");

                        Text::new()
                            .set("x", (borders + rect_pos.0) + tile_size * 0.5)
                            .set("y", (borders + rect_pos.1) + tile_size * 0.5)
                            .set("font-size", self.font_size)
                            .set("dominant-baseline", "central")
                            .set("text-anchor", "middle")
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
