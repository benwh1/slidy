//! Helpers for pattern databases that store each state's distance from solved mod 3, as the
//! [`Mtm`] ones do. A move changes the distance by at most 1, so this is enough to recover the
//! distances.
//!
//! [`Mtm`]: crate::algorithm::metric::Mtm

/// Returns the entry of the neighbors of a state with entry `entry` that are one move closer to
/// solved.
pub(super) const fn closer_entry(entry: u64) -> u64 {
    (entry + 2) % 3
}

/// Returns the distance from solved of a neighbor with entry `entry` of a state at distance
/// `distance`.
#[inline(always)]
pub(super) fn neighbor_distance(distance: u64, entry: u64) -> u64 {
    // The neighbor's distance is `distance + k - 1` for some `k` in `0..3`, and these have
    // different entries. `STEP[entry + 3 - distance % 3]` is `k`.
    const STEP: [u64; 8] = [1, 2, 0, 1, 2, 0, 1, 2];

    (distance + STEP[(entry + 3 - distance % 3) as usize % 8]).wrapping_sub(1)
}

/// Returns the distance from solved of `state`, which has entry `entry`, by following moves that
/// get closer to solved until `is_solved` is true. `closer(state, entry)` returns a neighbor of
/// `state` with entry `entry`, which is closer to solved.
#[inline(always)]
pub(super) fn walk_distance<T>(
    mut state: T,
    mut entry: u64,
    is_solved: impl Fn(&T) -> bool,
    mut closer: impl FnMut(&T, u64) -> T,
) -> u64 {
    let mut distance = 0;
    while !is_solved(&state) {
        entry = closer_entry(entry);
        state = closer(&state, entry);
        distance += 1;
    }
    distance
}
