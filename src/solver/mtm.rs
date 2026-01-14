use std::{cell::Cell, time::Instant};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
};

const SIZE: usize = 252252000;

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

fn multinomial(counts: &[u8]) -> u64 {
    let mut rem = counts.iter().copied().sum();
    let mut r = 1;

    for &c in counts {
        if c != 0 {
            r *= binomial(rem, c) as u64;
            rem -= c;
        }
    }

    r
}

fn encode_multiset<const LEN: usize, const DISTINCT: usize>(
    arr: [u8; LEN],
    tally: [u8; DISTINCT],
) -> u64 {
    let mut remaining = tally;
    let mut t = 0;

    for &v in &arr {
        let cur = v as usize;
        for s in 0..cur {
            if remaining[s] > 0 {
                remaining[s] -= 1;
                t += multinomial(&remaining);
                remaining[s] += 1;
            }
        }
        remaining[cur] -= 1;
    }

    t
}

fn decode_multiset<const LEN: usize, const DISTINCT: usize>(
    mut t: u64,
    tally: [u8; DISTINCT],
) -> [u8; LEN] {
    let mut remaining = tally;
    let mut out = [0u8; LEN];

    for i in 0..LEN {
        for s in 0..DISTINCT {
            if remaining[s] == 0 {
                continue;
            }
            remaining[s] -= 1;
            let m = multinomial(&remaining);
            if t < m {
                out[i] = s as u8;
                break;
            } else {
                t -= m;
                remaining[s] += 1;
            }
        }
    }

    out
}

const GAPS: [[u8; 4]; 16] = [
    [4, 1, u8::MAX, u8::MAX],
    [5, 2, u8::MAX, 0],
    [6, 3, u8::MAX, 1],
    [7, u8::MAX, u8::MAX, 2],
    [8, 5, 0, u8::MAX],
    [9, 6, 1, 4],
    [10, 7, 2, 5],
    [11, u8::MAX, 3, 6],
    [12, 9, 4, u8::MAX],
    [13, 10, 5, 8],
    [14, 11, 6, 9],
    [15, u8::MAX, 7, 10],
    [u8::MAX, 13, 8, u8::MAX],
    [u8::MAX, 14, 9, 12],
    [u8::MAX, 15, 10, 13],
    [u8::MAX, u8::MAX, 11, 14],
];

#[derive(Clone, Copy, Debug)]
struct FourBitPuzzle {
    pieces: u64,
    gap: u8,
}

impl FourBitPuzzle {
    const SOLVED: u64 = 0x0FEDCBA987654321;
    const SOLVED_REDUCED: u64 = 0x0444333322221110;

    fn new() -> Self {
        Self {
            pieces: Self::SOLVED,
            gap: 15,
        }
    }

    fn new_reduced() -> Self {
        Self {
            pieces: Self::SOLVED_REDUCED,
            gap: 15,
        }
    }

    fn do_move(&mut self, dir: Direction) -> bool {
        let gap = self.gap;
        let other = GAPS[gap as usize][dir as usize];

        if other == u8::MAX {
            return false;
        }

        let shift_gap = gap * 4;
        let shift_other = other * 4;

        let piece = (self.pieces >> shift_other) & 0xF;
        let mask = (piece << shift_gap) | (piece << shift_other);

        self.pieces ^= mask;
        self.gap = other;

        true
    }
}

impl From<[u8; 16]> for FourBitPuzzle {
    fn from(value: [u8; 16]) -> Self {
        let mut puzzle = FourBitPuzzle { pieces: 0, gap: 0 };
        for (i, &piece) in value.iter().enumerate() {
            puzzle.pieces |= (piece as u64) << (4 * i);
            if piece == 0 {
                puzzle.gap = i as u8;
            }
        }
        puzzle
    }
}

struct IndexingTable {
    high: Box<[u32]>,
    low: Box<[u16]>,
}

