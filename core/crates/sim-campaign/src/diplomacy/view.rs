//! Diplomatic panel view.

use super::*;

impl CampaignState {
    /// Diplomatic view of every other living faction for `faction`.
    pub fn diplomacy_view(&self, data: &GameData, faction: &FactionId) -> Vec<DiplomacyEntry> {
        self.factions
            .iter()
            .filter(|(id, f)| *id != faction && f.alive && !id.is_rebels())
            .map(|(id, f)| {
                let (attitude, reasons) = self.attitude(data, id, faction);
                let claims = self.factions[faction]
                    .claims
                    .iter()
                    .filter(|c| match c.kind {
                        ClaimKind::Throne => c.faction.as_ref() == Some(id),
                        ClaimKind::Province => c
                            .province
                            .as_ref()
                            .is_some_and(|p| self.province_owner(p) == Some(id)),
                    })
                    .map(|c| c.text_fr.clone())
                    .collect();
                DiplomacyEntry {
                    faction: id.clone(),
                    relation: self.relation(faction, id),
                    attitude,
                    attitude_reasons: reasons,
                    truce_turns_left: self.factions[faction]
                        .truces
                        .get(id)
                        .map_or(0, |until| until.saturating_sub(self.turn)),
                    embargo_by_us: self.factions[faction].embargoes.contains(id),
                    embargo_on_us: f.embargoes.contains(faction),
                    war_score: if self.is_at_war(faction, id) {
                        self.war_score(data, faction, id)
                    } else {
                        0
                    },
                    casus_belli: self.casus_belli(data, faction, id),
                    claims,
                    loyalty: if f.suzerain.as_ref() == Some(faction) {
                        Some(f.loyalty)
                    } else {
                        None
                    },
                    trade_agreement: self.has_trade_agreement(faction, id),
                }
            })
            .collect()
    }
}

/// One line of the diplomacy panel (attitude is theirs towards us).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiplomacyEntry {
    pub faction: FactionId,
    pub relation: RelationKind,
    pub attitude: i32,
    pub attitude_reasons: Vec<(String, i32)>,
    pub truce_turns_left: u32,
    pub embargo_by_us: bool,
    pub embargo_on_us: bool,
    pub war_score: i32,
    pub casus_belli: Option<String>,
    pub claims: Vec<String>,
    pub loyalty: Option<u8>,
    /// Lot C5: a formal trade agreement is in force (erased by war, its
    /// routes suspended by an embargo — see [`CampaignState::has_trade_agreement`]).
    #[serde(default)]
    pub trade_agreement: bool,
}
