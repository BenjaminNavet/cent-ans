//! H3 « La Table »: province diets (`docs/design/2026-09-23-histoire-et-savoir.md` § 2).
//!
//! Diet *definitions* live in `data/diets/*.json`; this module holds the rules:
//!
//! - every province eats [`DEFAULT_DIET`] (free, neutral) unless its
//!   controller picked another diet with `SetDiet { province, diet }`
//!   ([`set_diet`]); one change per province and per turn; a choice made by a
//!   previous controller does not bind the new one (the province falls back
//!   to the default);
//! - requirements ([`diet_blockers`]): resources accessible to the
//!   controller (the goods of the goods-satisfaction model), coastal
//!   province, technology, one of a list of buildings, terrain;
//! - cost ([`CampaignState::diet_cost`]): `population / 1000 ×
//!   cost_per_thousand` livres per season, ×1.5 in winter for `fresh` diets,
//!   paid in the economy phase (budget line « Table », [`pay_table`]); a
//!   diet the treasury cannot pay falls back to the default;
//! - requirements are checked every turn after the goods are recomputed
//!   ([`resolve_requirements`]); a diet whose requirements no longer hold
//!   falls back to the default;
//! - effects: population kinds (`Health`, `Growth`, `Wealth`, `Unrest`,
//!   `GoodsSatisfaction`, optionally per class) are merged with the
//!   buildings' and technologies' in `population` ([`diet_class_effects`]);
//!   `Health` is boosted by the controller's `DietHealth` percent;
//!   `ArmyMorale` raises the morale of the troops recruited in the province
//!   ([`recruit_morale_bonus`]); `Piety`/`Prestige` go to the ruler once per
//!   season (the best value among the faction's provinces, so that a big
//!   realm does not multiply them) in [`resolve_lent`];
//! - Lent (spring, [`resolve_lent`]): a faction with a `meat`/`dairy` diet
//!   somewhere loses ruler piety and those provinces' clergy grow restless;
//!   a faction with a `fish` diet gains piety.
//! - AI: [`ai_choose_diets`], shared by both planners (ADR 0003).

use data_model::{
    Diet, DietId, EffectKind, EffectMode, FactionId, GameData, LentRule, ProvinceId, SocialClass,
    WinterRule,
};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::province_policy::{self, changed_this_turn, controller_choice, ProvincePolicy};
use crate::state::{CampaignState, Season};

/// Diet every province starts with (neutral and free).
pub const DEFAULT_DIET: &str = "diet_bread_pottage";
/// Winter cost of a `fresh` diet, in percent of its base cost.
pub const WINTER_FRESH_COST_PERCENT: f64 = 150.0;
/// Ruler piety lost in spring by a faction eating meat or dairy during Lent.
pub const LENT_PIETY_PENALTY: i32 = 3;
/// Ruler piety gained in spring by a faction keeping a lean fish diet.
pub const LENT_FISH_PIETY: i32 = 2;
/// Clergy unrest added in spring to a province eating meat or dairy.
pub const LENT_CLERGY_UNREST: i32 = 10;
/// The AI only adopts a paying diet while its treasury covers this many
/// seasons of the resulting table budget.
pub const AI_TABLE_RESERVE_SEASONS: i64 = 8;
/// The AI keeps the table budget below this share of its income (percent).
pub const AI_TABLE_INCOME_PERCENT: i64 = 10;

/// The default diet id.
pub fn default_diet() -> DietId {
    DietId::new(DEFAULT_DIET).expect("well-formed id")
}

/// Diet chosen for a province (`ProvinceState::diet`); `None` = default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DietChoice {
    pub diet: DietId,
    /// Faction that chose it; the choice lapses when the controller changes.
    pub faction: FactionId,
    /// Turn of the last change (one change per province and per turn).
    pub turn: u32,
}

impl ProvincePolicy for DietChoice {
    fn faction(&self) -> &FactionId {
        &self.faction
    }
    fn turn(&self) -> u32 {
        self.turn
    }
}

