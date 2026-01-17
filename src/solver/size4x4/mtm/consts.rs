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

pub(super) const MULTINOMIAL: [[[[[u64; 5]; 6]; 5]; 3]; 2] = {
    let mut out = [[[[[0; 5]; 6]; 5]; 3]; 2];

    let mut a0 = 0;
    while a0 < 2 {
        let mut a1 = 0;
        while a1 < 3 {
            let mut a2 = 0;
            while a2 < 5 {
                let mut a3 = 0;
                while a3 < 6 {
                    let mut a4 = 0;
                    while a4 < 5 {
                        out[a0 as usize][a1 as usize][a2 as usize][a3 as usize][a4 as usize] =
                            indexing::multinomial(&[a0, a1, a2, a3, a4]);
                        a4 += 1;
                    }
                    a3 += 1;
                }
                a2 += 1;
            }
            a1 += 1;
        }
        a0 += 1;
    }

    out
};
