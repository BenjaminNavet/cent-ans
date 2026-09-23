//! Chronicle: historical and random events (M10, `docs/design/m10-events.md`).
//!
//! Each end of turn (after religion, before population), [`resolve_chronicle`]:
//! 1. resolves player decisions that expired (first option applies);
//! 2. fires the historical events whose date is reached and whose conditions
//!    hold (at most once each; history may diverge);
//! 3. rolls the random events, at most one per faction and per turn;
//! 4. advances the Black Death wave ([`EventEffect::PlagueWave`]).
//!
//! An AI faction applies the option of highest `ai_weight` at once (ties
//! broken by the campaign RNG); the player gets a [`Decision`] answered with
//! the `choose_event_option` order. Every effect goes through
//! [`apply_effect`]; an effect naming an unknown id does nothing (the loader
//! already warned about it).

use std::collections::BTreeSet;

use data_model::{
    CharacterId, CharacterRef, ClaimKind, Condition, Event, EventCategory, EventEffect, EventId,
    EventScope, EventSeason, FactionId, GameData, ProvinceId, ProvinceRef, SocialClass,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{faction_name, Claim, REBELS_FACTION};
use crate::events::{EventKind, GameEvent};
use crate::population::weighted_unrest;
use crate::state::{Army, CampaignState, Season, Stance, Unit, TURNS_PER_YEAR};
use crate::{characters, religion, skills};

/// Turns a player decision stays open before its first option applies.
pub const DECISION_TURNS: u32 = 2;
/// Default duration (turns) of an `opinion` effect.
pub const OPINION_TURNS: u32 = 20;
/// Truce (turns) after a `peace` effect: five years, as a negotiated peace (M5).
pub const PEACE_TRUCE_TURNS: u32 = 20;
/// Black Death, per province struck: health lost by every class.
pub const PLAGUE_HEALTH_LOSS: i32 = 30;
/// Black Death: population loss range, in percent.
pub const PLAGUE_POPULATION_LOSS: (u32, u32) = (15, 30);
/// Black Death: unrest added to every class.
pub const PLAGUE_UNREST: i32 = 20;

/// A choice waiting for the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub id: u32,
    pub event: EventId,
    pub faction: FactionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    /// Indices of the event's options offered.
    pub options: Vec<usize>,
    /// At the end of this turn, the first option applies automatically.
    pub expires_turn: u32,
}

/// Black Death in progress: every province is struck once, south first,
/// over `duration` turns starting at `start_turn`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlagueWave {
    pub start_turn: u32,
    pub duration: u32,
}

/// Chronicle part of the campaign state (M10; `#[serde(default)]` keeps
/// older saves loadable).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChronicleState {
    /// Historical events already fired.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub fired_events: BTreeSet<EventId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_decisions: Vec<Decision>,
    #[serde(default)]
    pub next_decision_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague_wave: Option<PlagueWave>,
    /// No new event fires while set (tests and controlled experiments);
    /// pending decisions still expire and the plague wave still spreads.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    /// F1: events scheduled by `schedule_event` effects (chains).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scheduled: Vec<ScheduledEvent>,
}

/// An event programmed by another one (F1), fired at the end of `turn` for
/// the same faction and province.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub event: EventId,
    pub turn: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
}

/// Why a `choose_event_option` order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChronicleError {
    #[error("décision inconnue : {0}")]
    UnknownDecision(u32),
    #[error("choix invalide : {0}")]
    InvalidOption(usize),
}

/// Faction and province an event is about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventContext {
    pub faction: Option<FactionId>,
    pub province: Option<ProvinceId>,
}

/// One option of a decision, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionOptionView {
    pub index: usize,
    pub text: String,
    /// French summary of the effects, one per line.
    pub effects_text: String,
}

/// A pending decision, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionView {
    pub id: u32,
    pub event: EventId,
    pub title: String,
    pub text: String,
    pub historical: bool,
    pub options: Vec<DecisionOptionView>,
    /// End-of-turns left before the first option applies (1 = this turn).
    pub expires_in: u32,
    pub province: Option<ProvinceId>,
    pub province_name: String,
}

fn season_of(season: Season) -> EventSeason {
    match season {
        Season::Spring => EventSeason::Spring,
        Season::Summer => EventSeason::Summer,
        Season::Autumn => EventSeason::Autumn,
        Season::Winter => EventSeason::Winter,
    }
}

