//! Lot C4: regional edicts (`docs/design/2026-09-24-analyse-total-war.md`
//! § 2.1 « Édits régionaux »), one active per province.
//!
//! Modelled on H3 « La Table » (`table.rs`), with three differences: an
//! edict is free (no recurring cost), only one can be chosen at a time
//! ([`EdictChoice`] on [`ProvinceState::edict`]) and a newly chosen edict
//! only takes effect after [`data_model::Edict::delay_turns`] turns — the
//! previous edict (kept in [`EdictChoice::previous`]) stays active in the
//! meantime, which is why [`CampaignState::province_edict`] needs no per-turn
//! mutation to "activate" a choice: it is derived on read.
//!
//! - requirement: the province must be *entirely* controlled by the faction
//!   that chose the edict ([`CampaignState::holds_whole_province`]); a
//!   choice made by a previous controller, or one that loses full control,
//!   lapses to [`default_edict`] (checked every turn, [`resolve_requirements`]);
//! - effects: the shared [`Effect`] vocabulary, merged into
//!   [`crate::buildings::EffectTotals`] exactly like a building's or a
//!   governor's (`CampaignState::province_effects`,
//!   `CampaignState::settlement_effects`) — no change needed to whatever
//!   reads those totals (tax, trade, unrest, recruitment...);
//! - AI: [`ai_choose_edicts`], one deterministic pass per faction, mirroring
//!   [`crate::table::ai_choose_diets`]'s scoring without the budget check
//!   (edicts are free).

use data_model::{Edict, EdictId, EffectKind, EffectMode, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Edict every province starts with (free, no effect).
pub const DEFAULT_EDICT: &str = "edict_none";

/// The default edict id.
pub fn default_edict() -> EdictId {
    EdictId::new(DEFAULT_EDICT).expect("well-formed id")
}

/// Edict chosen for a province (`ProvinceState::edict`); `None` = default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdictChoice {
    /// Edict being adopted (or already active once the delay has elapsed).
    pub edict: EdictId,
    /// Faction that chose it; the choice lapses when the controller changes.
    pub faction: FactionId,
    /// Turn of the change (one change per province and per turn).
    pub turn: u32,
    /// Edict that was active right before this choice; stays in effect
    /// until `turn + delay_turns` (the chosen edict's `delay_turns`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<EdictId>,
}

/// Why a `SetEdict` order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EdictError {
    #[error("édit inconnu : {0}")]
    UnknownEdict(EdictId),
    #[error("{0} n'est pas contrôlée par votre faction")]
    NotControlled(String),
    #[error("{0} n'est pas entièrement contrôlée par votre faction")]
    NotWhollyControlled(String),
    #[error("l'édit de {0} a déjà été changé ce tour-ci")]
    AlreadyChanged(String),
}

/// One line of the province panel's edict selector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdictOption {
    pub edict: EdictId,
    pub name: String,
    pub available: bool,
    /// French reason when unavailable (empty otherwise).
    pub reason: Option<String>,
    /// Currently chosen (whether or not it has taken effect yet).
    pub current: bool,
    /// Currently in effect (`false` while a delayed change is pending).
    pub active: bool,
}

fn edict_name(data: &GameData, id: &EdictId) -> String {
    data.edicts
        .get(id)
        .map_or_else(|| id.to_string(), |e| e.name.display.clone())
}

impl CampaignState {
    /// Edict chosen for `province`, ignoring the activation delay (what the
    /// controller picked, `None`: the default).
    fn edict_choice(&self, province: &ProvinceId) -> Option<&EdictChoice> {
        self.provinces.get(province).and_then(|p| {
            p.edict
                .as_ref()
                .filter(|choice| self.province_controller(province) == Some(&choice.faction))
        })
    }

    /// Edict actually in effect in `province`: the controller's choice once
    /// its delay has elapsed, the previous one while pending, else the
    /// default.
    pub fn province_edict(&self, data: &GameData, province: &ProvinceId) -> EdictId {
        let Some(choice) = self.edict_choice(province) else {
            return default_edict();
        };
        let delay = data.edicts.get(&choice.edict).map_or(0, |e| e.delay_turns);
        if self.turn.saturating_sub(choice.turn) >= delay {
            choice.edict.clone()
        } else {
            choice.previous.clone().unwrap_or_else(default_edict)
        }
    }

