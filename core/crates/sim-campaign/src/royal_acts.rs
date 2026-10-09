//! Royal acts (WH chars, ADR 0276): the historical counterpart of the rites
//! of Total War: Warhammer III. A sovereign performs a ceremonial or political
//! act — the coronation at Reims, a *lit de justice*, a *Joyeuse Entrée* —
//! for prestige and/or livres; the act then cools down for several seasons.
//!
//! - data: `data/royal_acts/*.json` ([`data_model::RoyalAct`]), effects in the
//!   shared [`data_model::Effect`] vocabulary;
//! - state: [`ActRecord`] per act in `FactionState::royal_acts` (last turn and
//!   end of the effects), nothing to mutate per turn: [`active_effects`] is
//!   derived on read and merged into `research::faction_tech_effects`, which
//!   the economy, recruitment, movement and population already read;
//! - army kinds (`ArmyMorale`, `Battle*`) are also folded into
//!   `skills::character_effects` so that the generals fight with them;
//! - the prestige is the ruler's (`CharacterState::prestige`), debited with
//!   `change_ruler_prestige`;
//! - AI: [`ai_choose_royal_act`], at most one act per turn.

use data_model::{EffectKind, EffectMode, FactionId, GameData, RoyalAct, RoyalActId};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Last use of one act by one faction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActRecord {
    /// Turn the act was performed.
    pub turn: u32,
    /// First turn the effects no longer apply.
    pub until: u32,
}

/// Why a `RoyalAct` order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoyalActError {
    #[error("acte royal inconnu : {0}")]
    Unknown(RoyalActId),
    #[error("cet acte n'est pas accessible à votre faction")]
    WrongFaction,
    #[error("la faction n'a pas de souverain vivant")]
    NoRuler,
    #[error("acte déjà accompli : encore {0} tour(s) de recharge")]
    Cooldown(u32),
    #[error("prestige insuffisant : {needed} nécessaires, {available} disponibles")]
    NotEnoughPrestige { needed: i32, available: i32 },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    NotEnoughFunds { needed: i64, available: i64 },
}

/// One line of the court panel's royal-act list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoyalActOption {
    pub id: RoyalActId,
    pub available: bool,
    /// French reason when unavailable (empty otherwise).
    pub reason: Option<String>,
    /// Turns before the act can be repeated (0: ready).
    pub cooldown_left: u32,
    /// Turns the effects still last (0: not active).
    pub active_left: u32,
}

fn allowed(act: &RoyalAct, faction: &FactionId) -> bool {
    act.factions.is_empty() || act.factions.contains(faction)
}

fn ruler_prestige(state: &CampaignState, faction: &FactionId) -> Option<i32> {
    let ruler = state.factions.get(faction)?.ruler.as_ref()?;
    state
        .characters
        .get(ruler)
        .filter(|c| c.alive)
        .map(|c| c.prestige)
}

/// Turns left before `faction` can repeat `act` (0: ready).
pub fn cooldown_left(state: &CampaignState, faction: &FactionId, act: &RoyalAct) -> u32 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.royal_acts.get(&act.id))
        .map_or(0, |r| {
            (r.turn + act.cooldown_turns).saturating_sub(state.turn)
        })
}

/// Turns the effects of `act` still last for `faction` (0: not active).
pub fn active_left(state: &CampaignState, faction: &FactionId, act: &RoyalAct) -> u32 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.royal_acts.get(&act.id))
        .map_or(0, |r| r.until.saturating_sub(state.turn))
}

/// Effects of the acts of `faction` that are still running.
pub fn active_effects(state: &CampaignState, data: &GameData, faction: &FactionId) -> EffectTotals {
    let mut totals = EffectTotals::default();
    let Some(f) = state.factions.get(faction) else {
        return totals;
    };
    for (id, record) in &f.royal_acts {
        if state.turn >= record.until {
            continue;
        }
        if let Some(act) = data.royal_acts.get(id) {
            for effect in &act.effects {
                totals.add_effect(effect);
            }
        }
    }
    totals
}

