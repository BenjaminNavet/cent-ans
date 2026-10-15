//! Recruitment, mercenaries, army roster details and stances.
//!
//! Replaces `CampaignSim.get_recruitable`, the recruitment part of `settlement_detail`
//! (queue, slots, armies a recruit may join), `get_mercenaries` and `get_stance_options`.
//! Orders are not here: the client submits `Order::Recruit`, `RecruitInto`, `CancelRecruit`,
//! `HireMercenary`, `DisbandUnit`, `SetStance`... through `CampaignState::submit_order_outcome`.

use data_model::{GameData, ProvinceId, SettlementId};
use serde::Serialize;
use sim_campaign::{ArmyId, CampaignState, OrderError};

/// One requirement of a unit (building, technology, resource id or a tag), met or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementView {
    pub id: String,
    pub met: bool,
}

/// One recruitment card: a unit type a settlement (or an army's province) can raise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecruitCard {
    pub unit_type: String,
    pub name: String,
    /// Price in livres, import cost included.
    pub cost: u32,
    /// Pay per season.
    pub upkeep: u32,
    pub available: bool,
    /// French refusal, empty when available.
    pub reason: String,
    /// Men of a full-strength unit.
    pub soldiers: u32,
    pub recruit_time_turns: u32,
    /// `infantry`, `ranged`, `cavalry`, `siege`.
    pub category: String,
    /// `ready`, `blocked`, `soon`, `elsewhere`.
    pub group: String,
    /// Part of `cost` spent importing missing resources.
    pub import_cost: u32,
    pub pool_available: u32,
    pub pool_cap: u32,
    /// `-1` when full or never refilled.
    pub pool_seasons_to_next: i64,
    pub pool_label: String,
    pub requirements: Vec<RequirementView>,
}

/// A recruit waiting in the settlement's queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueuedView {
    pub unit_type: String,
    pub name: String,
    pub turns_left: u32,
    /// Livres returned by cancelling now.
    pub refund: i64,
    /// Army that receives the unit, empty = garrison.
    pub army: String,
}

/// An own army a recruit may be sent to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecruitTarget {
    pub id: String,
    pub name: String,
    pub units: usize,
    /// The army stands in the province but not in this settlement.
    pub field: bool,
}

/// The recruitment screen of one settlement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecruitView {
    pub settlement: String,
    pub settlement_name: String,
    pub province: String,
    pub province_name: String,
    /// The player controls it (recruiting allowed).
    pub own: bool,
    pub slots: usize,
    pub slots_free: usize,
    pub treasury: i64,
    pub queue: Vec<QueuedView>,
    pub options: Vec<RecruitCard>,
    pub armies: Vec<RecruitTarget>,
}

fn unit_name(data: &GameData, id: &data_model::UnitTypeId) -> String {
    data.unit_types
        .get(id)
        .map_or_else(|| id.as_str().to_owned(), |t| t.name.display.clone())
}

/// A settlement id, or a province id standing for its city.
pub fn settlement_or_city(state: &CampaignState, raw: &str) -> Option<SettlementId> {
    if let Ok(id) = SettlementId::new(raw) {
        return state.settlement_state(&id).map(|_| id);
    }
    let province = ProvinceId::new(raw).ok()?;
    state.province_city_id(&province).cloned()
}

fn card(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
    option: &sim_campaign::orders::RecruitOption,
) -> RecruitCard {
    let unit = data.unit_types.get(&option.unit_type);
    RecruitCard {
        unit_type: option.unit_type.as_str().to_owned(),
        name: option.name.clone(),
        cost: option.cost,
        upkeep: option.upkeep,
        available: option.available,
        reason: option.reason.clone().unwrap_or_default(),
        soldiers: unit.map_or(0, |t| t.soldiers),
        recruit_time_turns: unit.and_then(|t| t.recruit_time_turns).unwrap_or(1),
        category: unit.map_or_else(String::new, |t| t.category.key().to_owned()),
        group: option.group().as_str().to_owned(),
        import_cost: option.import_cost,
        pool_available: option.pool.available,
        pool_cap: option.pool.cap,
        pool_seasons_to_next: option.pool.seasons_to_next.map_or(-1, i64::from),
        pool_label: option.pool.label_fr(),
        requirements: state
            .recruit_requirements(data, settlement, &option.unit_type)
            .into_iter()
            .map(|r| RequirementView {
                id: r.id,
                met: r.met,
            })
            .collect(),
    }
}

