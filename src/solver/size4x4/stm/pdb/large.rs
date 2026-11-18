//! The large set: a 7-8 partition and a 9-6 one, 3.5 GB.

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, delegate,
        group::{
            Format::{self, Bytes16, Min2, Pair},
            Group, Spec, parts,
        },
        region::Layout,
        tables::Tables,
    },
    utils::{Board, Move},
};

/// Top two rows but the blank's home.
const T7: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
/// Bottom two rows.
const B8: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15];
/// Top two rows and the next row's first two cells, but the blank's home.
const T9: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
/// The rest.
const R6: &[u8] = &[10, 11, 12, 13, 14, 15];

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// The groups, one table each.
const GROUPS: [Group; 4] = [
    g(T7, 0, Pair).with_full(),
    g(B8, 0, Pair).with_full(),
    g(T9, 1, Min2),
    g(R6, 1, Bytes16).with_full(),
];

/// The description of the set.
pub struct Groups;

impl Spec for Groups {
    const GROUPS: &'static [Group] = &GROUPS;
    const LAYOUT: Layout = Layout {
        file: "combined-large",
        parts: &parts(&GROUPS),
    };
}

type Inner = Tables<Groups, { GROUPS.len() }>;

/// The large set (3.5 GB): the maximum of [`LargeCompact`](super::LargeCompact)'s 7-8 partition
/// (here the 8-tile table has a bit pair for every blank cell) and a 9-6 partition, the 1-9
/// table as 2-bit codes of its value minimised over the blank's cells and the 10-15 one exact
/// with its full value.
pub struct Large(Inner);

delegate!(Large);
