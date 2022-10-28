use std::{collections::BTreeMap, ops::Range};

use thiserror::Error;

use super::label::Label;

/// Error type for [`Rect`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RectError {
    /// Returned when the width (`right - left`) or height (`bottom - top`) are negative.
    #[error("InvalidSize: width and height of the rectangle must be positive")]
    InvalidSize,
}

/// A rectangle on a grid of squares, with x increasing to the right and y increasing downwards.
///
/// Used to define a [`RectPartition`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Rect {
    /// Creates a new [`Rect`] given the coordinates of the top left and bottom right points.
    pub fn new(top_left: (u32, u32), bottom_right: (u32, u32)) -> Result<Self, RectError> {
        let (top, left, bottom, right) = (top_left.1, top_left.0, bottom_right.1, bottom_right.0);
        if bottom > top && right > left {
            Ok(Self {
                top,
                left,
                bottom,
                right,
            })
        } else {
            Err(RectError::InvalidSize)
        }
    }

    /// Width of the rectangle.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.right - self.left
    }

    /// Height of the rectangle.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.bottom - self.top
    }

    /// Checks if `(x, y)` is contained in the rectangle. The rectangle contains the top and left
    /// edges, but does not contain the bottom and right edges or the top right and bottom left
    /// corners.
    #[must_use]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }

    /// The top left corner
    #[must_use]
    pub fn top_left(&self) -> (u32, u32) {
        (self.left, self.top)
    }

    /// Size of the rectangle in the form `(width, height)`.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.right - self.left, self.bottom - self.top)
    }
}

#[derive(Debug)]
pub(super) struct PiecewiseConstant {
    data: BTreeMap<u32, u32>,
    domain: Range<u32>,
}

impl PiecewiseConstant {
    pub(super) fn new(domain: Range<u32>, value: u32) -> Self {
        let mut data = BTreeMap::new();
        data.insert(domain.start, value);
        Self { data, domain }
    }

    pub(super) fn value(&self, x: u32) -> u32 {
        let x = x.clamp(self.domain.start, self.domain.end);
        self.data
            .range(self.domain.start..=x)
            .last()
            .map(|(_, &v)| v)
            .unwrap()
    }

    pub(super) fn range_value(&self, range: Range<u32>) -> Option<u32> {
        let v = self.value(range.start);
        if self.data.range(range).map(|(_, &v)| v).all(|x| x == v) {
            Some(v)
        } else {
            None
        }
    }

    pub(super) fn set_range_value(&mut self, range: Range<u32>, value: u32) {
        // Keys that define values of the function within `range`
        let keys = self
            .data
            .range(range.clone())
            .map(|(&k, _)| k)
            .collect::<Vec<_>>();

        // Value of the function just before and just after `range`
        let prev_value = self.value(range.start.saturating_sub(1));
        let end_value = self.value(range.end);

        // Remove all values of the function that are in `range`
        for k in keys {
            self.data.remove(&k);
        }

        // If the value of the function just before `range` is different than the new value we
        // want to set, insert a new key.
        if value != prev_value {
            self.data.insert(range.start, value);
        }

        // If the value of the function just after `range` is different than the new value we
        // want to set, insert a new key.
        if self.domain.contains(&range.end) && value != end_value {
            self.data.insert(range.end, end_value);
        }
    }
}

/// A partition of a rectangle into smaller rectangles.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RectPartition {
    pub(in crate::puzzle) rects: Vec<Rect>,
}

/// Error type for [`RectPartition`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RectPartitionError {
    /// Returned from [`RectPartition::new`] when given an empty vector.
    #[error("Empty: a partition must contain at least one `Rect`")]
    Empty,

    /// Returned from [`RectPartition::new`] when the given vector of [`Rect`]s is not a partition
    /// of a large rectangle.
    #[error("NotPartition: the square at ({x}, {y}) is not covered exactly once")]
    NotPartition {
        /// x coordinate of a position that is not covered by exactly one rectangle.
        x: u32,
        /// y coordinate of a position that is not covered by exactly one rectangle.
        y: u32,
    },
}

impl RectPartition {
    /// Creates a new [`RectPartition`] from a list of [`Rect`]s that partition a larger rectangle.
    pub fn new(mut rects: Vec<Rect>) -> Result<Self, RectPartitionError> {
        if rects.is_empty() {
            return Err(RectPartitionError::Empty);
        }

        rects.sort_by_key(|r| (r.top, r.left));

        let top = rects.iter().map(|r| r.top).min().unwrap();
        let left = rects.iter().map(|r| r.left).min().unwrap();
        let right = rects.iter().map(|r| r.right).max().unwrap();

        let mut height_map = PiecewiseConstant::new(left..right, top);

        for slice in rects.group_by(|a, b| a.top == b.top) {
            for rect in slice {
                let height = height_map.range_value(rect.left..rect.right);
                if let Some(height) = height && height == rect.top {
                    height_map.set_range_value(rect.left..rect.right, rect.bottom);
                } else {
                    return Err(RectPartitionError::NotPartition {
                        x: rect.left,
                        y: rect.top,
                    });
                }
            }
        }

        let max_value = height_map.data.values().max().unwrap().to_owned();
        if let Some((key, value)) = height_map
            .data
            .iter()
            .find(|(_, &v)| v != max_value)
            .map(|(&k, &v)| (k, v))
        {
            Err(RectPartitionError::NotPartition { x: key, y: value })
        } else {
            Ok(Self { rects })
        }
    }

