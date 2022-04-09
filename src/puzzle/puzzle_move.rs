pub enum Direction {
    Up,
    Left,
    Down,
    Right,
}

struct Move {
    direction: Direction,
    amount: u32,
}
