use super::label::Label;

pub struct Id<L: Label>(L);
pub struct RotateCw<L: Label>(L);
pub struct RotateCcw<L: Label>(L);
pub struct RotateHalf<L: Label>(L);
pub struct ReflectVertical<L: Label>(L);
pub struct ReflectHorizontal<L: Label>(L);
pub struct ReflectDiagonal<L: Label>(L);
pub struct ReflectAntidiagonal<L: Label>(L);

impl<L: Label> Label for Id<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0.position_label_unchecked(width, height, x, y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for RotateCw<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, y, width - 1 - x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for RotateCcw<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, height - 1 - y, x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for RotateHalf<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, width - 1 - x, height - 1 - y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectVertical<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, x, height - 1 - y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectHorizontal<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, width - 1 - x, y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectDiagonal<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0.position_label_unchecked(height, width, y, x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for ReflectAntidiagonal<L> {
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, height - 1 - y, width - 1 - x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_label {
        (fn $name:ident, $label:expr, $w:literal x $h:literal, $pos_label:expr, $num_labels:expr) => {
            #[test]
            fn $name() {
                let wh = $w * $h;
                let pos = (0..wh)
                    .map(|i| $label(RowGrids).position_label_unchecked($w, $h, i % $w, i / $w))
                    .collect::<Vec<_>>();
                let num = $label(RowGrids).num_labels_unchecked($w, $h);
                assert_eq!(pos, $pos_label);
                assert_eq!(num, $num_labels);
            }
        };

        (fn $name:ident, $label:expr, $w:literal x $h:literal, $pos_label:expr) => {
            test_label!(fn $name, $label, $w x $h, $pos_label, $pos_label.iter().max().unwrap() + 1);
        };

        ($label:ty, $($w:literal x $h:literal : $pos:expr),+ $(,)?) => {
            ::paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::label::label::{Label, RowGrids};
                    use super::{$label};

                    $(test_label!( fn [< test_ $label:snake _ $w x $h >] , $label, $w x $h, $pos);)*
                }
            }
        };
    }

    test_label!(
        Id,
        4 x 4: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        4 x 6: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        6 x 4: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
    );

    test_label!(
        RotateCw,
        4 x 4: vec![12, 8, 4, 0, 13, 9, 5, 1, 14, 10, 6, 2, 15, 11, 7, 3],
        4 x 6: vec![18, 12, 6, 0, 19, 13, 7, 1, 20, 14, 8, 2, 21, 15, 9, 3, 22, 16, 10, 4, 23, 17, 11, 5],
        6 x 4: vec![20, 16, 12, 8, 4, 0, 21, 17, 13, 9, 5, 1, 22, 18, 14, 10, 6, 2, 23, 19, 15, 11, 7, 3],
    );

    test_label!(
        RotateCcw,
        4 x 4: vec![3, 7, 11, 15, 2, 6, 10, 14, 1, 5, 9, 13, 0, 4, 8, 12],
        4 x 6: vec![5, 11, 17, 23, 4, 10, 16, 22, 3, 9, 15, 21, 2, 8, 14, 20, 1, 7, 13, 19, 0, 6, 12, 18],
        6 x 4: vec![3, 7, 11, 15, 19, 23, 2, 6, 10, 14, 18, 22, 1, 5, 9, 13, 17, 21, 0, 4, 8, 12, 16, 20],
    );

    test_label!(
        RotateHalf,
        4 x 4: vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        4 x 6: vec![23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        6 x 4: vec![23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    );

    test_label!(
        ReflectVertical,
        4 x 4: vec![12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3],
        4 x 6: vec![20, 21, 22, 23, 16, 17, 18, 19, 12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3],
        6 x 4: vec![18, 19, 20, 21, 22, 23, 12, 13, 14, 15, 16, 17, 6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5],
    );

    test_label!(
        ReflectHorizontal,
        4 x 4: vec![3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12],
        4 x 6: vec![3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12, 19, 18, 17, 16, 23, 22, 21, 20],
        6 x 4: vec![5, 4, 3, 2, 1, 0, 11, 10, 9, 8, 7, 6, 17, 16, 15, 14, 13, 12, 23, 22, 21, 20, 19, 18],
    );

    test_label!(
        ReflectDiagonal,
        4 x 4: vec![0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15],
        4 x 6: vec![0, 6, 12, 18, 1, 7, 13, 19, 2, 8, 14, 20, 3, 9, 15, 21, 4, 10, 16, 22, 5, 11, 17, 23],
        6 x 4: vec![0, 4, 8, 12, 16, 20, 1, 5, 9, 13, 17, 21, 2, 6, 10, 14, 18, 22, 3, 7, 11, 15, 19, 23],
    );

    test_label!(
        ReflectAntidiagonal,
        4 x 4: vec![15, 11, 7, 3, 14, 10, 6, 2, 13, 9, 5, 1, 12, 8, 4, 0],
        4 x 6: vec![23, 17, 11, 5, 22, 16, 10, 4, 21, 15, 9, 3, 20, 14, 8, 2, 19, 13, 7, 1, 18, 12, 6, 0],
        6 x 4: vec![23, 19, 15, 11, 7, 3, 22, 18, 14, 10, 6, 2, 21, 17, 13, 9, 5, 1, 20, 16, 12, 8, 4, 0],
    );
}
