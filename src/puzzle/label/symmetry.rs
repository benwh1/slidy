use super::label::{BijectiveLabel, Label};

/// The identity symmetry. This does nothing and is only included for completeness.
pub struct Id<L: Label>(pub L);
/// Rotation clockwise by 90 degrees.
pub struct RotateCw<L: Label>(pub L);
/// Rotation anticlockwise by 90 degrees.
pub struct RotateCcw<L: Label>(pub L);
/// Rotation by 180 degrees.
pub struct RotateHalf<L: Label>(pub L);
/// Reflection in a vertical line.
pub struct ReflectVertical<L: Label>(pub L);
/// Reflection in a horizontal line.
pub struct ReflectHorizontal<L: Label>(pub L);
/// Reflection in the diagonal line from top left to bottom right.
pub struct ReflectDiagonal<L: Label>(pub L);
/// Reflection in the diagonal line from bottom left to top right.
pub struct ReflectAntidiagonal<L: Label>(pub L);

impl<L: Label> Label for Id<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(width, height)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0.position_label_unchecked(width, height, x, y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for RotateCw<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(height, width)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, y, width - 1 - x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for RotateCcw<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(height, width)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, height - 1 - y, x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for RotateHalf<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(width, height)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, width - 1 - x, height - 1 - y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectVertical<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(height, width)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, x, height - 1 - y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectHorizontal<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(width, height)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(width, height, width - 1 - x, y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(width, height)
    }
}

impl<L: Label> Label for ReflectDiagonal<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(height, width)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0.position_label_unchecked(height, width, y, x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
    }
}

impl<L: Label> Label for ReflectAntidiagonal<L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.0.is_valid_size(height, width)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        self.0
            .position_label_unchecked(height, width, height - 1 - y, width - 1 - x)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        self.0.num_labels_unchecked(height, width)
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
                    use crate::puzzle::label::label::{Label, RowGrids};
                    use super::{$label};

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h >] () {
                        let labels = (0..$w * $h)
                            .map(|i| $label(RowGrids).position_label_unchecked($w, $h, i % $w, i / $w))
                            .collect::<Vec<_>>();
                        let num_labels = $label(RowGrids).num_labels_unchecked($w, $h);
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
