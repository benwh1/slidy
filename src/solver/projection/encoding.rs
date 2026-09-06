pub(super) struct Encoding {
    tally: Box<[u8]>,
    size: u64,
}

impl Encoding {
    pub(super) fn new(tally: &[u8]) -> Self {
        let total: usize = tally.iter().map(|&c| c as usize).sum();
        let mut size = 1u64;
        let mut rem = total;
        for &count in tally {
            if count == 0 {
                continue;
            }
            size *= binomial(rem, count as usize);
            rem -= count as usize;
        }
        Self {
            tally: tally.to_vec().into_boxed_slice(),
            size,
        }
    }

    pub(super) const fn size(&self) -> u64 {
        self.size
    }

    pub(super) fn encode<const N: usize>(&self, arr: &[u8; N]) -> u64 {
        let mut remaining = [0u8; N];
        remaining[..self.tally.len()].copy_from_slice(&self.tally);
        let mut total = N;
        let mut mult = self.size;
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
}

fn binomial(n: usize, k: usize) -> u64 {
    let k = k.min(n - k);
    let mut result = 1u64;
    for i in 0..k {
        result = result * (n - i) as u64 / (i + 1) as u64;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_multiset_simple() {
        let enc = Encoding::new(&[2, 2]);
        let arr = [0, 0, 1, 1];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 0);

        let arr = [0, 1, 0, 1];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 1);

        let arr = [0, 1, 1, 0];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 2);

        let arr = [1, 0, 0, 1];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 3);

        let arr = [1, 0, 1, 0];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 4);

        let arr = [1, 1, 0, 0];
        let idx = enc.encode(&arr);
        assert_eq!(idx, 5);
    }

    #[test]
    fn test_pdb_size() {
        assert_eq!(Encoding::new(&[2, 2, 1]).size(), 30);
        assert_eq!(Encoding::new(&[4, 4, 4, 3, 1]).size(), 252_252_000);
    }

    #[test]
    fn test_encode_assigns_all_indices_unique() {
        let tally = [2, 2, 1];
        let enc = Encoding::new(&tally);
        let size = enc.size();
        let mut seen = vec![false; size as usize];

        fn visit(enc: &Encoding, remaining: &mut [u8; 3], placed: &mut [u8; 5], seen: &mut [bool]) {
            if placed.iter().all(|&v| v != u8::MAX) {
                seen[enc.encode(placed) as usize] = true;
                return;
            }
            let depth = placed.iter().take_while(|&&v| v != u8::MAX).count();
            for label in 0..remaining.len() {
                if remaining[label] > 0 {
                    remaining[label] -= 1;
                    placed[depth] = label as u8;
                    visit(enc, remaining, placed, seen);
                    placed[depth] = u8::MAX;
                    remaining[label] += 1;
                }
            }
        }

        let mut remaining = tally;
        let mut placed = [u8::MAX; 5];
        visit(&enc, &mut remaining, &mut placed, &mut seen);

        assert!(seen.iter().all(|&unique| unique));
    }
}
