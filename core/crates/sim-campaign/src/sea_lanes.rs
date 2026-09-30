//! Sea lanes (lot SL1, ADR 0139).
//!
//! The lanes of `data/naval/sea_lanes.json` ([`data_model::SeaLanes`]) are
//! sea edges of the settlement movement graph (`GameData::build_movement_graph`):
//! an army standing in one of their ports embarks for the other (`Embark`, the
//! whole season), and trade routes follow them. This module holds their rules:
//!
//! - **sea of a crossing**: the lane's sea, for interception and control
//!   ([`lane_sea`], read by `naval::crossing_sea`);
//! - **interception**: the lane's kind scales the chance (open sea is harder
//!   to watch, [`intercept_factor`]);
//! - **gales**: an army crossing loses a share of its men, by season and kind,
//!   doubled when a storm rages over the port it sails from
//!   ([`weather_the_crossing`]). Deterministic: no draw from the campaign RNG;
//! - **trade**: a sea leg of a trade route is cut when an enemy of either end
//!   holds its sea (control ≥ `blockade_control`) or blockades one of its
//!   ports, loses security with lesser enemy control, and is worth less in
//!   the stormy seasons ([`trade_sea_legs`]);
//! - **view** for the map layer and its tooltip ([`sea_lanes`]).

use data_model::{FactionId, GameData, SeaLane, SeaLaneKind, SeaZoneId, SettlementId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, Season};
use crate::weather::MapWeather;

/// Gale losses are multiplied by this when a storm rages over the port of
/// departure this turn (campaign weather, ADR 0027).
pub const STORM_WEATHER_FACTOR: f64 = 2.0;

fn season_index(season: Season) -> usize {
    match season {
        Season::Spring => 0,
        Season::Summer => 1,
        Season::Autumn => 2,
        Season::Winter => 3,
    }
}

/// Sea crossed by the lane between `from` and `to`, if a lane links them.
pub fn lane_sea(data: &GameData, from: &SettlementId, to: &SettlementId) -> Option<SeaZoneId> {
    data.naval
        .lane_between(from, to)
        .map(|lane| lane.sea.clone())
}

/// Factor of the interception chance of a crossing between two ports: the
/// kind of their lane, 1 for the short passages of the graph.
pub fn intercept_factor(data: &GameData, from: &SettlementId, to: &SettlementId) -> f64 {
    data.naval.lane_between(from, to).map_or(1.0, |lane| {
        data.naval.sea_lanes.rules.kind(lane.kind).intercept_factor
    })
}

/// Kind of the sea edge between two ports: its lane's, coastal for the short
/// passages of the graph.
fn edge_kind(data: &GameData, from: &SettlementId, to: &SettlementId) -> SeaLaneKind {
    data.naval
        .lane_between(from, to)
        .map_or(SeaLaneKind::Coastal, |lane| lane.kind)
}

/// Men lost (%) by an army crossing from `from` to `to` this turn: the
/// kind's seasonal rate, × [`STORM_WEATHER_FACTOR`] under a storm at the
/// port of departure.
pub fn storm_loss_percent(
    state: &CampaignState,
    data: &GameData,
    from: &SettlementId,
    to: &SettlementId,
) -> f64 {
    let rules = data.naval.sea_lanes.rules.kind(edge_kind(data, from, to));
    let base = rules.storm_loss_percent.at(season_index(state.season()));
    if base <= 0.0 {
        return 0.0;
    }
    let storm = state
        .settlement_province(from)
        .and_then(|p| crate::weather::province_weather(data, state, p))
        .is_some_and(|w| w.kind == MapWeather::Storm);
    if storm {
        base * STORM_WEATHER_FACTOR
    } else {
        base
    }
}

/// Gales on a crossing: `army`, still standing in its port of departure,
/// loses [`storm_loss_percent`] of every regiment (rounded down, never the
/// last man).
pub(crate) fn weather_the_crossing(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    to: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let Some(from) = state.armies.get(army).and_then(|a| a.settlement().cloned()) else {
        return;
    };
    if !crate::movement::is_sea_crossing(data, &from, to) {
        return;
    }
    let percent = storm_loss_percent(state, data, &from, to);
    if percent <= 0.0 {
        return;
    }
    let Some(entry) = state.armies.get_mut(army) else {
        return;
    };
    let mut lost = 0u32;
    for unit in &mut entry.units {
        let loss = ((f64::from(unit.strength) * percent / 100.0).floor() as u32)
            .min(unit.strength.saturating_sub(1));
        unit.strength -= loss;
        lost += loss;
    }
    if lost == 0 {
        return;
    }
    let faction = entry.faction.clone();
    let where_ = data.naval.lane_between(&from, to).map_or_else(
        || format!("dans {}", sea_name_between(state, data, &from, to)),
        |lane| format!("sur la {}", lowercase_first(&lane.name)),
    );
    events.push(
        GameEvent::new(
            EventKind::Attrition,
            format!(
                "Gros temps {where_} : {} perd {lost} hommes pendant la traversée.",
                state.army_name(data, army)
            ),
        )
        .army(army)
        .faction(&faction),
    );
}

