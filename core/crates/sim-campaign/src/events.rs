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
    /// A character is born (generated or historical, spec § 2).
    Birth,
    /// A regency is opened for a minor ruler (spec § 2).
    Regency,
    /// A character acquires a trait through an in-game event (spec § 2).
    TraitAcquired,
    // ----- M5 ---------------------------------------------------------------
    Marriage,
    WarDeclared,
    PeaceSigned,
    AllianceFormed,
    AllianceBroken,
    Vassalage,
    VassalRebellion,
    Embargo,
    DiplomaticOffer,
    /// Other diplomatic news (gifts, claims...).
    Diplomacy,
    Excommunication,
    Schism,
    Heresy,
    // ----- M10: campaign outcome -------------------------------------------
    Victory,
    Defeat,
    CampaignEnded,
    /// A faction completes a technology (M6).
    TechnologyResearched,
    /// Historical or random chronicle event (M10).
    Chronicle,
    /// H3 « La Table »: diet fallback, Lent.
    Table,
    /// H4: tended wounded, epidemic contained.
    Medicine,
    /// H5: coinage changed, seigniorage, recoinage, inflation.
    Coinage,
    /// H6: ransom set, paid, installment due or missed, parole.
    Ransom,
    /// H6: chivalric order founded, members named, order broken.
    Chivalry,
    /// C6: spies, heralds and preachers (actions, captures).
    Agent,
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

/// French preposition "de" with elision before a vowel: `de("Île-de-France")`
/// gives "d'Île-de-France", `de("Guyenne")` gives "de Guyenne". A leading "h"
/// is treated as aspirated ("de Hainaut"), as for most place names of the
/// period.
pub fn de(name: &str) -> String {
    let starts_with_vowel = name
        .chars()
        .next()
        .is_some_and(|c| "AEIOUYÉÈÊÂÎÔaeiouyéèêâîô".contains(c));
    if starts_with_vowel {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

#[cfg(test)]
mod elision_tests {
    use super::de;

    #[test]
    fn elides_before_vowels_only() {
        assert_eq!(de("Île-de-France"), "d'Île-de-France");
        assert_eq!(de("Artois"), "d'Artois");
        assert_eq!(de("Guyenne"), "de Guyenne");
        assert_eq!(de("Hainaut"), "de Hainaut");
    }
}
