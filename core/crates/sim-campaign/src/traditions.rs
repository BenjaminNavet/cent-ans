//! Lot TW2-T5 (ADR 0109): army traditions.
//!
//! The *army*, not only its general, gains experience from every battle it
//! fights, more when it wins (`data/rules/army_traditions.json`,
//! `experience`), times the nuanced outcome's multiplier (CV3: heroic ×2).
//! Each rank reached opens one choice among five branches (march,
//! stewardship, shooting, assault, discipline); a tier-N tradition needs the
//! tier N-1 of its branch. Effects hook into the existing modules:
//!
//! - march: movement allowance (`movement::army_movement_allowance`);
//! - stewardship: replenishment rate bonus (`replenish`, TW2-T2);
//! - shooting: ranged points of the shooting units, in the auto-resolver
//!   (`movement::side_from_army`) and the 3D battle
//!   (`battle_request::side_setup`);
//! - assault: siege duration, as the general's `SiegeSpeed` (`siege`);
//! - discipline: morale points of every unit, same two battle paths.
//!
//! The army keeps a name and a banner (the house of its first general) from
//! its first experience on: they no longer follow a new general. A disbanded
//! or destroyed army loses its traditions with it; a detachment (split)
//! starts without any.
//!
//! Reinforcements dilute experience: men added to a unit (T2 replenishment,
//! garrison levies) bring the unit's experience down pro rata
//! ([`add_recruits`]), in thousandths of a level so small refills add up.

use std::collections::BTreeMap;

use data_model::{ArmyTradition, FactionId, GameData, TraditionBranch, TraditionEffects};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{Army, ArmyId, CampaignState, Unit};

/// Experience, traditions, and kept name and banner of an army.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyTraditions {
    /// Army experience, cumulated.
    #[serde(default)]
    pub xp: u32,
    /// Chosen traditions (ids of `army_traditions.json`), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chosen: Vec<String>,
    /// Name kept from the first experience on (« l'ost d'Édouard III »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// House whose arms the army carries (its first general's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_house: Option<String>,
    /// Highest rank already announced to the player.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub announced_rank: u8,
}

fn is_zero(value: &u8) -> bool {
    *value == 0
}

impl ArmyTraditions {
    pub fn is_empty(&self) -> bool {
        self == &ArmyTraditions::default()
    }
}

/// Rank reached with `xp` (0 before the first threshold).
pub fn rank_for_xp(data: &GameData, xp: u32) -> u8 {
    data.army_tradition_rules
        .experience
        .rank_thresholds
        .iter()
        .take_while(|t| xp >= **t)
        .count() as u8
}

/// A tradition as the traditions panel offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraditionOption {
    pub id: String,
    pub branch: TraditionBranch,
    /// French name of the branch.
    pub branch_name: String,
    pub tier: u8,
    pub name: String,
    pub description: String,
    /// French effects (« +8 % de mouvement »).
    pub effects_text: String,
    /// Already chosen by the army.
    pub chosen: bool,
    /// Can be chosen now.
    pub allowed: bool,
    /// French reason when not allowed.
    pub reason: String,
}

/// What the traditions panel shows of an army.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraditionView {
    pub xp: u32,
    pub rank: u8,
    pub max_rank: u8,
    /// Experience of the next rank (`None` at the last rank).
    pub next_threshold: Option<u32>,
    /// Choices still to make (ranks reached minus traditions chosen).
    pub pending: u8,
    /// Army name (kept one, else the current one).
    pub name: String,
    /// `true` once the name is kept (no longer follows the general).
    pub named: bool,
    pub banner_house: Option<String>,
    /// Every tradition, in data order, with its state.
    pub options: Vec<TraditionOption>,
    /// Summed effects of the chosen traditions.
    pub effects: TraditionEffects,
}

/// Why `choose_tradition` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TraditionError {
    #[error("tradition inconnue : {0}")]
    UnknownTradition(String),
    #[error("tradition déjà choisie")]
    AlreadyChosen,
    #[error("aucun rang à dépenser : l'armée doit gagner de l'expérience")]
    NoRankAvailable,
    #[error("il faut d'abord la tradition précédente de cette branche")]
    MissingPrevious,
}

