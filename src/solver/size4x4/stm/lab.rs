//! Heuristic laboratory: a slow, flexible IDA* for measuring how strong a
//! heuristic is (nodes searched), not how fast it runs.
//!
//! Same search as [`crate::solver::size4x4::stm::large`] (duplicate-pruning automaton, the
//! (max, sum) best-first order, prune at `h >= depth`), but every
//! candidate's heuristic is computed from scratch from a configurable list
//! of components, so any combination of tables can be compared.

use std::{collections::HashMap, fs::File, path::Path};

use memmap2::Mmap;

use crate::solver::size4x4::stm::{
    fsm,
    tables::{self, Board, MIRROR},
};

const MAX_PATH: usize = 100;

/// One pattern database file.
pub struct Table {
    tiles: Vec<u8>,
    data: Mmap,
    /// Indexed by placement * (16 - k) + blank ordinal.
    exact: bool,
    /// 2-bit excess codes over the minimum table `data` (4 per byte, low
    /// bits first, indexed like an exact table): value = min + 2 * code.
    excess: Option<Mmap>,
    /// Minimum tables only: when the blank is walled in by pattern tiles
    /// (a one-cell pocket), any way out moves a pattern tile, so the value
    /// is at least 1 + the minimum over those moves' placements.
    pub pocket: bool,
    /// Same bound for the blank's whole region (any size): every way out of
    /// the region moves a pattern tile into it.
    pub region: bool,
}

impl Table {
    /// Map `path`, holding the table for `tiles` (in index order).
    #[must_use]
    pub fn open(path: &Path, tiles: Vec<u8>, exact: bool) -> Self {
        let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // SAFETY: read-only mapping of a file nobody modifies meanwhile.
        let data = unsafe { Mmap::map(&file).unwrap() };
        let k = tiles.len() as u64;
        let mut size: u64 = (0..k).map(|i| 16 - i).product();
        if exact {
            size *= 16 - k;
        }
        assert_eq!(data.len() as u64, size, "{}: wrong size", path.display());
        Self {
            tiles,
            data,
            exact,
            excess: None,
            pocket: false,
            region: false,
        }
    }

    /// A minimum table plus a 2-bit excess table.
    #[must_use]
    pub fn open_excess(min: &Path, excess: &Path, tiles: Vec<u8>) -> Self {
        let mut table = Self::open(min, tiles, false);
        let file = File::open(excess).unwrap_or_else(|e| panic!("{}: {e}", excess.display()));
        // SAFETY: read-only mapping of a file nobody modifies meanwhile.
        let codes = unsafe { Mmap::map(&file).unwrap() };
        let k = table.tiles.len() as u64;
        let entries = table.data.len() as u64 * (16 - k);
        assert_eq!(codes.len() as u64, entries.div_ceil(4), "excess size");
        table.excess = Some(codes);
        table
    }

    fn placement(&self, pos: &[u8; 16]) -> (u64, u16) {
        let mut used = 0u16;
        let mut idx = 0u64;
        for (i, &t) in self.tiles.iter().enumerate() {
            let p = pos[t as usize];
            let digit = u64::from(p) - u64::from((used & ((1u16 << p) - 1)).count_ones());
            idx = idx * (16 - i as u64) + digit;
            used |= 1 << p;
        }
        (idx, used)
    }

