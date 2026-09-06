pub(super) struct Encoding {
    table: Box<[u64]>,
    tally: Box<[u8]>,
    total: usize,
}

impl Encoding {
    pub(super) fn new(tally: &[u8]) -> Self {
        let total: usize = tally.iter().map(|&c| c as usize).sum();
        let rows = total + 1;
        let mut table = vec![0u64; rows * (rows + 1) / 2];
        for n in 0..=total {
            let base = n * (n + 1) / 2;
            table[base] = 1;
            table[base + n] = 1;
            for k in 1..n {
                let prev = (n - 1) * n / 2;
                table[base + k] = table[prev + k - 1] + table[prev + k];
            }
        }
        Self {
            table: table.into_boxed_slice(),
            tally: tally.to_vec().into_boxed_slice(),
            total,
        }
    }

    pub(super) fn size(&self) -> u64 {
        self.multinomial(&self.tally, self.total)
    }

    pub(super) fn encode<const N: usize>(&self, arr: &[u8; N]) -> u64 {
        let mut remaining = [0u8; N];
        remaining[..self.tally.len()].copy_from_slice(&self.tally);
        let mut total = N;
        let mut encoded = 0u64;

        for &value in arr {
            let cur = value as usize;
            for label in 0..cur {
                if remaining[label] > 0 {
                    remaining[label] -= 1;
                    encoded += self.multinomial(&remaining, total - 1);
                    remaining[label] += 1;
                }
            }
            remaining[cur] -= 1;
            total -= 1;
        }

        encoded
    }

    fn multinomial(&self, counts: &[u8], total: usize) -> u64 {
        let mut rem = total;
        let mut result = 1u64;
        for &count in counts {
            if count == 0 {
                continue;
            }
            result *= self.table[rem * (rem + 1) / 2 + count as usize];
            rem -= count as usize;
        }
        result
    }
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
}
