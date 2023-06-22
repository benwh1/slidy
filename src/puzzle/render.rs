//! Defines the [`Renderer`] struct for creating SVG images of [`SlidingPuzzle`]s.

use std::{fmt::Display, marker::PhantomData, ops::Deref};

use num_traits::Zero;
use palette::rgb::Rgba;
use svg::{
    node::{
        element::{Group, Rectangle, Style, Text as TextElement},
        Text as TextNode,
    },
    Document,
};
use thiserror::Error;

use crate::puzzle::color_scheme::{Black, SchemeList};

use super::{color_scheme::ColorScheme, sliding_puzzle::SlidingPuzzle};

/// Error type for [`Renderer`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RendererError {
    /// Returned when the given puzzle size is incompatible with the label.
    #[error(
        "IncompatibleLabel: puzzle size ({width}x{height}) can not be used with the given label"
    )]
    IncompatibleLabel {
        /// Width of the puzzle.
        width: usize,
        /// Height of the puzzle.
        height: usize,
    },
}

/// A font that can be used with [`Renderer`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Font<'a> {
    /// A font installed on the system, specified by the font name.
    Family(&'a str),
    /// A font defined by a URL (including a local file path) and a font format.
    Url {
        /// Path to the font
        path: &'a str,
        /// Format of the font file.
        format: &'a str,
    },
    /// A font defined by base 64 data and a font format.
    Base64 {
        /// Base 64 font data.
        data: &'a str,
        /// Format of the font data.
        format: &'a str,
    },
}

/// Struct containing the information needed to draw the borders of the puzzle.
pub struct Borders<S: ColorScheme> {
    scheme: S,
    thickness: f32,
}

impl Borders<Black> {
    /// Creates a new [`Borders`] instance using the [`Black`] [`ColorScheme`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_scheme(Black)
    }
}

impl Default for Borders<Black> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: ColorScheme> Borders<S> {
    /// Create a new [`Borders`] instance. The default is a 1 pixel wide black border.
    #[must_use]
    pub fn with_scheme(scheme: S) -> Self {
        Self {
            scheme,
            thickness: 1.0,
        }
    }

    /// Set the border color scheme.
    ///
    /// If the main color scheme (see [`Renderer::scheme`]) has a subscheme, and the subscheme
    /// style (see [`Renderer::subscheme_style`]) is [`SubschemeStyle::BorderColor`], then the
    /// subscheme color will override the border scheme.
    #[must_use]
    pub fn scheme(mut self, scheme: S) -> Self {
        self.scheme = scheme;
        self
    }

    /// Set the border thickness.
    #[must_use]
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

/// Struct containing the information needed to draw text on the pieces of the puzzle.
pub struct Text<'a, S: ColorScheme> {
    scheme: S,
    font: Font<'a>,
    font_size: f32,
    position: (f32, f32),
}

impl<'a> Text<'a, Black> {
    /// Creates a new [`Text`] instance using the [`Black`] [`ColorScheme`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_scheme(Black)
    }
}

impl<'a> Default for Text<'a, Black> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, S: ColorScheme> Text<'a, S> {
    /// Create a new [`Text`] instance.
    #[must_use]
    pub fn with_scheme(scheme: S) -> Self {
        Self {
            scheme,
            font: Font::Family("sans-serif"),
            font_size: 30.0,
            position: (0.5, 0.5),
        }
    }

    /// Set the text color scheme.
    ///
    /// If the main color scheme (see [`Renderer::scheme`]) has a subscheme, and the subscheme
    /// style (see [`Renderer::subscheme_style`]) is [`SubschemeStyle::TextColor`], then the
    /// subscheme color will override the text scheme.
    #[must_use]
    pub fn scheme(mut self, scheme: S) -> Self {
        self.scheme = scheme;
        self
    }

    /// Set the font.
    #[must_use]
    pub fn font(mut self, font: Font<'a>) -> Self {
        self.font = font;
        self
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(0.0);
        self
    }

    /// Set the position around which the text within each tile will be centered, as a fraction of
    /// the tile size. (0, 0) is the top left of the tile and (1, 1) is the bottom right. This is
    /// useful if your font does not render perfectly centered.
    #[must_use]
    pub fn position(mut self, pos: (f32, f32)) -> Self {
        self.position = pos;
        self
    }

    /// Write the formatting options into a CSS string.
    #[must_use]
    pub fn style_string(&self) -> String {
        if let Font::Family(f) = self.font {
            format!(
                "text {{ font-family: {f}; font-size: {fs}px; }}",
                fs = self.font_size
            )
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
                "@font-face {{ \
                    font-family: f; \
                    src: {src}; \
                }} \
                text {{ \
                    font-family: f; \
                    font-size: {fs}px; \
                }}",
                fs = self.font_size
            )
        }
    }
}

/// Ways that the subscheme can be displayed on the puzzle.
///
/// The default value is [`SubschemeStyle::Rectangle`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubschemeStyle {
    /// Draw the subscheme as a small rectangle at the bottom of each piece.
    #[default]
    Rectangle,
    /// Display the subscheme using the text color.
    TextColor,
    /// Display the subscheme using the border color.
    BorderColor,
}

