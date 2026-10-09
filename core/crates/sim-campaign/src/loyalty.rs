//! Living loyalty of generals and governors (WH charsb, ADR 0284).
//!
//! `CharacterState::loyalty` (0-100) used to be a dead field. Each season the
//! loyalty of every non-ruler general and governor drifts towards a target
//! (`data/rules/loyalty.json`): a base, plus the character's `loyalty` effects
//! (traits, skills), plus a chivalric order, minus a province in unrest, a
//! faction in debt and a recent defeat. Under `defect_below` he may defect: a
//! general takes his army to the strongest enemy at war with his faction
//! (or, with none, only grumbles); a governor loses his province, which
//! simmers. Above `high_above`, a general's men fight with more spirit
//! ([`morale_bonus`], read by `skills::character_effects`). Defection rolls
//! come from a derived generator, so the campaign's main random stream is not
//! disturbed.

use data_model::{CharacterId, EffectKind, FactionId, GameData, LoyaltyRules};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyPosition, CampaignState};

fn rules() -> &'static LoyaltyRules {
    LoyaltyRules::bundled()
}

/// Morale the units of general `id` gain from a high loyalty.
pub fn morale_bonus(state: &CampaignState, id: &CharacterId) -> f64 {
    let rules = rules();
    state
        .characters
        .get(id)
        .filter(|c| c.army.is_some() && c.loyalty > rules.high_above)
        .map_or(0.0, |_| rules.high_morale)
}

/// Loyalty `id` drifts towards this season (`None` for a character with no
/// post: the ruler, a consort, a captive).
pub fn loyalty_target(state: &CampaignState, data: &GameData, id: &CharacterId) -> Option<i32> {
    let rules = rules();
    let c = state.characters.get(id).filter(|c| c.alive && !c.captive)?;
    let faction = state.factions.get(&c.faction)?;
    if faction.ruler.as_ref() == Some(id) || (c.army.is_none() && c.governor_of.is_none()) {
        return None;
    }
    let mut target = rules.base_target;
    let effect = crate::skills::character_effects(state, data, id)[EffectKind::Loyalty].flat;
    target += (effect * f64::from(rules.effect_weight)).round() as i32;
    target += rules.order_bonus * i32::from(crate::chivalry::member_morale(state, data, id) > 0.0);
    if let Some(province) = c.governor_of.as_ref().and_then(|p| state.provinces.get(p)) {
        let unrest = crate::population::weighted_unrest(&province.population);
        if unrest >= f64::from(rules.unrest_threshold) {
            target -= rules.unrest_penalty;
        }
    }
    if faction.treasury < 0 {
        target -= rules.debt_penalty;
    }
    let recent_defeat = !c.last_battle_won
        && c.last_battle_turn
            .is_some_and(|t| state.turn.saturating_sub(t) <= rules.defeat_window_turns);
    if recent_defeat {
        target -= rules.defeat_penalty;
    }
    Some(target.clamp(0, 100))
}

/// Seasonal phase (after the characters' own phase): drift, warnings,
/// defections.
pub fn resolve_loyalty(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules();
    let ids: Vec<CharacterId> = state.characters.keys().cloned().collect();
    for id in ids {
        let Some(target) = loyalty_target(state, data, &id) else {
            continue;
        };
        let c = state.characters.get_mut(&id).expect("listed above");
        let before = c.loyalty;
        let step = rules.drift.max(0) as u8;
        c.loyalty = if i32::from(before) > target {
            (i32::from(before) - i32::from(step)).max(target) as u8
        } else {
            (i32::from(before) + i32::from(step)).min(target) as u8
        };
        let (now, faction) = (c.loyalty, c.faction.clone());
        let is_player = faction == state.player_faction;
        if is_player && now < rules.warn_below && before >= rules.warn_below {
            let name = state.character_name(data, &id);
            events.push(
                GameEvent::new(
                    EventKind::Loyalty,
                    format!("{name} se montre amer et peu sûr : sa loyauté vacille."),
                )
                .faction(&faction),
            );
        }
        if now < rules.defect_below {
            let mut roll = crate::agents::derived_rng(
                state.seed,
                state.turn,
                id.as_str().len() as u32 ^ stable_hash(id.as_str()),
                0x10A1,
            );
            if roll.below(1000) < rules.defect_permille {
                defect(state, data, &id, events);
            }
        }
    }
}

fn stable_hash(text: &str) -> u32 {
    text.bytes()
        .fold(2_166_136_261u32, |h, b| (h ^ u32::from(b)).wrapping_mul(16_777_619))
}

/// The enemy a defecting general joins: the strongest (most settlements) of
/// the factions at war with `from`.
fn defection_host(state: &CampaignState, from: &FactionId) -> Option<FactionId> {
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            f.alive
                && state.is_at_war(from, id)
                && id.as_str() != crate::diplomacy::REBELS_FACTION
                && id.as_str() != crate::diplomacy::PAPACY_FACTION
        })
        .map(|(id, _)| {
            let held = state
                .settlements
                .values()
                .filter(|s| &s.controller == id)
                .count();
            (held, std::cmp::Reverse(id.clone()))
        })
        .max()
        .map(|(_, std::cmp::Reverse(id))| id)
}

fn defect(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules();
    let (from, army_id, province) = {
        let c = &state.characters[id];
        (c.faction.clone(), c.army.clone(), c.governor_of.clone())
    };
    let name = state.character_name(data, id);
    let player = state.player_faction.clone();
    if let Some(army_id) = army_id {
        let besieging = state.armies.get(&army_id).is_some_and(|a| match &a.position {
            ArmyPosition::Settlement(s) => state
                .settlements
                .get(s)
                .is_some_and(|s| s.siege.is_some()),
            ArmyPosition::Field { .. } => false,
        });
        let host = defection_host(state, &from).filter(|_| !besieging);
        let Some(host) = host else {
            let c = state.characters.get_mut(id).expect("exists");
            c.loyalty = rules.recover_to;
            if from == player {
                events.push(
                    GameEvent::new(
                        EventKind::Loyalty,
                        format!("{name} songe à passer à l'ennemi, mais n'ose pas encore."),
                    )
                    .faction(&from),
                );
            }
            return;
        };
        if let Some(army) = state.armies.get_mut(&army_id) {
            army.faction = host.clone();
            army.planned_path.clear();
            army.destination = None;
        }
        let c = state.characters.get_mut(id).expect("exists");
        c.faction = host.clone();
        c.loyalty = rules.recover_to;
        let text = format!(
            "{name} trahit {} et passe avec son armée au service de {}.",
            data.faction_name(&from),
            data.faction_name(&host)
        );
        for f in [&from, &host] {
            if *f == player {
                events.push(GameEvent::new(EventKind::Loyalty, text.clone()).faction(f));
            }
        }
    } else if let Some(province_id) = province {
        let c = state.characters.get_mut(id).expect("exists");
        c.governor_of = None;
        c.loyalty = rules.recover_to;
        if let Some(p) = state.provinces.get_mut(&province_id) {
            for class in data_model::SocialClass::ALL {
                let gauge = &mut p.population.get_mut(class).unrest;
                *gauge = gauge.saturating_add(rules.defect_unrest).min(100);
            }
        }
        if from == player {
            events.push(
                GameEvent::new(
                    EventKind::Loyalty,
                    format!(
                        "{name} abandonne le gouvernement de sa province et en soulève les esprits."
                    ),
                )
                .faction(&from),
            );
        }
    }
}
