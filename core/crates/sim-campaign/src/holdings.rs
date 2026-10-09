//! Settlements quick-access list (touche B), lot HL1.
//! `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 1.
//!
//! [`holdings_overview`] is a pure query, without side effect: it never
//! changes `CampaignState`. It reuses the same building blocks as
//! `settlement_detail` in `godot-bridge/src/campaign_sim_settlements.rs`
//! (`settlement_tax`, `buildable`, `garrison_cap`) so both stay in sync.

use data_model::key_enum;
use data_model::{FactionId, GameData, ProvinceId, SettlementId, SettlementKind};

use crate::population;
use crate::state::CampaignState;

key_enum! {
/// Why a settlement is [`SettlementRow::endangered`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DangerReason {
    Siege => "siege",
    Occupied => "occupied",
    RevoltCountdown => "revolt_countdown",
    UnrestAboveThreshold => "unrest",
}
}

/// One buildable or promotable option offered by a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BuildOptionRow {
    pub building: data_model::BuildingId,
    pub name: String,
    pub cost: u32,
    pub is_upgrade: bool,
}

/// A construction under way in a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ConstructionRow {
    pub building: data_model::BuildingId,
    pub name: String,
    pub turns_left: u32,
}

/// A siege under way against a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SiegeRow {
    pub attacker: FactionId,
    pub turns_left: u32,
}

/// One settlement of a [`ProvinceRow`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SettlementRow {
    pub id: SettlementId,
    pub name: String,
    pub kind: SettlementKind,
    pub is_city: bool,
    pub occupied: bool,
    pub income: i64,
    pub construction: Option<ConstructionRow>,
    pub idle: bool,
    pub options_available: Vec<BuildOptionRow>,
    pub upgrade_available: bool,
    pub recruit_queue_len: usize,
    pub garrison_strength: u32,
    pub garrison_free: i64,
    pub siege: Option<SiegeRow>,
    pub endangered: bool,
    pub danger_reasons: Vec<DangerReason>,
}

/// One province holding at least one settlement of [`HoldingsOverview`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProvinceRow {
    pub province: ProvinceId,
    pub name: String,
    pub income: i64,
    pub unrest: i64,
    pub revolt_seasons: u32,
    pub revolt_seasons_needed: u32,
    pub revolt_threshold: f64,
    pub devastation: u8,
    pub slots_busy: usize,
    pub slots_total: usize,
    pub settlements: Vec<SettlementRow>,
}

/// Settlements quick-access overview of `faction` (touche B).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HoldingsOverview {
    pub treasury: i64,
    pub net_income_last_turn: i64,
    pub settlement_income_total: i64,
    pub count_idle: usize,
    pub count_upgrade: usize,
    pub count_endangered: usize,
    pub provinces: Vec<ProvinceRow>,
}

