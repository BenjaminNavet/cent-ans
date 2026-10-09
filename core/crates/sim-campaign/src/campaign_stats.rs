//! WH turn: figures of the player's campaign for the end screen's report
//! (provinces at the start, at the peak and now, battles won and lost,
//! objectives, score breakdown). Nothing here changes a rule: the counters
//! only listen to the outcome of battles and to the turn.

use data_model::{FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::state::CampaignState;

/// Counters kept with the state (absent from older saves: zero, and the
/// starting figures are taken at the first turn after loading).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignStats {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_provinces: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_year: Option<i32>,
    #[serde(default)]
    pub peak_provinces: u32,
    #[serde(default)]
    pub battles_won: u32,
    #[serde(default)]
    pub battles_lost: u32,
}

/// The report of the end screen (French labels are the interface's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignReport {
    pub start_year: i32,
    pub year: i32,
    pub turns: u32,
    pub provinces_start: u32,
    pub provinces_peak: u32,
    pub provinces_end: u32,
    pub battles_won: u32,
    pub battles_lost: u32,
    pub objectives_done: u32,
    pub objectives_total: u32,
    pub prestige: i32,
    pub treasury: i64,
    /// Score parts as `campaign_score` weighs them (`score_other` holds
    /// whatever the total counts beyond them, so the parts always add up).
    pub score_provinces: i64,
    pub score_objectives: i64,
    pub score_prestige: i64,
    pub score_treasury: i64,
    pub score_other: i64,
    pub score: i64,
}

/// A battle between the player and anyone: counted for the report.
pub(crate) fn note_battle(state: &mut CampaignState, winner: &FactionId, loser: &FactionId) {
    let player = state.player_faction.clone();
    if winner == &player {
        state.stats.battles_won += 1;
    } else if loser == &player {
        state.stats.battles_lost += 1;
    }
}

/// Start-of-season step: first figures, then the peak of provinces.
pub(crate) fn update(state: &mut CampaignState) {
    let player = state.player_faction.clone();
    let provinces = state.controlled_provinces(&player).count() as u32;
    if state.stats.start_provinces.is_none() {
        state.stats.start_provinces = Some(provinces);
        state.stats.start_year = Some(state.year);
    }
    state.stats.peak_provinces = state.stats.peak_provinces.max(provinces);
}

impl CampaignState {
    /// Figures of the player's campaign so far.
    pub fn campaign_report(&self, data: &GameData) -> CampaignReport {
        let player = &self.player_faction;
        let provinces_end = self.controlled_provinces(player).count() as u32;
        let objectives = self.objectives(data, player);
        let objectives_done = objectives.iter().filter(|o| o.done).count() as u32;
        let prestige = self
            .factions
            .get(player)
            .and_then(|f| f.ruler.as_ref())
            .and_then(|r| self.characters.get(r))
            .map_or(0, |c| c.prestige);
        let treasury = self.factions.get(player).map_or(0, |f| f.treasury);
        let score = self
            .outcome
            .as_ref()
            .map_or_else(|| self.campaign_score(data, player), |o| o.score);
        let score_provinces = i64::from(provinces_end) * 10;
        let score_objectives = i64::from(objectives_done) * 100;
        let score_prestige = i64::from(prestige);
        let score_treasury = treasury.max(0) / 2000;
        let parts = score_provinces + score_objectives + score_prestige + score_treasury;
        CampaignReport {
            start_year: self.stats.start_year.unwrap_or(self.year),
            year: self.year,
            turns: self.turn,
            provinces_start: self.stats.start_provinces.unwrap_or(provinces_end),
            provinces_peak: self.stats.peak_provinces.max(provinces_end),
            provinces_end,
            battles_won: self.stats.battles_won,
            battles_lost: self.stats.battles_lost,
            objectives_done,
            objectives_total: objectives.len() as u32,
            prestige,
            treasury,
            score_provinces,
            score_objectives,
            score_prestige,
            score_treasury,
            score_other: score - parts,
            score,
        }
    }
}