    fn lookup(&self, pos: &[u8; 16], blank: usize) -> u64 {
        let (mut idx, used) = self.placement(pos);
        if self.region {
            let min = u64::from(self.data[idx as usize]);
            if min == 0 {
                return 0;
            }
            let free = !used;
            let mut region = 1u16 << blank;
            loop {
                let mut grown = region;
                for c in 0..16 {
                    if region & (1 << c) != 0 {
                        for &n in &tables::NEIGHBORS[c] {
                            if n >= 0 && free & (1 << n) != 0 {
                                grown |= 1 << n;
                            }
                        }
                    }
                }
                if grown == region {
                    break;
                }
                region = grown;
            }
            let mut out = u64::MAX;
            for c in 0..16 {
                if region & (1 << c) == 0 {
                    continue;
                }
                for &n in &tables::NEIGHBORS[c] {
                    if n >= 0 && used & (1 << n) != 0 {
                        let t = self
                            .tiles
                            .iter()
                            .find(|&&t| pos[t as usize] as i8 == n)
                            .unwrap();
                        let mut moved = *pos;
                        moved[*t as usize] = c as u8;
                        let (i2, _) = self.placement(&moved);
                        out = out.min(1 + u64::from(self.data[i2 as usize]));
                    }
                }
            }
            return min.max(out);
        }
        if self.pocket {
            let min = u64::from(self.data[idx as usize]);
            let nbrs: Vec<usize> = tables::NEIGHBORS[blank]
                .iter()
                .filter(|&&n| n >= 0)
                .map(|&n| n as usize)
                .collect();
            // (At the goal itself the blank's cell is walled in too, and
            // the value is 0: the recurrence only holds away from it.)
            if min > 0 && nbrs.iter().all(|&n| used & (1 << n) != 0) {
                let mut out = u64::MAX;
                for &n in &nbrs {
                    let mut moved = *pos;
                    let t = self
                        .tiles
                        .iter()
                        .find(|&&t| pos[t as usize] as usize == n)
                        .unwrap();
                    moved[*t as usize] = blank as u8;
                    let (i2, _) = self.placement(&moved);
                    out = out.min(1 + u64::from(self.data[i2 as usize]));
                }
                return min.max(out);
            }
            return min;
        }
        let ord = u64::from((!used & ((1u16 << blank) - 1)).count_ones());
        let slots = 16 - self.tiles.len() as u64;
        if let Some(codes) = &self.excess {
            let e = idx * slots + ord;
            let code = (codes[(e / 4) as usize] >> (2 * (e % 4))) & 3;
            return u64::from(self.data[idx as usize]) + 2 * u64::from(code);
        }
        if self.exact {
            idx = idx * slots + ord;
        }
        u64::from(self.data[idx as usize])
    }
}

/// How a bit table stores an entry ([`BitTable`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BitKind {
    /// Exact table, a bit per free cell (`bitpack compact`, `bigbuild`):
    /// a byte (up to 8 free cells) or a little-endian u16 per placement.
    Compact,
    /// Exact table, a bit per cell: a little-endian u16 per placement
    /// (`bitpack cells`).
    Cells,
    /// Minimum table, 2-bit codes `(value >> 1) & 3` per placement, four
    /// per byte; a move changes it by 1 or 3.
    Min2,
    /// Full table (every move counted), a bit per cell like `Cells`: every
    /// move changes it by exactly 1.
    CellsFull,
    /// Full table, a bit per free cell like `Compact` (`bigbuild --mode
    /// full`).
    CompactFull,
}

/// A table holding only the bits the solver keeps (`(value >> 1) & 1`,
/// or 2-bit codes): its values are decoded against the parent's, like the
/// solver's, and the root's come from walking downhill to the goal.
pub struct BitTable {
    tiles: Vec<u8>,
    data: Mmap,
    kind: BitKind,
    /// Membership of each tile in the group.
    member: [bool; 16],
}

fn flood_region(seed: usize, free: u16) -> u16 {
    let mut region = 1u16 << seed;
    loop {
        let mut grown = region;
        for c in 0..16 {
            if region & (1 << c) != 0 {
                for &n in &tables::NEIGHBORS[c] {
                    if n >= 0 && free & (1 << n) != 0 {
                        grown |= 1 << n;
                    }
                }
            }
        }
        if grown == region {
            return region;
        }
        region = grown;
    }
}

impl BitTable {
    /// Map `path`, holding the table for `tiles` (in index order).
    #[must_use]
    pub fn open(path: &Path, tiles: Vec<u8>, kind: BitKind) -> Self {
        let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // SAFETY: read-only mapping of a file nobody modifies meanwhile.
        let data = unsafe { Mmap::map(&file).unwrap() };
        let k = tiles.len() as u64;
        let size: u64 = (0..k).map(|i| 16 - i).product();
        let bytes = match kind {
            BitKind::Compact | BitKind::CompactFull if k >= 8 => size,
            BitKind::Compact | BitKind::CompactFull | BitKind::Cells | BitKind::CellsFull => {
                2 * size
            }
            BitKind::Min2 => size.div_ceil(4),
        };
        assert_eq!(data.len() as u64, bytes, "{}: wrong size", path.display());
        let mut member = [false; 16];
        for &t in &tiles {
            member[t as usize] = true;
        }
        Self {
            tiles,
            data,
            kind,
            member,
        }
    }

    fn placement(&self, pos: &[u8; 16]) -> (u64, u16) {
        let mut used = 0u16;
        let mut idx = 0u64;
        for (i, &t) in self.tiles.iter().enumerate() {
            let p = pos[t as usize];
            let digit = u64::from(p) - u64::from((used & ((1u16 << p) - 1)).count_ones());
            idx = idx * (16 - i as u64) + digit;
            used |= 1 << p;
        }
        (idx, used)
    }

