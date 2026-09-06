use crate::solver::indexing;

pub(super) fn encode<const N: usize>(arr: &[u8; N], tally: &[u8]) -> u64 {
    let mut remaining = [0; N];
    remaining[..tally.len()].copy_from_slice(tally);
    let mut total = N;
    let mut mult = indexing::multinomial(tally);
    let mut encoded = 0;

    for &value in arr {
        let cur = value as usize;
        for &c in &remaining[..cur] {
            if c > 0 {
                encoded += mult * c as u64 / total as u64;
            }
        }
        let c = remaining[cur];
        mult = mult * c as u64 / total as u64;
        remaining[cur] -= 1;
        total -= 1;
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_multiset_simple() {
        let tally = [2, 2];
        let arr = [0, 0, 1, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 0);

        let arr = [0, 1, 0, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 1);

        let arr = [0, 1, 1, 0];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 2);

        let arr = [1, 0, 0, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 3);

        let arr = [1, 0, 1, 0];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 4);

        let arr = [1, 1, 0, 0];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 5);
    }

    #[test]
    fn test_pdb_size() {
        assert_eq!(indexing::multinomial(&[2, 2, 1]), 30);
        assert_eq!(indexing::multinomial(&[4, 4, 4, 3, 1]), 252_252_000);
    }
}