/// Why a `SetDiet` order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DietError {
    #[error("régime inconnu : {0}")]
    UnknownDiet(DietId),
    #[error("{0} n'est pas contrôlée par votre faction")]
    NotControlled(String),
    #[error("le régime de {0} a déjà été changé ce tour-ci")]
    AlreadyChanged(String),
    #[error("régime « {diet} » impossible à {province} : {reasons}")]
    Unavailable {
        diet: String,
        province: String,
        reasons: String,
    },
}

/// One line of the « La Table » panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DietOption {
    pub diet: DietId,
    pub name: String,
    /// Cost for this province this season (livres).
    pub cost: i64,
    pub available: bool,
    /// French reasons when unavailable (empty otherwise).
    pub reasons: Vec<String>,
    /// Currently eaten in the province.
    pub current: bool,
}

fn diet_name(data: &GameData, id: &DietId) -> String {
    data.diets
        .get(id)
        .map_or_else(|| id.to_string(), |d| d.name.display.clone())
}

/// French reasons why `faction` cannot feed `province` with `diet` (empty:
/// allowed). Control of the province is checked by [`set_diet`], not here.
pub fn diet_blockers(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    province: &ProvinceId,
    diet: &Diet,
) -> Vec<String> {
    let mut reasons = Vec::new();
    let requirements = &diet.requirements;
    let faction_state = state.factions.get(faction);
    let province_data = data.provinces.get(province);
    let province_state = state.provinces.get(province);
    for resource in &requirements.resources {
        let accessible = faction_state
            .and_then(|f| f.goods.get(resource))
            .is_some_and(|count| *count > 0);
        if !accessible {
            let name = data
                .resources
                .get(resource)
                .map_or_else(|| resource.to_string(), |r| r.name.display.clone());
            reasons.push(format!(
                "ressource inaccessible : {name} (aucune province contrôlée ou alliée n'en produit)"
            ));
        }
    }
    if requirements.coastal && !province_data.is_some_and(|p| p.coastal) {
        reasons.push("la province doit être côtière".to_owned());
    }
    if let Some(tech) = &requirements.technology {
        if !faction_state.is_some_and(|f| f.technologies.contains(tech)) {
            let name = data.tech_name(tech);
            reasons.push(format!("technologie requise : {name}"));
        }
    }
    if !requirements.any_building.is_empty()
        && !province_state.is_some_and(|_| {
            let buildings = state.province_buildings(province);
            requirements
                .any_building
                .iter()
                .any(|b| data.has_building(&buildings, b))
        })
    {
        let names: Vec<String> = requirements
            .any_building
            .iter()
            .map(|b| data.building_name(b))
            .collect();
        reasons.push(format!("bâtiment requis : {}", names.join(" ou ")));
    }
    if !requirements.terrains.is_empty()
        && !province_data.is_some_and(|p| requirements.terrains.contains(&p.terrain))
    {
        let names: Vec<&str> = requirements
            .terrains
            .iter()
            .map(|t| terrain_label_fr(*t))
            .collect();
        reasons.push(format!("terrain requis : {}", names.join(", ")));
    }
    reasons
}

fn terrain_label_fr(terrain: data_model::Terrain) -> &'static str {
    use data_model::Terrain as T;
    match terrain {
        T::Plains => "plaine",
        T::Hills => "collines",
        T::Mountains => "montagnes",
        T::Forest => "forêt",
        T::Marsh => "marais",
        T::Heath => "lande",
        T::Bocage => "bocage",
        T::Steppe => "steppe",
        T::Desert => "désert",
    }
}

impl CampaignState {
    /// Diet actually eaten in `province`: the controller's choice, else the
    /// default.
    pub fn province_diet(&self, province: &ProvinceId) -> DietId {
        self.provinces
            .get(province)
            .and_then(|p| {
                controller_choice(self, province, p.diet.as_ref()).map(|choice| choice.diet.clone())
            })
            .unwrap_or_else(default_diet)
    }

    /// `true` during the spring turn (Lent and lean days).
    pub fn is_lent(&self) -> bool {
        self.season == Season::Spring
    }

