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

pub(super) const GAPS: [[u8; 4]; 16] = [
    [4, 1, 0, 0],
    [5, 2, 1, 0],
    [6, 3, 2, 1],
    [7, 3, 3, 2],
    [8, 5, 0, 4],
    [9, 6, 1, 4],
    [10, 7, 2, 5],
    [11, 7, 3, 6],
    [12, 9, 4, 8],
    [13, 10, 5, 8],
    [14, 11, 6, 9],
    [15, 11, 7, 10],
    [12, 13, 8, 12],
    [13, 14, 9, 12],
    [14, 15, 10, 13],
    [15, 15, 11, 14],
];

pub(super) const SHIFTS: [[u8; 4]; 16] = {
    let mut out = [[0; 4]; 16];

    let mut gap = 0;
    while gap < 16 {
        let mut dir = 0;
        while dir < 4 {
            let other = GAPS[gap][dir];
            out[gap][dir] = if gap as u8 == other { 0 } else { other * 4 };
            dir += 1;
        }
        gap += 1;
    }

    out
};

pub(super) const MASKS: [[[u64; 16]; 4]; 16] = {
    let mut out = [[[0; 16]; 4]; 16];

    let mut gap = 0;
    while gap < 16 {
        let mut dir = 0;
        while dir < 4 {
            let other = GAPS[gap][dir];
            if gap as u8 != other {
                let mut piece = 0;
                while piece < 16 {
                    out[gap][dir][piece] = ((piece << (other * 4)) | (piece << (gap * 4))) as u64;
                    piece += 1;
                }
            }
            dir += 1;
        }
        gap += 1;
    }

    out
};