    /// Returns the large rectangle that is being partitioned.
    #[must_use]
    pub(super) fn rect(&self) -> Rect {
        let left = self.rects.iter().map(|r| r.left).min().unwrap();
        let top = self.rects.iter().map(|r| r.top).min().unwrap();
        let right = self.rects.iter().map(|r| r.right).max().unwrap();
        let bottom = self.rects.iter().map(|r| r.bottom).max().unwrap();
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Returns the number of rectangles in the partition.
    #[must_use]
    pub fn num_rects(&self) -> usize {
        self.rects.len()
    }
}

impl Label for RectPartition {
    /// [`RectPartition`] is only a valid label when the puzzle size equals the size of the
    /// partitioned rectangle.
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.rect().size() == (width as u32, height as u32)
    }

    fn position_label_unchecked(&self, _width: usize, _height: usize, x: usize, y: usize) -> usize {
        let (x, y) = (x as u32, y as u32);
        self.rects.iter().position(|r| r.contains(x, y)).unwrap()
    }

    fn num_labels_unchecked(&self, _width: usize, _height: usize) -> usize {
        self.num_rects()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod rect {
        use super::*;

        #[test]
        fn test_rect() {
            let r = Rect::new((0, 0), (3, 5));
            assert!(r.is_ok());
        }

        #[test]
        fn test_rect_2() {
            let r = Rect::new((3, 3), (4, 4));
            assert!(r.is_ok());
        }

        #[test]
        fn test_rect_3() {
            let r = Rect::new((3, 1), (5, 1));
            assert_eq!(r, Err(RectError::InvalidSize));
        }

        #[test]
        fn test_rect_4() {
            let r = Rect::new((3, 1), (3, 2));
            assert_eq!(r, Err(RectError::InvalidSize));
        }

        #[test]
        fn test_rect_5() {
            let r = Rect::new((3, 2), (0, 0));
            assert_eq!(r, Err(RectError::InvalidSize));
        }
    }

    mod rect_partition {
        use super::*;

        #[test]
        fn test_rect_partition() {
            assert!(RectPartition::new(vec![
                Rect::new((0, 0), (3, 2)).unwrap(),
                Rect::new((3, 0), (5, 3)).unwrap(),
                Rect::new((0, 2), (2, 5)).unwrap(),
                Rect::new((2, 2), (3, 3)).unwrap(),
                Rect::new((2, 3), (5, 5)).unwrap(),
            ])
            .is_ok());
        }

        #[test]
        fn test_rect_partition_2() {
            assert!(RectPartition::new(vec![Rect::new((0, 0), (1, 1)).unwrap()]).is_ok());
        }

        #[test]
        fn test_rect_partition_3() {
            assert!(RectPartition::new(vec![
                Rect::new((0, 0), (5, 1)).unwrap(),
                Rect::new((0, 1), (1, 3)).unwrap(),
                Rect::new((1, 1), (3, 2)).unwrap(),
                Rect::new((2, 2), (4, 4)).unwrap(),
                Rect::new((3, 1), (4, 2)).unwrap(),
                Rect::new((4, 1), (5, 4)).unwrap(),
                Rect::new((1, 2), (2, 5)).unwrap(),
                Rect::new((0, 3), (1, 5)).unwrap(),
                Rect::new((2, 4), (4, 5)).unwrap(),
                Rect::new((4, 4), (5, 5)).unwrap(),
            ])
            .is_ok());
        }

        #[test]
        fn test_rect_partition_4() {
            assert_eq!(RectPartition::new(vec![]), Err(RectPartitionError::Empty));
        }

        #[test]
        fn test_rect_partition_5() {
            assert_eq!(
                RectPartition::new(vec![
                    Rect::new((0, 0), (2, 2)).unwrap(),
                    Rect::new((2, 0), (4, 1)).unwrap(),
                    Rect::new((3, 1), (4, 2)).unwrap(),
                ]),
                Err(RectPartitionError::NotPartition { x: 2, y: 1 })
            );
        }

        #[test]
        fn test_rect_partition_6() {
            assert_eq!(
                RectPartition::new(vec![
                    Rect::new((0, 0), (3, 3)).unwrap(),
                    Rect::new((2, 0), (6, 3)).unwrap(),
                    Rect::new((0, 3), (3, 6)).unwrap(),
                    Rect::new((3, 3), (6, 6)).unwrap(),
                ]),
                Err(RectPartitionError::NotPartition { x: 2, y: 0 })
            );
        }

        #[test]
        fn test_rect_partition_7() {
            assert!(RectPartition::new(vec![
                Rect::new((4, 1), (6, 3)).unwrap(),
                Rect::new((4, 3), (6, 5)).unwrap(),
                Rect::new((6, 1), (8, 3)).unwrap(),
                Rect::new((6, 3), (8, 5)).unwrap(),
            ])
            .is_ok());
        }
    }
}