impl IndexingTable {
    fn new() -> Self {
        let max_counts = [1, 4, 4, 4, 3];
        let mut counts = [0, 0, 0, 0, 0];

        let mut high = vec![0; 5 * 5 * 5 * 5 * 5 * 5 * 5 * 5];
        let mut low = vec![0; 5 * 5 * 5 * 5 * 5 * 5 * 5 * 5];

        for p0 in 0..5 {
            counts[p0] += 1;
            for p1 in 0..5 {
                if counts[p1] >= max_counts[p1] {
                    continue;
                }
                counts[p1] += 1;
                for p2 in 0..5 {
                    if counts[p2] >= max_counts[p2] {
                        continue;
                    }
                    counts[p2] += 1;
                    for p3 in 0..5 {
                        if counts[p3] >= max_counts[p3] {
                            continue;
                        }
                        counts[p3] += 1;
                        for p4 in 0..5 {
                            if counts[p4] >= max_counts[p4] {
                                continue;
                            }
                            counts[p4] += 1;
                            for p5 in 0..5 {
                                if counts[p5] >= max_counts[p5] {
                                    continue;
                                }
                                counts[p5] += 1;
                                for p6 in 0..5 {
                                    if counts[p6] >= max_counts[p6] {
                                        continue;
                                    }
                                    counts[p6] += 1;
                                    for p7 in 0..5 {
                                        if counts[p7] >= max_counts[p7] {
                                            continue;
                                        }
                                        counts[p7] += 1;

                                        let index = p7
                                            + 5 * (p6
                                                + 5 * (p5
                                                    + 5 * (p4
                                                        + 5 * (p3
                                                            + 5 * (p2 + 5 * (p1 + 5 * p0))))));

                                        let mut pieces = [
                                            p0 as u8, p1 as u8, p2 as u8, p3 as u8, p4 as u8,
                                            p5 as u8, p6 as u8, p7 as u8, 0, 0, 0, 0, 0, 0, 0, 0,
                                        ];

                                        let mut piece_index = 8;
                                        for i in 0..5 {
                                            for _ in counts[i]..max_counts[i] {
                                                pieces[piece_index] = i as u8;
                                                piece_index += 1;
                                            }
                                        }
                                        high[index] = encode_multiset(pieces, max_counts) as u32;

                                        let pieces = [
                                            p0 as u8, p1 as u8, p2 as u8, p3 as u8, p4 as u8,
                                            p5 as u8, p6 as u8, p7 as u8,
                                        ];
                                        low[index] = encode_multiset(pieces, counts) as u16;

                                        counts[p7 as usize] -= 1;
                                    }
                                    counts[p6 as usize] -= 1;
                                }
                                counts[p5 as usize] -= 1;
                            }
                            counts[p4 as usize] -= 1;
                        }
                        counts[p3 as usize] -= 1;
                    }
                    counts[p2 as usize] -= 1;
                }
                counts[p1 as usize] -= 1;
            }
            counts[p0 as usize] -= 1;
        }

        Self {
            high: high.into_boxed_slice(),
            low: low.into_boxed_slice(),
        }
    }

    fn encode(&self, puzzle: u64) -> u32 {
        let shr = |n: u64| (puzzle >> n) & 0xF;

        let high = shr(28)
            + 5 * (shr(24)
                + 5 * (shr(20)
                    + 5 * (shr(16) + 5 * (shr(12) + 5 * (shr(8) + 5 * (shr(4) + 5 * shr(0)))))));
        let low = shr(60)
            + 5 * (shr(56)
                + 5 * (shr(52)
                    + 5 * (shr(48) + 5 * (shr(44) + 5 * (shr(40) + 5 * (shr(36) + 5 * shr(32)))))));

        self.high[high as usize] + self.low[low as usize] as u32
    }

    fn decode(&self, t: u32) -> FourBitPuzzle {
        let pieces: [u8; 16] = decode_multiset(t as u64, [1, 4, 4, 4, 3]);

        FourBitPuzzle::from(pieces)
    }
}

pub struct Pdb {
    pdb: Box<[u8]>,
}

