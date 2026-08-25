use crate::solver::size4x4::mtm::indexing;

pub(super) const SIZE: usize = 151351200;
pub(super) const TALLY: [u8; 5] = [1, 2, 4, 5, 4];

pub(super) const BINOMIAL: [[u64; 17]; 17] = {
    let mut out = [[0u64; 17]; 17];

    let mut n = 0;
    while n < 17 {
        let mut k = 0;
        while k < 17 {
            out[n as usize][k as usize] = indexing::binomial(n, k);
            k += 1;
        }
        n += 1;
    }

    out
};
