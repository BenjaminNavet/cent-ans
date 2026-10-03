//! Technologies and research (M6, `docs/design/m6-technologies.md` § 2).
//!
//! Technology *definitions* (branch, tier, cost, prerequisites, unlocks,
//! effects, historical year) live in `data/technologies/*.json`. This module
//! holds the rules that read them:
//!
//! - research points per turn ([`CampaignState::research_points_per_turn`]):
//!   base [`BASE_RESEARCH_POINTS`] + the `ResearchPoints` effects of the
//!   buildings of owned provinces and of acquired technologies + half the
//!   ruler's governance (rounded);
//! - the `research { technology }` order ([`start_research`]), which banks the
//!   progress of an abandoned research in `FactionState::research_banked`;
//! - completion at the end of the turn ([`resolve_research`], run right after
//!   the economy phase), with the anachronism surcharge of
//!   [`effective_cost`];
//! - technology effects: [`faction_tech_effects`] (income and population
//!   kinds, merged by `economy` and `population`) and [`tech_unit_bonus`]
//!   (per-`unit_category` battle bonuses, folded into the unit stats by
//!   `movement::side_from_army`);
//! - the minimal AI choice ([`ai_choose_research`]).
//!
//! Unit and building unlocks need no code here: `recruit_blocker` and
//! `build_blocker` already refuse a `required_technology` the faction lacks,
//! and every `unlocks.units`/`unlocks.buildings` entry of the data carries
//! that same `required_technology` (checked by `tests/m6.rs`).

use data_model::{
    EffectKind, EffectMode, FactionId, GameData, TechBranch, Technology, TechnologyId, UnitCategory,
};
use serde::{Deserialize, Serialize};

use crate::battle_auto::Side;
use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState};

/// Research points every faction produces per turn before any bonus.
pub const BASE_RESEARCH_POINTS: u32 = 5;
/// A technology whose `historical_year` is more than this many years after
/// the current year costs [`ANACHRONISM_SURCHARGE_PERCENT`] more.
pub const ANACHRONISM_YEARS: i32 = 20;
pub const ANACHRONISM_SURCHARGE_PERCENT: u32 = 25;

/// Why a `research` order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResearchError {
    #[error("technologie inconnue : {0}")]
    UnknownTechnology(TechnologyId),
    #[error("technologie déjà acquise")]
    AlreadyKnown,
    #[error("prérequis manquant : {0}")]
    MissingPrerequisite(String),
    #[error("la file de recherche est pleine")]
    QueueFull,
}

/// State of a technology for one faction (`get_tech_tree`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechStatus {
    /// Acquired.
    Known,
    /// Prerequisites acquired, can be researched.
    Available,
    /// A prerequisite is missing.
    Locked,
    /// Currently being researched.
    Researching,
}

impl TechStatus {
    pub fn key(self) -> &'static str {
        match self {
            TechStatus::Known => "known",
            TechStatus::Available => "available",
            TechStatus::Locked => "locked",
            TechStatus::Researching => "researching",
        }
    }
}

/// Current research of a faction (`get_research`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchInfo {
    pub technology: TechnologyId,
    pub progress: u32,
    /// Effective cost (anachronism surcharge included).
    pub cost: u32,
    pub points_per_turn: u32,
    /// Turns until completion at the current rate.
    pub turns_left: u32,
}

/// Flat battle bonuses a faction's technologies grant one unit category
/// (`ArmyMorale`, `ArmyMelee`, `ArmyRanged`, `ArmyArmor`; an effect without
/// `unit_category` applies to every category).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitTechBonus {
    pub morale: f64,
    pub melee: f64,
    pub ranged: f64,
    pub armor: f64,
}

/// Cost of `tech` in the given year: `cost × 1.25` when the technology's
/// historical year is more than [`ANACHRONISM_YEARS`] ahead (spec § 2).
pub fn effective_cost(tech: &Technology, year: i32) -> u32 {
    let too_early = tech
        .historical_year
        .as_ref()
        .and_then(|date| date.year())
        .is_some_and(|historical| historical > year + ANACHRONISM_YEARS);
    if too_early {
        (tech.cost * (100 + ANACHRONISM_SURCHARGE_PERCENT)).div_ceil(100)
    } else {
        tech.cost
    }
}