impl Pdb {
    fn new(indexing_table: &IndexingTable) -> Self {
        const FILENAME: &str = "mtm_pdb.bin";

        if let Ok(data) = std::fs::read(FILENAME) {
            let pdb = data.into_boxed_slice();

            return Self { pdb };
        }

        let mut pdb = vec![u8::MAX; SIZE];

        let puzzle = FourBitPuzzle::new_reduced();
        let solved_index = indexing_table.encode(puzzle.pieces) as usize;
        pdb[solved_index] = 0;

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
                    let mut puzzle = indexing_table.decode(i as u32);
                    while puzzle.do_move(mv) {
                        let idx = indexing_table.encode(puzzle.pieces) as usize;
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
    indexing_table: IndexingTable,
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    puzzle: Cell<FourBitPuzzle>,
}

impl Solver {
    pub fn new() -> Self {
        let indexing_table = IndexingTable::new();
        let pdb = Pdb::new(&indexing_table);

        Self {
            indexing_table,
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
            puzzle: Cell::new(FourBitPuzzle::new()),
        }
    }

    fn dfs(
        &self,
        depth: u8,
        last_axis: Option<Axis>,
        mut puzzle: FourBitPuzzle,
        mut transposed_puzzle: FourBitPuzzle,
    ) -> bool {
        let coord = self.indexing_table.encode(puzzle.pieces);
        let heuristic = self.pdb.pdb[coord as usize];

        if heuristic > depth {
            return false;
        }

        let coord = self.indexing_table.encode(transposed_puzzle.pieces);
        let heuristic = self.pdb.pdb[coord as usize];

        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            let mut p = self.puzzle.get();
            for mv in &self.solution[..self.solution_ptr.get()] {
                p.do_move(mv.get());
            }
            return p.pieces == FourBitPuzzle::SOLVED;
        }

        for (dir, transposed_dir) in [
            (Direction::Up, Direction::Left),
            (Direction::Left, Direction::Up),
            (Direction::Down, Direction::Right),
            (Direction::Right, Direction::Down),
        ] {
            if last_axis.is_some_and(|a| a == dir.into()) {
                continue;
            }

            let mut amount = 0;

            while puzzle.do_move(dir) {
                transposed_puzzle.do_move(transposed_dir);
                amount += 1;

                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle) {
                    return true;
                }
            }

            for _ in 0..amount {
                puzzle.do_move(dir.inverse());
                transposed_puzzle.do_move(transposed_dir.inverse());
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

        let mut pieces = [0; 16];
        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = puzzle.piece_at(i as u64).to_u8().unwrap();
        }

        let four_bit_puzzle = FourBitPuzzle::from(pieces);
        self.puzzle.set(four_bit_puzzle);

        const REDUCED: [u8; 16] = [0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4];
        let mut reduced_pieces = [0; 16];
        for (reduced_piece, piece) in reduced_pieces.iter_mut().zip(pieces.iter()) {
            *reduced_piece = REDUCED[*piece as usize];
        }

        let reduced_puzzle = FourBitPuzzle::from(reduced_pieces);

        // Compute transpose (swap piece 2/piece 5, etc. then swap position 2/position 5, etc.)
        let mut transposed_reduced_pieces = reduced_pieces;
        let pos = |i| pieces.iter().position(|&p| p == i).unwrap();
        transposed_reduced_pieces.swap(pos(2), pos(5));
        transposed_reduced_pieces.swap(pos(3), pos(9));
        transposed_reduced_pieces.swap(pos(4), pos(13));
        transposed_reduced_pieces.swap(pos(7), pos(10));
        transposed_reduced_pieces.swap(pos(8), pos(14));
        transposed_reduced_pieces.swap(pos(12), pos(15));
        transposed_reduced_pieces.swap(1, 4);
        transposed_reduced_pieces.swap(2, 8);
        transposed_reduced_pieces.swap(3, 12);
        transposed_reduced_pieces.swap(6, 9);
        transposed_reduced_pieces.swap(7, 13);
        transposed_reduced_pieces.swap(11, 14);

        let transposed_reduced_puzzle = FourBitPuzzle::from(transposed_reduced_pieces);

        let coord = self.indexing_table.encode(reduced_puzzle.pieces);
        let mut depth = self.pdb.pdb[coord as usize];

        let timer = Instant::now();
        loop {
            println!("depth {depth} elapsed {:?}", timer.elapsed());

            if self.dfs(depth, None, reduced_puzzle, transposed_reduced_puzzle) {
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

    mod four_bit_puzzle {
        use super::*;

        #[test]
        fn test_four_bit_puzzle_1() {
            let mut puzzle = FourBitPuzzle::new();
            assert!(puzzle.do_move(Direction::Down));
            assert_eq!(puzzle.pieces, 0xCFED0BA987654321);
            assert_eq!(puzzle.gap, 11);
            assert!(puzzle.do_move(Direction::Down));
            assert_eq!(puzzle.pieces, 0xCFED8BA907654321);
            assert_eq!(puzzle.gap, 7);
            assert!(puzzle.do_move(Direction::Down));
            assert_eq!(puzzle.pieces, 0xCFED8BA947650321);
            assert_eq!(puzzle.gap, 3);
            assert!(!puzzle.do_move(Direction::Down));
            assert_eq!(puzzle.pieces, 0xCFED8BA947650321);
            assert_eq!(puzzle.gap, 3);
        }

        #[test]
        fn test_four_bit_puzzle_2() {
            let mut puzzle = FourBitPuzzle::new();
            puzzle.do_move(Direction::Right);
            assert_eq!(puzzle.pieces, 0xF0EDCBA987654321);
        }
    }

    mod indexing_table {
        use super::*;

        #[test]
        fn test_indexing_table() {
            let table = IndexingTable::new();

            let max_counts = [1u8, 4, 4, 4, 3];
            let mut counts = [0u8, 0, 0, 0, 0];

            let mut index = 0;

            for p0 in 0..5 {
                counts[p0] += 1;
                for p1 in 0..5 {
                    if counts[p1] >= max_counts[p1] {
                        continue;
                    }
                    counts[p1] += 1;
                    for p2 in 0..5 {
                        if counts[p2] >= max_counts[p2] {
                            continue;
                        }
                        counts[p2] += 1;
                        for p3 in 0..5 {
                            if counts[p3] >= max_counts[p3] {
                                continue;
                            }
                            counts[p3] += 1;
                            for p4 in 0..5 {
                                if counts[p4] >= max_counts[p4] {
                                    continue;
                                }
                                counts[p4] += 1;
                                for p5 in 0..5 {
                                    if counts[p5] >= max_counts[p5] {
                                        continue;
                                    }
                                    counts[p5] += 1;
                                    for p6 in 0..5 {
                                        if counts[p6] >= max_counts[p6] {
                                            continue;
                                        }
                                        counts[p6] += 1;
                                        for p7 in 0..5 {
                                            if counts[p7] >= max_counts[p7] {
                                                continue;
                                            }
                                            counts[p7] += 1;

                                            for p8 in 0..5 {
                                                if counts[p8] >= max_counts[p8] {
                                                    continue;
                                                }
                                                counts[p8] += 1;

                                                for p9 in 0..5 {
                                                    if counts[p9] >= max_counts[p9] {
                                                        continue;
                                                    }
                                                    counts[p9] += 1;

                                                    for p10 in 0..5 {
                                                        if counts[p10] >= max_counts[p10] {
                                                            continue;
                                                        }
                                                        counts[p10] += 1;

                                                        for p11 in 0..5 {
                                                            if counts[p11] >= max_counts[p11] {
                                                                continue;
                                                            }
                                                            counts[p11] += 1;

                                                            for p12 in 0..5 {
                                                                if counts[p12] >= max_counts[p12] {
                                                                    continue;
                                                                }
                                                                counts[p12] += 1;

                                                                for p13 in 0..5 {
                                                                    if counts[p13]
                                                                        >= max_counts[p13]
                                                                    {
                                                                        continue;
                                                                    }
                                                                    counts[p13] += 1;

                                                                    for p14 in 0..5 {
                                                                        if counts[p14]
                                                                            >= max_counts[p14]
                                                                        {
                                                                            continue;
                                                                        }
                                                                        counts[p14] += 1;

                                                                        for p15 in 0..5 {
                                                                            if counts[p15]
                                                                                >= max_counts[p15]
                                                                            {
                                                                                continue;
                                                                            }
                                                                            counts[p15] += 1;

                                                                            let puzzle = ((p15
                                                                                as u64)
                                                                                << 60)
                                                                                | ((p14 as u64)
                                                                                    << 56)
                                                                                | ((p13 as u64)
                                                                                    << 52)
                                                                                | ((p12 as u64)
                                                                                    << 48)
                                                                                | ((p11 as u64)
                                                                                    << 44)
                                                                                | ((p10 as u64)
                                                                                    << 40)
                                                                                | ((p9 as u64)
                                                                                    << 36)
                                                                                | ((p8 as u64)
                                                                                    << 32)
                                                                                | ((p7 as u64)
                                                                                    << 28)
                                                                                | ((p6 as u64)
                                                                                    << 24)
                                                                                | ((p5 as u64)
                                                                                    << 20)
                                                                                | ((p4 as u64)
                                                                                    << 16)
                                                                                | ((p3 as u64)
                                                                                    << 12)
                                                                                | ((p2 as u64)
                                                                                    << 8)
                                                                                | ((p1 as u64)
                                                                                    << 4)
                                                                                | (p0 as u64);

                                                                            let encoded = table
                                                                                .encode(puzzle);

                                                                            assert_eq!(
                                                                                encoded,
                                                                                index,
                                                                                "puzzle {puzzle:x} encoded {encoded} index {index}"
                                                                            );

                                                                            index += 1;

                                                                            counts[p15] -= 1;
                                                                        }
                                                                        counts[p14] -= 1;
                                                                    }
                                                                    counts[p13] -= 1;
                                                                }
                                                                counts[p12] -= 1;
                                                            }
                                                            counts[p11] -= 1;
                                                        }
                                                        counts[p10] -= 1;
                                                    }
                                                    counts[p9] -= 1;
                                                }
                                                counts[p8] -= 1;
                                            }
                                            counts[p7] -= 1;
                                        }
                                        counts[p6] -= 1;
                                    }
                                    counts[p5] -= 1;
                                }
                                counts[p4] -= 1;
                            }
                            counts[p3] -= 1;
                        }
                        counts[p2] -= 1;
                    }
                    counts[p1] -= 1;
                }
                counts[p0] -= 1;
            }
        }
    }
}
