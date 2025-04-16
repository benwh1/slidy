//! Defines the [`Grid`] label.

use crate::puzzle::{
    grids::Grids,
    label::{label::Label, rect_partition::Rect},
    size::Size,
};

/// A [`Label`] that divides the puzzle into a grid of rectangles at the given `x` and `y`
/// coordinates.
pub struct Grid {
    xs: Vec<u64>,
    ys: Vec<u64>,
}

impl Grid {
    /// Creates a new [`Grid`] label with the given `x` and `y` coordinates.
    ///
    /// The two vectors are sorted and de-duplicated. Any zero values are also removed.
    ///
    /// The `xs` vector contains the `x` coordinates of the vertical lines that divide the puzzle
    /// into rectangles, and likewise for `ys`.
    pub fn new(mut xs: Vec<u64>, mut ys: Vec<u64>) -> Self {
        xs.sort_unstable();
        xs.dedup();
        xs.retain(|&i| i != 0);

        ys.sort_unstable();
        ys.dedup();
        ys.retain(|&i| i != 0);

        Self { xs, ys }
    }

    fn lr_count(&self, size: Size) -> u64 {
        self.xs.iter().take_while(|&&i| i < size.width()).count() as u64 + 1
    }

    fn ud_count(&self, size: Size) -> u64 {
        self.ys.iter().take_while(|&&i| i < size.height()).count() as u64 + 1
    }

    fn grid_index(&self, pos: (u64, u64)) -> (u64, u64) {
        let x = self.xs.iter().take_while(|&&i| i <= pos.0).count() as u64;
        let y = self.ys.iter().take_while(|&&i| i <= pos.1).count() as u64;

        (x, y)
    }
}

impl Label for Grid {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        let (x, y) = self.grid_index(pos);
        x + self.lr_count(size) * y
    }

    fn num_labels(&self, size: Size) -> u64 {
        self.lr_count(size) * self.ud_count(size)
    }
}

impl Grids for Grid {
    fn grid_containing_pos(&self, size: Size, pos: (u64, u64)) -> Rect {
        let (x, y) = self.grid_index(pos);

        let left = if x == 0 { 0 } else { self.xs[x as usize - 1] };
        let top = if y == 0 { 0 } else { self.ys[y as usize - 1] };

        let right = match self.xs.get(x as usize).copied() {
            Some(v) => v.min(size.width()),
            None => size.width(),
        };
        let bottom = match self.ys.get(y as usize).copied() {
            Some(v) => v.min(size.height()),
            None => size.height(),
        };

        Rect::new((left, top), (right, bottom)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(w: u64, h: u64) -> Size {
        Size::new(w, h).unwrap()
    }

    mod label {
        use super::*;

        #[test]
        fn test_num_labels() {
            let grid = Grid::new(vec![2, 4], vec![2, 4, 6]);

            assert_eq!(grid.num_labels(s(2, 2)), 1);
            assert_eq!(grid.num_labels(s(2, 3)), 2);
            assert_eq!(grid.num_labels(s(2, 4)), 2);
            assert_eq!(grid.num_labels(s(2, 5)), 3);
            assert_eq!(grid.num_labels(s(2, 6)), 3);
            assert_eq!(grid.num_labels(s(2, 7)), 4);
            assert_eq!(grid.num_labels(s(2, 8)), 4);
            assert_eq!(grid.num_labels(s(4, 2)), 2);
            assert_eq!(grid.num_labels(s(4, 3)), 4);
            assert_eq!(grid.num_labels(s(4, 4)), 4);
            assert_eq!(grid.num_labels(s(4, 5)), 6);
            assert_eq!(grid.num_labels(s(5, 4)), 6);
            assert_eq!(grid.num_labels(s(5, 5)), 9);
            assert_eq!(grid.num_labels(s(5, 6)), 9);
            assert_eq!(grid.num_labels(s(5, 7)), 12);
            assert_eq!(grid.num_labels(s(6, 5)), 9);
            assert_eq!(grid.num_labels(s(6, 6)), 9);
            assert_eq!(grid.num_labels(s(8, 5)), 9);
            assert_eq!(grid.num_labels(s(8, 8)), 12);
        }

        #[test]
        fn test_position_label() {
            let grid = Grid::new(vec![2, 4], vec![2, 4, 6]);
            let size = s(5, 5);

            let expected = [
                [0, 0, 1, 1, 2],
                [0, 0, 1, 1, 2],
                [3, 3, 4, 4, 5],
                [3, 3, 4, 4, 5],
                [6, 6, 7, 7, 8],
            ];

            let actual = (0..size.height())
                .map(|y| {
                    (0..size.width())
                        .map(|x| grid.position_label(size, (x, y)))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            assert_eq!(actual, expected);
        }
    }

    mod grids {
        use super::*;

        #[test]
        fn test_grid_containing_pos() {
            let grid = Grid::new(vec![2, 4], vec![2, 4, 6]);
            let size = s(5, 5);

            let rects = [
                Rect::new((0, 0), (2, 2)).unwrap(),
                Rect::new((2, 0), (4, 2)).unwrap(),
                Rect::new((4, 0), (5, 2)).unwrap(),
                Rect::new((0, 2), (2, 4)).unwrap(),
                Rect::new((2, 2), (4, 4)).unwrap(),
                Rect::new((4, 2), (5, 4)).unwrap(),
                Rect::new((0, 4), (2, 5)).unwrap(),
                Rect::new((2, 4), (4, 5)).unwrap(),
                Rect::new((4, 4), (5, 5)).unwrap(),
            ];

            let expected = [
                [0, 0, 1, 1, 2],
                [0, 0, 1, 1, 2],
                [3, 3, 4, 4, 5],
                [3, 3, 4, 4, 5],
                [6, 6, 7, 7, 8],
            ];

            for y in 0..size.height() {
                for x in 0..size.width() {
                    let rect = grid.grid_containing_pos(size, (x, y));
                    assert_eq!(rect, rects[expected[y as usize][x as usize]]);
                }
            }
        }
    }
}
