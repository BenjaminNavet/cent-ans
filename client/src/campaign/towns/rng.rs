//! Small deterministic generator (SplitMix64) for the town plans: same seed, same town on every
//! machine. Godot's `RandomNumberGenerator` is not reproduced (the plans are procedural, not
//! historical data).

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(u64::from(seed) ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn randf(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.randf()
    }

    /// Uniform integer in [lo, hi] (inclusive).
    pub fn int(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo + 1) as u64) as i32
    }
}

/// Stable 0..1 hash of two integers (`Hash.vec24`-like): placement jitter that must not depend
/// on the order of generation.
pub fn hash01(a: u32, b: u32) -> f32 {
    let mut r = Rng(u64::from(a) << 32 | u64::from(b));
    r.randf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            let x = a.randf();
            assert_eq!(x, b.randf());
            assert!((0.0..1.0).contains(&x));
        }
        let mut r = Rng::new(1);
        for _ in 0..200 {
            let k = r.int(2, 5);
            assert!((2..=5).contains(&k));
        }
    }
}
