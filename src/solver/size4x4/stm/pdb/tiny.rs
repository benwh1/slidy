//! The tiny set: two 6-6-3 partitions, the 6-tile groups as 2-bit minimum codes, 5.9 MB.

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, delegate,
        group::{
            Format::{self, Bytes8, Min2},
            Group, Spec, parts,
        },
        region::Layout,
        tables::Tables,
    },
    utils::{Board, Move},
};

/// Top row but the blank's home.
const A3: &[u8] = &[1, 2, 3];
/// Left half of the bottom three rows.
const B6: &[u8] = &[6, 7, 8, 9, 14, 15];
/// Right half of the bottom three rows.
const C6: &[u8] = &[4, 5, 10, 11, 12, 13];

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// The groups, one table each.
const GROUPS: [Group; 6] = [
    g(A3, 0, Bytes8),
    g(B6, 0, Min2),
    g(C6, 0, Min2),
    // The best of 120 hill-climbing evaluations against the first partition.
    g(&[3, 4, 11], 1, Bytes8),
    g(&[1, 2, 5, 6, 7, 10], 1, Min2),
    g(&[8, 9, 12, 13, 14, 15], 1, Min2),
];

/// The description of the set.
pub struct Groups;

impl Spec for Groups {
    const GROUPS: &'static [Group] = &GROUPS;
    const LAYOUT: Layout = Layout {
        file: "combined-tiny",
        parts: &parts(&GROUPS),
    };
}

type Inner = Tables<Groups, { GROUPS.len() }>;

/// The tiny set (5.9 MB): the maximum of two 6-6-3 partitions. Each 3-tile group is an exact
/// byte table and each 6-tile group a 2-bit code of its value minimised over the blank's cells.
/// Builds in seconds.
pub struct Tiny(Inner);

delegate!(Tiny);

#[cfg(test)]
mod tests {
    #[test]
    fn solves_optimally() {
        super::super::tests::check(super::Tiny::build());
    }
}