/// Summed effects of the traditions of `army`.
pub fn army_tradition_effects(data: &GameData, army: &Army) -> TraditionEffects {
    let mut total = TraditionEffects::default();
    for id in &army.traditions.chosen {
        if let Some(def) = data.army_tradition_rules.tradition(id) {
            total.add(&def.effects);
        }
    }
    total
}

/// Siege duration cut (percent) of the assault traditions of `army`.
pub fn siege_speed_percent(data: &GameData, army: &Army) -> f64 {
    f64::from(army_tradition_effects(data, army).siege_speed_percent)
}

/// Choices `army` may still make.
pub fn pending_choices(data: &GameData, army: &Army) -> u8 {
    rank_for_xp(data, army.traditions.xp).saturating_sub(army.traditions.chosen.len() as u8)
}

/// Why `army` cannot take `tradition` now (`None` = it can).
fn refusal(data: &GameData, army: &Army, tradition: &ArmyTradition) -> Option<TraditionError> {
    let chosen = &army.traditions.chosen;
    if chosen.contains(&tradition.id) {
        return Some(TraditionError::AlreadyChosen);
    }
    let previous_taken = tradition.tier <= 1
        || data
            .army_tradition_rules
            .traditions
            .iter()
            .filter(|t| t.branch == tradition.branch && t.tier + 1 == tradition.tier)
            .any(|t| chosen.contains(&t.id));
    if !previous_taken {
        return Some(TraditionError::MissingPrevious);
    }
    if pending_choices(data, army) == 0 {
        return Some(TraditionError::NoRankAvailable);
    }
    None
}

/// Traditions `army` may take now.
pub fn available_traditions<'a>(data: &'a GameData, army: &Army) -> Vec<&'a ArmyTradition> {
    data.army_tradition_rules
        .traditions
        .iter()
        .filter(|t| refusal(data, army, t).is_none())
        .collect()
}

impl CampaignState {
    /// The traditions panel of `army_id`; `None` for an unknown army.
    pub fn army_tradition_view(&self, data: &GameData, army_id: &ArmyId) -> Option<TraditionView> {
        let army = self.armies.get(army_id)?;
        let rules = &data.army_tradition_rules;
        let rank = rank_for_xp(data, army.traditions.xp);
        let options = rules
            .traditions
            .iter()
            .map(|t| {
                let refused = refusal(data, army, t);
                TraditionOption {
                    id: t.id.clone(),
                    branch: t.branch,
                    branch_name: rules
                        .branch(t.branch)
                        .map_or_else(|| t.branch.key().to_owned(), |b| b.name.clone()),
                    tier: t.tier,
                    name: t.name.clone(),
                    description: t.description.clone(),
                    effects_text: t.effects.text_fr(),
                    chosen: army.traditions.chosen.contains(&t.id),
                    allowed: refused.is_none(),
                    reason: refused.map_or_else(String::new, |e| e.to_string()),
                }
            })
            .collect();
        Some(TraditionView {
            xp: army.traditions.xp,
            rank,
            max_rank: rules.max_rank(),
            next_threshold: rules
                .experience
                .rank_thresholds
                .get(usize::from(rank))
                .copied(),
            pending: pending_choices(data, army),
            name: self.army_name(data, army_id),
            named: army.traditions.name.is_some(),
            banner_house: army.traditions.banner_house.clone().or_else(|| {
                army.general
                    .as_ref()
                    .and_then(|g| self.characters.get(g))
                    .map(|c| c.house.clone())
                    .filter(|h| !h.is_empty())
            }),
            options,
            effects: army_tradition_effects(data, army),
        })
    }

    /// Order `ChooseArmyTradition`: `faction`'s army `army_id` takes
    /// `tradition` (one per rank reached; tier N after tier N-1).
    pub fn choose_army_tradition(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army_id: &ArmyId,
        tradition: &str,
    ) -> Result<(), crate::orders::OrderError> {
        use crate::orders::OrderError;
        let army = self
            .armies
            .get(army_id)
            .ok_or_else(|| OrderError::UnknownArmy(army_id.clone()))?;
        if &army.faction != faction {
            return Err(OrderError::NotYourArmy(faction.clone()));
        }
        let def = data
            .army_tradition_rules
            .tradition(tradition)
            .ok_or_else(|| TraditionError::UnknownTradition(tradition.to_owned()))?;
        if let Some(error) = refusal(data, army, def) {
            return Err(error.into());
        }
        let army = self.armies.get_mut(army_id).expect("checked above");
        army.traditions.chosen.push(def.id.clone());
        Ok(())
    }
}

