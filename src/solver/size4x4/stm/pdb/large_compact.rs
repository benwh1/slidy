//! The compact large set: a 7-8 partition, 1.27 GB.

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, delegate,
        group::{
            Format::{self, Pair, Pair8},
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

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// The groups, one table each.
const GROUPS: [Group; 2] = [g(T7, 0, Pair).with_full(), g(B8, 0, Pair8).with_full()];

/// The description of the set.
pub struct Groups;

impl Spec for Groups {
    const GROUPS: &'static [Group] = &GROUPS;
    const LAYOUT: Layout = Layout {
        file: "combined-large-compact",
        parts: &parts(&GROUPS),
    };
}

type Inner = Tables<Groups, { GROUPS.len() }>;

/// The compact large set (1.27 GB): a 7-8 partition stored exactly with both groups' full
/// values, the 8-tile table holding the bits for each blank cell outside the group in a u16.
pub struct LargeCompact(Inner);

delegate!(LargeCompact);
