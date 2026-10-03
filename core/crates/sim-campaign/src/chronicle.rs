//! Chronicle: historical and random events (M10, `docs/design/m10-events.md`).
//!
//! Each end of turn (after religion, before population), [`resolve_chronicle`]:
//! 1. resolves player decisions that expired: the option the AI would pick
//!    for that realm applies (ADR 0122, [`ai_affordable_choice_among`]);
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
    EventPresentation, EventScope, EventSeason, FactionId, GameData, ProvinceId, ProvinceRef,
    SceneKind, SocialClass,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{faction_name, Claim, REBELS_FACTION};
use crate::events::{EventKind, GameEvent};
use crate::population::weighted_unrest;
use crate::state::{Army, CampaignState, Season, Unit, TURNS_PER_YEAR};
use crate::{characters, religion, skills};

/// Turns a player decision stays open before the AI's option applies
/// (ADR 0122).
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
    /// At the end of this turn, the option the AI would pick applies
    /// automatically (ADR 0122).
    pub expires_turn: u32,
}

impl Decision {
    /// FK1: map incident or dialog window (the event's effective
    /// presentation, [`Event::presentation`]); `dialog` for an unknown event.
    pub fn presentation(&self, data: &GameData) -> EventPresentation {
        data.events
            .get(&self.event)
            .map_or(EventPresentation::Dialog, Event::presentation)
    }
}

