//! Turn journal: events produced by [`crate::CampaignState::end_turn`].

use data_model::{FactionId, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::state::ArmyId;

/// Category of a campaign event (stable snake_case names used by the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Battle,
    SiegeStarted,
    SiegeLifted,
    ProvinceCaptured,
    Raid,
    Income,
    Bankruptcy,
    Attrition,
    Recruited,
    ArmyDestroyed,
    Death,
    Succession,
    NoHeir,
    FactionDestroyed,
    GeneralCaptured,
    BuildingCompleted,
    Revolt,
    Plague,
    Famine,
}

/// One entry of the turn journal, with a French summary for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEvent {
    pub kind: EventKind,
    pub text_fr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub army: Option<ArmyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
}

impl GameEvent {
    pub fn new(kind: EventKind, text_fr: impl Into<String>) -> Self {
        GameEvent {
            kind,
            text_fr: text_fr.into(),
            province: None,
            army: None,
            faction: None,
        }
    }

    pub fn province(mut self, province: &ProvinceId) -> Self {
        self.province = Some(province.clone());
        self
    }

    pub fn army(mut self, army: &ArmyId) -> Self {
        self.army = Some(army.clone());
        self
    }

    pub fn faction(mut self, faction: &FactionId) -> Self {
        self.faction = Some(faction.clone());
        self
    }
}
