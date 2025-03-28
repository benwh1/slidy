//! Defines the 8 symmetries of a square as label modifiers.

use crate::puzzle::{
    label::label::{BijectiveLabel, Label},
    size::Size,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

macro_rules! define_sym {
    ($($(#[$annot:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$annot])*
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct $name<L: Label>(pub L);
        )*
    };
}

define_sym!(
    /// The identity symmetry. This does nothing and is only included for completeness.
    Id,
    /// Rotation clockwise by 90 degrees.
    RotateCw,
    /// Rotation anticlockwise by 90 degrees.
    RotateCcw,
    /// Rotation by 180 degrees.
    RotateHalf,
    /// Reflection in a vertical line.
    ReflectVertical,
    /// Reflection in a horizontal line.
    ReflectHorizontal,
    /// Reflection in the diagonal line from top left to bottom right.
    ReflectDiagonal,
    /// Reflection in the diagonal line from bottom left to top right.
    ReflectAntidiagonal,
);

impl<L: Label> Label for Id<L> {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        self.0.position_label(size, pos)
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size)
    }
}

impl<L: Label> Label for RotateCw<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        self.0
            .position_label(size.transpose(), (y, size.width() - 1 - x))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size.transpose())
    }
}

impl<L: Label> Label for RotateCcw<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        self.0
            .position_label(size.transpose(), (size.height() - 1 - y, x))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size.transpose())
    }
}

impl<L: Label> Label for RotateHalf<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        self.0.position_label(size, (width - 1 - x, height - 1 - y))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size)
    }
}

impl<L: Label> Label for ReflectVertical<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        self.0.position_label(size, (x, size.height() - 1 - y))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size)
    }
}

impl<L: Label> Label for ReflectHorizontal<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        self.0.position_label(size, (size.width() - 1 - x, y))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size)
    }
}

impl<L: Label> Label for ReflectDiagonal<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        self.0.position_label(size.transpose(), (y, x))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size.transpose())
    }
}

impl<L: Label> Label for ReflectAntidiagonal<L> {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        self.0
            .position_label(size.transpose(), (height - 1 - y, width - 1 - x))
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.0.num_labels(size.transpose())
    }
}

impl<L: BijectiveLabel> BijectiveLabel for Id<L> {}
impl<L: BijectiveLabel> BijectiveLabel for RotateCw<L> {}
impl<L: BijectiveLabel> BijectiveLabel for RotateCcw<L> {}
impl<L: BijectiveLabel> BijectiveLabel for RotateHalf<L> {}
impl<L: BijectiveLabel> BijectiveLabel for ReflectVertical<L> {}
impl<L: BijectiveLabel> BijectiveLabel for ReflectHorizontal<L> {}
impl<L: BijectiveLabel> BijectiveLabel for ReflectDiagonal<L> {}
impl<L: BijectiveLabel> BijectiveLabel for ReflectAntidiagonal<L> {}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_label {
        ($label:ty, $($w:literal x $h:literal : $labels:expr),+ $(,)?) => {
            paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::{label::label::{Label as _, RowGrids}, size::Size};

                    use super::{$label};

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h >] () {
                        let size = Size::new($w, $h).unwrap();
                        let labels = (0..$w * $h)
                            .map(|i| $label(RowGrids).position_label(size, (i % $w, i / $w)))
                            .collect::<Vec<_>>();
                        let num_labels = $label(RowGrids).num_labels(size);
                        let expected_num_labels = $labels.iter().max().unwrap() + 1;
                        assert_eq!(labels, $labels);
                        assert_eq!(num_labels, expected_num_labels);
                    })*
                }
            }
        };
    }

    test_label!(
        Id,
        4 x 4: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        4 x 6: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        6 x 4: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
    );

    test_label!(
        RotateCw,
        4 x 4: [12, 8, 4, 0, 13, 9, 5, 1, 14, 10, 6, 2, 15, 11, 7, 3],
        4 x 6: [18, 12, 6, 0, 19, 13, 7, 1, 20, 14, 8, 2, 21, 15, 9, 3, 22, 16, 10, 4, 23, 17, 11, 5],
        6 x 4: [20, 16, 12, 8, 4, 0, 21, 17, 13, 9, 5, 1, 22, 18, 14, 10, 6, 2, 23, 19, 15, 11, 7, 3],
    );

    test_label!(
        RotateCcw,
        4 x 4: [3, 7, 11, 15, 2, 6, 10, 14, 1, 5, 9, 13, 0, 4, 8, 12],
        4 x 6: [5, 11, 17, 23, 4, 10, 16, 22, 3, 9, 15, 21, 2, 8, 14, 20, 1, 7, 13, 19, 0, 6, 12, 18],
        6 x 4: [3, 7, 11, 15, 19, 23, 2, 6, 10, 14, 18, 22, 1, 5, 9, 13, 17, 21, 0, 4, 8, 12, 16, 20],
    );

    test_label!(
        RotateHalf,
        4 x 4: [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        4 x 6: [23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        6 x 4: [23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    );

    test_label!(
        ReflectVertical,
        4 x 4: [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3],
        4 x 6: [20, 21, 22, 23, 16, 17, 18, 19, 12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3],
        6 x 4: [18, 19, 20, 21, 22, 23, 12, 13, 14, 15, 16, 17, 6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5],
    );

    test_label!(
        ReflectHorizontal,
        4 x 4: [3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12],
        4 x 6: [3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12, 19, 18, 17, 16, 23, 22, 21, 20],
        6 x 4: [5, 4, 3, 2, 1, 0, 11, 10, 9, 8, 7, 6, 17, 16, 15, 14, 13, 12, 23, 22, 21, 20, 19, 18],
    );

    test_label!(
        ReflectDiagonal,
        4 x 4: [0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15],
        4 x 6: [0, 6, 12, 18, 1, 7, 13, 19, 2, 8, 14, 20, 3, 9, 15, 21, 4, 10, 16, 22, 5, 11, 17, 23],
        6 x 4: [0, 4, 8, 12, 16, 20, 1, 5, 9, 13, 17, 21, 2, 6, 10, 14, 18, 22, 3, 7, 11, 15, 19, 23],
    );

    test_label!(
        ReflectAntidiagonal,
        4 x 4: [15, 11, 7, 3, 14, 10, 6, 2, 13, 9, 5, 1, 12, 8, 4, 0],
        4 x 6: [23, 17, 11, 5, 22, 16, 10, 4, 21, 15, 9, 3, 20, 14, 8, 2, 19, 13, 7, 1, 18, 12, 6, 0],
        6 x 4: [23, 19, 15, 11, 7, 3, 22, 18, 14, 10, 6, 2, 21, 17, 13, 9, 5, 1, 20, 16, 12, 8, 4, 0],
    );
}
