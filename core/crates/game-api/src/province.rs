//! Province sheet.
//!
//! Replaces `CampaignSim.get_province_state`, `get_province_city`, `get_province_tax`,
//! `get_province_breadcrumb` (and the settlement list of `settlement_detail`).

use data_model::{EffectKind, GameData, ProvinceId};
use serde::Serialize;
use sim_campaign::CampaignState;

use crate::common::{unit_views, KeyedAmount, UnitView};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildingView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub upkeep: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructionView {
    pub building: String,
    pub name: String,
    pub turns_left: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SettlementView {
    pub id: String,
    pub name: String,
    /// `city`, `town`, `village`, ... (the data's settlement kind key).
    pub kind: String,
    pub is_city: bool,
    pub owner: String,
    pub controller: String,
    pub fortification_level: u8,
    pub buildings: Vec<BuildingView>,
    pub construction: Option<ConstructionView>,
    pub garrison_size: usize,
    pub besieged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiegeView {
    pub attacker: String,
    pub turns_left: u32,
    pub turns_elapsed: u32,
    pub supplies: u8,
    pub breach: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClassView {
    /// `peasants`, `burghers`, `clergy`, `nobility`.
    pub class: String,
    pub count: u64,
    pub unrest: u8,
    pub health: u8,
    pub wealth: u8,
    pub goods_satisfaction: u8,
    /// Signed terms of the class's unrest target.
    pub unrest_terms: Vec<KeyedAmount>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectView {
    pub key: String,
    pub flat: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaxView {
    /// Effective bracket (`low`, `normal`, `high`).
    pub rate: String,
    /// Set for this province alone.
    pub own: bool,
    pub faction_rate: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BreadcrumbLinkView {
    pub title: String,
    pub title_name: String,
    pub rank: String,
    pub holder: String,
    pub holder_name: String,
}

/// Everything the province panel shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProvinceSheet {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub owner_name: String,
    pub controller: String,
    pub controller_name: String,
    pub city: String,
    pub governor: Option<String>,
    pub governor_name: Option<String>,
    pub settlements: Vec<SettlementView>,
    /// Buildings of the whole province (every settlement).
    pub buildings: Vec<BuildingView>,
    /// Garrison of the city.
    pub garrison: Vec<UnitView>,
    pub siege: Option<SiegeView>,
    /// Class-weighted unrest that drives revolts.
    pub unrest: i64,
    /// The province's disorder gauge (one cause of unrest among others).
    pub disorder: u8,
    pub revolt_seasons: u32,
    pub revolt_threshold: i64,
    pub revolt_seasons_needed: u32,
    /// B4 (ADR 0731): tension gauge (0 to `tension_max`), cumulative.
    pub tension: u32,
    pub tension_max: u32,
    /// B4: a rebel army holds the province (uprising under way).
    pub uprising: bool,
    pub devastation: u8,
    pub population_total: u64,
    pub classes: Vec<ClassView>,
    pub fortification_level: u32,
    pub capacity: u64,
    pub resources: Vec<String>,
    pub effects: Vec<EffectView>,
    pub construction: Option<ConstructionView>,
    pub build_queue: Vec<ConstructionView>,
    pub tax: TaxView,
    /// Gross tax income of the province for its controller this season.
    pub income: i64,
    pub breadcrumb: Vec<BreadcrumbLinkView>,
}

fn building_view(data: &GameData, id: &data_model::BuildingId) -> BuildingView {
    match data.buildings.get(id) {
        Some(b) => BuildingView {
            id: id.as_str().to_owned(),
            name: b.name.display.clone(),
            category: b.category.key().to_owned(),
            upkeep: i64::from(b.upkeep.unwrap_or(0)),
        },
        None => BuildingView {
            id: id.as_str().to_owned(),
            name: id.as_str().to_owned(),
            category: String::new(),
            upkeep: 0,
        },
    }
}

fn construction_view(
    data: &GameData,
    construction: &sim_campaign::state::Construction,
) -> ConstructionView {
    ConstructionView {
        building: construction.building.as_str().to_owned(),
        name: data.buildings.get(&construction.building).map_or_else(
            || construction.building.to_string(),
            |b| b.name.display.clone(),
        ),
        turns_left: construction.turns_left,
    }
}

const UI_EFFECT_KINDS: [(&str, EffectKind); 14] = [
    ("tax_income", EffectKind::TaxIncome),
    ("trade_income", EffectKind::TradeIncome),
    ("health", EffectKind::Health),
    ("unrest", EffectKind::Unrest),
    ("wealth", EffectKind::Wealth),
    ("goods_satisfaction", EffectKind::GoodsSatisfaction),
    ("growth", EffectKind::Growth),
    ("garrison", EffectKind::Garrison),
    ("fortification_level", EffectKind::FortificationLevel),
    ("recruit_cost", EffectKind::RecruitCost),
    ("supply", EffectKind::Supply),
    ("production", EffectKind::Production),
    ("siege_resistance", EffectKind::SiegeResistance),
    ("plague_resistance", EffectKind::PlagueResistance),
];

/// Sheet of `province`; `None` for an unknown province.
pub fn province_sheet(
    data: &GameData,
    state: &CampaignState,
    province: &ProvinceId,
) -> Option<ProvinceSheet> {
    let live = state.province_state(province)?;
    let city_state = state.city_state(province)?;
    let city = state.province_city(data, province)?;
    let unrest_terms = sim_campaign::population::unrest_breakdown(state, data, province);
    let class_entries = [
        ("peasants", &city.classes.peasants),
        ("burghers", &city.classes.burghers),
        ("clergy", &city.classes.clergy),
        ("nobility", &city.classes.nobility),
    ];
    let classes = class_entries
        .into_iter()
        .map(|(key, entry)| {
            let terms = unrest_terms
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|(class, _)| class.key() == key)
                .flat_map(|(_, terms)| terms.iter())
                .map(|(term, value)| KeyedAmount {
                    key: (*term).to_owned(),
                    value: *value as i64,
                })
                .collect();
            ClassView {
                class: key.to_owned(),
                count: entry.count,
                unrest: entry.unrest,
                health: entry.health,
                wealth: entry.wealth,
                goods_satisfaction: entry.goods_satisfaction,
                unrest_terms: terms,
            }
        })
        .collect();
    let settlements = live
        .settlements
        .iter()
        .filter_map(|id| {
            let place = state.settlement_state(id)?;
            Some(SettlementView {
                id: id.as_str().to_owned(),
                name: data
                    .settlements
                    .get(id)
                    .map_or_else(|| id.as_str().to_owned(), |s| s.name.display.clone()),
                kind: place.kind.key().to_owned(),
                is_city: *id == live.city,
                owner: place.owner.as_str().to_owned(),
                controller: place.controller.as_str().to_owned(),
                fortification_level: place.fortification_level,
                buildings: place
                    .buildings
                    .iter()
                    .map(|b| building_view(data, b))
                    .collect(),
                construction: place
                    .construction
                    .as_ref()
                    .map(|c| construction_view(data, c)),
                garrison_size: place.garrison.len(),
                besieged: place.siege.is_some(),
            })
        })
        .collect();
    let controller_faction = state.factions.get(&city_state.controller);
    let governor = state.province_governor(province);
    Some(ProvinceSheet {
        id: province.as_str().to_owned(),
        name: data.province_name(province),
        owner: city_state.owner.as_str().to_owned(),
        owner_name: data.faction_name(&city_state.owner),
        controller: city_state.controller.as_str().to_owned(),
        controller_name: data.faction_name(&city_state.controller),
        city: live.city.as_str().to_owned(),
        governor: governor.map(|g| g.as_str().to_owned()),
        governor_name: governor.map(|g| state.character_name(data, g)),
        settlements,
        buildings: city
            .buildings
            .iter()
            .map(|b| building_view(data, b))
            .collect(),
        garrison: unit_views(data, &city_state.garrison),
        siege: city_state.siege.as_ref().map(|s| SiegeView {
            attacker: s.attacker.as_str().to_owned(),
            turns_left: s.turns_left,
            turns_elapsed: s.turns_elapsed,
            supplies: s.supplies,
            breach: s.breach,
        }),
        unrest: sim_campaign::population::weighted_unrest(&live.population).round() as i64,
        disorder: live.unrest,
        revolt_seasons: live.revolt_seasons,
        revolt_threshold: data.population_rules.revolt_unrest_threshold.round() as i64,
        revolt_seasons_needed: data.revolt_rules.min_seasons_to_revolt(),
        tension: live.tension,
        tension_max: data.revolt_rules.tension_max,
        uprising: live.uprising.is_some(),
        devastation: live.devastation,
        population_total: live.population.total(),
        classes,
        fortification_level: city.fortification_level,
        capacity: city.capacity,
        resources: city
            .resources
            .iter()
            .map(|r| r.as_str().to_owned())
            .collect(),
        effects: UI_EFFECT_KINDS
            .iter()
            .map(|(key, kind)| EffectView {
                key: (*key).to_owned(),
                flat: city.effects[*kind].flat,
                percent: city.effects[*kind].percent,
            })
            .collect(),
        construction: city
            .construction
            .as_ref()
            .map(|c| construction_view(data, c)),
        build_queue: city
            .build_queue
            .iter()
            .map(|c| construction_view(data, c))
            .collect(),
        tax: TaxView {
            rate: state.effective_province_tax(province).key().to_owned(),
            own: state.has_province_tax(province),
            faction_rate: controller_faction
                .map(|f| f.tax_rate)
                .unwrap_or_default()
                .key()
                .to_owned(),
        },
        income: state
            .faction_province_incomes(data, &city_state.controller)
            .into_iter()
            .find(|(p, _)| p == province)
            .map_or(0, |(_, v)| v),
        breadcrumb: sim_campaign::feudal::province_breadcrumb(state, data, province)
            .iter()
            .map(|link| BreadcrumbLinkView {
                title: link.title.as_str().to_owned(),
                title_name: data.titles.get(&link.title).map_or_else(
                    || link.title.as_str().to_owned(),
                    |t| t.name.display.clone(),
                ),
                rank: data
                    .titles
                    .get(&link.title)
                    .map_or("", |t| t.rank.key())
                    .to_owned(),
                holder: link
                    .holder
                    .as_ref()
                    .map_or_else(String::new, |f| f.as_str().to_owned()),
                holder_name: link
                    .holder
                    .as_ref()
                    .map_or_else(String::new, |f| data.faction_name(f)),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::FactionId;

    #[test]
    fn capital_province_sheet() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let france = FactionId::new("fac_france").unwrap();
        let capital = state.factions[&france].capital.clone();
        let sheet = province_sheet(data, &state, &capital).unwrap();
        assert_eq!(sheet.owner, "fac_france");
        assert!(!sheet.name.is_empty());
        assert!(!sheet.settlements.is_empty());
        assert!(sheet.settlements[0].is_city);
        assert_eq!(sheet.classes.len(), 4);
        assert!(sheet.population_total > 0);
        assert!(!sheet.breadcrumb.is_empty());
        assert!(sheet.income > 0);
        assert!(["low", "normal", "high"].contains(&sheet.tax.rate.as_str()));
        assert!(serde_json::to_string(&sheet).is_ok());
        let unknown = ProvinceId::new("prov_nowhere").unwrap();
        assert!(province_sheet(data, &state, &unknown).is_none());
    }
}
