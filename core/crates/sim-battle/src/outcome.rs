//! Battle result handed back to the campaign, and the battle journal.

use serde::{Deserialize, Serialize};

use crate::setup::SideId;

/// What happened to one side (indices follow [`crate::SideSetup::units`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideResult {
    /// Soldiers lost per campaign unit.
    pub losses: Vec<u32>,
    pub total_losses: u32,
    /// Campaign morale change applied to every surviving unit.
    pub morale_delta: i32,
    pub routed: bool,
    #[serde(default)]
    pub general_killed: bool,
    #[serde(default)]
    pub general_captured: bool,
}

/// Result of a battle, accepted by `CampaignState::resolve_pending_battle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleOutcome {
    pub winner: SideId,
    pub attacker: SideResult,
    pub defender: SideResult,
    /// Simulated seconds.
    #[serde(default)]
    pub duration: f64,
}

impl BattleOutcome {
    pub fn side(&self, side: SideId) -> &SideResult {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }
}

/// One line of the battle journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleEvent {
    /// Simulated seconds since the start.
    pub time: f64,
    pub text_fr: String,
    #[serde(default)]
    pub side: Option<SideId>,
}
