use crate::solver::size4x4::mtm::consts::{BINOMIAL, MULTINOMIAL, TALLY};

pub(super) const fn binomial(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }

    let k = if k < n - k { k } else { n - k };

    let mut result = 1;
    let mut i = 0;
    while i < k {
        result = result * (n - i) / (i + 1);
        i += 1;
    }

    result
}

pub(super) const fn multinomial(counts: &[u8]) -> u64 {
    let mut rem = {
        let mut total = 0;
        let mut i = 0;
        while i < counts.len() {
            total += counts[i];
            i += 1;
        }
        total as usize
    };
    let mut r = 1;

    let mut i = 0;
    while i < counts.len() {
        let c = counts[i] as usize;
        if c != 0 {
            r *= BINOMIAL[rem][c];
            rem -= c;
        }
        i += 1;
    }

    r
}

pub(super) fn encode_multiset<const LEN: usize, const DISTINCT: usize>(
    arr: [u8; LEN],
    tally: [u8; DISTINCT],
) -> u64 {
    let mut remaining = tally;
    let mut t = 0;

    for &v in &arr {
        let cur = v as usize;
        for s in 0..cur {
            if remaining[s] > 0 {
                remaining[s] -= 1;
                t += multinomial(&remaining);
                remaining[s] += 1;
            }
        }
        remaining[cur] -= 1;
    }

    t
}

pub(super) fn decode_multiset_16(mut t: u64) -> [u8; 16] {
    let mut remaining = TALLY;
    let mut out = [0; 16];

    for i in 0..16 {
        for s in 0..5 {
            if remaining[s] == 0 {
                continue;
            }
            remaining[s] -= 1;

            let m = *unsafe {
                MULTINOMIAL
                    .get_unchecked(remaining[0] as usize)
                    .get_unchecked(remaining[1] as usize)
                    .get_unchecked(remaining[2] as usize)
                    .get_unchecked(remaining[3] as usize)
                    .get_unchecked(remaining[4] as usize)
            };

            if t < m {
                out[i] = s as u8;
                break;
            } else {
                t -= m;
                remaining[s] += 1;
            }
        }
    }

    out
}