    /// Cost of feeding `province` with `diet` this season (livres).
    pub fn diet_cost(&self, data: &GameData, province: &ProvinceId, diet: &DietId) -> i64 {
        let (Some(p), Some(diet)) = (self.provinces.get(province), data.diets.get(diet)) else {
            return 0;
        };
        let mut cost = p.population.total() as f64 / 1000.0 * diet.cost_per_thousand;
        if self.season == Season::Winter && diet.winter_rule == WinterRule::Fresh {
            cost *= WINTER_FRESH_COST_PERCENT / 100.0;
        }
        cost.round().max(0.0) as i64
    }

    /// Table budget of `faction` this season: the diets of every province it
    /// controls.
    pub fn faction_table_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.controlled_provinces(faction)
            .map(|id| self.diet_cost(data, id, &self.province_diet(id)))
            .sum()
    }

    /// Every diet for `province` as its controller sees it, in id order.
    pub fn diet_options(&self, data: &GameData, province: &ProvinceId) -> Vec<DietOption> {
        let Some(controller) = self.province_controller(province) else {
            return Vec::new();
        };
        let current = self.province_diet(province);
        data.diets
            .values()
            .map(|diet| {
                let reasons = diet_blockers(self, data, controller, province, diet);
                DietOption {
                    diet: diet.id.clone(),
                    name: diet.name.display.clone(),
                    cost: self.diet_cost(data, province, &diet.id),
                    available: reasons.is_empty(),
                    reasons,
                    current: diet.id == current,
                }
            })
            .collect()
    }
}

/// Validates and applies `SetDiet { province, diet }` for `faction`.
pub fn set_diet(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    province: &ProvinceId,
    diet: &DietId,
) -> Result<(), DietError> {
    let definition = data
        .diets
        .get(diet)
        .ok_or_else(|| DietError::UnknownDiet(diet.clone()))?;
    let name = data.province_name(province);
    let Some(p) = state.provinces.get(province) else {
        return Err(DietError::NotControlled(name));
    };
    if !state.controls_province(faction, province) {
        return Err(DietError::NotControlled(name));
    }
    if changed_this_turn(state, p.diet.as_ref(), faction) {
        return Err(DietError::AlreadyChanged(name));
    }
    let reasons = diet_blockers(state, data, faction, province, definition);
    if !reasons.is_empty() {
        return Err(DietError::Unavailable {
            diet: definition.name.display.clone(),
            province: name,
            reasons: reasons.join(" ; "),
        });
    }
    let turn = state.turn;
    state.provinces.get_mut(province).expect("checked").diet = Some(DietChoice {
        diet: diet.clone(),
        faction: faction.clone(),
        turn,
    });
    Ok(())
}

/// Sets `province` back to the default diet (the choice keeps its turn so
/// that the one-change-per-turn rule still holds).
fn revert_to_default(state: &mut CampaignState, province: &ProvinceId) {
    if let Some(p) = state.provinces.get_mut(province) {
        if let Some(choice) = &mut p.diet {
            choice.diet = default_diet();
        }
    }
}

/// Diet-related events are only journaled for the player.
fn push_player_event(
    state: &CampaignState,
    faction: &FactionId,
    province: Option<&ProvinceId>,
    text: String,
    events: &mut Vec<GameEvent>,
) {
    province_policy::push_player_event(state, EventKind::Table, faction, province, text, events);
}

/// Turn phase (after the goods): diets whose requirements no longer hold
/// fall back to the default.
pub(crate) fn resolve_requirements(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for id in ids {
        let diet_id = state.province_diet(&id);
        if diet_id.as_str() == DEFAULT_DIET {
            continue;
        }
        let Some(controller) = state.province_controller(&id).cloned() else {
            continue;
        };
        let reasons = match data.diets.get(&diet_id) {
            Some(diet) => diet_blockers(state, data, &controller, &id, diet),
            None => vec!["régime disparu des données".to_owned()],
        };
        if reasons.is_empty() {
            continue;
        }
        revert_to_default(state, &id);
        let text = format!(
            "La Table : {} ne peut plus tenir le régime « {} » ({}) ; retour au {}.",
            data.province_name(&id),
            diet_name(data, &diet_id),
            reasons.join(" ; "),
            diet_name(data, &default_diet()).to_lowercase(),
        );
        push_player_event(state, &controller, Some(&id), text, events);
    }
}

