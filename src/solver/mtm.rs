use std::{cell::Cell, time::Instant};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
};

const SIZE: usize = 12108096;

fn binomial(n: u8, k: u8) -> u64 {
    if k > n {
        return 0;
    }

    let k = k.min(n - k) as u64;
    let n = n as u64;

    let mut result = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }

    result
}

fn encode_multiset<const LEN: usize, const DISTINCT: usize>(
    arr: [u8; LEN],
    tally: [u8; DISTINCT],
) -> u64 {
    let mut x = 0;
    let mut j;
    let mut t;
    let mut r;
    let mut total = 0;
    for piece in (0..DISTINCT as u8).rev() {
        j = 0;
        t = 0;
        r = tally[piece as usize];
        total += tally[piece as usize];
        for i in (0..LEN as u8).rev() {
            if arr[i as usize] == piece {
                t += binomial(i + j + total - LEN as u8, r);
                r -= 1;
            }
            if arr[i as usize] < piece {
                j += 1;
            }
        }
        x *= binomial(total, tally[piece as usize]);
        x += t;
    }

    x
}

fn decode_multiset<const LEN: usize, const DISTINCT: usize>(
    mut t: u64,
    tally: [u8; DISTINCT],
) -> [u8; LEN] {
    let mut out = [u8::MAX; LEN];
    let mut t2;
    let mut r;
    let mut j;
    let mut total = 0;
    for piece in 0..DISTINCT {
        r = tally[piece];
        j = 0;
        let bin = binomial(LEN as u8 - total, r);
        t2 = t % bin;
        for i in (0..LEN).rev() {
            if out[i] != u8::MAX {
                j += 1;
                continue;
            }
            let b = binomial(i as u8 + j - total, r);
            if t2 >= b {
                t2 -= b;
                out[i] = piece as u8;
                r -= 1;
            }
        }
        t /= bin;
        total += tally[piece];
    }
    out
}

#[derive(Clone, Copy, Debug)]
struct Puzzle {
    pieces: [u8; 16],
    gap: u8,
}

impl Puzzle {
    const SOLVED_STATE: [u8; 16] = [1, 1, 2, 2, 1, 1, 2, 2, 1, 3, 3, 2, 3, 3, 3, 0];

    fn new() -> Self {
        Self {
            pieces: Self::SOLVED_STATE,
            gap: 15,
        }
    }

    fn do_move(&mut self, dir: Direction, amount: u8) -> bool {
        let dg = match dir {
            Direction::Up => {
                if self.gap >= 16 - 4 * amount {
                    return false;
                }
                4
            }
            Direction::Left => {
                if self.gap % 4 >= 4 - amount {
                    return false;
                }
                1
            }
            Direction::Down => {
                if self.gap < 4 * amount {
                    return false;
                }
                -4
            }
            Direction::Right => {
                if self.gap % 4 < amount {
                    return false;
                }
                -1
            }
        };

        for i in 0..amount as i8 {
            self.pieces[self.gap.wrapping_add_signed(i * dg) as usize] =
                self.pieces[self.gap.wrapping_add_signed((i + 1) * dg) as usize];
        }

        self.gap = self.gap.wrapping_add_signed(amount as i8 * dg);
        self.pieces[self.gap as usize] = 0;

        true
    }

    fn encode(&self) -> u64 {
        encode_multiset(self.pieces, [1, 5, 5, 5])
    }

    fn decode(&mut self, index: u64) {
        self.pieces = decode_multiset(index, [1, 5, 5, 5]);
        for i in 0..16 {
            if self.pieces[i] == 0 {
                self.gap = i as u8;
                break;
            }
        }
    }
}

struct TranspositionTable {
    transposition_table: Box<[[u32; 4]]>,
}