fn sea_name_between(
    state: &CampaignState,
    data: &GameData,
    from: &SettlementId,
    to: &SettlementId,
) -> String {
    crate::naval::crossing_sea(state, data, from, to)
        .map_or_else(|| "la mer".to_owned(), |sea| data.naval.sea_name(&sea))
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Effect of the sea legs of a trade route's `path` on its value: `factor`
/// (security × season, 1 without sea leg) and the reason of a cut.
#[derive(Debug, Clone, PartialEq)]
pub struct SeaLegs {
    pub security: f64,
    pub season_factor: f64,
    pub cut_reason: Option<String>,
    pub threat_reason: Option<String>,
}

/// Sea legs of a trade route between factions `a` and `b`: an enemy of
/// either holding a sea crossed (control ≥ `blockade_control`) or
/// blockading a port the route sails from or to cuts it; lesser enemy
/// control lowers its security; the season lowers the value of its
/// stormiest leg.
pub fn trade_sea_legs(
    state: &CampaignState,
    data: &GameData,
    path: &[SettlementId],
    a: &FactionId,
    b: &FactionId,
) -> SeaLegs {
    let rules = &data.naval.sea_lanes.rules;
    let blockade = data.naval.rules.blockade_control;
    let season = season_index(state.season());
    let mut legs = SeaLegs {
        security: 1.0,
        season_factor: 1.0,
        cut_reason: None,
        threat_reason: None,
    };
    for pair in path.windows(2) {
        let (from, to) = (&pair[0], &pair[1]);
        if !crate::movement::is_sea_crossing(data, from, to) {
            continue;
        }
        let kind = rules.kind(edge_kind(data, from, to));
        legs.season_factor = legs.season_factor.min(kind.trade_season_factor.at(season));
        // `naval.blockaded` lists the cities of the blockaded provinces.
        let blockaded = |port: &SettlementId| {
            state.naval.blockaded.contains(port)
                || state
                    .settlement_province(port)
                    .and_then(|p| state.provinces.get(p))
                    .is_some_and(|p| state.naval.blockaded.contains(&p.city))
        };
        if let Some(port) = [from, to].into_iter().find(|p| blockaded(p)) {
            legs.cut_reason = Some(format!("blocus de {port}"));
            legs.security = 0.0;
            return legs;
        }
        let Some(sea) = crate::naval::crossing_sea(state, data, from, to) else {
            continue;
        };
        let Some(control) = state.naval.control.get(&sea) else {
            continue;
        };
        if !(state.is_at_war(&control.faction, a) || state.is_at_war(&control.faction, b)) {
            continue;
        }
        if control.level >= blockade {
            legs.cut_reason = Some(format!("{} tenue par l'ennemi", data.naval.sea_name(&sea)));
            legs.security = 0.0;
            return legs;
        }
        legs.security *=
            (1.0 - rules.hostile_control_security * f64::from(control.level) / 100.0).max(0.0);
        legs.threat_reason
            .get_or_insert_with(|| format!("escadres ennemies dans {}", data.naval.sea_name(&sea)));
    }
    legs
}

/// One sea lane, resolved against the current turn for the map layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeaLaneView {
    pub id: String,
    pub name: String,
    pub from: SettlementId,
    pub to: SettlementId,
    pub sea: SeaZoneId,
    pub sea_name: String,
    pub kind: SeaLaneKind,
    pub kind_name: String,
    pub length_km: f64,
    /// Faction holding the sea, and how firmly (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_faction: Option<FactionId>,
    pub control_level: u32,
    /// The holder of the sea is at war with `viewer`.
    pub hostile: bool,
    /// Its control reaches the blockade threshold.
    pub blockade: bool,
    /// Men lost (%) by an army sailing from `from` this turn.
    pub storm_loss_percent: f64,
    /// Trade value factor of the season.
    pub trade_season_factor: f64,
    /// Catalogue trade routes whose path follows the lane.
    pub trade_routes: Vec<String>,
}

/// Every sea lane whose two ports exist, seen by `viewer`.
pub fn sea_lanes(state: &CampaignState, data: &GameData, viewer: &FactionId) -> Vec<SeaLaneView> {
    let rules = &data.naval.sea_lanes.rules;
    let season = season_index(state.season());
    data.naval
        .sea_lanes
        .lanes
        .iter()
        .filter(|lane| {
            data.settlements.contains_key(&lane.from) && data.settlements.contains_key(&lane.to)
        })
        .map(|lane| lane_view(state, data, lane, viewer, rules, season))
        .collect()
}

fn lane_view(
    state: &CampaignState,
    data: &GameData,
    lane: &SeaLane,
    viewer: &FactionId,
    rules: &data_model::SeaLaneRules,
    season: usize,
) -> SeaLaneView {
    let kind = rules.kind(lane.kind);
    let control = state.naval.control.get(&lane.sea);
    let trade_routes = data
        .trade
        .as_ref()
        .map(|catalog| {
            catalog
                .routes
                .iter()
                .filter(|route| {
                    let (Some(a), Some(b)) =
                        (catalog.hub(&route.from_hub), catalog.hub(&route.to_hub))
                    else {
                        return false;
                    };
                    data.trade_path(&a.settlement, &b.settlement)
                        .is_some_and(|(_, path)| path.windows(2).any(|w| lane.links(&w[0], &w[1])))
                })
                .map(|route| route.id.clone())
                .collect()
        })
        .unwrap_or_default();
    SeaLaneView {
        id: lane.id.clone(),
        name: lane.name.clone(),
        from: lane.from.clone(),
        to: lane.to.clone(),
        sea: lane.sea.clone(),
        sea_name: data.naval.sea_name(&lane.sea),
        kind: lane.kind,
        kind_name: kind.name.clone(),
        length_km: (data.sea_lane_length_km(lane) * 10.0).round() / 10.0,
        control_faction: control.map(|c| c.faction.clone()),
        control_level: control.map_or(0, |c| c.level),
        hostile: control.is_some_and(|c| state.is_at_war(&c.faction, viewer)),
        blockade: control.is_some_and(|c| c.level >= data.naval.rules.blockade_control),
        storm_loss_percent: storm_loss_percent(state, data, &lane.from, &lane.to),
        trade_season_factor: kind.trade_season_factor.at(season),
        trade_routes,
    }
}