    /// The stored bit (or 2-bit code) for tile cells `pos` and the blank
    /// at `blank`.
    fn code(&self, pos: &[u8; 16], blank: usize) -> u64 {
        let (idx, used) = self.placement(pos);
        let i = idx as usize;
        match self.kind {
            BitKind::Compact | BitKind::CompactFull => {
                let ord = (!used & ((1u16 << blank) - 1)).count_ones();
                let word = if self.tiles.len() >= 8 {
                    u16::from(self.data[i])
                } else {
                    u16::from_le_bytes([self.data[2 * i], self.data[2 * i + 1]])
                };
                u64::from((word >> ord) & 1)
            }
            BitKind::Cells | BitKind::CellsFull => {
                let word = u16::from_le_bytes([self.data[2 * i], self.data[2 * i + 1]]);
                u64::from((word >> blank) & 1)
            }
            BitKind::Min2 => u64::from((self.data[i / 4] >> (2 * (i % 4))) & 3),
        }
    }

    /// Whether the table is full (every move counted).
    fn full(&self) -> bool {
        matches!(self.kind, BitKind::CellsFull | BitKind::CompactFull)
    }

    /// The value with code `code` one group move from value `parent`.
    fn decode(&self, parent: u64, code: u64) -> u64 {
        if self.kind == BitKind::Min2 {
            // parent - 3 .. parent + 3 by 2, codes counting up mod 4.
            let step = code.wrapping_sub(parent.wrapping_add(5) / 2) & 3;
            parent + 2 * step - 3
        } else if code == ((parent + 1) >> 1) & 1 {
            parent + 1
        } else {
            parent - 1
        }
    }

    /// The value after `moved` moves into the blank's old cell (`pos` and
    /// `blank` describe the position after the move), from the value
    /// `parent` before it: unchanged unless a group tile moved.
    fn step(&self, parent: u64, pos: &[u8; 16], blank: usize, moved: u8) -> u64 {
        if self.member[moved as usize] || self.full() {
            self.decode(parent, self.code(pos, blank))
        } else {
            parent
        }
    }

    /// The exact value of `board`: walk downhill to the goal.
    fn root(&self, board: &Board) -> u64 {
        let mut pos = [0u8; 16];
        for (cell, &t) in board.iter().enumerate() {
            pos[t as usize] = cell as u8;
        }
        let mut blank = pos[0] as usize;
        let parity = if self.full() {
            // Every move flips the blank's colour; its home is cell 0.
            (blank & 1) as u64
        } else {
            self.tiles
                .iter()
                .fold(0, |p, &t| p ^ u64::from((pos[t as usize] ^ t) & 1))
        };
        let start = 1024 + 2 * self.code(&pos, blank) + parity;
        let mut value = start;
        if self.full() {
            return self.root_full(&mut pos, blank, start);
        }
        loop {
            let used = self
                .tiles
                .iter()
                .fold(0u16, |m, &t| m | (1 << pos[t as usize]));
            let home = self.tiles.iter().all(|&t| pos[t as usize] == t);
            let free = !used;
            let region = if self.kind == BitKind::Min2 {
                free
            } else {
                flood_region(blank, free)
            };
            if home && (self.kind == BitKind::Min2 || region & 1 != 0) {
                return start - value;
            }
            let mut stepped = false;
            'moves: for &t in &self.tiles {
                let c = pos[t as usize] as usize;
                for &n in &tables::NEIGHBORS[c] {
                    if n < 0 || region & (1 << n) == 0 {
                        continue;
                    }
                    pos[t as usize] = n as u8;
                    let child = self.decode(value, self.code(&pos, c));
                    if child < value {
                        value = child;
                        blank = c;
                        stepped = true;
                        break 'moves;
                    }
                    pos[t as usize] = c as u8;
                }
            }
            assert!(stepped, "no lower neighbour");
        }
    }

    /// [`Self::root`] for a full table: the blank moves, carrying a group
    /// tile it meets the other way.
    fn root_full(&self, pos: &mut [u8; 16], mut blank: usize, start: u64) -> u64 {
        let mut value = start;
        let mut owner = [usize::MAX; 16];
        for (i, &t) in self.tiles.iter().enumerate() {
            owner[pos[t as usize] as usize] = i;
        }
        loop {
            if blank == 0 && self.tiles.iter().all(|&t| pos[t as usize] == t) {
                return start - value;
            }
            let mut stepped = false;
            for &n in &tables::NEIGHBORS[blank] {
                if n < 0 {
                    continue;
                }
                let n = n as usize;
                let o = owner[n];
                if o != usize::MAX {
                    pos[self.tiles[o] as usize] = blank as u8;
                }
                let child = self.decode(value, self.code(pos, n));
                if child < value {
                    if o != usize::MAX {
                        owner[blank] = o;
                        owner[n] = usize::MAX;
                    }
                    value = child;
                    blank = n;
                    stepped = true;
                    break;
                }
                if o != usize::MAX {
                    pos[self.tiles[o] as usize] = n as u8;
                }
            }
            assert!(stepped, "no lower neighbour");
        }
    }
}

