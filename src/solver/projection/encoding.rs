use crate::solver::indexing;

pub(super) fn encode<const N: usize>(arr: &[u8; N], tally: &[u8]) -> u64 {
    debug_assert!(N <= 32);

    let k = tally.len();

    // `m[v]` = number of slots available to values >= v, i.e. N minus the tally of values < v.
    let mut m = [0u8; 32];
    let mut rem = N as u8;
    for v in 0..k {
        m[v] = rem;
        rem -= tally[v];
    }

    // `weight[v]` = product over u > v of C(m[u], tally[u]): the mixed-radix place value of the
    // ranked subset of value-v positions (least-significant digit is the highest value).
    let mut weight = [0u64; 32];
    let mut prod = 1;
    for v in (0..k).rev() {
        weight[v] = prod;
        prod *= indexing::BINOMIAL[m[v] as usize][tally[v] as usize];
    }

    // `rank[v]` = combinadic rank of the value-v positions, measured in the coordinate system that
    // compresses out the slots taken by smaller values. Processing positions left to right, each
    // position with value v contributes C(pos - less, occ[v] + 1), where `less` is the number of
    // earlier positions holding a smaller value and `occ[v]` the number of earlier value-v
    // positions. `less` is tracked with a Fenwick tree over prefix value counts.
    let mut rank = [0u64; 32];
    let mut occ = [0u8; 32];
    let mut bit = [0u8; 33];
    for (pos, &value) in arr.iter().enumerate() {
        let v = value as usize;

        let mut less = 0;
        let mut i = v;
        while i > 0 {
            less += bit[i];
            i -= i.isolate_lowest_one();
        }

        let occ_v = occ[v];
        rank[v] += indexing::BINOMIAL[pos - less as usize][occ_v as usize + 1];
        occ[v] = occ_v + 1;

        let mut j = v + 1;
        while j <= k {
            bit[j] += 1;
            j += j.isolate_lowest_one();
        }
    }

    let mut encoded = 0;
    for v in 0..k {
        let r = rank[v];
        if r != 0 {
            encoded += r * weight[v];
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enumerate<const N: usize>(
        counts: &mut [u8],
        tally: &[u8],
        arr: &mut [u8; N],
        depth: usize,
        out: &mut Vec<u64>,
    ) {
        if depth == N {
            out.push(encode(arr, tally));
            return;
        }
        for v in 0..tally.len() {
            if counts[v] > 0 {
                counts[v] -= 1;
                arr[depth] = v as u8;
                enumerate(counts, tally, arr, depth + 1, out);
                counts[v] += 1;
            }
        }
    }

    fn check_bijection<const N: usize>(tally: &[u8]) {
        let mult = indexing::multinomial(tally);
        let mut vals = Vec::with_capacity(mult as usize);
        let mut counts = tally.to_vec();
        let mut arr = [0u8; N];
        enumerate(&mut counts, tally, &mut arr, 0, &mut vals);
        vals.sort_unstable();
        assert_eq!(
            vals,
            (0..mult).collect::<Vec<u64>>(),
            "encode is not a bijection for tally = {tally:?}"
        );
    }

    #[test]
    fn test_encode_multiset_simple() {
        let tally = [2, 2];
        let arr = [0, 0, 1, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 0);

        let arr = [0, 1, 0, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 1);

        let arr = [1, 0, 0, 1];
        let idx = encode(&arr, &tally);
        assert_eq!(idx, 2);

        let arr = [0, 1, 1, 0];
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
    fn test_encode_is_bijection() {
        check_bijection::<4>(&[2, 2]);
        check_bijection::<5>(&[2, 2, 1]);
        check_bijection::<5>(&[3, 2]);
        check_bijection::<4>(&[1, 1, 1, 1]);
        check_bijection::<5>(&[2, 1, 1, 1]);
        check_bijection::<6>(&[3, 3]);
        check_bijection::<6>(&[2, 2, 2]);
        check_bijection::<7>(&[4, 2, 1]);
        check_bijection::<6>(&[2, 2, 1, 1]);
        check_bijection::<7>(&[3, 2, 2]);
        check_bijection::<7>(&[2, 2, 2, 1]);
        check_bijection::<8>(&[4, 4]);
    }

    #[test]
    fn test_pdb_size() {
        assert_eq!(indexing::multinomial(&[2, 2, 1]), 30);
        assert_eq!(indexing::multinomial(&[4, 4, 4, 3, 1]), 252_252_000);
    }
}