/// Builds the settlements quick-access overview of `faction`: the
/// settlements it controls, plus those it owns but does not control
/// (`occupied`), grouped by their province. Pure, no side effect. Empty for
/// an unknown faction.
pub fn holdings_overview(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> HoldingsOverview {
    let Some(faction_state) = state.factions.get(faction) else {
        return HoldingsOverview {
            treasury: 0,
            net_income_last_turn: 0,
            settlement_income_total: 0,
            count_idle: 0,
            count_upgrade: 0,
            count_endangered: 0,
            provinces: Vec::new(),
        };
    };

    let tech = crate::research::faction_province_tech_effects(state, data, faction);
    let mut provinces = Vec::new();
    let mut settlement_income_total: i64 = 0;
    let mut count_idle = 0;
    let mut count_upgrade = 0;
    let mut count_endangered = 0;

    for (province_id, province) in &state.provinces {
        let mut rows = Vec::new();
        let mut slots_busy = 0;
        let mut slots_total = 0;
        let mut province_income: i64 = 0;

        for settlement_id in &province.settlements {
            let Some(live) = state.settlements.get(settlement_id) else {
                continue;
            };
            let owned = &live.owner == faction;
            let controlled = &live.controller == faction;
            if !owned && !controlled {
                continue;
            }
            let occupied = owned && !controlled;

            let row = settlement_row(state, data, faction, &tech, settlement_id, live, occupied);
            settlement_income_total += row.income;
            province_income += row.income;
            if !occupied {
                slots_total += 1;
                if row.construction.is_some() {
                    slots_busy += 1;
                }
            }
            if row.idle {
                count_idle += 1;
            }
            if !row.options_available.is_empty() {
                count_upgrade += 1;
            }
            if row.endangered {
                count_endangered += 1;
            }
            rows.push(row);
        }

        if rows.is_empty() {
            continue;
        }

        let weighted = population::weighted_unrest(&province.population);
        let name = data.province_name(province_id);

        provinces.push(ProvinceRow {
            province: province_id.clone(),
            name,
            income: province_income,
            unrest: weighted.round() as i64,
            revolt_seasons: province.revolt_seasons,
            revolt_seasons_needed: data.population_rules.revolt_seasons.max(1),
            revolt_threshold: data.population_rules.revolt_unrest_threshold,
            devastation: province.devastation,
            slots_busy,
            slots_total,
            settlements: rows,
        });
    }

    HoldingsOverview {
        treasury: faction_state.treasury,
        net_income_last_turn: state.faction_net_last_turn(faction).unwrap_or(0),
        settlement_income_total,
        count_idle,
        count_upgrade,
        count_endangered,
        provinces,
    }
}

#[allow(clippy::too_many_arguments)]
fn settlement_row(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    tech: &crate::EffectTotals,
    id: &SettlementId,
    live: &crate::state::SettlementState,
    occupied: bool,
) -> SettlementRow {
    let entry = data.settlements.get(id);
    let name = entry.map_or_else(|| id.to_string(), |e| e.name.display.clone());
    let is_city = state.province_city_id(&live.province) == Some(id);

    let income = if live.siege.is_some() || occupied {
        0
    } else {
        let tax_rate = state
            .factions
            .get(faction)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        state.settlement_tax(data, id, tax_rate, tech).round() as i64
    };

    let construction = live.construction.as_ref().map(|c| {
        let name = data.building_name(&c.building);
        ConstructionRow {
            building: c.building.clone(),
            name,
            turns_left: c.turns_left,
        }
    });

    let idle = !occupied && construction.is_none();

    let options_available: Vec<BuildOptionRow> = if occupied || construction.is_some() {
        Vec::new()
    } else {
        state
            .buildable(data, id)
            .into_iter()
            .filter(|option| option.available)
            .map(|option| {
                let is_upgrade = data
                    .buildings
                    .get(&option.building)
                    .and_then(|b| b.upgrades_from.as_ref())
                    .is_some_and(|base| live.buildings.contains(base));
                BuildOptionRow {
                    building: option.building,
                    name: option.name,
                    cost: option.cost,
                    is_upgrade,
                }
            })
            .collect()
    };
    let upgrade_available = options_available.iter().any(|o| o.is_upgrade);

    let garrison_cap: Option<usize> = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&live.kind))
        .copied();
    let garrison_free = garrison_cap
        .map(|cap| cap.saturating_sub(live.garrison.len()) as i64)
        .unwrap_or(-1);

    let siege = live.siege.as_ref().map(|s| SiegeRow {
        attacker: s.attacker.clone(),
        turns_left: s.turns_left,
    });

    let province = state.provinces.get(&live.province);
    let revolt_seasons = province.map_or(0, |p| p.revolt_seasons);
    let weighted_unrest = province.map_or(0.0, |p| population::weighted_unrest(&p.population));

    let mut danger_reasons = Vec::new();
    if siege.is_some() {
        danger_reasons.push(DangerReason::Siege);
    }
    if occupied {
        danger_reasons.push(DangerReason::Occupied);
    }
    if revolt_seasons > 0 {
        danger_reasons.push(DangerReason::RevoltCountdown);
    }
    if weighted_unrest >= data.population_rules.revolt_unrest_threshold {
        danger_reasons.push(DangerReason::UnrestAboveThreshold);
    }
    let endangered = !danger_reasons.is_empty();

    SettlementRow {
        id: id.clone(),
        name,
        kind: live.kind,
        is_city,
        occupied,
        income,
        construction,
        idle,
        options_available,
        upgrade_available,
        recruit_queue_len: live.recruit_queue.len(),
        garrison_strength: live.garrison_strength(),
        garrison_free,
        siege,
        endangered,
        danger_reasons,
    }
}