    /// Chosen edict still waiting for its delay, with the turns left before
    /// it takes effect (`None` when nothing is pending).
    pub fn pending_edict(&self, data: &GameData, province: &ProvinceId) -> Option<(EdictId, u32)> {
        let choice = self.edict_choice(province)?;
        let delay = data.edicts.get(&choice.edict).map_or(0, |e| e.delay_turns);
        let elapsed = self.turn.saturating_sub(choice.turn);
        (elapsed < delay).then(|| (choice.edict.clone(), delay - elapsed))
    }

    /// `true` while a chosen edict has not taken effect yet.
    pub fn edict_pending(&self, data: &GameData, province: &ProvinceId) -> bool {
        let Some(choice) = self.edict_choice(province) else {
            return false;
        };
        let delay = data.edicts.get(&choice.edict).map_or(0, |e| e.delay_turns);
        self.turn.saturating_sub(choice.turn) < delay
    }

    /// Every edict for `province` as its controller sees it, in id order.
    pub fn edict_options(&self, data: &GameData, province: &ProvinceId) -> Vec<EdictOption> {
        let Some(controller) = self.province_controller(province) else {
            return Vec::new();
        };
        let current = self
            .edict_choice(province)
            .map_or_else(default_edict, |c| c.edict.clone());
        let active = self.province_edict(data, province);
        let whole = self.holds_whole_province(controller, province);
        data.edicts
            .values()
            .map(|edict| {
                let reason = if edict.id.as_str() != DEFAULT_EDICT && !whole {
                    Some("la province doit être entièrement contrôlée".to_owned())
                } else {
                    None
                };
                EdictOption {
                    edict: edict.id.clone(),
                    name: edict.name.display.clone(),
                    available: reason.is_none(),
                    reason,
                    current: edict.id == current,
                    active: edict.id == active,
                }
            })
            .collect()
    }
}

/// Validates and applies `SetEdict { province, edict }` for `faction`.
pub fn set_edict(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    province: &ProvinceId,
    edict: &EdictId,
) -> Result<(), EdictError> {
    let definition = data
        .edicts
        .get(edict)
        .ok_or_else(|| EdictError::UnknownEdict(edict.clone()))?;
    let name = data.province_name(province);
    if !state.controls_province(faction, province) {
        return Err(EdictError::NotControlled(name));
    }
    if definition.id.as_str() != DEFAULT_EDICT && !state.holds_whole_province(faction, province) {
        return Err(EdictError::NotWhollyControlled(name));
    }
    let p = state.provinces.get(province).expect("checked above");
    if p.edict
        .as_ref()
        .is_some_and(|c| c.turn == state.turn && &c.faction == faction)
    {
        return Err(EdictError::AlreadyChanged(name));
    }
    let previous = Some(state.province_edict(data, province));
    let turn = state.turn;
    state.provinces.get_mut(province).expect("checked").edict = Some(EdictChoice {
        edict: edict.clone(),
        faction: faction.clone(),
        turn,
        previous,
    });
    Ok(())
}

/// Diet-style event, only journaled for the player.
fn push_player_event(
    state: &CampaignState,
    faction: &FactionId,
    province: &ProvinceId,
    text: String,
    events: &mut Vec<GameEvent>,
) {
    if faction != &state.player_faction {
        return;
    }
    events.push(
        GameEvent::new(EventKind::Edict, text)
            .faction(faction)
            .province(province),
    );
}

/// Turn phase (after the buildings, before the economy): an edict whose
/// province is no longer entirely controlled falls back to the default.
pub(crate) fn resolve_requirements(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for id in ids {
        let Some(choice) = state.edict_choice(&id).cloned() else {
            continue;
        };
        if choice.edict.as_str() == DEFAULT_EDICT {
            continue;
        }
        if state.holds_whole_province(&choice.faction, &id) {
            continue;
        }
        let text = format!(
            "Édits : {} n'est plus entièrement tenue ; l'édit « {} » cesse.",
            data.province_name(&id),
            edict_name(data, &choice.edict),
        );
        if let Some(p) = state.provinces.get_mut(&id) {
            p.edict = None;
        }
        push_player_event(state, &choice.faction, &id, text, events);
    }
}