/// One term of an additive group: a byte table, or bit table `n` of
/// [`Heuristic::bits`].
pub enum Comp {
    Byte(Table),
    Bit(usize),
}

/// Walking distance: vertical plus horizontal, from row/column tile-class
/// counts (Ken'ichiro Takahashi's heuristic).
pub struct Walking {
    dist: HashMap<u64, u8>,
}

fn row_col(cell: usize) -> (usize, usize) {
    let row = cell / 4;
    let col = if row % 2 == 1 { 3 - cell % 4 } else { cell % 4 };
    (row, col)
}

fn wd_key(counts: &[[u8; 4]; 4], blank_line: usize) -> u64 {
    let mut key = blank_line as u64;
    for line in counts {
        for &c in line {
            key = key * 5 + u64::from(c);
        }
    }
    key
}

impl Walking {
    #[must_use]
    pub fn new() -> Self {
        // Goal: line g holds four tiles of class g, except the blank's line
        // (line 0 for both rows and columns: internal cell 0) holds three.
        let mut goal = [[0u8; 4]; 4];
        for (g, line) in goal.iter_mut().enumerate() {
            line[g] = if g == 0 { 3 } else { 4 };
        }
        let mut dist = HashMap::new();
        dist.insert(wd_key(&goal, 0), 0u8);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((goal, 0usize, 0u8));
        while let Some((counts, b, d)) = queue.pop_front() {
            for nb in [b.wrapping_sub(1), b + 1] {
                if nb >= 4 {
                    continue;
                }
                for g in 0..4 {
                    if counts[nb][g] == 0 {
                        continue;
                    }
                    let mut next = counts;
                    next[nb][g] -= 1;
                    next[b][g] += 1;
                    let key = wd_key(&next, nb);
                    if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(key) {
                        e.insert(d + 1);
                        queue.push_back((next, nb, d + 1));
                    }
                }
            }
        }
        Self { dist }
    }

    fn value(&self, board: &Board) -> u64 {
        let mut rows = [[0u8; 4]; 4];
        let mut cols = [[0u8; 4]; 4];
        let (mut br, mut bc) = (0, 0);
        for (cell, &t) in board.iter().enumerate() {
            let (r, c) = row_col(cell);
            if t == 0 {
                (br, bc) = (r, c);
                continue;
            }
            let (gr, gc) = row_col(t as usize);
            rows[r][gr] += 1;
            cols[c][gc] += 1;
        }
        u64::from(self.dist[&wd_key(&rows, br)]) + u64::from(self.dist[&wd_key(&cols, bc)])
    }
}

impl Default for Walking {
    fn default() -> Self {
        Self::new()
    }
}

/// The heuristic: the max over every component and orientation.
pub struct Heuristic {
    /// Additive groups: the sum over the tables of each group.
    pub additive: Vec<Vec<Comp>>,
    /// The bit tables the groups refer to.
    pub bits: Vec<BitTable>,
    /// Non-additive tables (all moves counted), max'ed.
    pub full: Vec<Comp>,
    pub walking: Option<Walking>,
    /// Also look up the mirrored board.
    pub mirror: bool,
    /// Also look up the dual of the board after walking the blank home.
    pub dual: bool,
    /// Additive groups used for the dual lookups (all when empty).
    pub dual_groups: Vec<usize>,
    /// Mirror the dual lookups too.
    pub dual_mirror: bool,
    /// Walk the blank home both ways (column first, row first), take the max.
    pub dual_both: bool,
    /// Tie-break: 0 = sum of the additive groups, 1 = also the full
    /// tables, 2 = none, 3 = full tables only, 4 = additive + 2 x full.
    pub key: u8,
}

