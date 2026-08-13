//! A small, fast PRNG used to shuffle playback order (see
//! `LocalBackend::toggle_shuffle` and the `select_*` methods in
//! `local_backend.rs`, plus `restore_state` in `state.rs`).

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::hash::Hasher;

/// A small and fast PRNG used for shuffling playback order.
pub(crate) struct Xorshift64 {
    state: u64,
}

/// Draws a single `u64` of OS randomness by way of `RandomState`, which
/// already pull from the OS's randomness source to seed hasher
/// instances. Used only to seed `Xorshift64`.
fn os_seeded_u64() -> u64 {
    RandomState::new().build_hasher().finish()
}

impl Xorshift64 {
    /// Seeds from OS randomness via `RandomState`. Falls back to a fixed
    /// non-zero constant in the near-impossible case the seed is 0, since
    /// xorshift can't recover from a zero state.
    pub(crate) fn new() -> Self {
        let seed = os_seeded_u64();
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Advances the internal state and returns the next pseudo-random
    /// `u64`. Standard xorshift64 shift triplet (13/7/17).
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Random index in `0..bound`. Uses plan modulo, an unbiased method is
    /// not worth the extra complexity.
    fn gen_range(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    /// In-place Fisher-Yates shuffle, used at all `order`-shuffling call
    /// sites.
    pub(crate) fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range(i + 1);
            slice.swap(i, j);
        }
    }
}
