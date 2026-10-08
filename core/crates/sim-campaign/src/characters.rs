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

/// Seasons of grace before a historical character's recorded death year
/// (F9): natural death is spared until two years before it, then doubled
/// once past it, so the realms of 1337 keep their historical rulers unless
/// war or the chronicle decide otherwise.
pub const HISTORICAL_GRACE_YEARS: i32 = 2;

/// `death_permille_for` shaped by the recorded death year of historical
/// characters (F9).
pub fn natural_death_permille(state: &CampaignState, data: &GameData, id: &CharacterId) -> u32 {
    let base = death_permille_for(state, id);
    let Some(death_year) = data
        .characters
        .get(id)
        .filter(|c| c.historical)
        .and_then(|c| c.death.as_ref())
        .and_then(|d| d.year())
    else {
        return base;
    };
    if state.year < death_year - HISTORICAL_GRACE_YEARS {
        0
    } else if state.year > death_year {
        (base.max(5) * 2).min(1000)
    } else {
        base
    }
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
        let permille = natural_death_permille(state, data, &id);
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
    character.death_year = Some(state.year);
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
    // C7: the retinue passes to the heir (after the succession) or leaves.
    crate::retinue::on_death(state, data, id, ruled, events);
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
    // FE (F3): designated heir and heir by law differ under a suzerain:
    // the suzerain arbitrates, the loser may start a succession war.
    let contested = crate::feudal::contested_succession(
        state,
        data,
        faction,
        dead_ruler.as_ref(),
        designated.as_ref(),
        events,
    );
    let successor = contested.or(designated).or_else(|| {
        dead_ruler
            .as_ref()
            .and_then(|ruler| dynasty::pick_heir_by_law(state, data, faction, ruler))
    });
    // LR-05: an elective realm (republic, see, order) has no line to die
    // out: it elects a new head below and keeps its titles.
    let elective = data
        .factions
        .get(faction)
        .is_some_and(|f| f.succession_law == data_model::SuccessionLaw::Elective);
    // LR-05: a dynastic realm without heir nor kin abroad may pass to a
    // cadet branch of the house instead of escheating.
    let successor = match successor {
        None if !elective => {
            crate::feudal::cadet_branch(state, data, faction, dead_ruler.as_ref(), events)
        }
        other => other,
    };
    // FE: an heir living in another faction comes home (or takes the realm
    // into his own, greater one).
    if let Some(heir) = &successor {
        if crate::feudal::heir_comes_home(state, data, faction, heir, events) {
            return;
        }
    }
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
                        data.faction_name(faction)
                    ),
                )
                .faction(faction),
            );
        }
        None => {
            state.factions.get_mut(faction).expect("exists").ruler = None;
            // FE (F3): each title passes to kin abroad or escheats; a
            // faction left without title has vanished into its heir's.
            if !elective
                && crate::feudal::inherit_titles_on_extinction(
                    state,
                    data,
                    faction,
                    dead_ruler.as_ref(),
                    events,
                )
            {
                return;
            }
            if let Some(house) = dead_ruler
                .as_ref()
                .filter(|_| !elective)
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
            let (ruler, text) = match fallback {
                Some(ruler) => {
                    let text = format!(
                        "{} s'empare du pouvoir en {}, faute d'héritier légitime.",
                        state.character_name(data, &ruler),
                        data.faction_name(faction)
                    );
                    (ruler, text)
                }
                None => {
                    let ruler = crate::dynasty::spawn_ruler(state, data, faction);
                    let text = if elective {
                        format!(
                            "{} est élu à la tête de {}.",
                            state.character_name(data, &ruler),
                            data.faction_name(faction)
                        )
                    } else {
                        format!(
                            "La lignée s'éteint : {} fonde une nouvelle maison à la tête de {}.",
                            state.character_name(data, &ruler),
                            data.faction_name(faction)
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

/// Marks `id` dead: its wars, alliances, truces, embargoes and vassal ties
/// end (also used when a faction without title is absorbed, lot F3).
pub(crate) fn dissolve_faction(state: &mut CampaignState, id: &FactionId) {
    let dead = state.factions.get_mut(id).expect("exists");
    dead.alive = false;
    dead.at_war_with.clear();
    dead.allies.clear();
    dead.truces.clear();
    dead.embargoes.clear();
    dead.suzerain = None;
    // Wars, alliances and vassal ties with a vanished faction end.
    for other in state.factions.values_mut() {
        other.at_war_with.remove(id);
        other.allies.remove(id);
        other.truces.remove(id);
        other.embargoes.remove(id);
        if other.suzerain.as_ref() == Some(id) {
            other.suzerain = None;
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
        let has_province = state.settlements.values().any(|s| s.controller == id);
        let has_army = state.armies.values().any(|a| a.faction == id);
        if !has_province && !has_army {
            dissolve_faction(state, &id);
            // FE (F3): its titles escheat to their liege or fall vacant.
            crate::feudal::on_faction_destroyed(state, data, &id, events);
            events.push(
                GameEvent::new(
                    EventKind::FactionDestroyed,
                    format!("{} disparaît de la carte.", data.faction_name(&id)),
                )
                .faction(&id),
            );
        }
    }
}
