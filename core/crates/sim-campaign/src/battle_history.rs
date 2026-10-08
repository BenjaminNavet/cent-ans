//! Bounded history of the recent land battles (chantier TB, lot
//! « historique des batailles », ADR 0157 « révision »).
//!
//! The map marks a battlefield for a few turns (mound, crows, debris, lot
//! TB4). The marks used to live in the renderer's memory and were lost on a
//! reload; the campaign state now remembers where and when the land battles
//! were fought: field battles (`movement::apply_battle_result`, auto-resolved
//! or fought in 3D), assaults (`siege::apply_assault_result`) and garrison
//! sorties. Naval battles are not recorded.
//!
//! No game rule reads the history. It is bounded by
//! `data/rules/battle_history.json` ([`data_model::BattleHistoryRules`]): a
//! record is dropped once `max_age_turns` old, and at most `max_records` are
//! kept. Saved with the state; absent from older saves (empty by default).

use data_model::key_enum;
use data_model::{BattleHistoryRules, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::battle_auto::{BattleResult, Winner};
use crate::state::CampaignState;

key_enum! {
/// What kind of land battle was fought.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleKind {
    /// Two hosts in the open.
    Field => "field",
    /// A settlement stormed by its besiegers.
    Assault => "assault",
    /// A garrison falling on its besiegers.
    Sortie => "sortie",
}
}

/// One side of a recorded battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSideRecord {
    /// Faction of the side's lead army (or of the garrison).
    pub faction: FactionId,
    /// Men engaged (allied armies included).
    pub strength: u32,
    /// Men lost.
    pub losses: u32,
}

/// A land battle of the recent past.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleRecord {
    /// Turn the battle was fought.
    pub turn: u32,
    pub province: ProvinceId,
    /// Map-pixel point of the battle: the defender's for a field battle,
    /// the besiegers' camp for an assault or a sortie.
    pub position: [f32; 2],
    pub kind: BattleKind,
    pub attacker: BattleSideRecord,
    pub defender: BattleSideRecord,
    pub attacker_won: bool,
}

impl BattleRecord {
    /// Faction of the winning side.
    pub fn winner(&self) -> &FactionId {
        if self.attacker_won {
            &self.attacker.faction
        } else {
            &self.defender.faction
        }
    }

    /// Turns elapsed since the battle at `turn` (0 the turn it was fought).
    pub fn age(&self, turn: u32) -> u32 {
        turn.saturating_sub(self.turn)
    }
}

/// The recent land battles, oldest first.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BattleHistory {
    records: Vec<BattleRecord>,
}

impl BattleHistory {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// The kept battles, oldest first.
    pub fn records(&self) -> &[BattleRecord] {
        &self.records
    }

    /// Adds `record` and enforces the bounds as of the record's turn.
    pub fn push(&mut self, rules: &BattleHistoryRules, record: BattleRecord) {
        let turn = record.turn;
        self.records.push(record);
        self.purge(rules, turn);
    }

    /// Drops the battles `max_age_turns` old or more at `turn`, then the
    /// oldest ones beyond `max_records`.
    pub fn purge(&mut self, rules: &BattleHistoryRules, turn: u32) {
        self.records
            .retain(|record| record.age(turn) < rules.max_age_turns);
        let cap = rules.max_records as usize;
        if self.records.len() > cap {
            let excess = self.records.len() - cap;
            self.records.drain(..excess);
        }
    }
}

/// A side as the resolution sites know it: faction and men engaged.
pub(crate) type Engaged<'a> = (&'a FactionId, u32);

/// Records a land battle resolved this turn in `province` at `position`
/// (map pixels). Losses and winner come from `result`.
pub(crate) fn record(
    state: &mut CampaignState,
    data: &GameData,
    kind: BattleKind,
    province: &ProvinceId,
    position: [f32; 2],
    (attacker, defender): (Engaged<'_>, Engaged<'_>),
    result: &BattleResult,
) {
    let record = BattleRecord {
        turn: state.turn,
        province: province.clone(),
        position,
        kind,
        attacker: BattleSideRecord {
            faction: attacker.0.clone(),
            strength: attacker.1,
            losses: result.attacker.total_losses,
        },
        defender: BattleSideRecord {
            faction: defender.0.clone(),
            strength: defender.1,
            losses: result.defender.total_losses,
        },
        attacker_won: result.winner == Winner::Attacker,
    };
    state
        .battle_history
        .push(&data.battle_history_rules, record);
}

/// Start of a new turn: forgets the battles that are too old.
pub(crate) fn on_new_turn(state: &mut CampaignState, data: &GameData) {
    let turn = state.turn;
    state.battle_history.purge(&data.battle_history_rules, turn);
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<BattleKind>();
    }
}
