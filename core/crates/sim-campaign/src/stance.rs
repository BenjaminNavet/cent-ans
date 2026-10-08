//! Lot DP2 (ADR 0075): diplomatic stance of a faction towards another, for
//! the « Diplomatie » map mode and minimap (green ally, blue agreement,
//! yellow neutral, orange tension, red war, grey vassal). Pure.

use data_model::key_enum;
use data_model::{FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::diplomacy::RelationKind;
use crate::state::CampaignState;

key_enum! {
/// How `viewer` stands with another faction, as the map shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// The viewer itself.
    Own => "self",
    /// Alliance.
    Ally => "ally",
    /// At peace with a standing agreement: trade, military access (either
    /// way) or a marriage between the houses.
    Agreement => "agreement",
    /// At peace (or truce), nothing particular.
    Neutral => "neutral",
    /// At peace, but hostile: bad attitude, embargo, trespass or grievance.
    Tension => "tension",
    War => "war",
    /// Vassal or suzerain.
    Vassal => "vassal",
}
}

impl Stance {
    /// French label (map legend, tooltips).
    pub fn label_fr(self) -> &'static str {
        match self {
            Stance::Own => "Nous",
            Stance::Ally => "Allié",
            Stance::Agreement => "Accord",
            Stance::Neutral => "Neutre",
            Stance::Tension => "Tension",
            Stance::War => "Guerre",
            Stance::Vassal => "Vassal ou suzerain",
        }
    }
}

/// Stance of `viewer` towards `other`. War first, then vassalage and
/// alliance; among factions at peace, tension (their attitude towards us at
/// or below `passage.tension_attitude`, an embargo, a trespass or a casus
/// belli for trespass either way) prevails over an agreement.
pub fn diplomatic_stance(
    state: &CampaignState,
    data: &GameData,
    viewer: &FactionId,
    other: &FactionId,
) -> Stance {
    if viewer == other {
        return Stance::Own;
    }
    match state.relation(viewer, other) {
        RelationKind::War => return Stance::War,
        RelationKind::Vassal | RelationKind::Suzerain => return Stance::Vassal,
        RelationKind::Alliance => return Stance::Ally,
        RelationKind::Peace | RelationKind::Truce => {}
    }
    let rules = &data.ai_diplomacy.passage;
    let (attitude, _) = state.attitude(data, other, viewer);
    let embargo = |a: &FactionId, b: &FactionId| {
        state
            .factions
            .get(a)
            .is_some_and(|f| f.embargoes.contains(b))
    };
    let trespass = |victim: &FactionId, intruder: &FactionId| {
        state
            .factions
            .get(victim)
            .and_then(|f| f.ledger.trespassers.get(intruder))
            .is_some_and(|t| t.seasons > 0)
            || crate::passage::has_grievance(state, victim, intruder)
    };
    if attitude <= rules.tension_attitude
        || embargo(viewer, other)
        || embargo(other, viewer)
        || trespass(viewer, other)
        || trespass(other, viewer)
    {
        return Stance::Tension;
    }
    let ledger = |a: &FactionId| state.factions.get(a).map(|f| &f.ledger);
    let agreement = ledger(viewer)
        .is_some_and(|l| l.trade_agreements.contains(other) || l.military_access.contains(other))
        || ledger(other).is_some_and(|l| {
            l.trade_agreements.contains(viewer) || l.military_access.contains(viewer)
        })
        || state.marriage_tie(viewer, other);
    if agreement {
        Stance::Agreement
    } else {
        Stance::Neutral
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<Stance>();
    }
}
