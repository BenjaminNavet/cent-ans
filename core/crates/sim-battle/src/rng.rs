//! Tiny deterministic RNG (SplitMix64) and a stateless hash for rendering noise.
//!
//! The battle does not need cryptographic quality, only a stream that depends
//! on the seed alone, on every platform and build profile.

use serde::{Deserialize, Serialize};

/// Seeded SplitMix64 generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleRng {
    state: u64,
}

impl BattleRng {
    pub fn from_seed(seed: u64) -> Self {
        BattleRng {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// Uniform float in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform float in `[low, high)`.
    pub fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }

    /// An independent stream derived from the current state and `salt`,
    /// without advancing `self` (lot B5: new features must not shift the
    /// draws of older battles).
    pub fn derive(&self, salt: u64) -> BattleRng {
        BattleRng {
            state: mix(self.state ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93)),
        }
    }

    /// Uniform integer in `0..bound` (`bound` > 0).
    pub fn below(&mut self, bound: u32) -> u32 {
        (self.next_u64() % u64::from(bound.max(1))) as u32
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stateless hash of two integers to `[-0.5, 0.5)`, used for the per-soldier
/// jitter of the rendering positions (never for rules).
pub fn jitter(a: u64, b: u64) -> f64 {
    let h = mix(a.wrapping_mul(0x1000_0000_01B3) ^ b.wrapping_add(0x51_7CC1_B727_220A));
    (h >> 11) as f64 / (1u64 << 53) as f64 - 0.5
}
