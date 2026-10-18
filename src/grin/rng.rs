//! The game's own random generator, one per round and owned by it, so the
//! same seed always gives the same boards and ammunition (never the global
//! `macroquad::rand`). Cosmetic randomness uses `emotion::hash01` instead,
//! so a blink can never change gameplay.

/// xorshift64* (Vigna). Version 1: changing the algorithm changes every
/// seeded board, so bump the version and update fixtures when doing so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

#[allow(dead_code)] // Documents the algorithm version for fixtures.
pub const RNG_VERSION: u32 = 1;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Mix the seed so small seeds do not start in a weak state; never 0.
        Rng(super::emotion::splitmix(seed) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n` (n > 0), without modulo bias that matters here.
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers_and_seeds_differ() {
        let a: Vec<u64> = (0..5)
            .map({
                let mut r = Rng::new(7);
                move |_| r.next_u64()
            })
            .collect();
        let b: Vec<u64> = (0..5)
            .map({
                let mut r = Rng::new(7);
                move |_| r.next_u64()
            })
            .collect();
        assert_eq!(a, b);
        assert_ne!(Rng::new(7).next_u64(), Rng::new(8).next_u64());
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut r = Rng::new(1);
        let mut seen = [0u32; 6];
        for _ in 0..6000 {
            let v = r.below(6);
            seen[v as usize] += 1;
        }
        assert!(seen.iter().all(|&n| n > 800), "{seen:?}");
    }
}
