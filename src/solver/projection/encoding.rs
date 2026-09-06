use crate::solver::indexing;

pub(super) fn encode<const N: usize>(arr: &[u8; N], tally: &[u8]) -> u64 {
    let mut remaining = [0u8; N];
    remaining[..tally.len()].copy_from_slice(tally);
    let mut total = N;
    let mut mult = indexing::multinomial(tally);
    let mut encoded = 0u64;

    for &value in arr {
        let cur = value as usize;
        for &c in &remaining[..cur] {
            if c > 0 {
                let saved = mult;
                mult = mult * c as u64 / total as u64;
                encoded += mult;
                mult = saved;
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

    #[test]
    fn test_encode_assigns_all_indices_unique() {
        let tally = [2, 2, 1];

        fn visit(tally: &[u8], remaining: &mut [u8; 3], placed: &mut [u8; 5], seen: &mut [bool]) {
            if placed.iter().all(|&v| v != u8::MAX) {
                seen[encode(placed, tally) as usize] = true;
                return;
            }
            let depth = placed.iter().take_while(|&&v| v != u8::MAX).count();
            for label in 0..remaining.len() {
                if remaining[label] > 0 {
                    remaining[label] -= 1;
                    placed[depth] = label as u8;
                    visit(tally, remaining, placed, seen);
                    placed[depth] = u8::MAX;
                    remaining[label] += 1;
                }
            }
        }

        let size = indexing::multinomial(&tally);
        let mut seen = vec![false; size as usize];
        let mut remaining = tally;
        let mut placed = [u8::MAX; 5];
        visit(&tally, &mut remaining, &mut placed, &mut seen);

        assert!(seen.iter().all(|&unique| unique));
    }
}