/// Acquired technologies of `faction` (empty for an unknown faction).
fn acquired<'a>(
    state: &'a CampaignState,
    data: &'a GameData,
    faction: &FactionId,
) -> impl Iterator<Item = &'a Technology> {
    state
        .factions
        .get(faction)
        .into_iter()
        .flat_map(|f| f.technologies.iter())
        .filter_map(|id| data.technologies.get(id))
}

/// Aggregated effects of `faction`'s technologies, for the kinds
/// [`EffectTotals`] knows (spec § 2 `faction_tech_effects`).
pub fn faction_tech_effects(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> EffectTotals {
    let mut totals = EffectTotals::default();
    for tech in acquired(state, data, faction) {
        for effect in &tech.effects {
            totals.add_effect(effect);
        }
    }
    totals
}

/// The subset of [`faction_tech_effects`] applied to province income
/// (`TaxIncome`, `TradeIncome`, F1 `Production`) and population (`Health`,
/// `Growth`, `Unrest`, F1 `Wealth` and every class-targeted effect), spec § 2.
pub fn faction_province_tech_effects(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> EffectTotals {
    let all = faction_tech_effects(state, data, faction);
    EffectTotals {
        tax_income: all.tax_income,
        trade_income: all.trade_income,
        health: all.health,
        growth: all.growth,
        unrest: all.unrest,
        wealth: all.wealth,
        production: all.production,
        classes: all.classes,
        ..EffectTotals::default()
    }
}

/// `(defence, siegecraft)` parts of `faction`'s technology
/// `SiegeResistance` effects (F1): positive values strengthen the faction's
/// own towns, negative ones weaken the towns it besieges.
pub fn tech_siege_resistance(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> (f64, f64) {
    let mut defence = 0.0;
    let mut siegecraft = 0.0;
    for tech in acquired(state, data, faction) {
        for effect in &tech.effects {
            if effect.effect == EffectKind::SiegeResistance && effect.mode == EffectMode::Add {
                if effect.value >= 0.0 {
                    defence += effect.value;
                } else {
                    siegecraft += effect.value;
                }
            }
        }
    }
    (defence, siegecraft)
}

/// Battle bonuses of `faction`'s technologies for units of `category`.
pub fn tech_unit_bonus(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    category: UnitCategory,
) -> UnitTechBonus {
    let mut bonus = UnitTechBonus::default();
    for tech in acquired(state, data, faction) {
        for effect in &tech.effects {
            if effect.mode != EffectMode::Add || effect.unit_category.is_some_and(|c| c != category)
            {
                continue;
            }
            match effect.effect {
                EffectKind::ArmyMorale => bonus.morale += effect.value,
                EffectKind::ArmyMelee => bonus.melee += effect.value,
                EffectKind::ArmyRanged => bonus.ranged += effect.value,
                EffectKind::ArmyArmor => bonus.armor += effect.value,
                _ => {}
            }
        }
    }
    bonus
}

/// Adds `bonus` to a 0-`max` stat.
pub(crate) fn boosted(stat: u8, bonus: f64, max: u8) -> u8 {
    (f64::from(stat) + bonus).round().clamp(0.0, f64::from(max)) as u8
}

/// Battle description of `army` as the auto-resolver sees it (general
/// bonuses and technology bonuses included); exposed for tests and the M7
/// battle setup.
pub fn army_battle_side(state: &CampaignState, data: &GameData, army: &ArmyId) -> Option<Side> {
    let army = state.armies.get(army)?;
    Some(crate::movement::side_from_army(state, data, army))
}

/// Status of `tech` for `faction`.
pub fn tech_status(state: &CampaignState, faction: &FactionId, tech: &Technology) -> TechStatus {
    let Some(faction_state) = state.factions.get(faction) else {
        return TechStatus::Locked;
    };
    if faction_state.technologies.contains(&tech.id) {
        TechStatus::Known
    } else if faction_state.research.as_ref() == Some(&tech.id) {
        TechStatus::Researching
    } else if tech
        .prerequisites
        .iter()
        .all(|p| faction_state.technologies.contains(p))
    {
        TechStatus::Available
    } else {
        TechStatus::Locked
    }
}

/// Progress already accumulated on `tech` by `faction` (current research or
/// banked).
pub fn tech_progress(state: &CampaignState, faction: &FactionId, tech: &TechnologyId) -> u32 {
    let Some(f) = state.factions.get(faction) else {
        return 0;
    };
    if f.research.as_ref() == Some(tech) {
        f.research_progress
    } else {
        f.research_banked.get(tech).copied().unwrap_or(0)
    }
}

impl CampaignState {
    /// Research points `faction` produces per turn (spec § 2).
    pub fn research_points_per_turn(&self, data: &GameData, faction: &FactionId) -> u32 {
        let Some(faction_state) = self.factions.get(faction) else {
            return 0;
        };
        let mut flat = f64::from(BASE_RESEARCH_POINTS);
        let mut percent = 0.0;
        let mut add = |kind: EffectKind, mode: EffectMode, value: f64| {
            if kind == EffectKind::ResearchPoints {
                match mode {
                    EffectMode::Add => flat += value,
                    EffectMode::Percent => percent += value,
                }
            }
        };
        // The places held (controller), as for taxes and upkeep. DC6b (ADR 0082): a
        // secondary place's libraries weigh its kind's `research_percent`, so that the
        // doubled abbeys of the dense map do not raise research by a fifth.
        for settlement in self
            .settlements
            .values()
            .filter(|s| &s.controller == faction)
        {
            let weight =
                f64::from(crate::buildings::research_percent(data, settlement.kind)) / 100.0;
            for id in &settlement.buildings {
                if let Some(building) = data.buildings.get(id) {
                    for effect in &building.effects {
                        add(effect.effect, effect.mode, effect.value * weight);
                    }
                }
            }
        }
        for tech in acquired(self, data, faction) {
            for effect in &tech.effects {
                add(effect.effect, effect.mode, effect.value);
            }
        }
        let ruler = faction_state
            .ruler
            .as_ref()
            .filter(|id| self.characters.get(*id).is_some_and(|c| c.alive));
        let governance = ruler
            .and_then(|id| self.characters.get(id))
            .map_or(0, |c| c.skills.governance);
        flat += f64::from(governance.div_ceil(2));
        // F1: the ruler's `ResearchCivil` / `ResearchMilitary` traits and
        // skills speed up research in the branch being researched.
        let branch = faction_state
            .research
            .as_ref()
            .and_then(|t| data.technologies.get(t))
            .map(|t| t.branch);
        if let (Some(ruler), Some(branch)) = (ruler, branch) {
            let effects = crate::skills::character_effects(self, data, ruler);
            let bonus = match branch {
                // Medicine is learned lore: civil scholarship speeds it up.
                TechBranch::Civil | TechBranch::Medicine => effects.research_civil,
                TechBranch::Military => effects.research_military,
            };
            flat += bonus.flat;
            percent += bonus.percent;
        }
        (flat * (1.0 + percent / 100.0)).round().max(0.0) as u32
    }

    /// Current research of `faction`, if any (spec § 3 `get_research`).
    pub fn research_info(&self, data: &GameData, faction: &FactionId) -> Option<ResearchInfo> {
        let f = self.factions.get(faction)?;
        let technology = f.research.clone()?;
        let tech = data.technologies.get(&technology)?;
        let cost = effective_cost(tech, self.year);
        let points_per_turn = self.research_points_per_turn(data, faction);
        let remaining = cost.saturating_sub(f.research_progress);
        let turns_left = if points_per_turn == 0 {
            u32::MAX
        } else {
            remaining.div_ceil(points_per_turn)
        };
        Some(ResearchInfo {
            technology,
            progress: f.research_progress,
            cost,
            points_per_turn,
            turns_left,
        })
    }
}

/// Validates and applies `research { technology }` for `faction` (spec § 2).
/// Switching research banks the progress of the abandoned technology and
/// resumes any progress banked for the new one.
pub fn start_research(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    technology: &TechnologyId,
) -> Result<(), ResearchError> {
    let tech = data
        .technologies
        .get(technology)
        .ok_or_else(|| ResearchError::UnknownTechnology(technology.clone()))?;
    let Some(f) = state.factions.get_mut(faction) else {
        return Err(ResearchError::UnknownTechnology(technology.clone()));
    };
    if f.technologies.contains(technology) {
        return Err(ResearchError::AlreadyKnown);
    }
    if let Some(missing) = tech
        .prerequisites
        .iter()
        .find(|p| !f.technologies.contains(*p))
    {
        let name = data
            .technologies
            .get(missing)
            .map_or_else(|| missing.to_string(), |t| t.name.display.clone());
        return Err(ResearchError::MissingPrerequisite(name));
    }
    if f.research.as_ref() == Some(technology) {
        return Ok(());
    }
    f.research_queue.retain(|queued| queued != technology);
    // F1: with no research running, `research_progress` holds the surplus
    // of the last completed technology; it carries over to the new one.
    let surplus = if f.research.is_none() {
        f.research_progress
    } else {
        0
    };
    if let Some(previous) = f.research.take() {
        if f.research_progress > 0 {
            f.research_banked.insert(previous, f.research_progress);
        }
    }
    f.research_progress = f.research_banked.remove(technology).unwrap_or(0) + surplus;
    f.research = Some(technology.clone());
    Ok(())
}

/// End-of-turn phase (after the economy): pays each faction's research
/// points into its current research and completes it when the effective cost
/// is reached (event `technology_researched`).
pub(crate) fn resolve_research(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let factions: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    let year = state.year;
    for faction_id in factions {
        let points = state.research_points_per_turn(data, &faction_id);
        let f = state.factions.get_mut(&faction_id).expect("listed above");
        f.research_points_last_turn = points;
        if f.research.is_none() {
            promote_queued_research(state, data, &faction_id);
        }
        let f = state.factions.get_mut(&faction_id).expect("listed above");
        let Some(technology) = f.research.clone() else {
            // A6-L4: idle, the points pile up in a capped reserve instead
            // of being lost; the next research starts with them.
            let cap = points.saturating_mul(data.economy_rules.research_reserve_turns);
            f.research_progress = f.research_progress.saturating_add(points).min(cap);
            continue;
        };
        let Some(tech) = data.technologies.get(&technology) else {
            f.research = None;
            f.research_progress = 0;
            continue;
        };
        f.research_progress += points;
        if f.research_progress < effective_cost(tech, year) {
            continue;
        }
        f.technologies.insert(technology.clone());
        f.research = None;
        // F1: the surplus is kept for the next research (see
        // `start_research`, which carries it over).
        f.research_progress -= effective_cost(tech, year);
        promote_queued_research(state, data, &faction_id);
        let faction_name = data
            .factions
            .get(&faction_id)
            .map_or_else(|| faction_id.to_string(), |d| d.name.display.clone());
        events.push(
            GameEvent::new(
                EventKind::TechnologyResearched,
                format!(
                    "{faction_name} maîtrise une nouvelle technologie : {}.",
                    tech.name.display
                ),
            )
            .faction(&faction_id),
        );
    }
}

/// A6-L4: starts the first queued technology that can still be researched
/// (the surplus / reserve carries over as in [`start_research`]); queued
/// entries that became known or whose prerequisites are missing are dropped.
fn promote_queued_research(state: &mut CampaignState, data: &GameData, faction: &FactionId) {
    loop {
        let Some(next) = state
            .factions
            .get(faction)
            .and_then(|f| f.research_queue.first().cloned())
        else {
            return;
        };
        if let Some(f) = state.factions.get_mut(faction) {
            f.research_queue.remove(0);
        }
        if start_research(state, data, faction, &next).is_ok() {
            return;
        }
    }
}

/// A6-L4: puts `technology` at the end of `faction`'s research queue. With no
/// research running it simply starts. Prerequisites may be known, being
/// researched or queued ahead.
pub fn queue_research(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    technology: &TechnologyId,
) -> Result<(), ResearchError> {
    let tech = data
        .technologies
        .get(technology)
        .ok_or_else(|| ResearchError::UnknownTechnology(technology.clone()))?;
    let Some(f) = state.factions.get(faction) else {
        return Err(ResearchError::UnknownTechnology(technology.clone()));
    };
    if f.research.is_none() && f.research_queue.is_empty() {
        return start_research(state, data, faction, technology);
    }
    if f.technologies.contains(technology) {
        return Err(ResearchError::AlreadyKnown);
    }
    if f.research.as_ref() == Some(technology) || f.research_queue.contains(technology) {
        return Ok(());
    }
    if let Some(missing) = tech.prerequisites.iter().find(|p| {
        !f.technologies.contains(*p)
            && f.research.as_ref() != Some(*p)
            && !f.research_queue.contains(p)
    }) {
        let name = data
            .technologies
            .get(missing)
            .map_or_else(|| missing.to_string(), |t| t.name.display.clone());
        return Err(ResearchError::MissingPrerequisite(name));
    }
    if f.research_queue.len() >= data.economy_rules.research_queue_max as usize {
        return Err(ResearchError::QueueFull);
    }
    let f = state.factions.get_mut(faction).expect("checked above");
    f.research_queue.push(technology.clone());
    Ok(())
}

/// A6-L4: removes `technology` from the queue (and what depends on it).
pub fn dequeue_research(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    technology: &TechnologyId,
) {
    let Some(f) = state.factions.get_mut(faction) else {
        return;
    };
    let Some(index) = f.research_queue.iter().position(|t| t == technology) else {
        return;
    };
    let removed: Vec<TechnologyId> = f.research_queue.drain(index..).collect();
    // Entries after it stay when they do not depend on the removed one.
    for later in removed.into_iter().skip(1) {
        let depends = data
            .technologies
            .get(&later)
            .is_some_and(|t| t.prerequisites.contains(technology));
        if !depends {
            f.research_queue.push(later);
        }
    }
}

impl CampaignState {
    /// A6-L4: `(reserve, cap)` of `faction`: points piled up while idle and
    /// their ceiling. Zero while a research runs.
    pub fn research_reserve(&self, data: &GameData, faction: &FactionId) -> (u32, u32) {
        let Some(f) = self.factions.get(faction) else {
            return (0, 0);
        };
        let cap = self
            .research_points_per_turn(data, faction)
            .saturating_mul(data.economy_rules.research_reserve_turns);
        if f.research.is_some() {
            (0, cap)
        } else {
            (f.research_progress, cap)
        }
    }
}

/// Minimal AI (spec § 2): when idle, research the cheapest available
/// technology, preferring the branch with fewer acquired technologies so that
/// the three trees (military, civil, medicine) alternate.
pub fn ai_choose_research(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<TechnologyId> {
    let f = state.factions.get(faction)?;
    if f.research.is_some() {
        return None;
    }
    let count = |branch: TechBranch| {
        acquired(state, data, faction)
            .filter(|t| t.branch == branch)
            .count()
    };
    // The branch with the fewest acquired technologies (ties: military,
    // civil, then medicine) so that the three trees alternate (H4).
    let preferred = TechBranch::ALL
        .into_iter()
        .min_by_key(|branch| count(*branch))
        .unwrap_or(TechBranch::Military);
    data.technologies
        .values()
        .filter(|t| tech_status(state, faction, t) == TechStatus::Available)
        .min_by_key(|t| {
            (
                t.branch != preferred,
                effective_cost(t, state.year),
                t.id.clone(),
            )
        })
        .map(|t| t.id.clone())
}
