pub(super) const BINOMIAL: [[u64; 33]; 33] = {
    let mut out = [[0; 33]; 33];

    let mut n = 0;
    while n < 33 {
        let mut k = 0;
        while k < 33 {
            out[n as usize][k as usize] = binomial(n, k);
            k += 1;
        }
        n += 1;
    }

    out
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multinomial() {
        assert_eq!(multinomial(&[2, 2, 1]), 30);
        assert_eq!(multinomial(&[4, 4, 4, 3, 1]), 252_252_000);
    }
}
