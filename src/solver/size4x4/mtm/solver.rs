use std::{cell::Cell, time::Instant};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::size4x4::mtm::{
        base_5_table::Base5Table,
        indexing_table::IndexingTable,
        pdb::Pdb,
        puzzle::{FourBitPuzzle, ReducedFourBitPuzzle},
    },
};

pub struct Solver {
    indexing_table: IndexingTable,
    base_5_table: Base5Table,
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    puzzle: Cell<FourBitPuzzle>,
}

impl Solver {
    pub fn new() -> Self {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table);

        Self {
            indexing_table,
            base_5_table,
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
        mut puzzle: ReducedFourBitPuzzle,
        mut transposed_puzzle: ReducedFourBitPuzzle,
    ) -> bool {
        let coord = self
            .indexing_table
            .encode(puzzle.pieces, &self.base_5_table) as usize;
        let heuristic = unsafe { self.pdb.get_unchecked(coord) };

        if heuristic > depth {
            return false;
        }

        let coord = self
            .indexing_table
            .encode(transposed_puzzle.pieces, &self.base_5_table) as usize;
        let heuristic = unsafe { self.pdb.get_unchecked(coord) };

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

        let original_puzzle = puzzle;
        let original_transposed = transposed_puzzle;

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

            puzzle = original_puzzle;
            transposed_puzzle = original_transposed;

            while puzzle.do_move(dir) {
                transposed_puzzle.do_move(transposed_dir);
                amount += 1;

                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle) {
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

        let mut pieces = [0; 16];
        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = puzzle.piece_at(i as u64).to_u8().unwrap();
        }

        let four_bit_puzzle = FourBitPuzzle::from(pieces);
        let reduced_puzzle = four_bit_puzzle.reduced();
        let transposed_reduced_puzzle = four_bit_puzzle.transposed().reduced();

        self.puzzle.set(four_bit_puzzle);

        let coord = self
            .indexing_table
            .encode(reduced_puzzle.pieces, &self.base_5_table);
        let mut depth = self.pdb.get(coord as usize);

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
