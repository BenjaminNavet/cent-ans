//! Data-driven trait triggers (WH charsb, ADR 0284).
//!
//! `data/rules/trait_triggers.json` lists which conditions grant an acquired
//! trait (veteran after N battles, siege master, old age at 60, indebted
//! ruler...). This module evaluates the table for one character
//! ([`evaluate`]) after a battle, siege or raid, and for everybody once per
//! season ([`evaluate_all`]). Evaluation draws no random number.

use data_model::{CharacterId, GameData, SkillRole, TraitTriggerRules, TriggerCondition};

use crate::events::{EventKind, GameEvent};
use crate::skills;
use crate::state::{CampaignState, CharacterState};

fn rules() -> &'static TraitTriggerRules {
    TraitTriggerRules::bundled()
}

/// `true` when every present condition holds for `c`.
fn holds(state: &CampaignState, c: &CharacterState, when: &TriggerCondition) -> bool {
    when.battles_fought_min
        .is_none_or(|n| c.battles_fought >= n)
        && when.battles_won_min.is_none_or(|n| c.battles_won >= n)
        && when.sieges_won_min.is_none_or(|n| c.sieges_won >= n)
        && when.raids_led_min.is_none_or(|n| c.raids_led >= n)
        && when.age_min.is_none_or(|n| c.age(state.year) >= n)
        && when.piety_min.is_none_or(|n| c.piety >= n)
        && when.treasury_below.is_none_or(|limit| {
            state
                .factions
                .get(&c.faction)
                .is_some_and(|f| f.treasury < limit)
        })
        && (!when.last_battle_won || c.last_battle_won)
}

/// Grants every trait whose trigger now holds for `id`; returns the journal
/// lines (already pushed to `events`).
pub fn evaluate(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    events: &mut Vec<GameEvent>,
) {
    let Some(c) = state.characters.get(id).filter(|c| c.alive) else {
        return;
    };
    let roles = skills::roles_of(state, id);
    let due: Vec<_> = rules()
        .triggers
        .iter()
        .filter(|t| !c.traits.contains(&t.trait_id))
        .filter(|t| t.roles.is_empty() || t.roles.iter().any(|r: &SkillRole| roles.contains(r)))
        .filter(|t| holds(state, c, &t.when))
        .cloned()
        .collect();
    for trigger in due {
        if skills::grant_trait(state, data, id, &trigger.trait_id) {
            let name = state.character_name(data, id);
            let faction = state.characters[id].faction.clone();
            events.push(
                GameEvent::new(
                    EventKind::TraitAcquired,
                    trigger.text_fr.replace("{name}", &name),
                )
                .faction(&faction),
            );
        }
    }
}

/// Seasonal pass over every living character (age, debt, piety...).
pub fn evaluate_all(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let ids: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| c.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        evaluate(state, data, &id, events);
    }
}