/// Recruitment screen of `place` (a settlement id, or a province id for its city).
/// `None` for an unknown place.
pub fn recruit_view(data: &GameData, state: &CampaignState, place: &str) -> Option<RecruitView> {
    let id = settlement_or_city(state, place)?;
    let live = state.settlement_state(&id)?;
    let player = state.player_faction();
    let own = &live.owner == player && &live.controller == player;
    let options = if own {
        state
            .recruitable(data, &id)
            .iter()
            .map(|option| card(data, state, &id, option))
            .collect()
    } else {
        Vec::new()
    };
    let queue = live
        .recruit_queue
        .iter()
        .map(|queued| QueuedView {
            unit_type: queued.unit_type.as_str().to_owned(),
            name: unit_name(data, &queued.unit_type),
            turns_left: queued.turns_left,
            refund: state.recruit_refund(data, queued),
            army: queued
                .into_army
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
        })
        .collect();
    let armies = state
        .armies()
        .iter()
        .filter(|(army_id, a)| {
            a.faction == live.controller
                && (a.is_at(&id)
                    || (state.army_recruit_blocker(data, army_id).is_none()
                        && state.army_province(data, a).as_ref() == Some(&live.province)))
        })
        .map(|(army_id, a)| RecruitTarget {
            id: army_id.to_string(),
            name: state.army_name(data, army_id),
            units: a.units.len(),
            field: !a.is_at(&id),
        })
        .collect();
    Some(RecruitView {
        settlement: id.as_str().to_owned(),
        settlement_name: data
            .settlements
            .get(&id)
            .map_or_else(|| id.as_str().to_owned(), |s| s.name.display.clone()),
        province: live.province.as_str().to_owned(),
        province_name: data.province_name(&live.province),
        own,
        slots: state.recruit_slots(data, &id),
        slots_free: state.recruit_slots_free(data, &id),
        treasury: state.faction_state(player).map_or(0, |f| f.treasury),
        queue,
        options,
        armies,
    })
}

/// One mercenary company on offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MercenaryCard {
    pub unit_type: String,
    pub name: String,
    pub band: String,
    pub band_name: String,
    pub cost: u32,
    pub upkeep: u32,
    pub experience: u8,
    pub available: bool,
    pub reason: String,
    pub soldiers: u32,
    pub category: String,
    pub pool_available: u32,
    pub pool_cap: u32,
    pub pool_seasons_to_next: i64,
    pub pool_label: String,
}

/// What an army may hire where it stands.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MercenaryView {
    pub region: String,
    pub region_name: String,
    pub hires_left: u32,
    /// French reason the army hires nothing here, empty when it may.
    pub blocked: String,
    /// Premium paid last season above the ordinary upkeep.
    pub premium_last_turn: i64,
    pub options: Vec<MercenaryCard>,
}

