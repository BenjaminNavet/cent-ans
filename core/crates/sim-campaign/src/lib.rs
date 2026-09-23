//! Campaign simulation: turn-based state of the Hundred Years' War campaign.
//!
//! The simulation is pure and deterministic: `state + orders + seed -> new
//! state`. All randomness goes through the seeded [`CampaignRng`] so that the
//! same seed and the same orders always produce the same state.

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Year the campaign starts (spring 1337, Edward III's claim to the French throne).
pub const START_YEAR: i32 = 1337;
/// Number of turns per year (one turn per season).
pub const TURNS_PER_YEAR: u32 = 4;

/// One of the four seasons; one campaign turn spans one season.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    /// Seasons in turn order, starting with spring.
    pub const ALL: [Season; 4] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];

    /// The season following this one (winter wraps to spring).
    pub fn next(self) -> Season {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }

    /// French display name used by the UI.
    pub fn label_fr(self) -> &'static str {
        match self {
            Season::Spring => "Printemps",
            Season::Summer => "Été",
            Season::Autumn => "Automne",
            Season::Winter => "Hiver",
        }
    }
}

/// Deterministic random number generator for the campaign.
///
/// Wraps ChaCha8 so that the stream depends only on the seed, regardless of
/// platform or build profile.
#[derive(Debug, Clone)]
pub struct CampaignRng(ChaCha8Rng);

impl CampaignRng {
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
}

/// Full state of a campaign at a given turn.
#[derive(Debug, Clone)]
pub struct CampaignState {
    /// Zero-based turn counter; turn 0 is spring 1337.
    pub turn: u32,
    pub season: Season,
    pub year: i32,
    pub seed: u64,
    pub rng: CampaignRng,
}

impl CampaignState {
    /// Creates a fresh campaign at spring 1337 with the given RNG seed.
    pub fn new(seed: u64) -> Self {
        CampaignState {
            turn: 0,
            season: Season::Spring,
            year: START_YEAR,
            seed,
            rng: CampaignRng::from_seed(seed),
        }
    }

    /// Advances the campaign by one turn (one season).
    pub fn end_turn(&mut self) {
        self.turn += 1;
        if self.season == Season::Winter {
            self.year += 1;
        }
        self.season = self.season.next();
    }

    /// Human-readable date in French, e.g. `"Printemps 1337"`.
    pub fn date_label(&self) -> String {
        format!("{} {}", self.season.label_fr(), self.year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_starts_in_spring_1337() {
        let state = CampaignState::new(1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.season, Season::Spring);
        assert_eq!(state.year, 1337);
        assert_eq!(state.date_label(), "Printemps 1337");
    }

    #[test]
    fn end_turn_cycles_seasons_and_advances_year() {
        let mut state = CampaignState::new(1);
        let expected = [
            (1, "Été 1337"),
            (2, "Automne 1337"),
            (3, "Hiver 1337"),
            (4, "Printemps 1338"),
            (5, "Été 1338"),
        ];
        for (turn, label) in expected {
            state.end_turn();
            assert_eq!(state.turn, turn);
            assert_eq!(state.date_label(), label);
        }
    }

    #[test]
    fn ten_turns_reach_autumn_1339() {
        let mut state = CampaignState::new(1);
        for _ in 0..10 {
            state.end_turn();
        }
        assert_eq!(state.turn, 10);
        assert_eq!(state.date_label(), "Automne 1339");
    }

    #[test]
    fn season_and_year_follow_turn_counter() {
        let mut state = CampaignState::new(1);
        for _ in 0..40 {
            state.end_turn();
            let expected_season = Season::ALL[(state.turn % TURNS_PER_YEAR) as usize];
            let expected_year = START_YEAR + (state.turn / TURNS_PER_YEAR) as i32;
            assert_eq!(state.season, expected_season);
            assert_eq!(state.year, expected_year);
        }
    }

    #[test]
    fn rng_is_deterministic_for_same_seed() {
        let mut first = CampaignRng::from_seed(42);
        let mut second = CampaignRng::from_seed(42);
        let draws_first: Vec<u64> = (0..100).map(|_| first.next_u64()).collect();
        let draws_second: Vec<u64> = (0..100).map(|_| second.next_u64()).collect();
        assert_eq!(draws_first, draws_second);
    }

    #[test]
    fn rng_differs_for_different_seeds() {
        let mut first = CampaignRng::from_seed(42);
        let mut second = CampaignRng::from_seed(43);
        let draws_first: Vec<u64> = (0..16).map(|_| first.next_u64()).collect();
        let draws_second: Vec<u64> = (0..16).map(|_| second.next_u64()).collect();
        assert_ne!(draws_first, draws_second);
    }

    #[test]
    fn rng_below_stays_in_range() {
        let mut rng = CampaignRng::from_seed(7);
        for _ in 0..1000 {
            assert!(rng.below(6) < 6);
        }
    }
}
