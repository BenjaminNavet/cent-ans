//! Historical objectives of the primary titles and generic victories of
//! the vassals (lot F3, spec § 4.8), plus the feudal phase of the turn.
//!
//! Generic victories, for a faction that started as a vassal:
//! - independence held `feudal_rules.independence_turns` turns;
//! - first vassal of the realm: the strongest (`faction_power`) of the
//!   direct vassals of the crown's holder, `feudal_rules.ascension_turns` turns;
//! - the crown above its 1337 primary title obtained.
//!
//! All the historical objectives of the primary title met also win.

use data_model::{FactionId, GameData, TitleId, TitleObjectiveCondition};

use super::{direct_vassals, holder_of, liege_of, ObjectiveProgress, MAX_DEPTH};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Topmost title above `title` (itself when sovereign).
pub(super) fn crown_above(data: &GameData, title: &TitleId) -> Option<TitleId> {
    let mut current = data.titles.get(title)?;
    for _ in 0..MAX_DEPTH {
        match current
            .de_jure_liege
            .as_ref()
            .and_then(|l| data.titles.get(l))
        {
            Some(liege) => current = liege,
            None => break,
        }
    }
    Some(current.id.clone())
}

/// Whether `faction` meets `condition`.
fn condition_met(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    condition: &TitleObjectiveCondition,
) -> bool {
    match condition {
        TitleObjectiveCondition::HoldTitle { title }
        | TitleObjectiveCondition::HoldCrown { title } => holder_of(state, title) == Some(faction),
        TitleObjectiveCondition::HoldProvinces { provinces } => provinces
            .iter()
            .all(|p| state.province_owner(p) == Some(faction)),
        TitleObjectiveCondition::BeIndependent => liege_of(state, data, faction).is_none(),
        TitleObjectiveCondition::BeLiegeOf { faction: vassal } => {
            liege_of(state, data, vassal).as_ref() == Some(faction)
        }
    }
}

/// Objectives of `faction`'s primary title with their status.
pub fn objective_status(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<ObjectiveProgress> {
    let Some(title) = state
        .feudal
        .primary
        .get(faction)
        .and_then(|t| data.titles.get(t))
    else {
        return Vec::new();
    };
    title
        .objectives
        .iter()
        .map(|objective| ObjectiveProgress {
            faction: faction.clone(),
            objective: objective.id.clone(),
            met: condition_met(state, data, faction, &objective.condition),
        })
        .collect()
}

/// See [`super::evaluate_objectives`].
pub(super) fn evaluate_objectives(
    state: &CampaignState,
    data: &GameData,
) -> Vec<ObjectiveProgress> {
    state
        .feudal
        .primary
        .keys()
        .filter(|f| state.factions.get(*f).is_some_and(|s| s.alive))
        .flat_map(|f| objective_status(state, data, f))
        .collect()
}

/// A generic (or historical) feudal victory of a faction (§ 4.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericVictory {
    /// Every historical objective of the primary title met.
    Objectives,
    Independence,
    FirstVassal,
    Crown,
}

impl GenericVictory {
    /// French text completing « Victoire ! <faction> … ».
    pub fn text_fr(self) -> &'static str {
        match self {
            GenericVictory::Objectives => "a accompli les objectifs de son titre",
            GenericVictory::Independence => "a tenu son indépendance",
            GenericVictory::FirstVassal => "s'est imposé comme premier vassal du royaume",
            GenericVictory::Crown => "a ceint la couronne de son suzerain",
        }
    }
}

/// Feudal victory reached by `faction`, if any.
pub fn generic_victory(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<GenericVictory> {
    if !state.factions.get(faction).is_some_and(|f| f.alive) {
        return None;
    }
    let objectives = objective_status(state, data, faction);
    if !objectives.is_empty() && objectives.iter().all(|o| o.met) {
        return Some(GenericVictory::Objectives);
    }
    let crown = state.feudal.start_crowns.get(faction)?;
    if holder_of(state, crown) == Some(faction) {
        return Some(GenericVictory::Crown);
    }
    let streaks = state
        .feudal
        .streaks
        .get(faction)
        .copied()
        .unwrap_or_default();
    let rules = &data.feudal_rules;
    if streaks.independent >= rules.independence_turns.max(1) {
        return Some(GenericVictory::Independence);
    }
    if streaks.first_vassal >= rules.ascension_turns.max(1) {
        return Some(GenericVictory::FirstVassal);
    }
    None
}

/// `faction` is the strongest direct vassal of the holder of `crown`.
fn is_first_vassal(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    crown: &TitleId,
) -> bool {
    let Some(sovereign) = holder_of(state, crown) else {
        return false;
    };
    let vassals = direct_vassals(state, data, sovereign);
    if !vassals.contains(faction) {
        return false;
    }
    let power = state.faction_power(faction);
    vassals
        .iter()
        .filter(|v| *v != faction)
        .all(|v| state.faction_power(v) < power)
}

/// Feudal phase of the turn: expired felony cases, alliances with a
/// liege's enemy, generic victory streaks, newly met objectives.
pub(crate) fn resolve_feudal(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    super::felony::expire_felonies(state);
    super::felony::detect_enemy_alliances(state, data);
    let starters: Vec<(FactionId, TitleId)> = state
        .feudal
        .start_crowns
        .iter()
        .map(|(f, c)| (f.clone(), c.clone()))
        .collect();
    for (faction, crown) in starters {
        if !state.factions.get(&faction).is_some_and(|f| f.alive) {
            state.feudal.streaks.remove(&faction);
            continue;
        }
        let independent = liege_of(state, data, &faction).is_none();
        let first = is_first_vassal(state, data, &faction, &crown);
        let streak = state.feudal.streaks.entry(faction).or_default();
        streak.independent = if independent {
            streak.independent + 1
        } else {
            0
        };
        streak.first_vassal = if first { streak.first_vassal + 1 } else { 0 };
    }
    for progress in evaluate_objectives(state, data) {
        let met = state
            .feudal
            .objectives_met
            .entry(progress.faction.clone())
            .or_default();
        if !progress.met {
            met.remove(&progress.objective);
            continue;
        }
        if !met.insert(progress.objective.clone()) {
            continue;
        }
        let title = state
            .feudal
            .primary
            .get(&progress.faction)
            .and_then(|t| data.titles.get(t))
            .and_then(|t| t.objectives.iter().find(|o| o.id == progress.objective))
            .map_or_else(|| progress.objective.clone(), |o| o.title.clone());
        events.push(
            GameEvent::new(
                EventKind::Diplomacy,
                format!(
                    "{} atteint son objectif : {title}.",
                    crate::diplomacy::faction_name(data, &progress.faction)
                ),
            )
            .faction(&progress.faction),
        );
    }
}
