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
pub fn resolve_loyalty(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = rules();
    resent_refused_ransoms(state, data, events);
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

/// Starting loyalty of `id` (1337 setup): the base, a bonus for the ruler's
/// house, the `loyalty` effects of his traits, a deterministic jitter, less
/// the ambitious's malus.
pub fn initial_loyalty(state: &CampaignState, data: &GameData, id: &CharacterId) -> u8 {
    let rules = rules();
    let Some(c) = state.characters.get(id) else {
        return rules.start_base;
    };
    let mut value = i32::from(rules.start_base);
    let kin = state
        .factions
        .get(&c.faction)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .is_some_and(|r| r.house == c.house && !c.house.is_empty());
    value += i32::from(rules.start_kin_bonus) * i32::from(kin);
    let effect = crate::skills::character_effects(state, data, id)[EffectKind::Loyalty].flat;
    value += (effect * f64::from(rules.effect_weight)).round() as i32;
    let span = u32::from(rules.start_jitter) * 2 + 1;
    value += (stable_hash(id.as_str()) % span) as i32 - i32::from(rules.start_jitter);
    if is_ambitious(c) {
        value -= i32::from(rules.start_ambition_malus);
    }
    value.clamp(0, 100) as u8
}

fn is_ambitious(c: &crate::state::CharacterState) -> bool {
    let rules = rules();
    !rules.ambition_trait.is_empty() && c.traits.iter().any(|t| t.as_str() == rules.ambition_trait)
}

/// Setup: gives every character his starting loyalty.
pub(crate) fn init_loyalty(state: &mut CampaignState, data: &GameData) {
    let ids: Vec<CharacterId> = state.characters.keys().cloned().collect();
    for id in ids {
        let value = initial_loyalty(state, data, &id);
        state.characters.get_mut(&id).expect("listed").loyalty = value;
    }
}

/// A captive whose lord could pay his ransom (terms in money, treasury above
/// the sum) and does not resents it, season after season. Rulers excepted.
fn resent_refused_ransoms(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = rules();
    if rules.ransom_refused_loss == 0 {
        return;
    }
    let ids: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && c.captive
                && c.ransom_terms
                    .clone()
                    .unwrap_or_default()
                    .eq(&crate::ransom::RansomTerms::Money)
                && state
                    .factions
                    .get(&c.faction)
                    .is_some_and(|f| f.alive && f.ruler.as_ref() != Some(*id))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let faction = state.characters[&id].faction.clone();
        let amount = crate::ransom::ransom_amount(state, data, &id);
        if state.factions[&faction].treasury < amount {
            continue;
        }
        let name = state.character_name(data, &id);
        let c = state.characters.get_mut(&id).expect("listed");
        let before = c.loyalty;
        c.loyalty = before.saturating_sub(rules.ransom_refused_loss);
        if faction == state.player_faction
            && before >= rules.warn_below
            && c.loyalty < rules.warn_below
        {
            events.push(
                GameEvent::new(
                    EventKind::Loyalty,
                    format!("{name}, captif, s'indigne que son seigneur ne paie pas sa rançon."),
                )
                .faction(&faction),
            );
        }
    }
}

/// A title was granted to `grantee` (a faction ruled by `ruler`) by `grantor`:
/// the generals and governors of the grantor whose prestige is not far below
/// the grantee's feel passed over, the ambitious more.
pub(crate) fn title_passed_over(
    state: &mut CampaignState,
    data: &GameData,
    grantor: &FactionId,
    grantee: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules();
    if rules.rival_loss == 0 {
        return;
    }
    let Some(grantee_ruler) = state.factions.get(grantee).and_then(|f| f.ruler.clone()) else {
        return;
    };
    let reference = state
        .characters
        .get(&grantee_ruler)
        .map_or(0, |c| c.prestige);
    let grantor_ruler = state.factions.get(grantor).and_then(|f| f.ruler.clone());
    let ids: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && !c.captive
                && &c.faction == grantor
                && **id != grantee_ruler
                && Some(*id) != grantor_ruler.as_ref()
                && (c.army.is_some() || c.governor_of.is_some())
                && c.prestige >= reference - rules.rival_prestige_gap
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let name = state.character_name(data, &id);
        let c = state.characters.get_mut(&id).expect("listed");
        let loss = rules.rival_loss + rules.rival_ambition_loss * u8::from(is_ambitious(c));
        let before = c.loyalty;
        c.loyalty = before.saturating_sub(loss);
        if grantor == &state.player_faction
            && before >= rules.warn_below
            && state.characters[&id].loyalty < rules.warn_below
        {
            events.push(
                GameEvent::new(
                    EventKind::Loyalty,
                    format!("{name} voit d'un mauvais œil le titre accordé à un autre."),
                )
                .faction(grantor),
            );
        }
    }
}

pub(crate) fn stable_hash(text: &str) -> u32 {
    text.bytes().fold(2_166_136_261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    })
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
        let besieging = state
            .armies
            .get(&army_id)
            .is_some_and(|a| match &a.position {
                ArmyPosition::Settlement(s) => {
                    state.settlements.get(s).is_some_and(|s| s.siege.is_some())
                }
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