impl TranspositionTable {
    fn new() -> Self {
        const FILENAME: &str = "mtm_transposition_table.bin";

        if let Ok(data) = std::fs::read(FILENAME) {
            let transposition_table: Box<[[u32; 4]]> =
                bytemuck::cast_slice(&data).to_vec().into_boxed_slice();

            return Self {
                transposition_table,
            };
        }

        let mut transposition_table = vec![[0; 4]; SIZE];
        let mut puzzle = Puzzle::new();

        for i in 0..SIZE {
            puzzle.decode(i as u64);

            let mut moves = [u32::MAX; 4];

            for mv in [
                Direction::Up,
                Direction::Left,
                Direction::Down,
                Direction::Right,
            ] {
                if puzzle.do_move(mv, 1) {
                    let index = puzzle.encode();
                    moves[mv as usize] = index as u32;
                    puzzle.do_move(mv.inverse(), 1);
                }
            }

            transposition_table[i] = moves;
        }

        let transposition_table = transposition_table.into_boxed_slice();

        std::fs::write(FILENAME, bytemuck::cast_slice(&*transposition_table)).unwrap();

        Self {
            transposition_table,
        }
    }
}

struct CoordPuzzle<'a> {
    puzzle: u32,
    transposition_table: &'a TranspositionTable,
}

impl<'a> CoordPuzzle<'a> {
    fn new(transposition_table: &'a TranspositionTable) -> Self {
        Self {
            puzzle: Puzzle::new().encode() as u32,
            transposition_table,
        }
    }

    fn do_move(&mut self, dir: Direction) -> bool {
        let next = self.transposition_table.transposition_table[self.puzzle as usize][dir as usize];
        if next == u32::MAX {
            return false;
        }
        self.puzzle = next;
        true
    }

    fn encode(&self) -> u32 {
        self.puzzle
    }

    fn decode(&mut self, index: u32) {
        self.puzzle = index;
    }
}

pub struct Pdb {
    pdb: Box<[u8]>,
}

impl Pdb {
    fn new(transposition_table: &TranspositionTable) -> Self {
        const FILENAME: &str = "mtm_pdb.bin";

        if let Ok(data) = std::fs::read(FILENAME) {
            let pdb = data.into_boxed_slice();

            return Self { pdb };
        }

        let mut pdb = vec![u8::MAX; SIZE];

        let mut puzzle = CoordPuzzle::new(transposition_table);
        pdb[puzzle.puzzle as usize] = 0;

        let mut new = 1;
        let mut total = 1;
        let mut depth = 0;

        while new != 0 {
            new = 0;

            for i in 0..SIZE {
                if pdb[i] != depth {
                    continue;
                }

                for mv in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    puzzle.decode(i as u32);
                    while puzzle.do_move(mv) {
                        let idx = puzzle.encode() as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            new += 1;
                        }
                    }
                }
            }

            total += new;
            depth += 1;

            println!("depth {depth} new {new} total {total}");
        }

        let pdb = pdb.into_boxed_slice();

        std::fs::write(FILENAME, &*pdb).unwrap();

        Self { pdb }
    }
}

pub struct Solver {
    transposition_table: TranspositionTable,
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    puzzle: Cell<Puzzle>,
}

impl Solver {
    pub fn new() -> Self {
        let transposition_table = TranspositionTable::new();
        let pdb = Pdb::new(&transposition_table);

        Self {
            transposition_table,
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
            puzzle: Cell::new(Puzzle::new()),
        }
    }