/// Used to build a [`Renderer`].
pub struct RendererBuilder<
    'a,
    List: AsRef<[S]>,
    S: ColorScheme = &'a dyn ColorScheme,
    T: ColorScheme = &'a dyn ColorScheme,
    B: ColorScheme = &'a dyn ColorScheme,
> {
    scheme: &'a SchemeList<S, List>,
    borders: Option<Borders<B>>,
    text: Option<Text<'a, T>>,
    tile_size: f32,
    tile_rounding: f32,
    tile_gap: f32,
    padding: f32,
    subscheme_style: Option<SubschemeStyle>,
    background_color: Rgba,
    phantom_s: PhantomData<S>,
}

/// Draws a [`SlidingPuzzle`] as an SVG image.
pub struct Renderer<
    'a,
    List: AsRef<[S]>,
    S: ColorScheme = &'a dyn ColorScheme,
    T: ColorScheme = &'a dyn ColorScheme,
    B: ColorScheme = &'a dyn ColorScheme,
>(RendererBuilder<'a, List, S, T, B>);

impl<'a, List: AsRef<[S]>, S: ColorScheme, T: ColorScheme, B: ColorScheme> Deref
    for Renderer<'a, List, S, T, B>
{
    type Target = RendererBuilder<'a, List, S, T, B>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, List: AsRef<[S]>, S: ColorScheme, T: ColorScheme, B: ColorScheme>
    RendererBuilder<'a, List, S, T, B>
{
    /// Create a new [`RendererBuilder`].
    #[must_use]
    pub fn with_scheme(scheme: &'a SchemeList<S, List>) -> Self {
        Self {
            scheme,
            borders: None,
            text: None,
            tile_size: 75.0,
            tile_rounding: 0.0,
            tile_gap: 0.0,
            padding: 0.0,
            subscheme_style: Some(SubschemeStyle::Rectangle),
            background_color: Rgba::new(1.0, 1.0, 1.0, 0.0),
            phantom_s: PhantomData,
        }
    }

    /// Set the color scheme.
    #[must_use]
    pub fn scheme(mut self, scheme: &'a SchemeList<S, List>) -> Self {
        self.scheme = scheme;
        self
    }

    /// Set the borders.
    #[must_use]
    pub fn borders(mut self, borders: Borders<B>) -> Self {
        self.borders = Some(borders);
        self
    }

    /// Set the text.
    #[must_use]
    pub fn text(mut self, text: Text<'a, T>) -> Self {
        self.text = Some(text);
        self
    }

    /// Set the tile size in pixels.
    #[must_use]
    pub fn tile_size(mut self, size: f32) -> Self {
        self.tile_size = size.max(0.0);
        self
    }

    /// Set the rounding radius of the tile corners in pixels.
    #[must_use]
    pub fn tile_rounding(mut self, rounding: f32) -> Self {
        self.tile_rounding = rounding.max(0.0);
        self
    }

    /// Set the gap between tiles in pixels.
    #[must_use]
    pub fn tile_gap(mut self, gap: f32) -> Self {
        self.tile_gap = gap;
        self
    }

    /// Set the padding around the edge of the puzzle in pixels.
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the subscheme style.
    #[must_use]
    pub fn subscheme_style(mut self, style: SubschemeStyle) -> Self {
        self.subscheme_style = Some(style);
        self
    }

    /// Set the background color.
    #[must_use]
    pub fn background_color(mut self, color: Rgba) -> Self {
        self.background_color = color;
        self
    }

    /// Builds a [`Renderer`].
    #[must_use]
    pub fn build(self) -> Renderer<'a, List, S, T, B> {
        Renderer(self)
    }
}

impl<'a, List: AsRef<[S]>, S: ColorScheme, T: ColorScheme, B: ColorScheme>
    Renderer<'a, List, S, T, B>
{
    /// Returns the CSS string used to style the image.
    pub fn style_string(&self) -> String {
        let font = self
            .text
            .as_ref()
            .map(|a| a.style_string())
            .unwrap_or_default();

        let bg = {
            let color: Rgba<_, u8> = self.background_color.into_format();
            format!("#{color:x}")
        };

        let border_thickness = self
            .borders
            .as_ref()
            .map(|a| a.thickness)
            .unwrap_or_default();

        format!(
            "svg {{ background-color: {bg}; }} \
            text {{ \
                text-anchor: middle; \
                dominant-baseline: central; \
            }} \
            rect.piece {{ \
                width: {ts}px; \
                height: {ts}px; \
                rx: {tr}px; \
                ry: {tr}px; \
                stroke-width: {sw}px; \
            }} \
            rect.sub {{ \
                width: {srw}px; \
                height: {srh}px; \
            }} \
            {font}",
            ts = self.tile_size,
            tr = self.tile_rounding,
            sw = border_thickness,
            srw = self.tile_size * 0.7,
            srh = self.tile_size * 0.1,
        )
    }

    /// Draws `puzzle` as an SVG image, wrapped in an SVG group element.
    pub fn group<Puzzle>(&self, puzzle: &Puzzle) -> Result<Group, RendererError>
    where
        Puzzle: SlidingPuzzle,
        Puzzle::Piece: Display,
    {
        let (width, height) = puzzle.size().into();

        if !self.scheme.is_valid_size(width, height) {
            return Err(RendererError::IncompatibleLabel { width, height });
        }

        let mut group = Group::new();

        for y in 0..height {
            for x in 0..width {
                let piece = puzzle.piece_at_xy(x, y);

                if piece != Puzzle::Piece::zero() {
                    group = group.add(self.render_piece(puzzle, x, y));
                }
            }
        }

        Ok(group)
    }

    /// Draws the piece of `puzzle` at position `(x, y)` as an SVG image, wrapped in an SVG group
    /// element.
    pub fn render_piece<Puzzle>(&self, puzzle: &Puzzle, x: usize, y: usize) -> Group
    where
        Puzzle: SlidingPuzzle,
        Puzzle::Piece: Display,
    {
        let (width, height) = puzzle.size().into();

        let border_thickness = self
            .borders
            .as_ref()
            .map(|a| a.thickness)
            .unwrap_or_default();

        let piece = puzzle.piece_at_xy(x, y);
        let solved_pos = puzzle.solved_pos_xy(piece);

        let (x, y) = (x as f32, y as f32);

        let rect_pos = (
            self.padding
                + border_thickness / 2.0
                + (self.tile_size + self.tile_gap + border_thickness) * x,
            self.padding
                + border_thickness / 2.0
                + (self.tile_size + self.tile_gap + border_thickness) * y,
        );

        let subscheme_color = self
            .scheme
            .subscheme()
            .map(|subscheme| subscheme.color(width, height, solved_pos.0, solved_pos.1));

        // Macro to get the color that we want for text and border colors, as a hex string.
        // If `self.subscheme_style` is TextColor or BorderColor, then this will override the
        // schemes that we have in self.text_scheme and self.borders.unwrap().scheme.
        macro_rules! color {
            ($scheme:expr, $subscheme:expr) => {{
                let color = if let Some(c) = subscheme_color && self.subscheme_style == Some($subscheme) {
                    // There is a subscheme color, and the subscheme style overrides the other
                    // scheme (text or border scheme).
                    c
                } else {
                    // If no override, then we use the text or border scheme color.
                    $scheme.color(width, height, solved_pos.0, solved_pos.1)
                };

                // Format as hex string
                let color: Rgba<_, u8> = color.into_format();
                format!("#{color:x}")
            }};
        }

        let rect = {
            let fill = {
                let color: Rgba<_, u8> = self
                    .scheme
                    .color(width, height, solved_pos.0, solved_pos.1)
                    .into_format();
                format!("#{color:x}")
            };

            let mut r = Rectangle::new()
                .set("x", rect_pos.0)
                .set("y", rect_pos.1)
                .set("class", "piece")
                .set("fill", fill);

            if let Some(s) = &self.borders {
                let stroke = color!(s.scheme, SubschemeStyle::BorderColor);
                r = r.set("stroke", stroke);
            }

            r
        };

        let text = self.text.as_ref().map(|text| {
            let fill = color!(text.scheme, SubschemeStyle::TextColor);
            let (tx, ty) = text.position;

            TextElement::new()
                .set("x", rect_pos.0 + self.tile_size * tx)
                .set("y", rect_pos.1 + self.tile_size * ty)
                .set("fill", fill)
                .add(TextNode::new(piece.to_string()))
        });

        let subscheme_render = if let Some(subcolor) = subscheme_color
            && self.subscheme_style == Some(SubschemeStyle::Rectangle)
        {
            let fill = {
                let color: Rgba<_, u8> = subcolor.into_format();
                format!("#{color:x}")
            };

            let subrect_pos = (0.15, 0.8);

            Some(
                Rectangle::new()
                    .set("x", rect_pos.0 + self.tile_size * subrect_pos.0)
                    .set("y", rect_pos.1 + self.tile_size * subrect_pos.1)
                    .set("class", "sub")
                    .set("fill", fill),
            )
        } else {
            None
        };

        let mut group = Group::new().add(rect);

        if let Some(text) = text {
            group = group.add(text);
        }

        if let Some(s) = subscheme_render {
            group = group.add(s);
        }

        group
    }

    /// Draws `puzzle` as an SVG image.
    pub fn render<Puzzle>(&self, puzzle: &Puzzle) -> Result<Document, RendererError>
    where
        Puzzle: SlidingPuzzle,
        Puzzle::Piece: Display,
    {
        let (width, height) = puzzle.size().into();

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
            w * self.tile_size
                + (w - 1.0) * self.tile_gap
                + w * border_thickness
                + 2.0 * self.padding,
            h * self.tile_size
                + (h - 1.0) * self.tile_gap
                + h * border_thickness
                + 2.0 * self.padding,
        );

        let style_str = self.style_string();

        let doc = Document::new()
            .add(Style::new(style_str))
            .add(self.group(puzzle)?)
            .set("width", image_w)
            .set("height", image_h);

        Ok(doc)
    }
}
