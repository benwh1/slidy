# slidy

Utilities for working with sliding puzzles, specifically arbitrary-sized variants of the
[15 puzzle](https://en.wikipedia.org/wiki/15_puzzle).

## Features

- Supports arbitrary-sized `WxH` sliding puzzles with the `SlidingPuzzle` trait, implemented by a general `Puzzle` type, and `Puzzle<W, H>` for efficient puzzles of small fixed size
- Represent sequences of moves with the `Algorithm` and `AlgorithmSlice` types
- Puzzle scrambling with the `Scrambler` trait, implemented by several scrambling algorithms (`RandomState`, `RandomMoves`, etc.)
- Solvability checking with the `SolvedState` trait
- Arbitrary puzzle color schemes with the `Label`, `Coloring`, and `ColorScheme` traits
- SVG rendering of puzzles
- Efficient optimal solvers for small puzzles in single-tile and multi-tile metrics

## Crate features

| Feature      | Default | Description                                |
| ------------ | ------- | ------------------------------------------ |
| `palette`    | no      | Color schemes, SVG rendering               |
| `serde`      | no      | `Serialize`/`Deserialize` on most types    |
| `thread_rng` | yes     | Scrambling using `rand`'s thread-local RNG |