/// Every act `faction` may perform, in id order, with what stops it now.
pub fn royal_act_options(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<RoyalActOption> {
    let prestige = ruler_prestige(state, faction);
    let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
    data.royal_acts
        .values()
        .filter(|act| allowed(act, faction))
        .map(|act| {
            let cooldown = cooldown_left(state, faction, act);
            let reason = if prestige.is_none() {
                Some(RoyalActError::NoRuler.to_string())
            } else if cooldown > 0 {
                Some(RoyalActError::Cooldown(cooldown).to_string())
            } else if prestige.unwrap_or(0) < act.cost_prestige {
                Some(
                    RoyalActError::NotEnoughPrestige {
                        needed: act.cost_prestige,
                        available: prestige.unwrap_or(0),
                    }
                    .to_string(),
                )
            } else if treasury < act.cost_livres {
                Some(
                    RoyalActError::NotEnoughFunds {
                        needed: act.cost_livres,
                        available: treasury,
                    }
                    .to_string(),
                )
            } else {
                None
            };
            RoyalActOption {
                id: act.id.clone(),
                available: reason.is_none(),
                reason,
                cooldown_left: cooldown,
                active_left: active_left(state, faction, act),
            }
        })
        .collect()
}

/// Validates and applies `RoyalAct { act }` for `faction`.
pub fn perform_royal_act(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    act_id: &RoyalActId,
) -> Result<(), RoyalActError> {
    let act = data
        .royal_acts
        .get(act_id)
        .ok_or_else(|| RoyalActError::Unknown(act_id.clone()))?;
    if !allowed(act, faction) {
        return Err(RoyalActError::WrongFaction);
    }
    let prestige = ruler_prestige(state, faction).ok_or(RoyalActError::NoRuler)?;
    let cooldown = cooldown_left(state, faction, act);
    if cooldown > 0 {
        return Err(RoyalActError::Cooldown(cooldown));
    }
    if prestige < act.cost_prestige {
        return Err(RoyalActError::NotEnoughPrestige {
            needed: act.cost_prestige,
            available: prestige,
        });
    }
    let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
    if treasury < act.cost_livres {
        return Err(RoyalActError::NotEnoughFunds {
            needed: act.cost_livres,
            available: treasury,
        });
    }
    let turn = state.turn;
    {
        let f = state.factions.get_mut(faction).expect("ruler checked");
        f.treasury -= act.cost_livres;
        f.royal_acts.insert(
            act.id.clone(),
            ActRecord {
                turn,
                until: turn + act.duration_turns,
            },
        );
    }
    state.change_ruler_prestige(faction, act.gain_prestige - act.cost_prestige);
    if act.gain_piety != 0 {
        let ruler = state.factions[faction].ruler.clone();
        if let Some(c) = ruler.and_then(|r| state.characters.get_mut(&r)) {
            c.piety = (i32::from(c.piety) + act.gain_piety).clamp(0, 100) as u8;
        }
    }
    state.pending_events.push(
        GameEvent::new(
            EventKind::RoyalAct,
            format!("{} : {}.", data.faction_label(faction), act.name.display),
        )
        .faction(faction),
    );
    Ok(())
}

/// Merges the battle-related kinds of the running acts into a general's
/// effects (the others reach the realm through `faction_tech_effects`).
pub(crate) fn add_army_effects(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    totals: &mut EffectTotals,
) {
    let acts = active_effects(state, data, faction);
    for kind in [
        EffectKind::ArmyMorale,
        EffectKind::BattleCharge,
        EffectKind::BattleRanged,
        EffectKind::BattleDefense,
    ] {
        let source = &acts[kind];
        totals[kind].flat += source.flat;
        totals[kind].percent += source.percent;
    }
}

/// AI: the most useful act it can afford while keeping a reserve; at most one
/// order per turn, deterministic (ties on the id).
pub fn ai_choose_royal_act(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    let at_war = !f.at_war_with.is_empty();
    let score = |act: &RoyalAct| -> f64 {
        act.effects
            .iter()
            .map(|e| {
                let value = if e.mode == EffectMode::Add {
                    e.value
                } else {
                    e.value / 10.0
                };
                match e.effect {
                    EffectKind::Unrest => -value,
                    EffectKind::ArmyMorale | EffectKind::Movement | EffectKind::RecruitSlots
                        if at_war =>
                    {
                        value * 2.0
                    }
                    EffectKind::ArmyUpkeep | EffectKind::RecruitCost => -value * 0.5,
                    EffectKind::ArmyMorale | EffectKind::Movement | EffectKind::RecruitSlots => 0.0,
                    _ => value,
                }
            })
            .sum::<f64>()
            + f64::from(act.gain_prestige) * 0.3
            + f64::from(act.gain_piety) * 0.1
    };
    state
        .royal_act_options_ready(data, faction)
        .into_iter()
        // Keep a reserve of three times the cost in the treasury.
        .filter(|act| f.treasury >= act.cost_livres * 3 && score(act) > 0.0)
        .max_by(|a, b| score(a).total_cmp(&score(b)).then_with(|| b.id.cmp(&a.id)))
        .map(|act| Order::RoyalAct {
            act: act.id.clone(),
        })
        .into_iter()
        .collect()
}

impl CampaignState {
    /// Acts `faction` can perform right now.
    fn royal_act_options_ready<'a>(
        &self,
        data: &'a GameData,
        faction: &FactionId,
    ) -> Vec<&'a RoyalAct> {
        royal_act_options(self, data, faction)
            .into_iter()
            .filter(|o| o.available)
            .filter_map(|o| data.royal_acts.get(&o.id))
            .collect()
    }
}
