//! The medium set: a 7-7-1 partition as 2-bit minimum codes, and an exact 6-6-3 one, 75 MB.

use crate::solver::size4x4::stm::{
    pdb::{
        Pdb, delegate,
        group::{
            Format::{self, Bytes8, Min2, Pair},
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
/// Top two rows but the blank's home.
const T7: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
/// Bottom two rows but the far corner.
const B7: &[u8] = &[8, 9, 10, 11, 13, 14, 15];

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// The groups, one table each.
const GROUPS: [Group; 6] = [
    g(T7, 0, Min2),
    g(B7, 0, Min2),
    g(&[12], 0, Bytes8),
    g(A3, 1, Bytes8),
    g(B6, 1, Pair).with_full(),
    g(C6, 1, Pair).with_full(),
];

/// The description of the set.
pub struct Groups;

impl Spec for Groups {
    const GROUPS: &'static [Group] = &GROUPS;
    const LAYOUT: Layout = Layout {
        file: "combined-medium",
        parts: &parts(&GROUPS),
    };
}

type Inner = Tables<Groups, { GROUPS.len() }>;

/// The medium set (75 MB): the maximum of a 7-7-1 partition, its 7-tile groups as 2-bit codes
/// of their values minimised over the blank's cells, and a 6-6-3 partition stored exactly, with
/// the 6-tile groups' full values. Builds in a minute or two.
pub struct Medium(Inner);

delegate!(Medium);

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "builds 7-tile tables: over a minute"]
    fn solves_optimally() {
        super::super::tests::check(super::Medium::build());
    }
}
