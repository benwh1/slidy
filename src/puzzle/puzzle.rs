pub struct Puzzle {
    pieces: Vec<u32>,
    width: u32,
    height: u32,
    gap: usize,
}

const GAP_PIECE: u32 = 0;

impl Puzzle {
    pub fn new(width: u32, height: u32) -> Puzzle {
        Puzzle {
            pieces: {
                let mut v: Vec<u32> = (1..width * height).collect();
                v.push(GAP_PIECE);
                v
            },
            width,
            height,
            gap: (width * height - 1) as usize,
        }
    }
}
