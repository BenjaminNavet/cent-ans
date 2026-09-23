//! Ageing, natural death and succession (spec § 1.3 step 8).

use data_model::{CharacterId, FactionId, GameData, Sex};

use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Probability (per mille, per season) of dying of natural causes at `age`.
pub fn death_permille(age: i32) -> u32 {
    match age {
        a if a < 40 => 0,
        a if a < 60 => 5,
        a if a < 75 => 20,
        _ => 80,
    }
}

fn character_name(data: &GameData, id: &CharacterId) -> String {
    data.characters
        .get(id)
        .map_or_else(|| id.to_string(), |c| c.name.display.clone())
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

/// Phase 8: roll natural deaths, then resolve successions.
pub(crate) fn resolve_characters(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let year = state.year;
    let ids: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| c.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let age = state.characters[&id].age(year);
        let permille = death_permille(age);
        if permille == 0 || !state.rng.chance_permille(permille) {
            continue;
        }
        kill(state, data, &id, events);
    }
}

/// Marks `id` dead, removes it from its army and triggers succession if it ruled.
pub(crate) fn kill(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    events: &mut Vec<GameEvent>,
) {
    state.detach_general(id);
    let Some(character) = state.characters.get_mut(id) else {
        return;
    };
    character.alive = false;
    let faction = character.faction.clone();
    let location = character.location.clone();
    let age = character.age(state.year);
    let mut event = GameEvent::new(
        EventKind::Death,
        format!("{} meurt à {age} ans.", character_name(data, id)),
    )
    .faction(&faction);
    if let Some(location) = location {
        event = event.province(&location);
    }
    events.push(event);

    let ruled = state
        .factions
        .get(&faction)
        .is_some_and(|f| f.ruler.as_ref() == Some(id));
    if ruled {
        succeed(state, data, &faction, events);
    } else if state
        .factions
        .get(&faction)
        .is_some_and(|f| f.heir.as_ref() == Some(id))
    {
        let heir = pick_heir(state, &faction, None);
        state.factions.get_mut(&faction).expect("exists").heir = heir;
    }
}

/// Eldest living male of the ruler's house (excluding `exclude`), if any.
fn pick_heir(
    state: &CampaignState,
    faction: &FactionId,
    exclude: Option<&CharacterId>,
) -> Option<CharacterId> {
    let faction_state = state.factions.get(faction)?;
    let house = faction_state
        .ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .map(|r| r.house.clone())?;
    state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && c.sex == Sex::Male
                && &c.faction == faction
                && c.house == house
                && Some(*id) != faction_state.ruler.as_ref()
                && Some(*id) != exclude
        })
        .min_by_key(|(id, c)| (c.birth_year, (*id).clone()))
        .map(|(id, _)| id.clone())
}

/// Replaces the dead ruler of `faction` by its heir, or the eldest male of the house.
pub(crate) fn succeed(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let designated = state
        .factions
        .get(faction)
        .and_then(|f| f.heir.clone())
        .filter(|h| state.characters.get(h).is_some_and(|c| c.alive));
    let successor = designated.or_else(|| pick_heir(state, faction, None));
    match successor {
        Some(new_ruler) => {
            let faction_state = state.factions.get_mut(faction).expect("exists");
            faction_state.ruler = Some(new_ruler.clone());
            faction_state.heir = None;
            let next_heir = pick_heir(state, faction, Some(&new_ruler));
            state.factions.get_mut(faction).expect("exists").heir = next_heir;
            events.push(
                GameEvent::new(
                    EventKind::Succession,
                    format!(
                        "{} succède à la tête de {}.",
                        character_name(data, &new_ruler),
                        faction_name(data, faction)
                    ),
                )
                .faction(faction),
            );
        }
        None => {
            state.factions.get_mut(faction).expect("exists").ruler = None;
            events.push(
                GameEvent::new(
                    EventKind::NoHeir,
                    format!("{} se retrouve sans héritier.", faction_name(data, faction)),
                )
                .faction(faction),
            );
        }
    }
}

/// Marks factions without provinces nor armies as dead.
pub(crate) fn resolve_faction_deaths(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in ids {
        if !state.factions[&id].alive {
            continue;
        }
        let has_province = state.provinces.values().any(|p| p.controller == id);
        let has_army = state.armies.values().any(|a| a.faction == id);
        if !has_province && !has_army {
            state.factions.get_mut(&id).expect("exists").alive = false;
            events.push(
                GameEvent::new(
                    EventKind::FactionDestroyed,
                    format!("{} disparaît de la carte.", faction_name(data, &id)),
                )
                .faction(&id),
            );
        }
    }
}
