//! Deterministic random number generator for the campaign.
//!
//! Wraps ChaCha8 so that the stream depends only on the seed, regardless of
//! platform or build profile. The generator is serialised with the rest of the
//! state so that a loaded game continues the exact same random stream.

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Seeded, serialisable random number generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignRng(ChaCha8Rng);

impl CampaignRng {
    /// Creates a generator from a 64-bit seed.
    pub fn from_seed(seed: u64) -> Self {
        CampaignRng(ChaCha8Rng::seed_from_u64(seed))
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// Uniform integer in `0..bound` (`bound` must be > 0).
    pub fn below(&mut self, bound: u32) -> u32 {
        debug_assert!(bound > 0, "bound must be positive");
        (self.next_u64() % u64::from(bound)) as u32
    }

    /// Uniform float in `[0, 1)`.
    pub fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `true` with probability `permille / 1000`.
    pub fn chance_permille(&mut self, permille: u32) -> bool {
        self.below(1000) < permille
    }
}
