//! The huge set: 9-6 and 6-9 partitions, everything exact with full values, 17.0 GB.

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, delegate,
        group::{
            Format::{self, Bytes16, Pair8},
            Group, Spec,
        },
        region::Layout,
        tables::Tables,
    },
    utils::{Board, Move},
};

/// Tiles 1-6.
const L6: &[u8] = &[1, 2, 3, 4, 5, 6];
/// Tiles 7-15.
const R9: &[u8] = &[7, 8, 9, 10, 11, 12, 13, 14, 15];
/// Top two rows and the next row's first two cells, but the blank's home.
const T9: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
/// The rest.
const R6: &[u8] = &[10, 11, 12, 13, 14, 15];

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// The groups, one table each.
const GROUPS: [Group; 4] = [
    g(R6, 0, Bytes16).with_full(),
    g(L6, 1, Bytes16).with_full(),
    g(T9, 0, Pair8).with_full(),
    g(R9, 1, Pair8).with_full(),
];

/// The description of the set.
pub struct Groups;

impl Spec for Groups {
    const GROUPS: &'static [Group] = &GROUPS;
    // solver3's file names, so its tables load as they are.
    const LAYOUT: Layout = Layout {
        file: "combined-x9669",
        parts: &[
            ("p1015w16", GROUPS[0].bytes()),
            ("p16w16", GROUPS[1].bytes()),
            ("p19pair", GROUPS[2].bytes()),
            ("p715pair", GROUPS[3].bytes()),
        ],
    };
}

type Inner = Tables<Groups, { GROUPS.len() }>;

/// The huge set (17.0 GB): the maximum of a 9-6 and a 6-9 partition, every table exact with its
/// group's full value. The 9-tile tables hold the additive and full bits for each blank cell
/// outside the group in a u16, the 6-tile ones plain bytes. Too big to build in memory here:
/// load solver3's files (`p1015w16`, `p16w16`, `p19pair`, `p715pair`).
pub struct Huge(Inner);

delegate!(Huge);