/// Effects of the edict in force in `province` (province-wide targets:
/// `Unrest`, `TaxIncome`, `TradeIncome`, `GoodsSatisfaction`, `Piety`,
/// class-targeted `Wealth`/`Unrest`...). Merged into
/// `CampaignState::province_effects`/`settlement_effects`.
pub fn edict_effects(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
) -> EffectTotals {
    let mut totals = EffectTotals::default();
    let Some(edict) = data.edicts.get(&state.province_edict(data, province)) else {
        return totals;
    };
    for effect in &edict.effects {
        totals.add_effect(effect);
    }
    totals
}

/// Yearly piety the ruler of `faction` draws from its edicts (lot B7b): the
/// flat `Piety` of the edict in force in each province it wholly holds —
/// only the best province counts, so that a big realm does not multiply it
/// (same rule as the court table, `table::resolve_lent`). Paid each winter
/// by `dynasty::resolve_court_prestige`.
pub fn yearly_edict_piety(state: &CampaignState, data: &GameData, faction: &FactionId) -> i32 {
    state
        .controlled_provinces(faction)
        .iter()
        .filter(|id| state.holds_whole_province(faction, id))
        .map(|id| edict_effects(state, data, id).piety.flat.round() as i32)
        .max()
        .unwrap_or(0)
        .max(0)
}

/// Unrest at which the AI weighs an edict's unrest effects at face value
/// (EQ1): less in a calm province, more near revolt.
const AI_EDICT_UNREST_REFERENCE: f64 = 30.0;

/// Seasons of income under which the AI deems its treasury low (EQ2).
const AI_EDICT_RESERVE_SEASONS: i64 = 3;
/// Vassals counted in the weight of tax edicts (EQ2: the feudal aid is
/// levied on the vassals), and their weight each.
const AI_EDICT_MAX_VASSALS: usize = 3;
const AI_EDICT_VASSAL_WEIGHT: f64 = 0.25;

/// Weight of a tax edict's income for the AI (EQ2): 1 at peace with a
/// balanced budget, 2 at war or in deficit, 4 when the treasury is also
/// below [`AI_EDICT_RESERVE_SEASONS`] of income, 6 in debt; each vassal
/// adds [`AI_EDICT_VASSAL_WEIGHT`] (up to [`AI_EDICT_MAX_VASSALS`]). The
/// feudal aid, the only such edict, is thus chosen by a realm at war and
/// short of money, in its calm provinces, and dropped once the need ends.
fn ai_money_weight(state: &CampaignState, faction: &FactionId) -> f64 {
    let Some(f) = state.factions.get(faction) else {
        return 1.0;
    };
    let at_war = f.at_war_with.iter().any(|e| e.as_str() != "fac_rebels");
    let deficit = f.income_last_turn < f.upkeep_last_turn;
    let low = f.treasury < AI_EDICT_RESERVE_SEASONS * f.income_last_turn.max(0);
    let base = if f.treasury < 0 {
        6.0
    } else if (at_war || deficit) && low {
        4.0
    } else if at_war || deficit {
        2.0
    } else {
        1.0
    };
    let vassals = state
        .factions
        .values()
        .filter(|v| v.alive && v.suzerain.as_ref() == Some(faction))
        .count()
        .min(AI_EDICT_MAX_VASSALS);
    base + vassals as f64 * AI_EDICT_VASSAL_WEIGHT
}

