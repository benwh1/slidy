use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Rect {
    fn new(top_left: (u32, u32), bottom_right: (u32, u32)) -> Self {
        Self {
            top: top_left.1,
            left: top_left.0,
            bottom: bottom_right.1,
            right: bottom_right.0,
        }
    }
}

#[derive(Debug)]
struct PiecewiseConstant {
    data: BTreeMap<u32, u32>,
    domain: Range<u32>,
}

impl PiecewiseConstant {
    fn new(domain: Range<u32>) -> Self {
        let mut data = BTreeMap::new();
        data.insert(domain.start, 0);
        Self { data, domain }
    }

    fn value(&self, x: u32) -> u32 {
        self.data
            .range(self.domain.start..=x)
            .last()
            .map(|(_, &v)| v)
            .unwrap()
    }

    fn range_value(&self, range: Range<u32>) -> Option<u32> {
        let v = self.value(range.start);
        if self.data.range(range).map(|(_, &v)| v).all(|x| x == v) {
            Some(v)
        } else {
            None
        }
    }

    fn set_range_value(&mut self, range: Range<u32>, value: u32) {
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

struct RectPartition {
    rects: Vec<Rect>,
}

impl RectPartition {
    fn new(mut rects: Vec<Rect>) -> Option<Self> {
        if rects.is_empty() {
            return None;
        }

        rects.sort_by_key(|r| (r.top, r.left));

        let left = rects.iter().map(|r| r.left).min().unwrap();
        let right = rects.iter().map(|r| r.right).max().unwrap();

        let mut height_map = PiecewiseConstant::new(left..right);
        height_map.data.insert(left, 0);

        for slice in rects.group_by(|a, b| a.top == b.top) {
            let y = slice[0].top;
            for rect in slice {
                let height = height_map.range_value(rect.left..rect.right);
                if let Some(height) = height {
                    if height == rect.top {
                        height_map.set_range_value(rect.left..rect.right, rect.bottom);
                    } else {
                        return None;
                    }
                }
            }
            if height_map.data.values().find(|&&a| a == y).is_some() {
                return None;
            }
        }

        if height_map.range_value(left..right).is_some() {
            Some(Self { rects })
        } else {
            None
        }
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
        .is_some());
    }

    #[test]
    fn test_rect_partition_2() {
        assert!(RectPartition::new(vec![Rect::new((0, 0), (1, 1))]).is_some());
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
        .is_some());
    }

    #[test]
    fn test_rect_partition_4() {
        assert!(RectPartition::new(vec![]).is_none());
    }

    #[test]
    fn test_rect_partition_5() {
        assert!(RectPartition::new(vec![
            Rect::new((0, 0), (3, 2)),
            Rect::new((3, 0), (5, 3)),
            Rect::new((0, 2), (2, 5)),
            Rect::new((2, 2), (3, 3)),
            Rect::new((2, 3), (5, 5)),
            Rect::new((5, 0), (5, 5)),
        ])
        .is_none());
    }

    #[test]
    fn test_rect_partition_6() {
        assert!(RectPartition::new(vec![Rect::new((0, 0), (0, 0))]).is_none());
    }

    #[test]
    fn test_rect_partition_7() {
        assert!(RectPartition::new(vec![
            Rect::new((0, 0), (2, 2)),
            Rect::new((2, 0), (4, 1)),
            Rect::new((3, 1), (4, 2)),
        ])
        .is_none());
    }

    #[test]
    fn test_rect_partition_8() {
        assert!(RectPartition::new(vec![
            Rect::new((0, 0), (3, 3)),
            Rect::new((2, 0), (6, 3)),
            Rect::new((0, 3), (3, 6)),
            Rect::new((3, 3), (6, 6)),
        ])
        .is_none());
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
        .is_none());
    }
}