impl Heuristic {
    /// (max, sum of the additive group values) for one board in
    /// orientation `o`, whose bit-table values are `vals[n][o]`.
    fn tables(&self, board: &Board, vals: &[[u64; 2]], o: usize) -> (u64, u64) {
        let mut pos = [0u8; 16];
        for (cell, &t) in board.iter().enumerate() {
            pos[t as usize] = cell as u8;
        }
        let blank = pos[0] as usize;
        let mut best = 0;
        let mut sum = 0;
        for group in &self.additive {
            let v: u64 = group
                .iter()
                .map(|c| match c {
                    Comp::Byte(t) => t.lookup(&pos, blank),
                    Comp::Bit(n) => vals[*n][o],
                })
                .sum();
            best = best.max(v);
            if self.key != 3 {
                sum += v;
            }
        }
        for c in &self.full {
            let v = match c {
                Comp::Byte(t) => t.lookup(&pos, blank),
                Comp::Bit(n) => vals[*n][o],
            };
            best = best.max(v);
            match self.key {
                1 | 3 => sum += v,
                4 => sum += 2 * v,
                _ => {}
            }
        }
        if self.key == 2 {
            sum = 0;
        }
        (best, sum)
    }

    fn oriented(&self, board: &Board, vals: &[[u64; 2]]) -> (u64, u64) {
        let (mut best, mut sum) = self.tables(board, vals, 0);
        if self.mirror {
            let (b, s) = self.tables(&tables::reflect(board), vals, 1);
            best = best.max(b);
            sum += s;
        }
        (best, sum)
    }

    fn dual_tables(&self, board: &Board) -> u64 {
        let mut pos = [0u8; 16];
        for (cell, &t) in board.iter().enumerate() {
            pos[t as usize] = cell as u8;
        }
        let blank = pos[0] as usize;
        let mut best = 0;
        for (g, group) in self.additive.iter().enumerate() {
            if self.dual_groups.is_empty() || self.dual_groups.contains(&g) {
                let mut v = 0;
                for c in group {
                    match c {
                        Comp::Byte(t) => v += t.lookup(&pos, blank),
                        Comp::Bit(_) => panic!("dual lookups need byte tables"),
                    }
                }
                best = best.max(v);
            }
        }
        best
    }

    /// (estimate, tie-break sum), with the bit tables' values `vals`.
    #[must_use]
    pub fn eval(&self, board: &Board, vals: &[[u64; 2]]) -> (u64, u64) {
        let (mut best, sum) = self.oriented(board, vals);
        if let Some(w) = &self.walking {
            best = best.max(w.value(board));
        }
        if self.dual {
            let walks: &[bool] = if self.dual_both {
                &[false, true]
            } else {
                &[false]
            };
            for &row_first in walks {
                let (dual, cost) = dual_board(board, row_first);
                let mut d = self.dual_tables(&dual);
                if self.dual_mirror {
                    d = d.max(self.dual_tables(&tables::reflect(&dual)));
                }
                best = best.max(d.saturating_sub(cost));
            }
        }
        (best, sum)
    }

    /// Bit-table values of `board` (the position after `moved` moved), per
    /// orientation, from the parent's `parent`.
    fn child_vals(&self, parent: &[[u64; 2]], board: &Board, moved: u8) -> Vec<[u64; 2]> {
        if self.bits.is_empty() {
            return Vec::new();
        }
        let mirrored = tables::reflect(board);
        let mut pos = [[0u8; 16]; 2];
        for (o, b) in [board, &mirrored].into_iter().enumerate() {
            for (cell, &t) in b.iter().enumerate() {
                pos[o][t as usize] = cell as u8;
            }
        }
        let moved = [moved, MIRROR[moved as usize]];
        self.bits
            .iter()
            .zip(parent)
            .map(|(t, v)| [0, 1].map(|o| t.step(v[o], &pos[o], pos[o][0] as usize, moved[o])))
            .collect()
    }

    fn root_vals(&self, board: &Board) -> Vec<[u64; 2]> {
        let mirrored = tables::reflect(board);
        self.bits
            .iter()
            .map(|t| [t.root(board), t.root(&mirrored)])
            .collect()
    }
}

fn cell_at(row: usize, col: usize) -> usize {
    (0..16).find(|&x| row_col(x) == (row, col)).unwrap()
}

