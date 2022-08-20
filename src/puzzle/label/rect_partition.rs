use std::{collections::BTreeMap, ops::Range};

use thiserror::Error;

use super::label::Label;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Rect {
    #[must_use]
    pub fn new(top_left: (u32, u32), bottom_right: (u32, u32)) -> Self {
        Self {
            top: top_left.1,
            left: top_left.0,
            bottom: bottom_right.1,
            right: bottom_right.0,
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.right - self.left
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.bottom - self.top
    }

    #[must_use]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }

    #[must_use]
    pub fn top_left(&self) -> (u32, u32) {
        (self.left, self.top)
    }

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
    pub(super) fn new(domain: Range<u32>) -> Self {
        let mut data = BTreeMap::new();
        data.insert(domain.start, 0);
        Self { data, domain }
    }

    pub(super) fn value(&self, x: u32) -> u32 {
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
        let next_point = self
            .data
            .range(range.end..self.domain.end)
            .next()
            .map(|(&k, &v)| (k, v));

        let keys = self
            .data
            .range(range.clone())
            .map(|(&k, _)| k)
            .collect::<Vec<_>>();

        let prev_value = range.start.checked_sub(1).map(|a| self.value(a));
        let end_value = self.value(range.end);

        for k in keys {
            self.data.remove(&k);
        }
        if let Some((k, v)) = next_point && v == end_value {
            self.data.remove(&k);
        }

        match prev_value {
            Some(v) if value != v => {
                self.data.insert(range.start, value);
            }
            None => {
                self.data.insert(range.start, value);
            }
            _ => {}
        }

        if self.domain.contains(&range.end) && value != end_value {
            self.data.insert(range.end, end_value);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RectPartition {
    rects: Vec<Rect>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RectPartitionError {
    #[error("Empty: a partition must contain at least one `Rect`")]
    Empty,

    #[error("NotPartition: the square at ({x}, {y}) is not covered exactly once")]
    NotPartition { x: u32, y: u32 },

    #[error("InvalidRect: `Rect`s must have positive width and height")]
    InvalidRect,
}

impl RectPartition {
    pub fn new(mut rects: Vec<Rect>) -> Result<Self, RectPartitionError> {
        if rects.is_empty() {
            return Err(RectPartitionError::Empty);
        }

        rects.sort_by_key(|r| (r.top, r.left));

        let left = rects.iter().map(|r| r.left).min().unwrap();
        let right = rects.iter().map(|r| r.right).max().unwrap();

        let mut height_map = PiecewiseConstant::new(left..right);
        height_map.data.insert(left, 0);

        for slice in rects.group_by(|a, b| a.top == b.top) {
            for rect in slice {
                if rect.width() == 0 || rect.height() == 0 {
                    return Err(RectPartitionError::InvalidRect);
                }

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

    #[must_use]
    pub fn num_rects(&self) -> usize {
        self.rects.len()
    }
}

impl Label for RectPartition {
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

    #[test]
    fn test_rect_partition() {
        assert!(RectPartition::new(vec![
            Rect::new((0, 0), (3, 2)),
            Rect::new((3, 0), (5, 3)),
            Rect::new((0, 2), (2, 5)),
            Rect::new((2, 2), (3, 3)),
            Rect::new((2, 3), (5, 5)),
        ])
        .is_ok());
    }

    #[test]
    fn test_rect_partition_2() {
        assert!(RectPartition::new(vec![Rect::new((0, 0), (1, 1))]).is_ok());
    }

    #[test]
    fn test_rect_partition_3() {
        assert!(RectPartition::new(vec![
            Rect::new((0, 0), (5, 1)),
            Rect::new((0, 1), (1, 3)),
            Rect::new((1, 1), (3, 2)),
            Rect::new((2, 2), (4, 4)),
            Rect::new((3, 1), (4, 2)),
            Rect::new((4, 1), (5, 4)),
            Rect::new((1, 2), (2, 5)),
            Rect::new((0, 3), (1, 5)),
            Rect::new((2, 4), (4, 5)),
            Rect::new((4, 4), (5, 5)),
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
                Rect::new((0, 0), (3, 2)),
                Rect::new((3, 0), (5, 3)),
                Rect::new((0, 2), (2, 5)),
                Rect::new((2, 2), (3, 3)),
                Rect::new((2, 3), (5, 5)),
                Rect::new((5, 0), (5, 5)),
            ]),
            Err(RectPartitionError::InvalidRect)
        );
    }

    #[test]
    fn test_rect_partition_6() {
        assert_eq!(
            RectPartition::new(vec![Rect::new((0, 0), (0, 0))]),
            Err(RectPartitionError::InvalidRect)
        );
    }

    #[test]
    fn test_rect_partition_7() {
        assert_eq!(
            RectPartition::new(vec![
                Rect::new((0, 0), (2, 2)),
                Rect::new((2, 0), (4, 1)),
                Rect::new((3, 1), (4, 2)),
            ]),
            Err(RectPartitionError::NotPartition { x: 2, y: 1 })
        );
    }

    #[test]
    fn test_rect_partition_8() {
        assert_eq!(
            RectPartition::new(vec![
                Rect::new((0, 0), (3, 3)),
                Rect::new((2, 0), (6, 3)),
                Rect::new((0, 3), (3, 6)),
                Rect::new((3, 3), (6, 6)),
            ]),
            Err(RectPartitionError::NotPartition { x: 2, y: 0 })
        );
    }

    #[test]
    fn test_rect_partition_9() {
        assert!(RectPartition::new(vec![
            Rect::new((3, 2), (0, 0)),
            Rect::new((3, 0), (5, 3)),
            Rect::new((0, 2), (2, 5)),
            Rect::new((2, 2), (3, 3)),
            Rect::new((2, 3), (5, 5)),
        ])
        .is_err());
    }

    #[test]
    fn test_rect_partition_10() {
        assert_eq!(
            RectPartition::new(vec![
                Rect::new((0, 0), (4, 1)),
                Rect::new((0, 1), (1, 4)),
                Rect::new((1, 1), (4, 1)),
                Rect::new((1, 1), (4, 4)),
            ]),
            Err(RectPartitionError::InvalidRect)
        );
    }
}
