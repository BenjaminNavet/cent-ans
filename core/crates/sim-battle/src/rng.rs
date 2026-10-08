//! Tiny deterministic RNG (SplitMix64) and a stateless hash for rendering noise.
//!
//! The battle does not need cryptographic quality, only a stream that depends
//! on the seed alone, on every platform and build profile.

use data_model::util::{splitmix64, splitmix_mix};
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
        let out = splitmix64(self.state);
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        out
    }

    /// Uniform float in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        unit_float(self.next_u64())
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
            state: splitmix_mix(self.state ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93)),
        }
    }

    /// Uniform integer in `0..bound` (`bound` > 0).
    pub fn below(&mut self, bound: u32) -> u32 {
        (self.next_u64() % u64::from(bound.max(1))) as u32
    }
}

/// The high 53 bits of `bits` as a float in `[0, 1)`.
pub fn unit_float(bits: u64) -> f64 {
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

/// Deterministic draw in `[0, 1)` from a key and a salt, for layouts that
/// must never touch the battle's random stream (furniture, props).
pub fn hash01(key: u64, salt: u64) -> f64 {
    let z = key
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    unit_float(splitmix_mix(z))
}

/// FNV-1a, 64 bits: digests and seeds of strings and records.
pub struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Self {
        Fnv1a(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv1a {
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        self
    }

    pub fn u64(&mut self, value: u64) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// Stateless hash of two integers to `[-0.5, 0.5)`, used for the per-soldier
/// jitter of the rendering positions (never for rules).
pub fn jitter(a: u64, b: u64) -> f64 {
    let h = splitmix_mix(a.wrapping_mul(0x1000_0000_01B3) ^ b.wrapping_add(0x51_7CC1_B727_220A));
    unit_float(h) - 0.5
}
