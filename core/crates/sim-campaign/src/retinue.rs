//! General's retinue (lot C7).
//!
//! Companions are defined in `data/retinue.json` (`GameData::retinue`); a
//! character keeps the ids of his companions in
//! `CharacterState::retinue`. This module holds the rules:
//!
//! - **Acquisition** ([`try_acquire`]): on each occasion (battle won or
//!   fought, siege won, chevauchée led, season spent by the general's army
//!   in a friendly settlement whose province has a given building, ransom
//!   received by a ruler) the companions of the catalogue are rolled in
//!   order; the first success joins, at most one per occasion. The roll is
//!   a hash of the campaign seed, the turn, the character, the companion
//!   and the occasion ([`roll_permille`]): deterministic for a seed, and it
//!   does not consume the main random stream (older battles, births and
//!   deaths keep their outcome).
//! - **Effects** ([`add_companion_effects`], read by
//!   `skills::character_effects`): every effect of every companion adds to
//!   the holder's traits and skills, except `Prestige`, which is a yearly
//!   figure paid to the holder in winter ([`resolve_retinue`]; for a ruler,
//!   through `dynasty::yearly_court_prestige`).
//! - **Death** ([`on_death`]): inheritable companions pass to the heir (the
//!   eldest living adult son, else daughter, of the same faction; else the
//!   new ruler when the dead one reigned), the others leave.
//! - **Transfer** ([`transfer_companion`]): between two generals of the
//!   same faction whose armies stand in the same settlement.
//!
//! The cap is `Retinue::max_per_character`.

use data_model::{
    AcquisitionTrigger, BuildingId, CharacterId, CompanionId, EffectKind, FactionId, GameData,
};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Why `transfer_companion` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetinueError {
    #[error("personnage inconnu, mort ou captif : {0}")]
    UnknownCharacter(CharacterId),
    #[error("compagnon inconnu : {0}")]
    UnknownCompanion(CompanionId),
    #[error("ce compagnon n'est pas dans la suite de {0}")]
    NotInRetinue(CharacterId),
    #[error("ce compagnon est déjà dans la suite de {0}")]
    AlreadyInRetinue(CharacterId),
    #[error("la suite de {0} est complète")]
    RetinueFull(CharacterId),
    #[error("les deux généraux doivent commander des armées réunies au même endroit")]
    NotTogether,
    #[error("{0} ne remplit pas les conditions de ce compagnon (faction, batailles, traits…)")]
    NotEligible(CharacterId),
}

/// Most companions `data` lets one character keep (0 without a catalogue).
pub fn max_per_character(data: &GameData) -> usize {
    data.retinue
        .as_ref()
        .map_or(0, |r| r.max_per_character as usize)
}

/// Adds the effects of `id`'s companions to `totals` (all but `Prestige`,
/// paid yearly by [`resolve_retinue`]).
pub fn add_companion_effects(
    state: &CampaignState,
    data: &GameData,
    id: &CharacterId,
    totals: &mut EffectTotals,
) {
    let (Some(retinue), Some(character)) = (&data.retinue, state.characters.get(id)) else {
        return;
    };
    for companion in character
        .retinue
        .iter()
        .filter_map(|c| retinue.companion(c))
    {
        for effect in &companion.effects {
            if effect.effect != EffectKind::Prestige {
                totals.add_effect(effect);
            }
        }
    }
}

/// Yearly prestige `id` draws from his companions.
pub fn yearly_prestige(state: &CampaignState, data: &GameData, id: &CharacterId) -> i32 {
    let (Some(retinue), Some(character)) = (&data.retinue, state.characters.get(id)) else {
        return 0;
    };
    character
        .retinue
        .iter()
        .filter_map(|c| retinue.companion(c))
        .flat_map(|c| c.effects.iter())
        .filter(|e| e.effect == EffectKind::Prestige)
        .map(|e| e.value)
        .sum::<f64>()
        .round() as i32
}