/// The dual (inverse permutation) of `board` after walking the blank to
/// cell 0 (up its column then along row 0, or row first), and the walk's
/// length: d(board) >= d(dual) - length.
fn dual_board(board: &Board, row_first: bool) -> (Board, u64) {
    let mut s = *board;
    let mut b = tables::blank_position(&s);
    let mut cost = 0u64;
    while b != 0 {
        let (r, c) = row_col(b);
        let target = if (row_first && c > 0) || r == 0 {
            cell_at(r, c - 1)
        } else {
            cell_at(r - 1, c)
        };
        s[b] = s[target];
        s[target] = 0;
        b = target;
        cost += 1;
    }
    let mut dual = [0u8; 16];
    for (cell, &t) in s.iter().enumerate() {
        dual[t as usize] = cell as u8;
    }
    (dual, cost)
}

/// Search statistics.
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub nodes: u64,
    pub evals: u64,
    pub bpmx_cuts: u64,
}

struct Search<'h> {
    h: &'h Heuristic,
    board: Board,
    bpmx: bool,
    stats: Stats,
    path: [u8; MAX_PATH],
    /// `vals[depth]`: the node's bit-table values, per orientation.
    vals: Vec<Vec<[u64; 2]>>,
}

impl Search<'_> {
    fn dfs(&mut self, blank: usize, node: u32, depth: u64) -> bool {
        self.stats.nodes += 1;
        if depth == 0 {
            // h == 0 only implies the goal when the tables cover every tile.
            return self.board.iter().enumerate().all(|(c, &t)| c == t as usize);
        }
        let state = (node >> 4) as usize;
        let list = &tables::MOVES[blank][(node & 15) as usize];
        let mut cand = [(0u64, 0usize, 0u32); 4];
        let mut cand_vals: [Vec<[u64; 2]>; 4] = Default::default();
        let mut n = 0;
        for m in &list.moves[..list.n as usize] {
            let next = m.next[0] as usize;
            let tile = self.board[next];
            self.board[blank] = tile;
            self.board[next] = 0;
            let vals = self
                .h
                .child_vals(&self.vals[depth as usize], &self.board, tile);
            let (h, sum) = self.h.eval(&self.board, &vals);
            self.board[next] = tile;
            self.board[blank] = 0;
            self.stats.evals += 1;
            if self.bpmx && h >= depth + 2 {
                self.stats.bpmx_cuts += 1;
                return false;
            }
            if h >= depth {
                continue;
            }
            let ns = u32::from(fsm::FSM[state][m.dir as usize]);
            let nn = (ns << 4) | u32::from(fsm::MASK_OF[(ns >> 6) as usize]);
            cand[n] = ((h << 9) + sum, next, nn);
            cand_vals[n] = vals;
            n += 1;
        }
        // Stable sort by key, like the solver's insertion sort.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&k| cand[k].0);
        for &k in &order {
            let (_, next, nn) = cand[k];
            self.vals[depth as usize - 1] = std::mem::take(&mut cand_vals[k]);
            let tile = self.board[next];
            self.board[blank] = tile;
            self.board[next] = 0;
            self.path[depth as usize] = blank as u8;
            let found = self.dfs(next, nn, depth - 1);
            self.board[next] = tile;
            self.board[blank] = 0;
            if found {
                return true;
            }
        }
        false
    }
}

/// Solve one (internal-order) board; returns the solution length.
pub fn solve(h: &Heuristic, internal: &Board, bpmx: bool, stats: &mut Stats) -> u64 {
    let blank = tables::blank_position(internal);
    if internal.iter().enumerate().all(|(c, &t)| c == t as usize) {
        return 0;
    }
    let root = h.root_vals(internal);
    let (distance, _) = h.eval(internal, &root);
    let mut threshold = 2 - (blank % 2) as u64;
    if distance > threshold {
        threshold = distance + (threshold.wrapping_sub(distance) & 1);
    }
    let mut search = Search {
        h,
        board: *internal,
        bpmx,
        stats: Stats::default(),
        path: [0; MAX_PATH],
        vals: vec![Vec::new(); MAX_PATH],
    };
    loop {
        search.vals[threshold as usize] = root.clone();
        if search.dfs(blank, fsm::START, threshold) {
            break;
        }
        threshold += 2;
    }
    stats.nodes += search.stats.nodes;
    stats.evals += search.stats.evals;
    stats.bpmx_cuts += search.stats.bpmx_cuts;
    let _ = MIRROR;
    threshold
}

/// Convert a user board to internal order.
#[must_use]
pub fn internal(board: &Board) -> Board {
    tables::to_internal(board)
}