/// Simple deterministic AI, one pass per faction (mirrors
/// `table::ai_choose_diets`'s scoring, without the budget check since
/// edicts cost nothing): in each wholly-held province, switch to the
/// available edict with the best score (`Unrest`/negative-class-unrest
/// counted positively, other `add`/`percent` effects summed) when it beats
/// the current one by a margin, so the AI does not flip-flop on a tie.
pub fn ai_choose_edicts(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    if data.edicts.is_empty() || !f.alive {
        return Vec::new();
    }
    const SWITCH_MARGIN: f64 = 2.0;
    let mut orders = Vec::new();
    for id in state.controlled_provinces(faction) {
        if !state.holds_whole_province(faction, &id) {
            continue;
        }
        let choice_turn = state.edict_choice(&id).map(|c| c.turn);
        if choice_turn == Some(state.turn) {
            continue;
        }
        let current = state
            .edict_choice(&id)
            .map_or_else(default_edict, |c| c.edict.clone());
        // EQ1: the scores follow the province and the realm: appeasement
        // matters in proportion to the local unrest, taxes and levies when
        // the realm is at war or in deficit (the static score always chose
        // the Peace of God, 89 % of the provinces).
        let unrest = state
            .provinces
            .get(&id)
            .map_or(0.0, |p| crate::population::weighted_unrest(&p.population));
        let unrest_weight = (unrest / AI_EDICT_UNREST_REFERENCE).clamp(0.25, 3.0);
        let needs_men = !f.at_war_with.is_empty();
        let money_weight = ai_money_weight(state, faction);
        let score = |edict: &Edict| -> f64 {
            edict
                .effects
                .iter()
                .map(|e| {
                    let value = if e.mode == EffectMode::Add {
                        e.value
                    } else {
                        e.value / 10.0
                    };
                    match e.effect {
                        EffectKind::Unrest => -value * unrest_weight,
                        EffectKind::TaxIncome => value * money_weight,
                        EffectKind::RecruitSlots
                        | EffectKind::RecruitCost
                        | EffectKind::Garrison
                            if needs_men =>
                        {
                            let value = if e.effect == EffectKind::RecruitCost {
                                -value
                            } else {
                                value
                            };
                            value * 2.0
                        }
                        EffectKind::RecruitCost => -value,
                        _ => value,
                    }
                })
                .sum()
        };
        let current_score = data.edicts.get(&current).map_or(0.0, score);
        let best = data
            .edicts
            .values()
            .filter(|e| e.id != current)
            .map(|e| (score(e), e.id.clone()))
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        if let Some((best_score, edict)) = best {
            if best_score > current_score + SWITCH_MARGIN {
                orders.push(Order::SetEdict {
                    province: id,
                    edict,
                });
            }
        }
    }
    orders
}

#[cfg(test)]
mod tests {
    use data_model::GameData;

    use super::*;
    use crate::state::CampaignState;