/// FNV-1a over `parts`, separated so that `("ab", "c")` ≠ `("a", "bc")`.
fn fnv1a(parts: &[&[u8]]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.iter().chain(std::iter::once(&0xff)) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// SplitMix64 finaliser.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Deterministic roll in `0..1000` for `character` meeting `companion` on
/// `occasion` this turn (seed, turn, ids and occasion hashed together).
pub fn roll_permille(
    state: &CampaignState,
    character: &CharacterId,
    companion: &CompanionId,
    occasion: &str,
) -> u32 {
    let key = fnv1a(&[
        &state.seed.to_le_bytes(),
        &state.turn.to_le_bytes(),
        character.as_str().as_bytes(),
        companion.as_str().as_bytes(),
        occasion.as_bytes(),
    ]);
    (mix(key) % 1000) as u32
}

/// `true` when `id` may gain `companion` now: alive, free, room left, not
/// already held, and the companion's conditions met.
pub fn can_gain(
    state: &CampaignState,
    data: &GameData,
    id: &CharacterId,
    companion: &CompanionId,
) -> bool {
    let (Some(retinue), Some(c)) = (&data.retinue, state.characters.get(id)) else {
        return false;
    };
    let Some(def) = retinue.companion(companion) else {
        return false;
    };
    if !c.alive
        || c.captive
        || c.retinue.contains(companion)
        || c.retinue.len() >= retinue.max_per_character as usize
    {
        return false;
    }
    let cond = &def.conditions;
    if !cond.factions.is_empty() && !cond.factions.contains(&c.faction) {
        return false;
    }
    if cond.min_command.is_some_and(|min| c.skills.command < min) {
        return false;
    }
    if cond.min_battles.is_some_and(|min| c.battles_fought < min) {
        return false;
    }
    if !cond.requires_traits.is_empty()
        && !cond.requires_traits.iter().any(|t| c.traits.contains(t))
    {
        return false;
    }
    !cond.excludes_traits.iter().any(|t| c.traits.contains(t))
}

/// Rolls the catalogue for `id` on `trigger` (with the buildings around him
/// for `SeasonInSettlement`); the first companion whose roll succeeds joins.
/// Returns it.
pub fn try_acquire(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    trigger: AcquisitionTrigger,
    buildings: &[BuildingId],
    events: &mut Vec<GameEvent>,
) -> Option<CompanionId> {
    let retinue = data.retinue.as_ref()?;
    let mut joined = None;
    'companions: for companion in &retinue.companions {
        if !can_gain(state, data, id, &companion.id) {
            continue;
        }
        for (index, rule) in companion.acquisition.iter().enumerate() {
            if rule.trigger != trigger {
                continue;
            }
            if let Some(building) = &rule.building {
                if !buildings.contains(building) {
                    continue;
                }
            }
            let occasion = format!("{}#{index}", trigger.key());
            if roll_permille(state, id, &companion.id, &occasion) < rule.chance_permille {
                joined = Some(companion);
                break 'companions;
            }
        }
    }
    let companion = joined?;
    let character = state.characters.get_mut(id).expect("checked by can_gain");
    character.retinue.push(companion.id.clone());
    let faction = character.faction.clone();
    events.push(
        GameEvent::new(
            EventKind::TraitAcquired,
            format!(
                "Un {} rejoint la suite de {}.",
                companion.name.display.to_lowercase(),
                state.character_name(data, id)
            ),
        )
        .faction(&faction),
    );
    Some(companion.id.clone())
}