/// Economy phase: pays the table of `faction` out of `available` livres
/// (treasury after income and other upkeep), province by province in id
/// order; a diet the purse cannot cover falls back to the default. Returns
/// the amount paid.
pub(crate) fn pay_table(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    available: i64,
    events: &mut Vec<GameEvent>,
) -> i64 {
    let provinces: Vec<ProvinceId> = state.controlled_provinces(faction).cloned().collect();
    let mut paid = 0;
    for id in provinces {
        let diet = state.province_diet(&id);
        let cost = state.diet_cost(data, &id, &diet);
        if cost == 0 {
            continue;
        }
        if available - paid >= cost {
            paid += cost;
            continue;
        }
        revert_to_default(state, &id);
        let text = format!(
            "La Table : le trésor ne peut payer le régime « {} » de {} ({cost} livres) ; retour au {}.",
            diet_name(data, &diet),
            data.province_name(&id),
            diet_name(data, &default_diet()).to_lowercase(),
        );
        push_player_event(state, faction, Some(&id), text, events);
    }
    paid
}

/// Percent bonus of `faction`'s technologies to diet `Health` effects.
fn diet_health_percent(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    let Some(f) = state.factions.get(faction) else {
        return 0.0;
    };
    f.technologies
        .iter()
        .filter_map(|id| data.technologies.get(id))
        .flat_map(|t| t.effects.iter())
        .filter(|e| e.effect == EffectKind::DietHealth)
        .map(|e| e.value)
        .sum()
}

/// Population effects of the diet of `province` for `class` (effects
/// without a class apply to every class).
pub fn diet_class_effects(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
    class: SocialClass,
) -> EffectTotals {
    let mut totals = EffectTotals::default();
    let Some(controller) = state.province_controller(province) else {
        return totals;
    };
    let Some(diet) = data.diets.get(&state.province_diet(province)) else {
        return totals;
    };
    let health_factor = 1.0 + diet_health_percent(state, data, controller) / 100.0;
    for effect in diet
        .effects
        .iter()
        .filter(|e| e.class.is_none_or(|c| c == class))
    {
        match effect.effect {
            EffectKind::Health => {
                totals.add(effect.effect, effect.mode, effect.value * health_factor);
            }
            EffectKind::Growth
            | EffectKind::Wealth
            | EffectKind::Unrest
            | EffectKind::GoodsSatisfaction => totals.add(effect.effect, effect.mode, effect.value),
            _ => {}
        }
    }
    totals
}

/// Morale bonus of the troops raised in `province` (diet `ArmyMorale`).
pub fn recruit_morale_bonus(state: &CampaignState, data: &GameData, province: &ProvinceId) -> f64 {
    data.diets
        .get(&state.province_diet(province))
        .map_or(0.0, |diet| {
            diet.effects
                .iter()
                .filter(|e| e.effect == EffectKind::ArmyMorale && e.mode == EffectMode::Add)
                .map(|e| e.value)
                .sum()
        })
}

fn add_piety(state: &mut CampaignState, faction: &FactionId, delta: i32) {
    let Some(ruler) = state.factions.get(faction).and_then(|f| f.ruler.clone()) else {
        return;
    };
    if let Some(c) = state.characters.get_mut(&ruler).filter(|c| c.alive) {
        c.piety = (i32::from(c.piety) + delta).clamp(0, 100) as u8;
    }
}

fn add_prestige(state: &mut CampaignState, faction: &FactionId, delta: i32) {
    let Some(ruler) = state.factions.get(faction).and_then(|f| f.ruler.clone()) else {
        return;
    };
    if let Some(c) = state.characters.get_mut(&ruler).filter(|c| c.alive) {
        c.prestige += delta;
    }
}

