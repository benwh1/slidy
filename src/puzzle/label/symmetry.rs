use super::label::Label;
use std::marker::PhantomData;

mod sym {
    pub struct Id;
    pub struct RotateCw;
    pub struct RotateCcw;
    pub struct RotateHalf;
    pub struct ReflectVertical;
    pub struct ReflectHorizontal;
    pub struct ReflectDiagonal;
    pub struct ReflectAntidiagonal;
}

pub struct Symmetry<Piece, L, T>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    phantom_piece: PhantomData<Piece>,
    phantom_l: PhantomData<L>,
    phantom_t: PhantomData<T>,
}

macro_rules! make_type {
    ($t:ident) => {
        pub type $t<Piece, L> = Symmetry<Piece, L, sym::$t>;
    };
    ($t:ident, $($t2:ident),+) => {
        make_type!($t);
        make_type!($($t2),+);
    }
}

make_type!(
    Id,
    RotateCw,
    RotateCcw,
    RotateHalf,
    ReflectVertical,
    ReflectHorizontal,
    ReflectDiagonal,
    ReflectAntidiagonal
);

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::Id>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(width, height, x, y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(width, height)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::RotateCw>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(height, width, height - 1 - y, x)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(height, width)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::RotateCcw>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(height, width, y, width - 1 - x)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(height, width)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::RotateHalf>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(width, height, width - 1 - x, height - 1 - y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(width, height)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::ReflectVertical>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(width, height, x, height - 1 - y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(width, height)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::ReflectHorizontal>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(width, height, width - 1 - x, y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(width, height)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::ReflectDiagonal>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(height, width, y, x)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(height, width)
    }
}

impl<Piece, L> Label<Piece> for Symmetry<Piece, L, sym::ReflectAntidiagonal>
where
    Piece: Into<u64>,
    L: Label<Piece>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        L::position_label(height, width, height - 1 - y, width - 1 - x)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        L::num_labels(height, width)
    }
}