/// Phase (every season, characters phase): generals whose army spends the
/// season in a friendly settlement may gain a companion tied to the
/// province's buildings; in winter every holder draws his companions'
/// yearly prestige.
pub(crate) fn resolve_retinue(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    if data.retinue.is_none() {
        return;
    }
    let stays: Vec<(CharacterId, Vec<BuildingId>)> = state
        .armies
        .values()
        .filter_map(|army| {
            let general = army.general.clone()?;
            let place = army.settlement()?;
            let settlement = state.settlements.get(place)?;
            if !state.is_friendly_settlement(&army.faction, place) {
                return None;
            }
            Some((general, state.province_buildings(&settlement.province)))
        })
        .collect();
    for (general, buildings) in stays {
        try_acquire(
            state,
            data,
            &general,
            AcquisitionTrigger::SeasonInSettlement,
            &buildings,
            events,
        );
    }
    if state.season != crate::state::Season::Winter {
        return;
    }
    // Rulers draw it through `dynasty::yearly_court_prestige`.
    let rulers: std::collections::BTreeSet<CharacterId> = state
        .factions
        .values()
        .filter_map(|f| f.ruler.clone())
        .collect();
    let holders: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(id, c)| c.alive && !c.retinue.is_empty() && !rulers.contains(*id))
        .map(|(id, _)| id.clone())
        .collect();
    for id in holders {
        let gain = yearly_prestige(state, data, &id);
        if gain != 0 {
            state.characters.get_mut(&id).expect("listed").prestige += gain;
        }
    }
}

/// A ransom was paid to `captor`: its ruler may gain a companion.
pub(crate) fn on_ransom_received(
    state: &mut CampaignState,
    data: &GameData,
    captor: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let Some(ruler) = state.factions.get(captor).and_then(|f| f.ruler.clone()) else {
        return;
    };
    try_acquire(
        state,
        data,
        &ruler,
        AcquisitionTrigger::RansomReceived,
        &[],
        events,
    );
}

/// Who inherits the retinue of `dead`: the eldest living adult son of the
/// same faction, else daughter; else, when `ruled`, the faction's current
/// ruler (already his successor).
fn retinue_heir(state: &CampaignState, dead: &CharacterId, ruled: bool) -> Option<CharacterId> {
    let c = state.characters.get(dead)?;
    let year = state.year;
    let child = c
        .children
        .iter()
        .filter_map(|id| state.characters.get(id).map(|child| (id, child)))
        .filter(|(_, child)| {
            child.alive && !child.captive && child.faction == c.faction && child.is_major(year)
        })
        .min_by_key(|(id, child)| {
            (
                child.sex != data_model::Sex::Male,
                child.birth_year,
                (*id).clone(),
            )
        })
        .map(|(id, _)| id.clone());
    child.or_else(|| {
        if !ruled {
            return None;
        }
        state
            .factions
            .get(&c.faction)
            .and_then(|f| f.ruler.clone())
            .filter(|r| r != dead && state.characters.get(r).is_some_and(|r| r.alive))
    })
}

/// `dead` has just died (`characters::kill`, after the succession):
/// inheritable companions pass to his heir while there is room, the others
/// leave. `ruled`: he was the faction's ruler.
pub(crate) fn on_death(
    state: &mut CampaignState,
    data: &GameData,
    dead: &CharacterId,
    ruled: bool,
    events: &mut Vec<GameEvent>,
) {
    let Some(retinue) = &data.retinue else {
        return;
    };
    let heir = retinue_heir(state, dead, ruled);
    let Some(c) = state.characters.get_mut(dead) else {
        return;
    };
    let companions = std::mem::take(&mut c.retinue);
    let faction = c.faction.clone();
    let Some(heir) = heir else {
        return;
    };
    let mut passed = Vec::new();
    for companion in companions {
        let inheritable = retinue.companion(&companion).is_some_and(|d| d.inheritable);
        if inheritable && can_gain(state, data, &heir, &companion) {
            state
                .characters
                .get_mut(&heir)
                .expect("checked by can_gain")
                .retinue
                .push(companion.clone());
            passed.push(companion);
        }
    }
    if passed.is_empty() {
        return;
    }
    let names: Vec<String> = passed
        .iter()
        .filter_map(|c| retinue.companion(c))
        .map(|c| c.name.display.to_lowercase())
        .collect();
    events.push(
        GameEvent::new(
            EventKind::TraitAcquired,
            format!(
                "{} recueille la suite de {} : {}.",
                state.character_name(data, &heir),
                state.character_name(data, dead),
                names.join(", ")
            ),
        )
        .faction(&faction),
    );
}