/// Mercenary market of army `army`; `None` for an unknown army.
pub fn mercenary_view(
    data: &GameData,
    state: &CampaignState,
    army: &ArmyId,
) -> Option<MercenaryView> {
    let market = state.mercenary_market(data, army)?;
    let faction = &state.army(army)?.faction;
    let premium = state
        .mercenaries
        .premium_last_turn
        .get(faction)
        .copied()
        .unwrap_or(0);
    Some(MercenaryView {
        region: market.region.clone().unwrap_or_default(),
        region_name: market.region_name.clone(),
        hires_left: market.hires_left,
        blocked: market.blocked.clone().unwrap_or_default(),
        premium_last_turn: premium,
        options: market
            .options
            .iter()
            .map(|o| {
                let unit = data.unit_types.get(&o.unit_type);
                MercenaryCard {
                    unit_type: o.unit_type.as_str().to_owned(),
                    name: o.name.clone(),
                    band: o.band.clone(),
                    band_name: o.band_name.clone(),
                    cost: o.cost,
                    upkeep: o.upkeep,
                    experience: o.experience,
                    available: o.available,
                    reason: o.reason.clone().unwrap_or_default(),
                    soldiers: unit.map_or(0, |t| t.soldiers),
                    category: unit.map_or_else(String::new, |t| t.category.key().to_owned()),
                    pool_available: o.pool.available,
                    pool_cap: o.pool.cap,
                    pool_seasons_to_next: o.pool.seasons_to_next.map_or(-1, i64::from),
                    pool_label: o.pool.label_fr(),
                }
            })
            .collect(),
    })
}

/// Data of one regiment the army sheet's `UnitView` lacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegimentExtra {
    pub upkeep: u32,
    pub category: String,
    pub soldiers: u32,
    pub mercenary: bool,
}

/// Per-regiment extras of army `army`, in roster order (same order as `ArmySheet::units`),
/// and the army's total upkeep per season. `None` for an unknown army.
pub fn army_extras(
    data: &GameData,
    state: &CampaignState,
    army: &ArmyId,
) -> Option<(Vec<RegimentExtra>, u32)> {
    let found = state.army(army)?;
    let extras: Vec<RegimentExtra> = found
        .units
        .iter()
        .map(|u| {
            let t = data.unit_types.get(&u.unit_type);
            RegimentExtra {
                upkeep: t.map_or(0, |t| t.upkeep),
                category: t.map_or_else(String::new, |t| t.category.key().to_owned()),
                soldiers: t.map_or(0, |t| t.soldiers),
                mercenary: t.is_some_and(|t| t.mercenary),
            }
        })
        .collect();
    let total = extras.iter().map(|e| e.upkeep).sum();
    Some((extras, total))
}

/// One numeric stat of a unit type, in the order the sheet shows them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DetailStat {
    /// `melee`, `ranged`, `range`, `armor`, `morale`, `speed`, `charge`, `siege_attack`, `ammo`.
    pub key: String,
    pub value: f64,
}

/// Detailed sheet of one regiment (right-click, pinned tooltip): the unit type's definition next
/// to the live figures. No rule lives in the client: strengths and weaknesses are computed here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegimentDetail {
    pub unit_type: String,
    pub name: String,
    pub category: String,
    pub soldiers: u32,
    pub mounted: bool,
    pub mercenary: bool,
    pub upkeep: u32,
    pub recruit_time_turns: Option<u32>,
    pub strength: u32,
    pub max_strength: u32,
    pub morale: u8,
    pub experience: u8,
    pub stats: Vec<DetailStat>,
    /// Stat keys clearly above the average of all unit types (>= 130 %).
    pub strengths: Vec<String>,
    /// Stat keys clearly below the average (<= 70 %); `ranged` with value 0 means "does not shoot".
    pub weaknesses: Vec<String>,
    /// Ability keys (`rain_penalty` excluded: it is a weakness).
    pub abilities: Vec<String>,
    pub rain_penalty: bool,
    pub required_technology: Option<String>,
    pub source_class: String,
    pub available_from: Option<i32>,
    pub available_until: Option<i32>,
    pub description: String,
}

const COMPARED_STATS: [&str; 7] = [
    "melee",
    "ranged",
    "armor",
    "morale",
    "speed",
    "charge",
    "siege_attack",
];

