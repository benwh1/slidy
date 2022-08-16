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