/// Turn phase (before the population): the ruler's piety and prestige from
/// the table, and Lent in spring.
pub(crate) fn resolve_lent(
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
    let lent = state.is_lent();
    for faction in factions {
        let diets: Vec<(ProvinceId, &Diet)> = state
            .controlled_provinces(&faction)
            .filter_map(|id| {
                data.diets
                    .get(&state.province_diet(id))
                    .map(|d| (id.clone(), d))
            })
            .collect();
        // Piety and prestige of the court table: the best province counts.
        let best = |kind: EffectKind| -> i32 {
            diets
                .iter()
                .flat_map(|(_, d)| d.effects.iter())
                .filter(|e| e.effect == kind && e.mode == EffectMode::Add)
                .map(|e| e.value.round() as i32)
                .max()
                .unwrap_or(0)
                .max(0)
        };
        let piety = best(EffectKind::Piety);
        let prestige = best(EffectKind::Prestige);
        if !lent {
            add_piety(state, &faction, piety);
            add_prestige(state, &faction, prestige);
            continue;
        }
        let breaking: Vec<ProvinceId> = diets
            .iter()
            .filter(|(_, d)| matches!(d.lent_rule, LentRule::Meat | LentRule::Dairy))
            .map(|(id, _)| id.clone())
            .collect();
        let fasting = diets.iter().any(|(_, d)| d.lent_rule == LentRule::Fish);
        let mut delta = piety;
        if !breaking.is_empty() {
            delta -= LENT_PIETY_PENALTY;
        }
        if fasting {
            delta += LENT_FISH_PIETY;
        }
        add_piety(state, &faction, delta);
        add_prestige(state, &faction, prestige);
        for id in &breaking {
            if let Some(p) = state.provinces.get_mut(id) {
                let clergy = &mut p.population.clergy;
                clergy.unrest = (i32::from(clergy.unrest) + LENT_CLERGY_UNREST).clamp(0, 100) as u8;
            }
        }
        if !breaking.is_empty() {
            let names: Vec<String> = breaking.iter().map(|p| data.province_name(p)).collect();
            push_player_event(
                state,
                &faction,
                breaking.first(),
                format!(
                    "Carême : on mange gras à {} ; le clergé s'indigne et la piété du souverain en souffre (−{LENT_PIETY_PENALTY}).",
                    names.join(", ")
                ),
                events,
            );
        }
        if fasting {
            push_player_event(
                state,
                &faction,
                None,
                format!(
                    "Carême : le poisson des jours maigres édifie le clergé (piété +{LENT_FISH_PIETY})."
                ),
                events,
            );
        }
    }
}

/// Simple deterministic AI (ADR 0003, shared by both planners): in each
/// controlled province, keep the current diet while it is affordable,
/// otherwise pick the available diet with the best score per livre while the
/// table budget stays below [`AI_TABLE_INCOME_PERCENT`] % of income and the
/// treasury covers [`AI_TABLE_RESERVE_SEASONS`] seasons of it; in debt,
/// every province returns to the default.
pub fn ai_choose_diets(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    if data.diets.is_empty() || !f.alive {
        return Vec::new();
    }
    let default = default_diet();
    let provinces: Vec<ProvinceId> = state.controlled_provinces(faction).cloned().collect();
    let mut orders = Vec::new();
    if f.treasury < 0 {
        for id in provinces {
            if state.province_diet(&id) != default {
                orders.push(Order::SetDiet {
                    province: id,
                    diet: default.clone(),
                });
            }
        }
        return orders;
    }
    let income = f.last_budget.income.max(f.last_budget.income).max(0);
    let mut budget = state.faction_table_upkeep(data, faction);
    let ceiling =
        (income * AI_TABLE_INCOME_PERCENT / 100).min(f.treasury / AI_TABLE_RESERVE_SEASONS.max(1));
    for id in provinces {
        let current = state.province_diet(&id);
        if current != default {
            continue;
        }
        let controller = faction;
        let best = data
            .diets
            .values()
            .filter(|d| d.id != default)
            .filter(|d| diet_blockers(state, data, controller, &id, d).is_empty())
            .map(|d| {
                let cost = state.diet_cost(data, &id, &d.id).max(1);
                let value: f64 = d
                    .effects
                    .iter()
                    .filter(|e| e.mode == EffectMode::Add)
                    .map(|e| match e.effect {
                        EffectKind::Unrest => -e.value,
                        _ => e.value,
                    })
                    .sum();
                (value / cost as f64, cost, d.id.clone())
            })
            .filter(|(score, _, _)| *score > 0.0)
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.2.cmp(&a.2)));
        if let Some((_, cost, diet)) = best {
            if budget + cost <= ceiling {
                budget += cost;
                orders.push(Order::SetDiet { province: id, diet });
            }
        }
    }
    orders
}