    fn setup() -> (CampaignState, GameData) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("data");
        let (data, _warnings) = GameData::load(&root).expect("data/ must load");
        let faction = data_model::FactionId::new("fac_france").expect("well-formed id");
        let state = CampaignState::new_1337(&data, faction, 1).expect("new_1337 must build");
        (state, data)
    }

    #[test]
    fn defaults_to_edict_none() {
        let (state, data) = setup();
        let province = state
            .provinces
            .keys()
            .next()
            .expect("at least one province")
            .clone();
        assert_eq!(
            state.province_edict(&data, &province).as_str(),
            DEFAULT_EDICT
        );
        assert!(!state.edict_pending(&data, &province));
    }

    #[test]
    fn set_edict_is_delayed_then_takes_effect() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let edict = EdictId::new("edict_peace_of_god").expect("well-formed id");
        set_edict(&mut state, &data, &faction, &province, &edict).expect("valid order");
        // delay_turns: 1 for edict_peace_of_god -> not active on the same turn.
        assert!(state.edict_pending(&data, &province));
        assert_eq!(
            state.pending_edict(&data, &province),
            Some((edict.clone(), 1))
        );
        assert_eq!(
            state.province_edict(&data, &province).as_str(),
            DEFAULT_EDICT
        );
        state.turn += 1;
        assert!(!state.edict_pending(&data, &province));
        assert_eq!(state.province_edict(&data, &province), edict);
    }

    #[test]
    fn set_edict_rejects_second_change_same_turn() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let edict = EdictId::new("edict_feudal_aid").expect("well-formed id");
        set_edict(&mut state, &data, &faction, &province, &edict).expect("first change ok");
        let other = EdictId::new("edict_market_freedoms").expect("well-formed id");
        let err = set_edict(&mut state, &data, &faction, &province, &other).unwrap_err();
        assert!(matches!(err, EdictError::AlreadyChanged(_)));
    }

    #[test]
    fn set_edict_rejects_unknown_id() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let unknown = EdictId::new("edict_does_not_exist").expect("well-formed id");
        let err = set_edict(&mut state, &data, &faction, &province, &unknown).unwrap_err();
        assert!(matches!(err, EdictError::UnknownEdict(_)));
    }

    #[test]
    fn edict_piety_reaches_the_ruler_each_winter() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let provinces: Vec<ProvinceId> = state
            .controlled_provinces(&faction)
            .into_iter()
            .filter(|p| state.holds_whole_province(&faction, p))
            .take(2)
            .collect();
        assert_eq!(provinces.len(), 2, "player holds two whole provinces");
        assert_eq!(yearly_edict_piety(&state, &data, &faction), 0);
        let lent = EdictId::new("edict_strict_lent").expect("well-formed id");
        let peace = EdictId::new("edict_peace_of_god").expect("well-formed id");
        set_edict(&mut state, &data, &faction, &provinces[0], &lent).expect("valid order");
        set_edict(&mut state, &data, &faction, &provinces[1], &peace).expect("valid order");
        // Not in force before the delay.
        assert_eq!(yearly_edict_piety(&state, &data, &faction), 0);
        state.turn += 1;
        let expected = |id: &EdictId| -> i32 {
            data.edicts[id]
                .effects
                .iter()
                .filter(|e| e.effect == EffectKind::Piety)
                .map(|e| e.value.round() as i32)
                .sum()
        };
        // Only the best province counts (no stacking across the realm).
        let best = expected(&lent).max(expected(&peace));
        assert!(best > 0, "strict Lent gives piety in data/edicts");
        assert_eq!(yearly_edict_piety(&state, &data, &faction), best);

        let ruler = state.factions[&faction].ruler.clone().expect("a ruler");
        state.characters.get_mut(&ruler).expect("ruler").piety = 40;
        let buildings = crate::dynasty::yearly_building_piety(&state, &data, &faction);
        state.season = crate::state::Season::Summer;
        crate::dynasty::resolve_court_prestige(&mut state, &data);
        assert_eq!(state.characters[&ruler].piety, 40, "paid in winter only");
        state.season = crate::state::Season::Winter;
        crate::dynasty::resolve_court_prestige(&mut state, &data);
        assert_eq!(
            i32::from(state.characters[&ruler].piety),
            40 + buildings + best
        );
    }

    #[test]
    fn edict_effects_reach_province_effects() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let edict = EdictId::new("edict_feudal_aid").expect("well-formed id");
        let before = edict_effects(&state, &data, &province);
        let merged_before = state.province_effects(&data, &province);
        set_edict(&mut state, &data, &faction, &province, &edict).expect("valid order");
        // edict_feudal_aid has delay_turns: 0, so it is active immediately.
        let after = edict_effects(&state, &data, &province);
        assert_eq!(before.tax_income.percent, 0.0);
        assert_eq!(after.tax_income.percent, 20.0);
        assert_eq!(after.unrest.flat, 6.0);
        // Merged into the province's full effect totals too, on top of
        // whatever the buildings/governor already contribute.
        let merged_after = state.province_effects(&data, &province);
        assert_eq!(
            merged_after.tax_income.percent - merged_before.tax_income.percent,
            20.0
        );
    }

    #[test]
    fn lapses_when_province_no_longer_wholly_controlled() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let edict = EdictId::new("edict_feudal_aid").expect("well-formed id");
        set_edict(&mut state, &data, &faction, &province, &edict).expect("valid order");
        assert_eq!(state.province_edict(&data, &province), edict);
        // Hand one settlement of the province to another faction: the
        // province is no longer wholly held.
        let other = state
            .factions
            .keys()
            .find(|f| *f != &faction)
            .cloned()
            .expect("at least one other faction");
        let settlement = state
            .settlements_of(&province)
            .nth(1)
            .map(|(id, _)| id.clone());
        if let Some(settlement) = settlement {
            if let Some(s) = state.settlements.get_mut(&settlement) {
                s.owner = other.clone();
                s.controller = other;
            }
            let mut events = Vec::new();
            resolve_requirements(&mut state, &data, &mut events);
            assert_eq!(
                state.province_edict(&data, &province).as_str(),
                DEFAULT_EDICT
            );
        }
    }

    #[test]
    fn requires_whole_province() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        // Find a province the faction controls but does not wholly hold, if any.
        let partial = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| !state.holds_whole_province(&faction, p));
        if let Some(province) = partial {
            let edict = EdictId::new("edict_feudal_aid").expect("well-formed id");
            let err = set_edict(&mut state, &data, &faction, &province, &edict).unwrap_err();
            assert!(matches!(err, EdictError::NotWhollyControlled(_)));
        }
    }

    #[test]
    fn old_save_without_edict_field_defaults() {
        // A `ProvinceState` JSON without an `edict` key (pre-lot-C4 save)
        // must still deserialize, falling back to the default edict.
        let json = r#"{
            "city": "set_test",
            "settlements": ["set_test"],
            "unrest": 0,
            "devastation": 0,
            "population": {
                "peasants": {"count": 100, "wealth": 0, "unrest": 0, "health": 100, "goods_satisfaction": 0},
                "burghers": {"count": 10, "wealth": 0, "unrest": 0, "health": 100, "goods_satisfaction": 0},
                "clergy": {"count": 5, "wealth": 0, "unrest": 0, "health": 100, "goods_satisfaction": 0},
                "nobility": {"count": 2, "wealth": 0, "unrest": 0, "health": 100, "goods_satisfaction": 0}
            }
        }"#;
        let province: crate::state::ProvinceState =
            serde_json::from_str(json).expect("old save without `edict` must still deserialize");
        assert!(province.edict.is_none());
    }

    #[test]
    fn edicts_survive_a_save_round_trip() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let province = state
            .controlled_provinces(&faction)
            .into_iter()
            .find(|p| state.holds_whole_province(&faction, p))
            .expect("player holds at least one whole province");
        let edict = EdictId::new("edict_militia_levy").expect("well-formed id");
        set_edict(&mut state, &data, &faction, &province, &edict).expect("valid order");
        let json = state.save_json();
        let loaded = CampaignState::load_json(&json).expect("round trip");
        assert_eq!(
            loaded.provinces[&province]
                .edict
                .as_ref()
                .map(|c| c.edict.clone()),
            Some(edict)
        );
    }

    /// Deterministic (ADR 0003): two clones fed the same AI orders over
    /// several turns must reach the exact same state, and every order the
    /// AI proposes must be accepted.
    #[test]
    fn ai_edicts_are_valid_and_deterministic() {
        let (mut a, data) = setup();
        let mut b = a.clone();
        for _ in 0..8 {
            a.end_turn(&data);
            b.end_turn(&data);
        }
        assert_eq!(a.save_json(), b.save_json());
        for faction in a.factions.keys().cloned().collect::<Vec<_>>() {
            let mut probe = a.clone();
            for order in ai_choose_edicts(&a, &data, &faction) {
                probe.apply_order(&data, &faction, order).unwrap();
            }
        }
    }

    /// EQ2: the feudal aid is chosen by a realm at war and short of money
    /// in its calm provinces, never by a rich realm at peace.
    #[test]
    fn ai_levies_the_feudal_aid_only_when_short_of_money() {
        let (mut state, data) = setup();
        let faction = state.player_faction.clone();
        let aid = EdictId::new("edict_feudal_aid").expect("well-formed id");
        let calm: Vec<ProvinceId> = state
            .controlled_provinces(&faction)
            .into_iter()
            .filter(|p| state.holds_whole_province(&faction, p))
            .collect();
        for id in &calm {
            let p = state.provinces.get_mut(id).expect("province");
            for class in [
                &mut p.population.peasants,
                &mut p.population.burghers,
                &mut p.population.clergy,
                &mut p.population.nobility,
            ] {
                class.unrest = 5;
            }
        }
        let chooses_aid = |state: &CampaignState| {
            ai_choose_edicts(state, &data, &faction)
                .iter()
                .any(|o| matches!(o, Order::SetEdict { edict, .. } if *edict == aid))
        };
        let f = state.factions.get_mut(&faction).expect("faction");
        f.at_war_with.clear();
        f.treasury = 100_000;
        f.income_last_turn = 5_000;
        f.upkeep_last_turn = 3_000;
        assert!(!chooses_aid(&state), "rich realm at peace");
        let f = state.factions.get_mut(&faction).expect("faction");
        f.at_war_with
            .insert(FactionId::new("fac_england").expect("well-formed id"));
        assert!(!chooses_aid(&state), "rich realm at war");
        state.factions.get_mut(&faction).expect("faction").treasury = 2_000;
        assert!(chooses_aid(&state), "realm at war with a low treasury");
    }
}