fn is_rebels(faction: &FactionId) -> bool {
    faction.as_str() == REBELS_FACTION
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

// =========================================================================
// Conditions
// =========================================================================

impl CampaignState {
    fn faction_alive(&self, faction: &FactionId) -> bool {
        self.factions.get(faction).is_some_and(|f| f.alive)
    }

    fn ruler_state(&self, faction: &FactionId) -> Option<&crate::state::CharacterState> {
        let ruler = self.factions.get(faction)?.ruler.as_ref()?;
        self.characters.get(ruler).filter(|c| c.alive)
    }

    /// Evaluates one condition in `ctx`; a condition that needs a faction or
    /// a province the context lacks is false.
    pub fn condition_holds(
        &self,
        data: &GameData,
        condition: &Condition,
        ctx: &EventContext,
    ) -> bool {
        let scope_faction = |explicit: &Option<FactionId>| explicit.clone().or(ctx.faction.clone());
        let scope_province =
            |explicit: &Option<ProvinceId>| explicit.clone().or(ctx.province.clone());
        match condition {
            Condition::FactionExists { faction } => self.faction_alive(faction),
            Condition::FactionIsPlayer { faction } => {
                &self.player_faction == faction && self.faction_alive(faction)
            }
            Condition::AtWar { a, b } => {
                let Some(a) = scope_faction(a) else {
                    return false;
                };
                match b {
                    Some(b) => self.is_at_war(&a, b),
                    None => self
                        .factions
                        .get(&a)
                        .is_some_and(|f| f.at_war_with.iter().any(|enemy| !is_rebels(enemy))),
                }
            }
            Condition::Controls { faction, province } => scope_faction(faction).is_some_and(|f| {
                self.provinces
                    .get(province)
                    .is_some_and(|p| p.controller == f)
            }),
            Condition::CharacterAlive { id } => self.characters.get(id).is_some_and(|c| c.alive),
            Condition::CharacterCaptive { id } => self
                .characters
                .get(id)
                .is_some_and(|c| c.alive && c.captive),
            Condition::RulerIs { faction, character } => {
                self.factions
                    .get(faction)
                    .is_some_and(|f| f.ruler.as_ref() == Some(character))
                    && self.characters.get(character).is_some_and(|c| c.alive)
            }
            Condition::RulerTrait { faction, trait_id } => scope_faction(faction)
                .and_then(|f| self.ruler_state(&f))
                .is_some_and(|ruler| ruler.traits.contains(trait_id)),
            Condition::RulerHouse { faction, house } => scope_faction(faction)
                .and_then(|f| self.ruler_state(&f))
                .is_some_and(|ruler| &ruler.house == house),
            Condition::RulerAgeBetween { faction, min, max } => scope_faction(faction)
                .and_then(|f| self.ruler_state(&f))
                .is_some_and(|ruler| (*min..=*max).contains(&ruler.age(self.year))),
            Condition::YearBetween { from, to } => (*from..=*to).contains(&self.year),
            Condition::ProvinceUnrestAbove { province, amount } => scope_province(province)
                .and_then(|p| self.provinces.get(&p))
                .is_some_and(|p| weighted_unrest(&p.population) > f64::from(*amount)),
            Condition::ProvinceBesieged { province, by } => scope_province(province)
                .and_then(|p| self.provinces.get(&p))
                .and_then(|p| p.siege.as_ref())
                .is_some_and(|siege| by.as_ref().is_none_or(|by| &siege.attacker == by)),
            Condition::ProvinceCoastal { province } => scope_province(province)
                .and_then(|p| data.provinces.get(&p))
                .is_some_and(|p| p.coastal),
            Condition::TreasuryAbove { faction, amount } => scope_faction(faction)
                .and_then(|f| self.factions.get(&f))
                .is_some_and(|f| f.treasury > *amount),
            Condition::ProvincesBelow { faction, count } => {
                scope_faction(faction).is_some_and(|f| {
                    let controlled = self
                        .provinces
                        .values()
                        .filter(|p| p.controller == f)
                        .count();
                    controlled < *count as usize
                })
            }
            Condition::ReligionIs { faction, religion } => {
                scope_faction(faction).is_some_and(|f| {
                    religion::faction_religion(self, data, &f).as_ref() == Some(religion)
                })
            }
            Condition::Schism { active } => self.schism == *active,
            Condition::NotFired { event } => !self.chronicle.fired_events.contains(event),
            Condition::Fired { event } => self.chronicle.fired_events.contains(event),
            Condition::Season { season } => season_of(self.season) == *season,
            Condition::AnyOf { conditions } => conditions
                .iter()
                .any(|inner| self.condition_holds(data, inner, ctx)),
        }
    }

    /// `true` when every trigger condition of `event` holds in `ctx`.
    pub fn event_conditions_hold(
        &self,
        data: &GameData,
        event: &Event,
        ctx: &EventContext,
    ) -> bool {
        event
            .trigger
            .conditions
            .iter()
            .all(|condition| self.condition_holds(data, condition, ctx))
    }

    /// `true` once the date of a historical event is reached and not passed.
    fn historical_window_open(&self, event: &Event) -> bool {
        let Some(date) = &event.trigger.date else {
            return false;
        };
        let now = (self.year, season_of(self.season).index());
        let start = (date.year, date.season.map_or(0, EventSeason::index));
        let until = event.trigger.until_year.unwrap_or(date.year + 2);
        now >= start && self.year <= until
    }

    /// Pending decisions of `faction`, for the UI.
    pub fn decision_views(&self, data: &GameData, faction: &FactionId) -> Vec<DecisionView> {
        self.chronicle
            .pending_decisions
            .iter()
            .filter(|d| &d.faction == faction)
            .filter_map(|decision| {
                let event = data.events.get(&decision.event)?;
                let ctx = EventContext {
                    faction: Some(decision.faction.clone()),
                    province: decision.province.clone(),
                };
                let options = decision
                    .options
                    .iter()
                    .filter_map(|&index| {
                        let option = event.options.get(index)?;
                        let effects_text = option
                            .effects
                            .iter()
                            .map(|effect| self.describe_effect(data, effect, &ctx))
                            .filter(|line| !line.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n");
                        Some(DecisionOptionView {
                            index,
                            text: option.text.clone(),
                            effects_text,
                        })
                    })
                    .collect();
                Some(DecisionView {
                    id: decision.id,
                    event: decision.event.clone(),
                    title: event.title.clone(),
                    text: event.text.clone(),
                    // Chained events (F1) follow historical ones.
                    historical: event.kind != EventCategory::Random,
                    options,
                    expires_in: (decision.expires_turn + 1).saturating_sub(self.turn),
                    province: decision.province.clone(),
                    province_name: decision
                        .province
                        .as_ref()
                        .map(|p| province_name(data, p))
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    /// Applies the player's (or `faction`'s) choice for a pending decision
    /// (order `choose_event_option`).
    pub fn choose_event_option(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        decision: u32,
        option: usize,
    ) -> Result<(), ChronicleError> {
        let position = self
            .chronicle
            .pending_decisions
            .iter()
            .position(|d| d.id == decision && &d.faction == faction)
            .ok_or(ChronicleError::UnknownDecision(decision))?;
        if !self.chronicle.pending_decisions[position]
            .options
            .contains(&option)
        {
            return Err(ChronicleError::InvalidOption(option));
        }
        let decision = self.chronicle.pending_decisions.remove(position);
        let mut events = Vec::new();
        resolve_decision(self, data, &decision, option, false, &mut events);
        for event in events {
            self.push_order_event(event);
        }
        Ok(())
    }

    /// French one-line summary of an effect (tooltips).
    pub fn describe_effect(
        &self,
        data: &GameData,
        effect: &EventEffect,
        ctx: &EventContext,
    ) -> String {
        let signed = |value: i64| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };
        let faction_label = |explicit: &Option<FactionId>| {
            explicit
                .as_ref()
                .filter(|f| Some(*f) != ctx.faction.as_ref())
                .map(|f| format!(" ({})", faction_name(data, f)))
                .unwrap_or_default()
        };
        let where_ = |target: &Option<ProvinceRef>| match (target, &ctx.province) {
            (Some(ProvinceRef::All), _) | (None, None) => " (toutes vos provinces)".to_owned(),
            (Some(ProvinceRef::Id(p)), _) | (None, Some(p)) => {
                format!(" ({})", province_name(data, p))
            }
        };
        let who = |character: &CharacterRef, faction: &Option<FactionId>| {
            let faction = faction.clone().or(ctx.faction.clone());
            match self.resolve_character(character, faction.as_ref()) {
                Some(id) => format!(" ({})", self.character_name(data, &id)),
                None => String::new(),
            }
        };
        match effect {
            EventEffect::Treasury { faction, amount } => {
                format!(
                    "Trésor {} livres{}",
                    signed(*amount),
                    faction_label(faction)
                )
            }
            EventEffect::Unrest { province, amount } => {
                format!(
                    "Mécontentement {}{}",
                    signed(i64::from(*amount)),
                    where_(province)
                )
            }
            EventEffect::Population { province, percent } => format!(
                "Population {} %{}",
                signed(i64::from(*percent)),
                where_(province)
            ),
            EventEffect::Health { province, amount } => {
                format!("Santé {}{}", signed(i64::from(*amount)), where_(province))
            }
            EventEffect::Wealth { province, amount } => {
                format!(
                    "Richesse {}{}",
                    signed(i64::from(*amount)),
                    where_(province)
                )
            }
            EventEffect::Devastation { province, amount } => {
                format!(
                    "Dévastation {}{}",
                    signed(i64::from(*amount)),
                    where_(province)
                )
            }
            EventEffect::Prestige {
                character,
                faction,
                amount,
            } => format!(
                "Prestige {}{}",
                signed(i64::from(*amount)),
                who(character, faction)
            ),
            EventEffect::Piety {
                character,
                faction,
                amount,
            } => format!(
                "Piété {}{}",
                signed(i64::from(*amount)),
                who(character, faction)
            ),
            EventEffect::PapalFavor { faction, amount } => format!(
                "Faveur pontificale {}{}",
                signed(i64::from(*amount)),
                faction_label(faction)
            ),
            EventEffect::Opinion {
                faction,
                towards,
                amount,
                ..
            } => {
                let towards = towards
                    .as_ref()
                    .or(ctx.faction.as_ref())
                    .map(|f| format!(" envers {}", faction_name(data, f)))
                    .unwrap_or_default();
                format!(
                    "Attitude de {}{} {}",
                    faction_name(data, faction),
                    towards,
                    signed(i64::from(*amount))
                )
            }
            EventEffect::DeclareWar { a, b } => format!(
                "Guerre : {} contre {}",
                faction_name(data, a),
                faction_name(data, b)
            ),
            EventEffect::Peace { a, b } => format!(
                "Paix entre {} et {}",
                faction_name(data, a),
                faction_name(data, b)
            ),
            EventEffect::AddTrait {
                character,
                faction,
                trait_id,
            } => {
                let name = data
                    .traits
                    .get(trait_id)
                    .map_or_else(|| trait_id.to_string(), |t| t.name.display.clone());
                format!("Trait « {name} »{}", who(character, faction))
            }
            EventEffect::KillCharacter { id, faction } => {
                format!("Mort{}", who(id, faction))
            }
            EventEffect::SpawnArmy {
                faction,
                province,
                units,
            } => {
                let place = province
                    .as_ref()
                    .or(ctx.province.as_ref())
                    .map(|p| format!(" en {}", province_name(data, p)))
                    .unwrap_or_default();
                format!(
                    "Nouvelle armée de {} unités{}{}",
                    units.len(),
                    place,
                    faction_label(faction)
                )
            }
            EventEffect::Claim {
                faction,
                kind,
                target,
            } => {
                let target_name = match kind {
                    ClaimKind::Throne => FactionId::new(target.clone())
                        .map(|f| format!("le trône de {}", faction_name(data, &f)))
                        .unwrap_or_else(|_| target.clone()),
                    ClaimKind::Province => ProvinceId::new(target.clone())
                        .map(|p| province_name(data, &p))
                        .unwrap_or_else(|_| target.clone()),
                };
                format!("Prétention sur {target_name}{}", faction_label(faction))
            }
            EventEffect::Loyalty { vassal, amount } => {
                let who = vassal
                    .as_ref()
                    .map(|v| format!(" de {}", faction_name(data, v)))
                    .unwrap_or_else(|| " des vassaux".to_owned());
                format!("Loyauté{who} {}", signed(i64::from(*amount)))
            }
            EventEffect::PlagueWave { .. } => {
                "La peste gagne toutes les provinces, du sud vers le nord (santé, population, \
                 mécontentement)"
                    .to_owned()
            }
            EventEffect::CaptureCharacter {
                id,
                faction,
                captor,
            } => format!(
                "Captivité{} aux mains de {}",
                who(id, faction),
                faction_name(data, captor)
            ),
            EventEffect::ReleaseCharacter {
                id,
                faction,
                ransom,
            } => {
                if *ransom > 0 {
                    format!(
                        "Libération{} contre {ransom} livres de rançon",
                        who(id, faction)
                    )
                } else {
                    format!("Libération{}", who(id, faction))
                }
            }
            EventEffect::ScheduleEvent { event, delay } => {
                let title = data
                    .events
                    .get(event)
                    .map_or_else(|| event.to_string(), |e| e.title.clone());
                format!("Suite : « {title} » dans {delay} saison(s)")
            }
            EventEffect::Marry { a, b } => format!(
                "Mariage de {} et {}",
                self.character_name(data, a),
                self.character_name(data, b)
            ),
        }
    }

    fn resolve_character(
        &self,
        character: &CharacterRef,
        faction: Option<&FactionId>,
    ) -> Option<CharacterId> {
        let id = match character {
            CharacterRef::Ruler => self.factions.get(faction?)?.ruler.clone()?,
            CharacterRef::Heir => self.factions.get(faction?)?.heir.clone()?,
            CharacterRef::Id(id) => id.clone(),
        };
        self.characters.get(&id).filter(|c| c.alive).map(|_| id)
    }

    /// Provinces targeted by a province effect.
    fn effect_provinces(
        &self,
        target: &Option<ProvinceRef>,
        ctx: &EventContext,
    ) -> Vec<ProvinceId> {
        let all = || match &ctx.faction {
            Some(faction) => self
                .provinces
                .iter()
                .filter(|(_, p)| &p.controller == faction)
                .map(|(id, _)| id.clone())
                .collect(),
            None => Vec::new(),
        };
        match (target, &ctx.province) {
            (Some(ProvinceRef::All), _) | (None, None) => all(),
            (Some(ProvinceRef::Id(p)), _) | (None, Some(p)) => {
                if self.provinces.contains_key(p) {
                    vec![p.clone()]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

// =========================================================================
// Effects
// =========================================================================

fn add_clamped(value: u8, delta: i32) -> u8 {
    (i32::from(value) + delta).clamp(0, 100) as u8
}

fn for_each_class(
    state: &mut CampaignState,
    province: &ProvinceId,
    mut apply: impl FnMut(&mut data_model::PopulationClass),
) {
    let Some(p) = state.provinces.get_mut(province) else {
        return;
    };
    for class in SocialClass::ALL {
        let entry = match class {
            SocialClass::Peasants => &mut p.population.peasants,
            SocialClass::Burghers => &mut p.population.burghers,
            SocialClass::Clergy => &mut p.population.clergy,
            SocialClass::Nobility => &mut p.population.nobility,
        };
        apply(entry);
    }
}

/// Applies one effect in `ctx` (the deciding faction and the event's
/// province). Unknown ids and impossible actions are ignored.
pub fn apply_effect(
    state: &mut CampaignState,
    data: &GameData,
    effect: &EventEffect,
    ctx: &EventContext,
    events: &mut Vec<GameEvent>,
) {
    let target_faction = |explicit: &Option<FactionId>| explicit.clone().or(ctx.faction.clone());
    match effect {
        EventEffect::Treasury { faction, amount } => {
            if let Some(f) = target_faction(faction).and_then(|f| state.factions.get_mut(&f)) {
                f.treasury += amount;
            }
        }
        EventEffect::Unrest { province, amount } => {
            for id in state.effect_provinces(province, ctx) {
                for_each_class(state, &id, |c| c.unrest = add_clamped(c.unrest, *amount));
            }
        }
        EventEffect::Health { province, amount } => {
            for id in state.effect_provinces(province, ctx) {
                for_each_class(state, &id, |c| c.health = add_clamped(c.health, *amount));
            }
        }
        EventEffect::Wealth { province, amount } => {
            for id in state.effect_provinces(province, ctx) {
                for_each_class(state, &id, |c| c.wealth = add_clamped(c.wealth, *amount));
            }
        }
        EventEffect::Population { province, percent } => {
            let factor = f64::from((100 + percent).max(0)) / 100.0;
            for id in state.effect_provinces(province, ctx) {
                for_each_class(state, &id, |c| {
                    c.count = (c.count as f64 * factor).round() as u64;
                });
            }
        }
        EventEffect::Devastation { province, amount } => {
            for id in state.effect_provinces(province, ctx) {
                if let Some(p) = state.provinces.get_mut(&id) {
                    p.devastation = add_clamped(p.devastation, *amount);
                }
            }
        }
        EventEffect::Prestige {
            character,
            faction,
            amount,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(character, faction.as_ref()) {
                if let Some(c) = state.characters.get_mut(&id) {
                    c.prestige += amount;
                }
            }
        }
        EventEffect::Piety {
            character,
            faction,
            amount,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(character, faction.as_ref()) {
                if let Some(c) = state.characters.get_mut(&id) {
                    c.piety = add_clamped(c.piety, *amount);
                }
            }
        }
        EventEffect::PapalFavor { faction, amount } => {
            if let Some(f) = target_faction(faction) {
                religion::change_favor(state, &f, *amount);
            }
        }
        EventEffect::Opinion {
            faction,
            towards,
            amount,
            reason,
            duration,
        } => {
            let Some(towards) = towards.clone().or(ctx.faction.clone()) else {
                return;
            };
            if faction != &towards
                && state.faction_alive(faction)
                && state.factions.contains_key(&towards)
            {
                state.add_modifier(
                    faction,
                    &towards,
                    *amount,
                    reason,
                    duration.unwrap_or(OPINION_TURNS),
                );
            }
        }
        EventEffect::DeclareWar { a, b } => {
            if state.faction_alive(a) && state.faction_alive(b) && !state.is_at_war(a, b) {
                let _ = state.declare_war(data, a, b);
            }
        }
        EventEffect::Peace { a, b } => {
            if state.is_at_war(a, b) {
                state.make_peace(data, a, b, &[], 0, PEACE_TRUCE_TURNS);
            }
        }
        EventEffect::AddTrait {
            character,
            faction,
            trait_id,
        } => {
            let faction = target_faction(faction);
            if !data.traits.contains_key(trait_id) {
                return;
            }
            if let Some(id) = state.resolve_character(character, faction.as_ref()) {
                if skills::grant_trait(state, data, &id, trait_id) {
                    let name = &data.traits[trait_id].name.display;
                    let owner = state.characters[&id].faction.clone();
                    events.push(
                        GameEvent::new(
                            EventKind::TraitAcquired,
                            format!("{} devient « {name} ».", state.character_name(data, &id)),
                        )
                        .faction(&owner),
                    );
                }
            }
        }
        EventEffect::KillCharacter { id, faction } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                characters::kill(state, data, &id, events);
            }
        }
        EventEffect::SpawnArmy {
            faction,
            province,
            units,
        } => {
            let Some(faction) = target_faction(faction).filter(|f| state.faction_alive(f)) else {
                return;
            };
            let location = province
                .clone()
                .or(ctx.province.clone())
                .or_else(|| state.factions.get(&faction).map(|f| f.capital.clone()))
                .filter(|p| state.provinces.contains_key(p));
            let Some(location) = location else {
                return;
            };
            let units: Vec<Unit> = units
                .iter()
                .filter_map(|u| data.unit_types.get(u))
                .map(Unit::fresh)
                .collect();
            if units.is_empty() {
                return;
            }
            let id = state.allocate_army_id();
            let movement_points = state.season.movement_points();
            state.armies.insert(
                id.clone(),
                Army {
                    faction: faction.clone(),
                    general: None,
                    location: location.clone(),
                    units,
                    movement_points,
                    supply: 100,
                    stance: Stance::Normal,
                    path: Vec::new(),
                },
            );
            events.push(
                GameEvent::new(
                    EventKind::Chronicle,
                    format!(
                        "Une nouvelle armée de {} se lève en {}.",
                        faction_name(data, &faction),
                        province_name(data, &location)
                    ),
                )
                .faction(&faction)
                .province(&location)
                .army(&id),
            );
        }
        EventEffect::Claim {
            faction,
            kind,
            target,
        } => {
            let Some(holder) = target_faction(faction) else {
                return;
            };
            let (claim_faction, claim_province) = match kind {
                ClaimKind::Throne => (FactionId::new(target.clone()).ok(), None),
                ClaimKind::Province => (None, ProvinceId::new(target.clone()).ok()),
            };
            let known = claim_faction
                .as_ref()
                .is_some_and(|f| state.factions.contains_key(f))
                || claim_province
                    .as_ref()
                    .is_some_and(|p| state.provinces.contains_key(p));
            let Some(f) = state.factions.get_mut(&holder).filter(|_| known) else {
                return;
            };
            let duplicate = f.claims.iter().any(|c| {
                c.kind == *kind && c.faction == claim_faction && c.province == claim_province
            });
            if !duplicate {
                f.claims.push(Claim {
                    kind: *kind,
                    faction: claim_faction,
                    province: claim_province,
                    text_fr: "prétention née de la chronique".to_owned(),
                    expires_turn: None,
                });
            }
        }
        EventEffect::Loyalty { vassal, amount } => {
            let vassals: Vec<FactionId> = match vassal {
                Some(v) => vec![v.clone()],
                None => match &ctx.faction {
                    Some(suzerain) => state
                        .factions
                        .iter()
                        .filter(|(_, f)| f.suzerain.as_ref() == Some(suzerain))
                        .map(|(id, _)| id.clone())
                        .collect(),
                    None => Vec::new(),
                },
            };
            for v in vassals {
                if let Some(f) = state.factions.get_mut(&v) {
                    f.loyalty = add_clamped(f.loyalty, *amount);
                }
            }
        }
        EventEffect::CaptureCharacter {
            id,
            faction,
            captor,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                capture_character(state, data, &id, captor, events);
            }
        }
        EventEffect::ReleaseCharacter {
            id,
            faction,
            ransom,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                release_character(state, data, &id, *ransom, events);
            }
        }
        EventEffect::ScheduleEvent { event, delay } => {
            if data.events.contains_key(event) {
                state.chronicle.scheduled.push(ScheduledEvent {
                    event: event.clone(),
                    turn: state.turn + (*delay).max(1),
                    faction: ctx.faction.clone(),
                    province: ctx.province.clone(),
                });
            }
        }
        EventEffect::Marry { a, b } => {
            marry(state, data, a, b, events);
        }
        EventEffect::PlagueWave { from_year, to_year } => {
            if state.chronicle.plague_wave.is_none() {
                let years = (to_year - from_year).max(0) as u32;
                state.chronicle.plague_wave = Some(PlagueWave {
                    start_turn: state.turn,
                    duration: (years * TURNS_PER_YEAR).max(1),
                });
            }
        }
    }
}

/// F1 `capture_character`: `id` becomes the prisoner of `captor` (it
/// leaves its army and its governorship; a ruler keeps the crown).
pub fn capture_character(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    captor: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let Some(c) = state.characters.get(id).filter(|c| c.alive && !c.captive) else {
        return;
    };
    if &c.faction == captor || !state.factions.contains_key(captor) {
        return;
    }
    let owner = c.faction.clone();
    state.detach_general(id);
    let c = state.characters.get_mut(id).expect("checked above");
    c.captive = true;
    c.captor = Some(captor.clone());
    c.governor_of = None;
    c.location = state.factions.get(captor).map(|f| f.capital.clone());
    events.push(
        GameEvent::new(
            EventKind::GeneralCaptured,
            format!(
                "{} est retenu prisonnier par {}.",
                state.character_name(data, id),
                faction_name(data, captor)
            ),
        )
        .faction(&owner),
    );
}

/// F1 `release_character`: a captive is freed against `ransom` livres paid
/// by its faction to its captor, and returns to its capital.
pub fn release_character(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    ransom: i64,
    events: &mut Vec<GameEvent>,
) {
    let Some(c) = state.characters.get(id).filter(|c| c.alive && c.captive) else {
        return;
    };
    let owner = c.faction.clone();
    let captor = c.captor.clone();
    let ransom = ransom.max(0);
    if ransom > 0 {
        if let Some(f) = state.factions.get_mut(&owner) {
            f.treasury -= ransom;
        }
        if let Some(f) = captor.as_ref().and_then(|c| state.factions.get_mut(c)) {
            f.treasury += ransom;
        }
    }
    let capital = state.factions.get(&owner).map(|f| f.capital.clone());
    let c = state.characters.get_mut(id).expect("checked above");
    c.captive = false;
    c.captor = None;
    c.location = capital;
    crate::dynasty::on_ransomed(state, data, id);
    let text = if ransom > 0 {
        format!(
            "{} est libéré contre une rançon de {ransom} livres.",
            state.character_name(data, id)
        )
    } else {
        format!("{} est libéré.", state.character_name(data, id))
    };
    events.push(GameEvent::new(EventKind::Chronicle, text).faction(&owner));
}

/// F1 `marry`: a historical marriage between two living, unmarried
/// characters (no-op otherwise: history diverged).
fn marry(
    state: &mut CampaignState,
    data: &GameData,
    a: &CharacterId,
    b: &CharacterId,
    events: &mut Vec<GameEvent>,
) {
    let free = |id: &CharacterId| {
        state
            .characters
            .get(id)
            .is_some_and(|c| c.alive && c.spouse.is_none())
    };
    if a == b || !free(a) || !free(b) {
        return;
    }
    for (x, y) in [(a, b), (b, a)] {
        let c = state.characters.get_mut(x).expect("checked above");
        c.spouse = Some(y.clone());
        c.prestige += crate::dynasty::PRESTIGE_MARRIAGE;
    }
    let faction = state.characters[a].faction.clone();
    events.push(
        GameEvent::new(
            EventKind::Chronicle,
            format!(
                "Mariage de {} et de {}.",
                state.character_name(data, a),
                state.character_name(data, b)
            ),
        )
        .faction(&faction),
    );
}

// =========================================================================
// Turn phase
// =========================================================================

/// Applies option `option` of the decision's event and journals it.
fn resolve_decision(
    state: &mut CampaignState,
    data: &GameData,
    decision: &Decision,
    option: usize,
    expired: bool,
    events: &mut Vec<GameEvent>,
) {
    let Some(event) = data.events.get(&decision.event) else {
        return;
    };
    let ctx = EventContext {
        faction: Some(decision.faction.clone()),
        province: decision.province.clone(),
    };
    apply_option(state, data, event, option, &ctx, events);
    let chosen = event.options.get(option).map_or("", |o| o.text.as_str());
    let suffix = if expired { " (délai écoulé)" } else { "" };
    let mut entry = GameEvent::new(
        EventKind::Chronicle,
        format!("{} : {chosen}{suffix}.", event.title),
    )
    .faction(&decision.faction);
    if let Some(p) = &decision.province {
        entry = entry.province(p);
    }
    events.push(entry);
}

fn apply_option(
    state: &mut CampaignState,
    data: &GameData,
    event: &Event,
    option: usize,
    ctx: &EventContext,
    events: &mut Vec<GameEvent>,
) {
    let Some(option) = event.options.get(option) else {
        return;
    };
    // H4: medicine softens the harm of a local epidemic.
    let resistance = match &ctx.province {
        Some(province) if crate::medicine::is_epidemic(&event.id) => {
            crate::medicine::plague_resistance(state, data, province)
        }
        _ => 0.0,
    };
    for effect in &option.effects {
        let effect = if resistance > 0.0 {
            mitigate_effect(effect, resistance)
        } else {
            effect.clone()
        };
        apply_effect(state, data, &effect, ctx, events);
    }
}

/// `effect` with its health and population harm scaled by `1 - resistance`.
fn mitigate_effect(effect: &EventEffect, resistance: f64) -> EventEffect {
    let mut effect = effect.clone();
    match &mut effect {
        EventEffect::Health { amount, .. } => {
            *amount = crate::medicine::mitigated(*amount, resistance);
        }
        EventEffect::Population { percent, .. } => {
            *percent = crate::medicine::mitigated(*percent, resistance);
        }
        _ => {}
    }
    effect
}

/// Option of highest `ai_weight`, ties broken by the campaign RNG.
pub fn ai_choice(state: &mut CampaignState, event: &Event) -> usize {
    let best = event.options.iter().map(|o| o.ai_weight).max().unwrap_or(0);
    let candidates: Vec<usize> = event
        .options
        .iter()
        .enumerate()
        .filter(|(_, o)| o.ai_weight == best)
        .map(|(i, _)| i)
        .collect();
    match candidates.len() {
        0 => 0,
        1 => candidates[0],
        n => candidates[state.rng.below(n as u32) as usize],
    }
}

/// Fires `event` for `decider` (None: nobody decides, the AI weights apply).
fn fire(
    state: &mut CampaignState,
    data: &GameData,
    event: &Event,
    decider: Option<FactionId>,
    province: Option<ProvinceId>,
    events: &mut Vec<GameEvent>,
) {
    // H4: a local epidemic may be contained before it spreads.
    if let Some(p) = province
        .as_ref()
        .filter(|_| crate::medicine::is_epidemic(&event.id))
    {
        let resistance = crate::medicine::plague_resistance(state, data, p);
        if resistance > 0.0 && state.rng.unit_f64() < resistance {
            if decider.as_ref() == Some(&state.player_faction) {
                events.push(
                    GameEvent::new(
                        EventKind::Medicine,
                        format!(
                            "Une fièvre s'est déclarée à {} : les médecins l'ont circonscrite.",
                            province_name(data, p)
                        ),
                    )
                    .province(p)
                    .faction(&state.player_faction),
                );
            }
            return;
        }
    }
    let historical = event.kind != EventCategory::Random;
    let player_decides = decider
        .as_ref()
        .is_some_and(|f| f == &state.player_faction && state.faction_alive(f));
    if player_decides {
        let faction = decider.expect("checked");
        let id = state.chronicle.next_decision_id.max(1);
        state.chronicle.next_decision_id = id + 1;
        state.chronicle.pending_decisions.push(Decision {
            id,
            event: event.id.clone(),
            faction: faction.clone(),
            province: province.clone(),
            options: (0..event.options.len()).collect(),
            expires_turn: state.turn + DECISION_TURNS,
        });
        let mut entry = GameEvent::new(
            EventKind::Chronicle,
            format!("Chronique : {}.", event.title),
        )
        .faction(&faction);
        if let Some(p) = &province {
            entry = entry.province(p);
        }
        events.push(entry);
        return;
    }
    let option = ai_choice(state, event);
    let ctx = EventContext {
        faction: decider.clone(),
        province: province.clone(),
    };
    apply_option(state, data, event, option, &ctx, events);
    // Random events of AI factions stay out of the journal (noise).
    if historical {
        let who = decider
            .as_ref()
            .map(|f| format!(" ({})", faction_name(data, f)))
            .unwrap_or_default();
        let chosen = event.options.get(option).map_or("", |o| o.text.as_str());
        let mut entry = GameEvent::new(
            EventKind::Chronicle,
            format!("{}{who} : {chosen}.", event.title),
        );
        if let Some(f) = &decider {
            entry = entry.faction(f);
        }
        if let Some(p) = &province {
            entry = entry.province(p);
        }
        events.push(entry);
    }
}

/// Factions that can receive events (alive, not the virtual rebels).
fn living_factions(state: &CampaignState) -> Vec<FactionId> {
    state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && !is_rebels(id))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Provinces matching a province-scoped event, restricted to `faction`'s.
fn matching_provinces(
    state: &CampaignState,
    data: &GameData,
    event: &Event,
    faction: Option<&FactionId>,
) -> Vec<ProvinceId> {
    state
        .provinces
        .iter()
        .filter(|(_, p)| faction.is_none_or(|f| &p.controller == f) && !is_rebels(&p.controller))
        .filter(|(id, p)| {
            let ctx = EventContext {
                faction: Some(p.controller.clone()),
                province: Some((*id).clone()),
            };
            state.event_conditions_hold(data, event, &ctx)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Targets `(decider, province)` of an event this turn, in a stable order.
/// `faction`: restrict to this faction (random events are rolled per faction).
fn targets(
    state: &mut CampaignState,
    data: &GameData,
    event: &Event,
    faction: Option<&FactionId>,
) -> Vec<(Option<FactionId>, Option<ProvinceId>)> {
    match &event.scope {
        EventScope::Global => {
            let ctx = EventContext::default();
            if faction.is_none() && state.event_conditions_hold(data, event, &ctx) {
                let player = state.player_faction.clone();
                let decider = state.faction_alive(&player).then_some(player);
                vec![(decider, None)]
            } else {
                Vec::new()
            }
        }
        EventScope::Faction { faction: target } => {
            let candidates: Vec<FactionId> = match (target, faction) {
                (Some(t), Some(f)) if t != f => Vec::new(),
                (Some(t), _) => vec![t.clone()],
                (None, Some(f)) => vec![f.clone()],
                (None, None) => living_factions(state),
            };
            candidates
                .into_iter()
                .filter(|f| state.faction_alive(f))
                .filter(|f| {
                    let ctx = EventContext {
                        faction: Some(f.clone()),
                        province: None,
                    };
                    state.event_conditions_hold(data, event, &ctx)
                })
                .map(|f| (Some(f), None))
                .collect()
        }
        EventScope::Province {
            faction: scope_faction,
            province,
        } => {
            let restrict = match (scope_faction, faction) {
                (Some(s), Some(f)) if s != f => return Vec::new(),
                (Some(s), _) => Some(s.clone()),
                (None, f) => f.cloned(),
            };
            let chosen = match province {
                Some(p) => {
                    let Some(controller) = state.provinces.get(p).map(|s| s.controller.clone())
                    else {
                        return Vec::new();
                    };
                    let allowed = restrict.as_ref().is_none_or(|r| r == &controller)
                        && !is_rebels(&controller);
                    let ctx = EventContext {
                        faction: Some(controller),
                        province: Some(p.clone()),
                    };
                    (allowed && state.event_conditions_hold(data, event, &ctx)).then(|| p.clone())
                }
                None => {
                    let matching = matching_provinces(state, data, event, restrict.as_ref());
                    if matching.is_empty() {
                        None
                    } else {
                        let index = state.rng.below(matching.len() as u32) as usize;
                        Some(matching[index].clone())
                    }
                }
            };
            chosen
                .map(|p| {
                    let controller = state.provinces[&p].controller.clone();
                    vec![(Some(controller), Some(p))]
                })
                .unwrap_or_default()
        }
    }
}

/// Per-turn chance of a random event, in thousandths.
pub fn random_permille(event: &Event) -> u32 {
    event.trigger.chance_permille.unwrap_or_else(|| {
        event
            .trigger
            .mean_time_to_happen
            .map_or(0, |mtth| 1000u32.div_ceil(mtth.max(1)))
    })
}

/// End-of-turn phase (after religion, before population).
pub(crate) fn resolve_chronicle(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    // 1. Expired player decisions: the first option applies.
    let turn = state.turn;
    let (expired, kept): (Vec<Decision>, Vec<Decision>) =
        std::mem::take(&mut state.chronicle.pending_decisions)
            .into_iter()
            .partition(|d| d.expires_turn <= turn);
    state.chronicle.pending_decisions = kept;
    for decision in expired {
        let first = decision.options.first().copied().unwrap_or(0);
        resolve_decision(state, data, &decision, first, true, events);
    }

    if state.chronicle.disabled {
        resolve_plague_wave(state, data, events);
        events.append(&mut state.pending_events);
        return;
    }

    // 1b. F1: events scheduled by earlier ones (chains).
    let (due, later): (Vec<ScheduledEvent>, Vec<ScheduledEvent>) =
        std::mem::take(&mut state.chronicle.scheduled)
            .into_iter()
            .partition(|s| s.turn <= turn);
    state.chronicle.scheduled = later;
    for scheduled in due {
        let Some(event) = data.events.get(&scheduled.event) else {
            continue;
        };
        let decider = scheduled.faction.filter(|f| state.faction_alive(f));
        let ctx = EventContext {
            faction: decider.clone(),
            province: scheduled.province.clone(),
        };
        if decider.is_none() || !state.event_conditions_hold(data, event, &ctx) {
            continue;
        }
        state.chronicle.fired_events.insert(event.id.clone());
        fire(state, data, event, decider, scheduled.province, events);
    }

    // 2. Historical events.
    let historical: Vec<&Event> = data
        .events
        .values()
        .filter(|e| e.kind == EventCategory::Historical)
        .collect();
    for event in historical {
        if state.chronicle.fired_events.contains(&event.id) || !state.historical_window_open(event)
        {
            continue;
        }
        let targets = targets(state, data, event, None);
        if targets.is_empty() {
            continue;
        }
        state.chronicle.fired_events.insert(event.id.clone());
        for (decider, province) in targets {
            fire(state, data, event, decider, province, events);
        }
    }

    // 3. Random events: at most one per faction and per turn.
    let random: Vec<&Event> = data
        .events
        .values()
        .filter(|e| e.kind == EventCategory::Random)
        .collect();
    for event in random.iter().filter(|e| e.scope == EventScope::Global) {
        if state.rng.chance_permille(random_permille(event)) {
            for (decider, province) in targets(state, data, event, None) {
                fire(state, data, event, decider, province, events);
            }
        }
    }
    for faction in living_factions(state) {
        for event in random.iter().filter(|e| e.scope != EventScope::Global) {
            if !state.faction_alive(&faction) {
                break;
            }
            // Roll first: the costly province scan only runs on a hit.
            if !state.rng.chance_permille(random_permille(event)) {
                continue;
            }
            let targets = targets(state, data, event, Some(&faction));
            if let Some((decider, province)) = targets.into_iter().next() {
                fire(state, data, event, decider, province, events);
                break;
            }
        }
    }

    // 4. Black Death wave.
    resolve_plague_wave(state, data, events);

    // Events queued by effects (wars, peace) join this turn's journal.
    events.append(&mut state.pending_events);
}

/// Provinces sorted south to north (latitude of the capital; unknown last).
fn provinces_by_latitude(state: &CampaignState, data: &GameData) -> Vec<ProvinceId> {
    let mut provinces: Vec<(f64, ProvinceId)> = state
        .provinces
        .keys()
        .map(|id| {
            let latitude = data
                .provinces
                .get(id)
                .and_then(|p| p.geo.as_ref())
                .map_or(f64::MAX, |geo| geo.capital_lonlat[1]);
            (latitude, id.clone())
        })
        .collect();
    provinces.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    provinces.into_iter().map(|(_, id)| id).collect()
}

/// Provinces the wave strikes at step `step` (0-based) out of `duration`.
pub fn plague_slice(
    state: &CampaignState,
    data: &GameData,
    step: u32,
    duration: u32,
) -> Vec<ProvinceId> {
    let ordered = provinces_by_latitude(state, data);
    let count = ordered.len() as u64;
    ordered
        .into_iter()
        .enumerate()
        .filter(|(index, _)| (*index as u64 * u64::from(duration) / count.max(1)) as u32 == step)
        .map(|(_, id)| id)
        .collect()
}

fn resolve_plague_wave(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let Some(wave) = state.chronicle.plague_wave.clone() else {
        return;
    };
    let step = state.turn.saturating_sub(wave.start_turn);
    if step >= wave.duration {
        state.chronicle.plague_wave = None;
        return;
    }
    let reached = plague_slice(state, data, step, wave.duration);
    let (low, high) = PLAGUE_POPULATION_LOSS;
    let mut struck = Vec::new();
    let mut spared = Vec::new();
    for id in reached {
        let loss = low + state.rng.below(high - low + 1);
        // H4: plague resistance may spare the province (the roll only
        // happens with some resistance) and softens the blow.
        let resistance = crate::medicine::plague_resistance(state, data, &id);
        if resistance > 0.0
            && state.rng.unit_f64()
                < resistance * crate::medicine::BLACK_DEATH_SPARE_PERCENT / 100.0
        {
            spared.push(id);
            continue;
        }
        let factor = 1.0 - f64::from(loss) / 100.0 * (1.0 - resistance);
        let health = crate::medicine::mitigated(-PLAGUE_HEALTH_LOSS, resistance);
        let unrest = -crate::medicine::mitigated(-PLAGUE_UNREST, resistance);
        for_each_class(state, &id, |c| {
            c.health = add_clamped(c.health, health);
            c.unrest = add_clamped(c.unrest, unrest);
            c.count = (c.count as f64 * factor).round() as u64;
        });
        struck.push(id);
    }
    let player = state.player_faction.clone();
    for id in &spared {
        if state
            .provinces
            .get(id)
            .is_some_and(|p| p.controller == player)
        {
            events.push(
                GameEvent::new(
                    EventKind::Medicine,
                    format!(
                        "La Grande Mortalité épargne {} : quarantaine, fumigations et apothicaires ont tenu.",
                        province_name(data, id)
                    ),
                )
                .province(id)
                .faction(&player),
            );
        }
    }
    if !struck.is_empty() {
        let names: Vec<String> = struck.iter().map(|p| province_name(data, p)).collect();
        let mut entry = GameEvent::new(
            EventKind::Plague,
            format!("La Grande Mortalité frappe : {}.", names.join(", ")),
        );
        if let Some(own) = struck.iter().find(|p| {
            state
                .provinces
                .get(*p)
                .is_some_and(|s| s.controller == player)
        }) {
            entry = entry.province(own).faction(&player);
        }
        events.push(entry);
    }
    if step + 1 >= wave.duration {
        state.chronicle.plague_wave = None;
    }
}