/// FK1: a province scene born of a recent chronicle event (or of the Black
/// Death wave), kept while [`data_model::MapSceneRules::duration`] runs.
/// Purely visual (`crate::map_scenes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentScene {
    pub kind: SceneKind,
    pub province: ProvinceId,
    /// Turn whose end fired the event (the scene shows from the next one).
    pub turn: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<EventId>,
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
    /// FK1: recent events carrying a `map_scene`, by province (visual only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_scenes: Vec<RecentScene>,
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
    /// End-of-turns left before the AI's option applies (1 = this turn).
    pub expires_in: u32,
    pub province: Option<ProvinceId>,
    pub province_name: String,
    /// FK1: map incident or dialog window.
    pub presentation: EventPresentation,
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
            Condition::Controls { faction, province } => {
                scope_faction(faction).is_some_and(|f| self.controls_province(&f, province))
            }
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
                .and_then(|p| self.city_state(&p))
                .and_then(|city| city.siege.as_ref())
                .is_some_and(|siege| by.as_ref().is_none_or(|by| &siege.attacker == by)),
            Condition::ProvinceCoastal { province } => scope_province(province)
                .and_then(|p| data.provinces.get(&p))
                .is_some_and(|p| p.coastal),
            Condition::TreasuryAbove { faction, amount } => scope_faction(faction)
                .and_then(|f| self.factions.get(&f))
                .is_some_and(|f| f.treasury > *amount),
            Condition::ProvincesBelow { faction, count } => {
                scope_faction(faction).is_some_and(|f| {
                    let controlled = self.controlled_provinces(&f).len();
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
                    presentation: event.presentation(),
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
                // UI audit A3 E3: livres tournois with the ₶ sign and
                // grouped digits, as everywhere else in the interface.
                // EQ1: the amount the faction will really pay or receive.
                let target = faction.clone().or(ctx.faction.clone());
                let amount = event_treasury_amount(self, data, target.as_ref(), *amount);
                format!(
                    "Trésor {}{}",
                    crate::economy_balance::signed_livres(amount),
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
            EventEffect::FoundChivalricOrder { order, .. } => {
                let name = data
                    .chivalric_orders
                    .get(order)
                    .map_or_else(|| order.to_string(), |o| o.name.display.clone());
                format!("Fondation de l'ordre « {name} »")
            }
            EventEffect::TransferProvince {
                province,
                faction,
                price,
                payer,
                ..
            } => {
                let text = match faction {
                    Some(f) => format!(
                        "{} passe à {}",
                        province_name(data, province),
                        faction_name(data, f)
                    ),
                    None => format!("{} rejoint le domaine", province_name(data, province)),
                };
                format!("{text}{}", price_label(data, *price, payer))
            }
            EventEffect::TransferTitle {
                title,
                faction,
                price,
                payer,
                ..
            } => {
                let name = data
                    .titles
                    .get(title)
                    .map_or_else(|| title.to_string(), |t| t.name.display.clone());
                let text = match faction {
                    Some(f) => format!("{name} passe à {}", faction_name(data, f)),
                    None => format!("{name} rejoint le domaine"),
                };
                format!("{text}{}", price_label(data, *price, payer))
            }
            EventEffect::SetRuler { character, faction } => {
                let realm = faction
                    .as_ref()
                    .or(ctx.faction.as_ref())
                    .map(|f| format!(" de {}", faction_name(data, f)))
                    .unwrap_or_default();
                format!(
                    "{} prend la tête{realm}",
                    self.character_name(data, character)
                )
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
            Some(faction) => self.controlled_provinces(faction),
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

/// EQ1 (`data/rules/economy.json`): the treasury effect `amount` of an
/// event for `faction`, scaled down for a faction whose seasonal income is
/// below the reference (never below the minimum share): event costs are
/// written for a middling realm, and a small county must not be ruined by a
/// fire it could not have prevented.
pub fn event_treasury_amount(
    state: &CampaignState,
    data: &GameData,
    faction: Option<&FactionId>,
    amount: i64,
) -> i64 {
    let rules = &data.economy_rules;
    let Some(f) = faction.and_then(|id| state.factions.get(id).map(|f| (id, f))) else {
        return amount;
    };
    let income = if f.1.income_last_turn > 0 {
        f.1.income_last_turn
    } else {
        state.faction_income_effective(data, f.0)
    };
    let reference = rules.event_treasury_reference_income.max(1);
    if income >= reference {
        return amount;
    }
    let scale = (income.max(0) as f64 / reference as f64)
        .max(rules.event_treasury_min_scale)
        .min(1.0);
    (amount as f64 * scale).round() as i64
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
            let target = target_faction(faction);
            let amount = event_treasury_amount(state, data, target.as_ref(), *amount);
            if let Some(f) = target.and_then(|f| state.factions.get_mut(&f)) {
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
            let Some(city) = state.province_city_id(&location).cloned() else {
                return;
            };
            let id = state.allocate_army_id();
            let mut army = Army::new(
                faction.clone(),
                crate::state::ArmyPosition::Settlement(city),
                units,
            );
            army.movement_left = crate::march::km_to_grid_points(
                data,
                f64::from(state.season_movement_points(data)),
            );
            state.armies.insert(id.clone(), army);
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
        EventEffect::TransferProvince {
            province,
            faction,
            from,
        } => {
            let holder = state
                .province_owner(province)
                .zip(state.province_controller(province));
            let from_ok = from
                .as_ref()
                .is_none_or(|f| holder.is_some_and(|(o, c)| o == f || c == f));
            let seller = state.province_owner(province).cloned();
            if let Some(faction) = target_faction(faction).filter(|_| from_ok) {
                if transfer_province(state, data, province, &faction, events) {
                    let payer = payer.clone().unwrap_or_else(|| faction.clone());
                    pay_sale_price(state, data, &payer, seller.as_ref(), *price, events);
                }
            }
        }
        EventEffect::TransferTitle {
            title,
            faction,
            from,
            price,
            payer,
        } => {
            let holder = crate::feudal::holder_of(state, title).cloned();
            let from_ok = from.as_ref().is_none_or(|f| holder.as_ref() == Some(f));
            if let Some(faction) = target_faction(faction).filter(|_| from_ok) {
                if transfer_title(state, data, title, &faction, events) {
                    let payer = payer.clone().unwrap_or_else(|| faction.clone());
                    pay_sale_price(state, data, &payer, holder.as_ref(), *price, events);
                }
            }
        }
        EventEffect::SetRuler { character, faction } => {
            if let Some(faction) = target_faction(faction) {
                set_ruler(state, data, &faction, character, events);
            }
        }
        EventEffect::FoundChivalricOrder { order, faction } => {
            if let Some(faction) = target_faction(faction) {
                crate::chivalry::found_order_by_event(state, data, &faction, order, events);
            }
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

/// G1 `transfer_province`: `province` passes to `faction`, ownership and
/// control (purchase, treaty), through `ransom::cede_province`. Ignored for
/// unknown ids, a dead recipient, a province it already holds, or the
/// capital of its current owner. Returns whether the province changed hands.
pub fn transfer_province(
    state: &mut CampaignState,
    data: &GameData,
    province: &ProvinceId,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(owner) = state.province_owner(province).cloned() else {
        return false;
    };
    let alive = state.factions.get(faction).is_some_and(|f| f.alive);
    let held = state.holds_province(faction, province);
    let capital = state
        .factions
        .get(&owner)
        .is_some_and(|f| &f.capital == province);
    if !alive || held || capital {
        return false;
    }
    let previous = owner;
    crate::ransom::cede_province(state, &previous, faction, province);
    events.push(
        GameEvent::new(
            EventKind::ProvinceCaptured,
            format!(
                "{} passe de {} à {}.",
                province_name(data, province),
                faction_name(data, &previous),
                faction_name(data, faction)
            ),
        )
        .province(province)
        .faction(faction),
    );
    true
}

/// LR-17 ` (price N livres)` suffix of a sale, naming a payer other than
/// the recipient.
fn price_label(data: &GameData, price: i64, payer: &Option<FactionId>) -> String {
    if price <= 0 {
        return String::new();
    }
    let amount = crate::economy_balance::signed_livres(price);
    let amount = amount.trim_start_matches('+');
    match payer {
        Some(p) => format!(" contre {amount}, payés par {}", faction_name(data, p)),
        None => format!(" contre {amount}"),
    }
}

/// LR-17 `transfer_title`: `title` passes to `faction` through the feudal
/// transfer (its provinces held by the seller follow, capital included; a
/// seller left without title is absorbed). Ignored for an unknown title, a
/// dead recipient or one that already holds it. Returns whether the title
/// changed hands.
pub fn transfer_title(
    state: &mut CampaignState,
    data: &GameData,
    title: &data_model::TitleId,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) -> bool {
    if crate::feudal::holder_of(state, title) == Some(faction) {
        return false;
    }
    crate::feudal::transfer_title_into(state, data, title, faction, events).is_ok()
}

/// LR-17: `payer` pays `price` livres for a sale (even into debt); `seller`
/// receives them if it still exists, otherwise the money leaves the map (a
/// crown that is not playable, a seller absorbed by the sale).
fn pay_sale_price(
    state: &mut CampaignState,
    data: &GameData,
    payer: &FactionId,
    seller: Option<&FactionId>,
    price: i64,
    events: &mut Vec<GameEvent>,
) {
    if price <= 0 || seller == Some(payer) {
        return;
    }
    let Some(paying) = state.factions.get_mut(payer).filter(|f| f.alive) else {
        return;
    };
    paying.treasury -= price;
    let receiver = seller.filter(|s| state.factions.get(*s).is_some_and(|f| f.alive));
    if let Some(receiver) = receiver {
        state.factions.get_mut(receiver).expect("alive").treasury += price;
    }
    let amount = crate::economy_balance::signed_livres(price);
    let amount = amount.trim_start_matches('+');
    let text = match receiver {
        Some(r) => format!(
            "{} verse {amount} à {}.",
            faction_name(data, payer),
            faction_name(data, r)
        ),
        None => format!("{} verse {amount} pour son achat.", faction_name(data, payer)),
    };
    events.push(GameEvent::new(EventKind::Chronicle, text).faction(payer));
}

/// LR-17 `set_ruler`: `character`, alive and of `faction`, takes its throne
/// (usurpation, election); the former ruler lives on and the heir is picked
/// again by the succession law. Returns whether the ruler changed.
pub fn set_ruler(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let eligible = state
        .characters
        .get(character)
        .is_some_and(|c| c.alive && &c.faction == faction);
    let Some(realm) = state.factions.get(faction).filter(|f| f.alive) else {
        return false;
    };
    if !eligible || realm.ruler.as_ref() == Some(character) {
        return false;
    }
    let deposed = realm.ruler.clone();
    let realm = state.factions.get_mut(faction).expect("alive");
    realm.ruler = Some(character.clone());
    realm.heir = None;
    let heir = crate::dynasty::pick_heir_by_law(state, data, faction, character);
    state.factions.get_mut(faction).expect("alive").heir = heir;
    let deposed = deposed
        .filter(|d| state.characters.get(d).is_some_and(|c| c.alive))
        .map(|d| format!(" ; {} est écarté", state.character_name(data, &d)))
        .unwrap_or_default();
    events.push(
        GameEvent::new(
            EventKind::Succession,
            format!(
                "{} prend la tête de {}{deposed}.",
                state.character_name(data, character),
                faction_name(data, faction)
            ),
        )
        .faction(faction),
    );
    true
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
        // C7: a Lombard banker may follow the money to the captor's ruler.
        if let Some(captor) = &captor {
            crate::retinue::on_ransom_received(state, data, captor, events);
        }
    }
    let capital = state.factions.get(&owner).map(|f| f.capital.clone());
    let c = state.characters.get_mut(id).expect("checked above");
    c.captive = false;
    c.captor = None;
    c.ransom_terms = None;
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
    let every: Vec<usize> = (0..event.options.len()).collect();
    weighted_pick(state, event, &every)
}

/// F4: like [`ai_choice`], but an AI realm first sets aside the options it
/// cannot afford (a cost above its treasury plus two seasons of income) when
/// another option remains: a small realm does not rebuild a fleet on credit.
pub fn ai_affordable_choice(
    state: &mut CampaignState,
    data: &GameData,
    event: &Event,
    decider: Option<&FactionId>,
) -> usize {
    let every: Vec<usize> = (0..event.options.len()).collect();
    ai_affordable_choice_among(state, data, event, decider, &every)
}

/// FK1 (ADR 0122): [`ai_affordable_choice`] restricted to the options
/// `offered` (indices into `event.options`; unknown ones are ignored).
/// Same random stream as [`ai_affordable_choice`] when every option is
/// offered: one draw only when several options tie.
pub fn ai_affordable_choice_among(
    state: &mut CampaignState,
    data: &GameData,
    event: &Event,
    decider: Option<&FactionId>,
    offered: &[usize],
) -> usize {
    let offered: Vec<usize> = offered
        .iter()
        .copied()
        .filter(|i| *i < event.options.len())
        .collect();
    let Some(means) = decider
        .and_then(|f| state.factions.get(f))
        .map(|f| f.treasury.max(0) + 2 * f.income_last_turn.max(0))
    else {
        return weighted_pick(state, event, &offered);
    };
    let cost = |option: &data_model::EventOption| -> i64 {
        option
            .effects
            .iter()
            .map(|e| match e {
                EventEffect::Treasury {
                    faction: None,
                    amount,
                } if *amount < 0 => -event_treasury_amount(state, data, decider, *amount),
                EventEffect::TransferProvince {
                    faction: None,
                    payer: None,
                    price,
                    ..
                }
                | EventEffect::TransferTitle {
                    faction: None,
                    payer: None,
                    price,
                    ..
                } => *price,
                _ => 0,
            })
            .sum()
    };
    let affordable: Vec<usize> = offered
        .iter()
        .copied()
        .filter(|i| cost(&event.options[*i]) <= means)
        .collect();
    if affordable.is_empty() || affordable.len() == offered.len() {
        return weighted_pick(state, event, &offered);
    }
    weighted_pick(state, event, &affordable)
}

/// Option of highest `ai_weight` among `among` (indices into
/// `event.options`), ties broken by the campaign RNG; 0 when empty.
fn weighted_pick(state: &mut CampaignState, event: &Event, among: &[usize]) -> usize {
    let best = among
        .iter()
        .map(|i| event.options[*i].ai_weight)
        .max()
        .unwrap_or(0);
    let candidates: Vec<usize> = among
        .iter()
        .copied()
        .filter(|i| event.options[*i].ai_weight == best)
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
    record_scene(state, event, decider.as_ref(), province.as_ref());
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
    let option = ai_affordable_choice(state, data, event, decider.as_ref());
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

/// FK1: remembers the scene of an event carrying a `map_scene`, at its
/// province or else at the deciding realm's capital (sacres, peaces,
/// weddings); an event with neither has no scene. No RNG read.
fn record_scene(
    state: &mut CampaignState,
    event: &Event,
    decider: Option<&FactionId>,
    province: Option<&ProvinceId>,
) {
    let Some(kind) = event.map_scene else {
        return;
    };
    let place = province.cloned().or_else(|| {
        decider
            .and_then(|f| state.factions.get(f))
            .map(|f| f.capital.clone())
    });
    let Some(place) = place.filter(|p| state.provinces.contains_key(p)) else {
        return;
    };
    let turn = state.turn;
    state.chronicle.recent_scenes.push(RecentScene {
        kind,
        province: place,
        turn,
        event: Some(event.id.clone()),
    });
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
        .keys()
        .filter_map(|id| state.province_controller(id).map(|c| (id, c)))
        .filter(|(_, controller)| {
            faction.is_none_or(|f| *controller == f) && !is_rebels(controller)
        })
        .filter(|(id, controller)| {
            let ctx = EventContext {
                faction: Some((*controller).clone()),
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
                    let Some(controller) = state.province_controller(p).cloned() else {
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
                    let controller = state.province_controller(&p).cloned();
                    vec![(controller, Some(p))]
                })
                .unwrap_or_default()
        }
    }
}

/// `false` when `scope` names a faction other than `faction`.
pub fn scope_allows(scope: &EventScope, faction: &FactionId) -> bool {
    match scope {
        EventScope::Faction { faction: Some(f) }
        | EventScope::Province {
            faction: Some(f), ..
        } => f == faction,
        _ => true,
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
    // 1. Expired player decisions: the option the AI would pick for that
    //    realm among those offered applies (ADR 0122).
    let turn = state.turn;
    let (expired, kept): (Vec<Decision>, Vec<Decision>) =
        std::mem::take(&mut state.chronicle.pending_decisions)
            .into_iter()
            .partition(|d| d.expires_turn <= turn);
    state.chronicle.pending_decisions = kept;
    for decision in expired {
        let option = match data.events.get(&decision.event) {
            Some(event) => ai_affordable_choice_among(
                state,
                data,
                event,
                Some(&decision.faction),
                &decision.options,
            ),
            None => decision.options.first().copied().unwrap_or(0),
        };
        resolve_decision(state, data, &decision, option, true, events);
    }

    // 1a. FK1: scenes of recent events fade out (visual only, no RNG).
    let rules = &data.map_scene_rules;
    state
        .chronicle
        .recent_scenes
        .retain(|scene| scene.turn + rules.duration(scene.kind) > turn);

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
            // F7b: an event reserved to another faction costs no roll, so
            // adding one (Venice, Bohemia…) leaves the others' luck alone.
            if !scope_allows(&event.scope, &faction) {
                continue;
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
    // FK1: every province struck shows its plague scene (visual only).
    let turn = state.turn;
    state
        .chronicle
        .recent_scenes
        .extend(struck.iter().map(|id| RecentScene {
            kind: SceneKind::Plague,
            province: id.clone(),
            turn,
            event: None,
        }));
    let player = state.player_faction.clone();
    for id in &spared {
        if state.controls_province(&player, id) {
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
        if let Some(own) = struck.iter().find(|p| state.controls_province(&player, p)) {
            entry = entry.province(own).faction(&player);
        }
        events.push(entry);
    }
    if step + 1 >= wave.duration {
        state.chronicle.plague_wave = None;
    }
}

// =========================================================================
// Staging (UI tests and screenshots, lot FK5)
// =========================================================================

impl CampaignState {
    /// Offers `event` to the player as a pending decision (all its options,
    /// the usual delay), anchored on `province`; returns its id, or `None`
    /// for an unknown event or province. Staging only: no trigger, effect,
    /// journal entry or scene applies.
    pub fn debug_offer_decision(
        &mut self,
        data: &GameData,
        event: &EventId,
        province: Option<&ProvinceId>,
    ) -> Option<u32> {
        let event = data.events.get(event)?;
        if province.is_some_and(|p| !self.provinces.contains_key(p)) {
            return None;
        }
        let id = self.chronicle.next_decision_id.max(1);
        self.chronicle.next_decision_id = id + 1;
        self.chronicle.pending_decisions.push(Decision {
            id,
            event: event.id.clone(),
            faction: self.player_faction.clone(),
            province: province.cloned(),
            options: (0..event.options.len()).collect(),
            expires_turn: self.turn + DECISION_TURNS,
        });
        Some(id)
    }
}
