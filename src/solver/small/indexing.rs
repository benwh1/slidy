pub(super) fn encode<const N: usize>(perm: [u8; N]) -> u64 {
    let mut perm2 = [0; N];
    let mut i = 0;
    let mut gap = 0;
    for j in 0..N {
        if perm[j] == 0 {
            gap = j;
            continue;
        }
        perm2[i] = perm[j] - 1;
        i += 1;
    }

    let n = N - 1;

    let mut code = [0u8; N];
    let mut seen = 0u32;

    for (i, &p) in perm2.iter().enumerate().take(n).rev() {
        code[i] = (seen & ((1u32 << p) - 1)).count_ones() as u8;
        seen |= 1u32 << p;
    }

    let encoded = code
        .iter()
        .enumerate()
        .take(n)
        .fold(0, |acc, (i, &c)| acc * (n - i) as u64 + c as u64);

    (encoded / 2) * N as u64 + gap as u64
}

pub(super) fn decode<const W: usize, const N: usize>(k: u64) -> [u8; N] {
    let gap = k % N as u64;
    let gap_parity = !((N as u64 - 1 - gap) / W as u64).is_multiple_of(2);
    let width_parity = W.is_multiple_of(2);

    let n = N - 1;
    let mut k = (k / N as u64) * 2;
    let mut code = [0u8; N];
    let mut total = 0;

    for i in 0..n {
        let a = k % (i + 1) as u64;
        total += a;
        code[n - i - 1] = a as u8;
        k /= (i + 1) as u64;
    }

    let lehmer_parity = !total.is_multiple_of(2);

    // If width is even and gap is an odd number of rows up from the bottom, then there should be
    // lehmer parity (i.e. permutation should be odd)
    let should_flip = (width_parity && gap_parity) ^ lehmer_parity;

    // Calculate the second to last element based on parity
    if should_flip {
        code[n - 2] = 1 - code[n - 2];
    }

    let mut permutation = [0u8; N];
    let mut remaining = (1..N).collect::<Vec<_>>();

    for (i, &c) in code.iter().enumerate().take(n) {
        permutation[i] = remaining.remove(c as usize) as u8;
    }

    // Insert at 0 at the gap position, moving other elements to the right one step.
    // Can't use `insert` because it doesn't exist on arrays or slices, only vectors.
    for i in (gap as usize..N - 1).rev() {
        permutation[i + 1] = permutation[i];
    }
    permutation[gap as usize] = 0;

    permutation
}

#[cfg(test)]
mod tests {
    use crate::{
        puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
        solver::small::indexing,
    };

    macro_rules! test {
        ($w:literal, $h:literal) => {
            ::paste::paste! {
                #[test]
                fn [< test_decode_encode_ $w x $h >]() {
                    const W: usize = $w;
                    const N: usize = $w * $h;

                    type P = crate::puzzle::small::[< Puzzle $w x $h >];

                    let states = Size::new($w, $h).unwrap().num_states() as u64;

                    for i in 0..states {
                        let perm = indexing::decode::<W, N>(i);
                        assert_eq!(i, indexing::encode(perm));

                        let puzzle = P::try_from(perm).unwrap();
                        assert!(puzzle.is_solvable());
                    }
                }
            }
        };
    }

    test!(2, 2);
    test!(2, 3);
    test!(2, 4);
    test!(2, 5);
    test!(3, 2);
    test!(3, 3);
    test!(4, 2);
    test!(5, 2);
}