/// `companion` joins `id`'s retinue directly (debug order, tests), within
/// the cap and the companion's conditions.
pub fn grant_companion(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    companion: &CompanionId,
) -> Result<(), RetinueError> {
    if data
        .retinue
        .as_ref()
        .and_then(|r| r.companion(companion))
        .is_none()
    {
        return Err(RetinueError::UnknownCompanion(companion.clone()));
    }
    let character = state
        .characters
        .get(id)
        .ok_or_else(|| RetinueError::UnknownCharacter(id.clone()))?;
    if character.retinue.contains(companion) {
        return Err(RetinueError::AlreadyInRetinue(id.clone()));
    }
    if character.retinue.len() >= max_per_character(data) {
        return Err(RetinueError::RetinueFull(id.clone()));
    }
    if !can_gain(state, data, id, companion) {
        return Err(RetinueError::NotEligible(id.clone()));
    }
    state
        .characters
        .get_mut(id)
        .expect("checked above")
        .retinue
        .push(companion.clone());
    Ok(())
}

/// Generals `from` may hand a companion to: living, free characters of his
/// faction commanding an army in the same settlement as his own, with room
/// left in their retinue.
pub fn transfer_targets(
    state: &CampaignState,
    data: &GameData,
    from: &CharacterId,
) -> Vec<CharacterId> {
    let Some(giver) = state.characters.get(from) else {
        return Vec::new();
    };
    let Some(own) = giver.army.as_ref().and_then(|a| state.armies.get(a)) else {
        return Vec::new();
    };
    let cap = max_per_character(data);
    state
        .armies
        .values()
        .filter(|a| a.faction == giver.faction && state.armies_together(data, a, own))
        .filter_map(|a| a.general.clone())
        .filter(|g| g != from)
        .filter(|g| {
            state
                .characters
                .get(g)
                .is_some_and(|c| c.alive && !c.captive && c.retinue.len() < cap)
        })
        .collect()
}

/// `transfer_companion`: `companion` leaves `from` for `to`, two living,
/// free characters of `faction` commanding armies in the same settlement.
pub fn transfer_companion(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    from: &CharacterId,
    to: &CharacterId,
    companion: &CompanionId,
) -> Result<(), RetinueError> {
    let retinue = data
        .retinue
        .as_ref()
        .ok_or_else(|| RetinueError::UnknownCompanion(companion.clone()))?;
    if retinue.companion(companion).is_none() {
        return Err(RetinueError::UnknownCompanion(companion.clone()));
    }
    let usable = |id: &CharacterId| {
        state
            .characters
            .get(id)
            .filter(|c| c.alive && !c.captive && &c.faction == faction)
    };
    let giver = usable(from).ok_or_else(|| RetinueError::UnknownCharacter(from.clone()))?;
    let taker = usable(to).ok_or_else(|| RetinueError::UnknownCharacter(to.clone()))?;
    if !giver.retinue.contains(companion) {
        return Err(RetinueError::NotInRetinue(from.clone()));
    }
    if taker.retinue.contains(companion) {
        return Err(RetinueError::AlreadyInRetinue(to.clone()));
    }
    if taker.retinue.len() >= retinue.max_per_character as usize {
        return Err(RetinueError::RetinueFull(to.clone()));
    }
    let army_of =
        |c: &crate::state::CharacterState| c.army.as_ref().and_then(|a| state.armies.get(a));
    match (army_of(giver), army_of(taker)) {
        (Some(a), Some(b)) if state.armies_together(data, a, b) && from != to => {}
        _ => return Err(RetinueError::NotTogether),
    }
    state
        .characters
        .get_mut(from)
        .expect("checked above")
        .retinue
        .retain(|c| c != companion);
    state
        .characters
        .get_mut(to)
        .expect("checked above")
        .retinue
        .push(companion.clone());
    Ok(())
}
