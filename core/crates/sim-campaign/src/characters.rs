//! Ageing, natural death and succession (spec § 1.3 step 8, spec § 2 for the
//! M4 trait modifiers and the law-driven succession).

use data_model::{CharacterId, FactionId, GameData};

use crate::dynasty;
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

/// `death_permille` scaled by `trait_sickly` (×2) / `trait_strong` (×0.7),
/// spec § 2.
pub fn death_permille_for(state: &CampaignState, id: &CharacterId) -> u32 {
    let Some(character) = state.characters.get(id) else {
        return 0;
    };
    let mut permille = death_permille(character.age(state.year)) as f64;
    let has = |name: &str| character.traits.iter().any(|t| t.as_str() == name);
    if has("trait_sickly") {
        permille *= 2.0;
    }
    if has("trait_strong") {
        permille *= 0.7;
    }
    permille.round().min(1000.0) as u32
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
    let ids: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| c.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let permille = death_permille_for(state, &id);
        if permille == 0 || !state.rng.chance_permille(permille) {
            continue;
        }
        kill(state, data, &id, events);
    }
}

/// Marks `id` dead, removes it from its army and triggers succession if it ruled.
pub fn kill(
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
    character.governor_of = None;
    let widowed = character.spouse.clone();
    let faction = character.faction.clone();
    let location = character.location.clone();
    let age = character.age(state.year);
    let mut event = GameEvent::new(
        EventKind::Death,
        format!("{} meurt à {age} ans.", state.character_name(data, id)),
    )
    .faction(&faction);
    if let Some(location) = location {
        event = event.province(&location);
    }
    events.push(event);
    // The surviving spouse is free to remarry (the dead keep the link for
    // the family tree).
    if let Some(survivor) = widowed.and_then(|w| state.characters.get_mut(&w)) {
        if survivor.spouse.as_ref() == Some(id) {
            survivor.spouse = None;
        }
    }

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
        let heir = state
            .factions
            .get(&faction)
            .and_then(|f| f.ruler.clone())
            .and_then(|ruler| dynasty::pick_heir_by_law(state, data, &faction, &ruler));
        state.factions.get_mut(&faction).expect("exists").heir = heir;
    }
}

/// Replaces the dead ruler of `faction` by its designated heir if still
/// alive, otherwise the heir picked by the faction's `succession_law` (spec
/// § 2; `dynasty::pick_heir_by_law` falls back to the eldest living male of
/// the house for laws it does not special-case, matching the pre-M4
/// behaviour). No heir at all: the faction keeps its provinces/armies under
/// perpetual AI regency (`NoHeir`) unless it has no living character left,
/// in which case it is destroyed outright.
pub(crate) fn succeed(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let dead_ruler = state.factions.get(faction).and_then(|f| f.ruler.clone());
    let designated = state
        .factions
        .get(faction)
        .and_then(|f| f.heir.clone())
        .filter(|h| state.characters.get(h).is_some_and(|c| c.alive));
    let successor = designated.or_else(|| {
        dead_ruler
            .as_ref()
            .and_then(|ruler| dynasty::pick_heir_by_law(state, data, faction, ruler))
    });
    match successor {
        Some(new_ruler) => {
            let faction_state = state.factions.get_mut(faction).expect("exists");
            faction_state.ruler = Some(new_ruler.clone());
            faction_state.heir = None;
            let next_heir = dynasty::pick_heir_by_law(state, data, faction, &new_ruler);
            state.factions.get_mut(faction).expect("exists").heir = next_heir;
            events.push(
                GameEvent::new(
                    EventKind::Succession,
                    format!(
                        "{} succède à la tête de {}.",
                        state.character_name(data, &new_ruler),
                        faction_name(data, faction)
                    ),
                )
                .faction(faction),
            );
        }
        None => {
            state.factions.get_mut(faction).expect("exists").ruler = None;
            if let Some(house) = dead_ruler
                .as_ref()
                .and_then(|r| state.characters.get(r))
                .map(|c| c.house.clone())
            {
                crate::diplomacy::on_line_extinct(state, data, faction, &house, events);
            }
            // Personal union may have made the realm a vassal; either way it
            // needs a ruler: the eldest living adult of the faction, or else
            // a newly elected / newly risen lord.
            let year = state.year;
            let fallback = state
                .characters
                .iter()
                .filter(|(_, c)| c.alive && &c.faction == faction && c.is_major(year))
                .min_by_key(|(id, c)| (c.birth_year, (*id).clone()))
                .map(|(id, _)| id.clone());
            let elective = data
                .factions
                .get(faction)
                .is_some_and(|f| f.succession_law == data_model::SuccessionLaw::Elective);
            let (ruler, text) = match fallback {
                Some(ruler) => {
                    let text = format!(
                        "{} s'empare du pouvoir en {}, faute d'héritier légitime.",
                        state.character_name(data, &ruler),
                        faction_name(data, faction)
                    );
                    (ruler, text)
                }
                None => {
                    let ruler = crate::dynasty::spawn_ruler(state, data, faction);
                    let text = if elective {
                        format!(
                            "{} est élu à la tête de {}.",
                            state.character_name(data, &ruler),
                            faction_name(data, faction)
                        )
                    } else {
                        format!(
                            "La lignée s'éteint : {} fonde une nouvelle maison à la tête de {}.",
                            state.character_name(data, &ruler),
                            faction_name(data, faction)
                        )
                    };
                    (ruler, text)
                }
            };
            let faction_state = state.factions.get_mut(faction).expect("exists");
            faction_state.ruler = Some(ruler.clone());
            faction_state.heir = None;
            let next_heir = dynasty::pick_heir_by_law(state, data, faction, &ruler);
            state.factions.get_mut(faction).expect("exists").heir = next_heir;
            events.push(GameEvent::new(EventKind::Succession, text).faction(faction));
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
        // The virtual rebels faction lives on without land (spec M3 § 1.1).
        if !state.factions[&id].alive || id.as_str() == crate::diplomacy::REBELS_FACTION {
            continue;
        }
        let has_province = state.provinces.values().any(|p| p.controller == id);
        let has_army = state.armies.values().any(|a| a.faction == id);
        if !has_province && !has_army {
            let dead = state.factions.get_mut(&id).expect("exists");
            dead.alive = false;
            dead.at_war_with.clear();
            dead.allies.clear();
            dead.truces.clear();
            dead.embargoes.clear();
            dead.suzerain = None;
            // Wars, alliances and vassal ties with a vanished faction end.
            for other in state.factions.values_mut() {
                other.at_war_with.remove(&id);
                other.allies.remove(&id);
                other.truces.remove(&id);
                other.embargoes.remove(&id);
                if other.suzerain.as_ref() == Some(&id) {
                    other.suzerain = None;
                }
            }
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
