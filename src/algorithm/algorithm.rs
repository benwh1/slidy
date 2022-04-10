use super::puzzle_move::Move;

pub struct Algorithm {
    pub moves: Vec<Move>,
}

impl Algorithm {
    pub fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    pub fn length(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    pub fn push(&mut self, m: Move) {
        self.moves.push(m)
    }
}