fn stat_value(stats: &data_model::UnitStats, key: &str) -> Option<f64> {
    Some(match key {
        "melee" => f64::from(stats.melee),
        "ranged" => f64::from(stats.ranged),
        "range" => f64::from(stats.range),
        "armor" => f64::from(stats.armor),
        "morale" => f64::from(stats.morale),
        "speed" => f64::from(stats.speed),
        "ammo" => f64::from(stats.ammo),
        "charge" => f64::from(stats.charge?),
        "siege_attack" => f64::from(stats.siege_attack?),
        _ => return None,
    })
}

/// Detailed sheet of regiment `index` of army `army`; `None` for an unknown army or index.
pub fn regiment_detail(
    data: &GameData,
    state: &CampaignState,
    army: &ArmyId,
    index: usize,
) -> Option<RegimentDetail> {
    let unit = state.army(army)?.units.get(index)?;
    let definition = data.unit_types.get(&unit.unit_type);
    let mut detail = RegimentDetail {
        unit_type: unit.unit_type.as_str().to_owned(),
        name: definition.map_or_else(
            || unit.unit_type.as_str().to_owned(),
            |t| t.name.display.clone(),
        ),
        category: definition.map_or_else(String::new, |t| t.category.key().to_owned()),
        soldiers: definition.map_or(0, |t| t.soldiers),
        mounted: definition.is_some_and(|t| t.mounted),
        mercenary: definition.is_some_and(|t| t.mercenary),
        upkeep: definition.map_or(0, |t| t.upkeep),
        recruit_time_turns: definition.and_then(|t| t.recruit_time_turns),
        strength: unit.strength,
        max_strength: unit.max_strength,
        morale: unit.morale,
        experience: unit.experience,
        stats: Vec::new(),
        strengths: Vec::new(),
        weaknesses: Vec::new(),
        abilities: Vec::new(),
        rain_penalty: false,
        required_technology: None,
        source_class: String::new(),
        available_from: None,
        available_until: None,
        description: String::new(),
    };
    let Some(definition) = definition else {
        return Some(detail);
    };
    let stats = &definition.stats;
    for key in [
        "melee",
        "ranged",
        "range",
        "armor",
        "morale",
        "speed",
        "charge",
        "siege_attack",
        "ammo",
    ] {
        let shown = stat_value(stats, key)
            .filter(|value| *value > 0.0 || matches!(key, "melee" | "armor" | "morale"));
        if let Some(value) = shown {
            detail.stats.push(DetailStat {
                key: key.to_owned(),
                value,
            });
        }
    }
    for key in COMPARED_STATS {
        let Some(value) = stat_value(stats, key) else {
            continue;
        };
        let (total, count) = data
            .unit_types
            .values()
            .filter_map(|t| stat_value(&t.stats, key))
            .fold((0.0, 0u32), |(sum, n), v| (sum + v, n + 1));
        if count == 0 || total <= 0.0 {
            continue;
        }
        let average = total / f64::from(count);
        if value >= average * 1.3 {
            detail.strengths.push(key.to_owned());
        } else if value <= average * 0.7 && key != "siege_attack" && key != "charge" {
            detail.weaknesses.push(key.to_owned());
        }
    }
    for ability in &definition.abilities {
        let key = serde_json::to_value(ability)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        if key == "rain_penalty" {
            detail.rain_penalty = true;
        } else {
            detail.abilities.push(key);
        }
    }
    detail.required_technology = definition
        .required_technology
        .as_ref()
        .map(|t| t.as_str().to_owned());
    detail.source_class = definition.source_class.key().to_owned();
    detail.available_from = definition.available_from;
    detail.available_until = definition.available_until;
    detail.description = definition.description.clone().unwrap_or_default();
    Some(detail)
}

/// One stance the army may take: `refusal` is empty when allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StanceOption {
    pub key: String,
    pub refusal: String,
}

