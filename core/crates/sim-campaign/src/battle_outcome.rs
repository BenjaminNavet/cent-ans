//! Nuanced battle outcomes (lot CV3-1, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 3): heroic, decisive,
//! Pyrrhic victories; honourable defeat, disaster. Thresholds and
//! consequences in `data/rules/battle_outcome.json`
//! ([`data_model::BattleOutcomeRules`]).
//!
//! The classification runs in `movement::apply_battle_result`, the common
//! path of auto-resolved and 3D field battles: the general's experience is
//! scaled, the ruler gains or loses prestige, the surviving armies carry a
//! temporary morale modifier ([`crate::state::MoraleModifier`]) and the
//! chronicle tells the class. The last classification is kept in
//! [`CampaignState::last_battle_outcome`] for the UI.

use data_model::util::lowercase_first;
use data_model::{BattleOutcomeClass, BattleOutcomeRules, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, MoraleModifier};

/// What the classification needs to know of one side.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SideTally {
    /// Men engaged.
    pub strength: u32,
    /// Men lost.
    pub losses: u32,
    /// The commanding general was killed or taken.
    pub general_lost: bool,
}

impl SideTally {
    /// Losses as a share (0-1) of the men engaged.
    pub fn loss_share(&self) -> f64 {
        if self.strength == 0 {
            0.0
        } else {
            (f64::from(self.losses) / f64::from(self.strength)).min(1.0)
        }
    }
}

/// Class of the battle for a side that `won` (or not) with `own` against
/// `enemy`, tested in the order of the spec.
pub fn classify(
    rules: &BattleOutcomeRules,
    won: bool,
    own: &SideTally,
    enemy: &SideTally,
) -> BattleOutcomeClass {
    let t = &rules.thresholds;
    let own_share = own.loss_share();
    if won {
        if enemy.strength > 0
            && f64::from(own.strength) / f64::from(enemy.strength) <= t.heroic_max_strength_ratio
        {
            BattleOutcomeClass::Heroic
        } else if enemy.loss_share() >= t.decisive_enemy_losses_min
            && own_share < t.decisive_own_losses_max
        {
            BattleOutcomeClass::Decisive
        } else if own_share >= t.pyrrhic_own_losses_min {
            BattleOutcomeClass::Pyrrhic
        } else {
            BattleOutcomeClass::Victory
        }
    } else if f64::from(enemy.losses) >= t.honourable_inflicted_ratio_min * f64::from(own.losses) {
        BattleOutcomeClass::HonourableDefeat
    } else if own_share >= t.disaster_own_losses_min || own.general_lost {
        BattleOutcomeClass::Disaster
    } else {
        BattleOutcomeClass::Defeat
    }
}

/// A class with its French label, as shown by the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeClassView {
    pub class: BattleOutcomeClass,
    /// `snake_case` key of the class ("heroic").
    pub key: String,
    /// French label ("Victoire héroïque").
    pub label: String,
}

impl OutcomeClassView {
    fn new(rules: &BattleOutcomeRules, class: BattleOutcomeClass) -> Self {
        OutcomeClassView {
            class,
            key: class.key().to_owned(),
            label: rules.consequence(class).label,
        }
    }
}

/// The classification of the last field battle (both sides).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleOutcomeReport {
    pub turn: u32,
    pub province: ProvinceId,
    pub attacker_faction: FactionId,
    pub defender_faction: FactionId,
    pub attacker: OutcomeClassView,
    pub defender: OutcomeClassView,
}

impl BattleOutcomeReport {
    /// The class of `faction`'s side, if it fought.
    pub fn class_of(&self, faction: &FactionId) -> Option<&OutcomeClassView> {
        if &self.attacker_faction == faction {
            Some(&self.attacker)
        } else if &self.defender_faction == faction {
            Some(&self.defender)
        } else {
            None
        }
    }
}

/// One side of a battle being classified.
pub(crate) struct OutcomeSide<'a> {
    pub faction: &'a FactionId,
    pub armies: &'a [ArmyId],
    pub tally: SideTally,
    pub won: bool,
}

/// Classifies both sides, applies prestige and morale modifiers, writes the
/// chronicle line and records the report. Returns the XP multipliers of
/// the attacker's and the defender's general.
pub(crate) fn apply(
    state: &mut CampaignState,
    data: &GameData,
    province: &ProvinceId,
    place: &str,
    attacker: &OutcomeSide<'_>,
    defender: &OutcomeSide<'_>,
    events: &mut Vec<GameEvent>,
) -> (f64, f64) {
    let rules = &data.battle_outcome_rules;
    let attacker_class = classify(rules, attacker.won, &attacker.tally, &defender.tally);
    let defender_class = classify(rules, defender.won, &defender.tally, &attacker.tally);
    for (side, class) in [(attacker, attacker_class), (defender, defender_class)] {
        let consequence = rules.consequence(class);
        if consequence.prestige != 0 {
            state.change_ruler_prestige(side.faction, consequence.prestige);
        }
        if consequence.morale != 0 && consequence.morale_turns > 0 {
            for id in side.armies {
                if let Some(army) = state.armies.get_mut(id) {
                    army.morale_modifiers.push(MoraleModifier {
                        value: consequence.morale,
                        turns: consequence.morale_turns,
                    });
                }
            }
        }
    }
    let (winner, winner_class, loser, loser_class) = if attacker.won {
        (attacker, attacker_class, defender, defender_class)
    } else {
        (defender, defender_class, attacker, attacker_class)
    };
    // NT3: a won battle counts towards the player's missions.
    crate::missions::note_battle_won(state, winner.faction);
    if winner_class != BattleOutcomeClass::Victory || loser_class != BattleOutcomeClass::Defeat {
        let name = |f: &FactionId| data.faction_name(f);
        let loser_label = rules.consequence(loser_class).label;
        let text = format!(
            "{} de l'ost {} à {place} ; {} pour l'ost {}.",
            rules.consequence(winner_class).label,
            sim_battle::sim::of_faction(&name(winner.faction)),
            lowercase_first(&loser_label),
            sim_battle::sim::of_faction(&name(loser.faction)),
        );
        events.push(
            GameEvent::new(EventKind::Battle, text)
                .province(province)
                .faction(winner.faction),
        );
    }
    state.last_battle_outcome = Some(BattleOutcomeReport {
        turn: state.turn,
        province: province.clone(),
        attacker_faction: attacker.faction.clone(),
        defender_faction: defender.faction.clone(),
        attacker: OutcomeClassView::new(rules, attacker_class),
        defender: OutcomeClassView::new(rules, defender_class),
    });
    (
        rules.consequence(attacker_class).xp_multiplier,
        rules.consequence(defender_class).xp_multiplier,
    )
}
