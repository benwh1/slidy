use super::puzzle_move::Move;

pub struct Algorithm {
    pub moves: Vec<Move>,
}

impl Algorithm {
    fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    fn length(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    fn push(&mut self, m: Move) {
        self.moves.push(m)
    }
}