    fn dfs(&self, depth: u8, last_axis: Option<Axis>, coord: u32) -> bool {
        let heuristic = self.pdb.pdb[coord as usize];

        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            let mut puzzle = self.puzzle.get();
            for mv in &self.solution[..self.solution_ptr.get()] {
                puzzle.do_move(mv.get(), 1);
            }
            return puzzle.pieces == [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
        }

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_axis.is_some_and(|a| a == dir.into()) {
                continue;
            }

            let mut amount = 0;
            let mut next = coord;

            loop {
                next = self.transposition_table.transposition_table[next as usize][dir as usize];

                if next == u32::MAX {
                    break;
                }

                amount += 1;

                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.into()), next) {
                    return true;
                }
            }

            self.solution_ptr
                .set(self.solution_ptr.get() - amount as usize);
        }

        false
    }

    pub fn solve<P: SlidingPuzzle>(&self, puzzle: &P) -> Option<Algorithm> {
        if puzzle.size() != Size::new(4, 4).unwrap() {
            return None;
        }

        // Reset state
        self.solution_ptr.set(0);

        let mut pieces = [0u8; 16];
        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = puzzle.piece_at(i as u64).to_u8().unwrap();
        }
        let gap = pieces.iter().position(|&p| p == 0).unwrap() as u8;

        let puzzle = Puzzle { pieces, gap };
        self.puzzle.set(puzzle);

        let mut coord_pieces = [0; 16];
        for (coord_piece, piece) in coord_pieces.iter_mut().zip(pieces.iter()) {
            *coord_piece = Puzzle::SOLVED_STATE[(*piece as usize + 15) % 16];
        }

        let coord = encode_multiset(coord_pieces, [1, 5, 5, 5]) as u32;
        let mut depth = self.pdb.pdb[coord as usize];

        let timer = Instant::now();
        loop {
            println!("depth {depth} elapsed {:?}", timer.elapsed());

            if self.dfs(depth, None, coord) {
                let mut solution = Algorithm::new();

                for dir in self.solution[..self.solution_ptr.get()]
                    .iter()
                    .map(|c| c.get())
                {
                    solution.push_combine(dir.into());
                }

                println!("found {solution} elapsed {:?}", timer.elapsed());
                return Some(solution);
            }

            depth += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puzzle_1() {
        let mut puzzle = Puzzle::new();
        assert!(!puzzle.do_move(Direction::Up, 1));
        assert!(!puzzle.do_move(Direction::Left, 1));
        assert!(puzzle.do_move(Direction::Down, 3));
        assert!(puzzle.do_move(Direction::Right, 3));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(!puzzle.do_move(Direction::Up, 1));
        assert!(!puzzle.do_move(Direction::Left, 4));
        assert!(puzzle.do_move(Direction::Left, 3));
        assert!(!puzzle.do_move(Direction::Down, 4));
        assert!(puzzle.do_move(Direction::Down, 3));
        assert!(!puzzle.do_move(Direction::Up, 4));
        assert!(puzzle.do_move(Direction::Right, 2));
        assert!(!puzzle.do_move(Direction::Right, 2));
        assert!(puzzle.do_move(Direction::Left, 2));
        assert!(!puzzle.do_move(Direction::Right, 4));
        assert!(puzzle.do_move(Direction::Right, 3));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(puzzle.do_move(Direction::Up, 1));
        assert!(!puzzle.do_move(Direction::Up, 1));
    }

    #[test]
    fn test_puzzle_2() {
        let mut p = Puzzle {
            pieces: [0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3],
            gap: 0,
        };
        assert!(p.do_move(Direction::Up, 1));
        assert_eq!(p.pieces, [1, 1, 1, 1, 0, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3]);
        assert!(p.do_move(Direction::Up, 1));
        assert_eq!(p.pieces, [1, 1, 1, 1, 2, 1, 2, 2, 0, 2, 2, 3, 3, 3, 3, 3]);
        assert!(p.do_move(Direction::Up, 1));
        assert_eq!(p.pieces, [1, 1, 1, 1, 2, 1, 2, 2, 3, 2, 2, 3, 0, 3, 3, 3]);
        assert!(!p.do_move(Direction::Up, 1));
        assert_eq!(p.pieces, [1, 1, 1, 1, 2, 1, 2, 2, 3, 2, 2, 3, 0, 3, 3, 3]);
    }

    #[test]
    fn test_multiset() {
        for i in 0..SIZE {
            let mut puzzle = Puzzle::new();
            puzzle.decode(i as u64);
            let encoded = puzzle.encode();
            assert_eq!(i as u64, encoded);
        }
    }
}