/// Every stance with the core's refusal, in the core's order; empty for an unknown army.
pub fn stance_options(data: &GameData, state: &CampaignState, army: &ArmyId) -> Vec<StanceOption> {
    if state.army(army).is_none() {
        return Vec::new();
    }
    sim_campaign::posture::stance_options(state, data, army)
        .into_iter()
        .map(|(stance, check)| StanceOption {
            key: stance.key().to_owned(),
            refusal: match check {
                Ok(()) => String::new(),
                Err(OrderError::StanceRefused(reason)) => reason,
                Err(error) => error.to_string(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::{Order, Place};

    fn start() -> (&'static GameData, CampaignState) {
        let data = data_model::test_support::game_data();
        (
            data,
            sim_campaign::test_support::start_quiet(data, "fac_france", 1337),
        )
    }

    #[test]
    fn capital_recruit_view_and_order() {
        let (data, mut state) = start();
        let city = sim_campaign::test_support::capital_city(&state, "fac_france");
        let view = recruit_view(data, &state, city.as_str()).unwrap();
        assert!(view.own && view.slots > 0 && view.slots_free > 0);
        assert!(!view.options.is_empty());
        let ready = view
            .options
            .iter()
            .find(|o| o.available && i64::from(o.cost) <= view.treasury)
            .expect("an available unit");
        assert!(ready.soldiers > 0 && !ready.category.is_empty());
        let by_province = recruit_view(data, &state, &view.province).unwrap();
        assert_eq!(by_province.settlement, view.settlement);
        let before = view.treasury;
        state
            .submit_order_outcome(
                data,
                Order::Recruit {
                    settlement: Place::Settlement(city.clone()),
                    unit_type: sim_campaign::test_support::unit_type(&ready.unit_type),
                },
            )
            .unwrap();
        let after = recruit_view(data, &state, city.as_str()).unwrap();
        assert_eq!(after.queue.len(), 1);
        assert!(after.treasury < before);
        assert_eq!(after.slots_free, view.slots_free - 1);
        assert!(after.queue[0].refund > 0);
        assert!(serde_json::to_string(&after).is_ok());
    }

    #[test]
    fn foreign_place_and_unknown() {
        let (data, state) = start();
        assert!(recruit_view(data, &state, "nowhere").is_none());
        let english = sim_campaign::test_support::capital_city(&state, "fac_england");
        let view = recruit_view(data, &state, english.as_str()).unwrap();
        assert!(!view.own && view.options.is_empty());
    }

    #[test]
    fn mercenaries_stances_extras() {
        let (data, state) = start();
        let army = sim_campaign::test_support::main_army(&state, "fac_france");
        let market = mercenary_view(data, &state, &army).unwrap();
        assert!(!market.region_name.is_empty());
        assert!(serde_json::to_string(&market).is_ok());
        let stances = stance_options(data, &state, &army);
        assert!(stances
            .iter()
            .any(|s| s.key == "normal" && s.refusal.is_empty()));
        let (extras, upkeep) = army_extras(data, &state, &army).unwrap();
        assert_eq!(
            extras.len(),
            crate::army::army_sheet(data, &state, &army)
                .unwrap()
                .units
                .len()
        );
        assert!(upkeep > 0);
        let ghost = ArmyId::parse("army_9999").unwrap();
        assert!(mercenary_view(data, &state, &ghost).is_none());
        assert!(stance_options(data, &state, &ghost).is_empty());
        assert!(army_extras(data, &state, &ghost).is_none());
    }

    #[test]
    fn regiment_detail_matches_roster() {
        let (data, state) = start();
        let army = sim_campaign::test_support::main_army(&state, "fac_france");
        let sheet = crate::army::army_sheet(data, &state, &army).unwrap();
        let detail = regiment_detail(data, &state, &army, 0).unwrap();
        assert_eq!(detail.unit_type, sheet.units[0].unit_type);
        assert_eq!(detail.strength, sheet.units[0].strength);
        assert!(detail.stats.iter().any(|s| s.key == "melee"));
        assert!(!detail.description.is_empty() || detail.soldiers > 0);
        assert!(regiment_detail(data, &state, &army, 999).is_none());
        assert!(serde_json::to_string(&detail).is_ok());
    }
}
