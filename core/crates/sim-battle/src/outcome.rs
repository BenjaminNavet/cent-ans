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
    /// The side gave the "no quarter" order (oriflamme, dragon banner): it
    /// took no prisoners. The campaign may use it later (lost ransoms).
    #[serde(default)]
    pub no_quarter: bool,
    /// The losing side left the field in good order (retreat sounded) rather
    /// than being broken: none of its regiments routed (Q2).
    #[serde(default)]
    pub withdrew: bool,
    /// EP5: enemy standards taken by this side (trophies).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standards_taken: Vec<StandardTrophy>,
    /// EP5: standards of this side taken by the enemy.
    #[serde(default)]
    pub standards_lost: u32,
    /// EP6: the enemy looted this side's camp and baggage.
    #[serde(default)]
    pub baggage_lost: bool,
}

/// A standard taken in battle (lot EP5, ADR 0034).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardTrophy {
    /// Unit type of the regiment that lost it.
    pub unit_type: String,
    /// French name of that regiment.
    pub unit_name: String,
    /// Faction id of its owner.
    pub faction: String,
    /// The general's banner.
    #[serde(default)]
    pub general: bool,
    /// French name of the regiment that took it (`None`: picked up on the
    /// field held by the victor at the end).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captor: Option<String>,
    /// Simulated seconds since the start.
    pub time: f64,
    /// Side that took it.
    pub taken_by: SideId,
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
    /// EP9: how the battle ended (a refused battle carries no rout).
    #[serde(default)]
    pub end: crate::decision::BattleEnd,
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
