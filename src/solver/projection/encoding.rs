fn binomial(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1u64;
    for i in 0..k {
        result = result * (n - i) as u64 / (i + 1) as u64;
    }
    result
}

pub(super) fn multinomial(counts: &[u8]) -> u64 {
    let total = counts.iter().map(|&c| c as usize).sum();
    let mut r = 1u64;
    let mut rem = total;
    for &c in counts {
        if c != 0 {
            r *= binomial(rem, c as usize);
            rem -= c as usize;
        }
    }
    r
}

pub(super) fn encode_multiset(arr: &[u8], tally: &[u8]) -> u64 {
    let mut remaining = tally.to_vec();
    let mut t = 0;
    for &v in arr {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_multiset_simple() {
        let tally = [2, 2];
        let arr = [0, 0, 1, 1];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 0);

        let arr = [0, 1, 0, 1];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 1);

        let arr = [0, 1, 1, 0];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 2);

        let arr = [1, 0, 0, 1];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 3);

        let arr = [1, 0, 1, 0];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 4);

        let arr = [1, 1, 0, 0];
        let idx = encode_multiset(&arr, &tally);
        assert_eq!(idx, 5);
    }

    #[test]
    fn test_pdb_size() {
        assert_eq!(multinomial(&[2, 2, 1]), 30);
        assert_eq!(multinomial(&[4, 4, 4, 3, 1]), 252_252_000);
    }
}