/// Grants `amount` experience to `army_id`; the first experience fixes its
/// name and banner. The player hears of each new rank.
pub fn grant_army_xp(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    amount: u32,
    events: &mut Vec<GameEvent>,
) {
    if amount == 0 {
        return;
    }
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    // Name and banner kept from now on (only once the army has a general:
    // « l'ost de France » says nothing).
    if army.traditions.name.is_none() {
        if let Some(general) = army.general.as_ref().and_then(|g| state.characters.get(g)) {
            let house = Some(general.house.clone()).filter(|h| !h.is_empty());
            let name = state.army_name(data, army_id);
            let army = state.armies.get_mut(army_id).expect("checked above");
            army.traditions.name = Some(name);
            army.traditions.banner_house = house;
        }
    }
    let player = state.player_faction.clone();
    let army = state.armies.get_mut(army_id).expect("checked above");
    army.traditions.xp = army.traditions.xp.saturating_add(amount);
    let rank = rank_for_xp(data, army.traditions.xp);
    if rank <= army.traditions.announced_rank {
        return;
    }
    army.traditions.announced_rank = rank;
    let faction = army.faction.clone();
    if faction != player {
        return;
    }
    let name = crate::events::capitalize(&state.army_name(data, army_id));
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!("{name} atteint le rang {rank} : une tradition d'armée est à choisir."),
        )
        .army(army_id)
        .faction(&faction),
    );
}

/// One side of a battle for the army experience.
pub(crate) struct BattleSide<'a> {
    pub armies: &'a [ArmyId],
    /// Men of the side before the battle, per army.
    pub strength_before: &'a BTreeMap<ArmyId, u32>,
    pub won: bool,
    /// Men of the enemy before the battle.
    pub enemy_strength: u32,
    /// CV3 outcome multiplier (heroic ×2...), 1 without one.
    pub multiplier: f64,
}

/// Men of each of `armies` now (before a battle).
pub(crate) fn strengths(state: &CampaignState, armies: &[ArmyId]) -> BTreeMap<ArmyId, u32> {
    armies
        .iter()
        .filter_map(|id| Some((id.clone(), state.armies.get(id)?.total_strength())))
        .collect()
}

/// After a battle (field, assault, sortie; auto-resolved or fought in 3D):
/// every surviving army of the side gains its experience, unless the enemy
/// was a mere band (under `min_enemy_percent` of the side).
pub(crate) fn on_battle(
    state: &mut CampaignState,
    data: &GameData,
    side: &BattleSide<'_>,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.army_tradition_rules.experience;
    let own: u32 = side
        .armies
        .iter()
        .filter_map(|id| side.strength_before.get(id))
        .sum();
    if u64::from(side.enemy_strength) * 100 < u64::from(own) * u64::from(rules.min_enemy_percent) {
        return;
    }
    let base = rules.battle_fought + if side.won { rules.battle_won } else { 0 };
    let amount = (f64::from(base) * side.multiplier.max(0.0)).round() as u32;
    for id in side.armies {
        grant_army_xp(state, data, id, amount, events);
    }
}

/// Adds `men` recruits of experience `recruit_experience` (0-10) to `unit`:
/// the unit's experience becomes the head-count weighted mean, kept in
/// thousandths of a level (`Unit::experience_residue`).
pub fn add_recruits(unit: &mut Unit, men: u32, recruit_experience: u8) {
    if men == 0 {
        return;
    }
    let veterans = u64::from(unit.strength);
    let total = veterans + u64::from(men);
    let old = u64::from(unit.experience) * 1000 + u64::from(unit.experience_residue.min(999));
    let mixed =
        (old * veterans + u64::from(recruit_experience.min(10)) * 1000 * u64::from(men)) / total;
    unit.experience = (mixed / 1000).min(10) as u8;
    unit.experience_residue = if unit.experience >= 10 {
        0
    } else {
        (mixed % 1000) as u16
    };
    unit.strength += men;
}
