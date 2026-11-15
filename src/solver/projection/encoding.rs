use crate::solver::{indexing, projection::LARGE};

pub(super) fn encode<const N: usize>(arr: &[u8; N], tally: &[u8]) -> u64 {
    let n = tally.iter().map(|&t| t as usize).sum::<usize>();
    debug_assert!(n <= N);

    // Need `n < LARGE` because `encode_impl` uses an array of length `LARGE` where an array of
    // length `N + 1` is needed, because const generic operations aren't currently supported. So
    // `N + 1 <= LARGE` is required.
    debug_assert!(n < LARGE);

    let k = tally.len();
    let weight = weights(tally);

    // `rank[v]` = combinadic rank of the value-v positions, measured in the coordinate system that
    // compresses out the slots taken by smaller values. Processing positions left to right, each
    // position with value v contributes C(pos - less, occ[v] + 1), where `less` is the number of
    // earlier positions holding a smaller value and `occ[v]` the number of earlier value-v
    // positions. `less` is tracked with a Fenwick tree over prefix value counts.
    let mut rank = [0u64; N];
    let mut occ = [0u8; N];

    // Should actually be length N + 1, but const generic operations aren't supported yet.
    let mut bit = [0u8; LARGE];

    for (pos, &value) in arr.iter().take(n).enumerate() {
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

/// Returns the place value in [`encode`] of the rank of each value's positions: the product over
/// larger values `u` of `C(m[u], tally[u])`, where `m[u]` is the number of positions holding
/// values of at least `u`. The highest value is the least significant digit.
pub(super) fn weights(tally: &[u8]) -> [u64; LARGE] {
    let mut weight = [0; LARGE];
    let mut prod = 1;
    let mut m = 0;
    for (v, &t) in tally.iter().enumerate().rev() {
        weight[v] = prod;
        m += t as usize;
        prod *= indexing::BINOMIAL[m][t as usize];
    }
    weight
}

/// Returns the number of cells before `cell` holding values less than `value`, and the number
/// holding `value`.
#[inline(always)]
fn counts_before<const N: usize>(pieces: &[u8; N], cell: usize, value: u8) -> (usize, usize) {
    // Count over the whole array so that the loop has a fixed length and is vectorized.
    let mut less = 0u8;
    let mut equal = 0u8;
    for (i, &v) in pieces.iter().enumerate() {
        let before = i < cell;
        less += u8::from(before && v < value);
        equal += u8::from(before && v == value);
    }
    (less as usize, equal as usize)
}

/// Returns the change (mod 2^64) in [`encode`] of `pieces` when the gap at `cell` swaps with the
/// piece below it, at `cell + width`, where `weights` are the [`weights`] of the tally.
///
/// [`encode`] is a sum over cells of `weights[v] * C(cell - less, equal + 1)`, where `v` is the
/// value in the cell and `less` and `equal` are the numbers of earlier cells holding smaller
/// values and `v`. Only the two swapped cells and the cells between them holding values at most
/// that of the moved piece change their terms.
#[inline(always)]
pub(super) fn vertical_delta<const N: usize>(
    pieces: &[u8; N],
    cell: usize,
    width: usize,
    weights: &[u64; LARGE],
) -> u64 {
    // Cells are less than `N` and values are less than `LARGE`, which are powers of two, so the
    // masks don't change the indices.
    let below = cell + width;
    let x = pieces[below % N];
    let wx = weights[x as usize % LARGE];

    // The gap is the only cell with value 0, so its term is its position.
    let mut delta = weights[0].wrapping_mul(width as u64);

    // The moved piece's term after the move. The cells before `cell` are the same before and
    // after the move.
    let (less, equal) = counts_before(pieces, cell, x);
    delta = delta.wrapping_add(wx.wrapping_mul(indexing::BINOMIAL[cell - less][equal + 1]));

    // The numbers of cells before `c` holding values less than `x` and equal to `x`, before the
    // move, starting after the gap.
    let (mut less_x, mut equal_x) = (less + 1, equal);

    // Cells between with smaller values have the gap moved from before them to after them, so
    // they have one fewer smaller value before them. Cells between with value `x` also have the
    // moved piece moved to before them.
    for c in cell + 1..below {
        let u = pieces[c % N];
        if u == x {
            let pos = c - less_x;
            let new = indexing::BINOMIAL[pos + 1][equal_x + 2];
            let old = indexing::BINOMIAL[pos][equal_x + 1];
            delta = delta.wrapping_add(wx.wrapping_mul(new.wrapping_sub(old)));
            equal_x += 1;
        } else if u < x {
            let (less, equal) = counts_before(pieces, c, u);
            let pos = c - less;
            let new = indexing::BINOMIAL[pos + 1][equal + 1];
            let old = indexing::BINOMIAL[pos][equal + 1];
            delta =
                delta.wrapping_add(weights[u as usize % LARGE].wrapping_mul(new.wrapping_sub(old)));
            less_x += 1;
        }
    }

    // The moved piece's term before the move.
    delta.wrapping_sub(wx.wrapping_mul(indexing::BINOMIAL[below - less_x][equal_x + 1]))
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng as _, seq::SliceRandom as _};
    use rand_xoshiro::Xoroshiro128StarStar;

    use super::*;
    use crate::solver::projection::LARGE;

    fn enumerate(
        counts: &mut [u8],
        tally: &[u8],
        arr: &mut [u8; LARGE],
        depth: usize,
        out: &mut Vec<u64>,
    ) {
        let n = tally.iter().map(|&t| t as usize).sum::<usize>();
        if depth == n {
            out.push(encode::<LARGE>(arr, tally));
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

    fn check_bijection(tally: &[u8]) {
        let mult = indexing::multinomial(tally);
        let mut vals = Vec::with_capacity(mult as usize);
        let mut counts = tally.to_vec();
        let mut arr = [0u8; LARGE];
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
        let mut arr = [0u8; LARGE];

        arr[..4].copy_from_slice(&[0, 0, 1, 1]);
        let tally = [2, 2];
        assert_eq!(encode::<LARGE>(&arr, &tally), 0);

        arr[..4].copy_from_slice(&[0, 1, 0, 1]);
        assert_eq!(encode::<LARGE>(&arr, &tally), 1);

        arr[..4].copy_from_slice(&[1, 0, 0, 1]);
        assert_eq!(encode::<LARGE>(&arr, &tally), 2);

        arr[..4].copy_from_slice(&[0, 1, 1, 0]);
        assert_eq!(encode::<LARGE>(&arr, &tally), 3);

        arr[..4].copy_from_slice(&[1, 0, 1, 0]);
        assert_eq!(encode::<LARGE>(&arr, &tally), 4);

        arr[..4].copy_from_slice(&[1, 1, 0, 0]);
        assert_eq!(encode::<LARGE>(&arr, &tally), 5);
    }

    #[test]
    fn test_encode_is_bijection() {
        check_bijection(&[2, 2]);
        check_bijection(&[2, 2, 1]);
        check_bijection(&[3, 2]);
        check_bijection(&[1, 1, 1, 1]);
        check_bijection(&[2, 1, 1, 1]);
        check_bijection(&[3, 3]);
        check_bijection(&[2, 2, 2]);
        check_bijection(&[4, 2, 1]);
        check_bijection(&[2, 2, 1, 1]);
        check_bijection(&[3, 2, 2]);
        check_bijection(&[2, 2, 2, 1]);
        check_bijection(&[4, 4]);
    }

    /// Checks [`vertical_delta`] and that moving the gap right adds `weights[0]` against
    /// [`encode`] for random states, on a puzzle of the given width with `tally[v]` pieces of
    /// value `v` (`tally[0]` must be 1).
    fn check_deltas<const N: usize>(width: usize, tally: &[u8], rng: &mut impl rand::Rng) {
        let weights = weights(tally);
        let mut values: Vec<u8> = (0..tally.len() as u8)
            .flat_map(|v| std::iter::repeat_n(v, tally[v as usize] as usize))
            .collect();
        let n = values.len();
        assert!(n.is_multiple_of(width));

        for _ in 0..200 {
            values.shuffle(rng);
            let mut pieces = [0; N];
            pieces[..n].copy_from_slice(&values);
            let gap = pieces.iter().position(|&v| v == 0).unwrap();

            if gap % width + 1 < width {
                let mut moved = pieces;
                moved.swap(gap, gap + 1);
                assert_eq!(
                    encode(&pieces, tally).wrapping_add(weights[0]),
                    encode(&moved, tally),
                    "pieces {pieces:?}, width {width}"
                );
            }

            if gap + width >= n {
                continue;
            }

            let mut moved = pieces;
            moved.swap(gap, gap + width);
            let delta = vertical_delta(&pieces, gap, width, &weights);
            assert_eq!(
                encode(&pieces, tally).wrapping_add(delta),
                encode(&moved, tally),
                "pieces {pieces:?}, width {width}"
            );
        }
    }

    #[test]
    fn test_deltas() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
        for (width, tally) in [
            (2, &[1, 1, 1, 1][..]),
            (3, &[1, 2, 3, 2, 1]),
            (3, &[1, 1, 1, 1, 1, 1, 1, 1, 1]),
            (4, &[1, 7, 8]),
            (4, &[1, 7, 5, 3]),
            (4, &[1, 3, 4, 4, 4]),
            (4, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
            (1, &[1, 2, 1]),
            (5, &[1, 4, 4, 4, 2]),
        ] {
            check_deltas::<16>(width, tally, &mut rng);
            check_deltas::<LARGE>(width, tally, &mut rng);
        }

        for (width, tally) in [
            (5, &[1, 9, 7, 5, 3][..]),
            (6, &[1, 5, 6, 6, 6, 6]),
            (5, &[1; 25][..]),
            (4, &[1, 3, 4, 4, 4, 4, 4, 4]),
        ] {
            check_deltas::<LARGE>(width, tally, &mut rng);
        }
    }
}
