//! Trade routes and agreements (lot C5,
//! `docs/design/2026-09-24-rapprochement-total-war.md` § C5).
//!
//! Historic marketplaces and the routes that link them live in
//! `data/economy/trade.json` ([`data_model::TradeCatalog`]). Every season
//! [`resolve_trade`] walks each [`data_model::TradeRouteDef`], finds its path
//! on the settlement movement graph ([`trade_routes`]), and values it from
//! `base_value` scaled by:
//! - **distance**: the path's total movement cost, a longer road/crossing is
//!   worth less ([`distance_factor`]);
//! - **security**: a siege on any settlement of the path cuts the route
//!   outright; a hostile army camped on it halves the value per threatened
//!   node ([`THREAT_SECURITY_FACTOR`]);
//! - **war and embargo**: a route whose two hubs are held by factions at war,
//!   or under an embargo either way, earns nothing;
//! - **agreement**: a formal trade agreement between the two controllers
//!   (the DP1 treaty article [`crate::negotiation::Article::TradeAgreement`],
//!   stored once in the ledger, see [`CampaignState::has_trade_agreement`])
//!   raises the value by [`AGREEMENT_BONUS_PERCENT`]; war erases it;
//! - **coinage**: a debased currency trades less well (H5);
//! - **sea legs** (lot SL1, [`crate::sea_lanes::trade_sea_legs`]): an enemy
//!   holding a sea crossed or blockading a port of the way cuts the route,
//!   lesser enemy control lowers its security, gales lower its value in
//!   autumn and winter.
//!
//! Value is split evenly between the controllers of the two hubs (customs at
//! each end); a route whose two hubs share a controller pays that faction in
//! full. `resolve_trade` credits the treasuries and reports newly-cut routes
//! that touch the player.

use data_model::key_enum;
use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ResourceId, SettlementId, TradeRouteDef};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyPosition, CampaignState};

/// Bonus (%) a formal trade agreement adds to the routes between its two factions.
pub const AGREEMENT_BONUS_PERCENT: f64 = 30.0;
/// Movement-point cost beyond which distance starts eating into a route's value.
pub const REFERENCE_COST: f64 = 400.0;
/// Security lost (multiplicative) for every settlement of the path held by a
/// hostile army.
pub const THREAT_SECURITY_FACTOR: f64 = 0.5;
/// Value factor of a declining route (the Champagne fairs, spec C5).
pub const DECLINING_FACTOR: f64 = 0.6;
/// Coinage factor floor: even a heavily debased currency keeps some trade.
pub const COINAGE_FACTOR_MIN: f64 = 0.4;

key_enum! {
/// Whether a route's path is entirely by land, entirely by sea, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeMode {
    Land => "land",
    Sea => "sea",
    Mixed => "mixed",
}
}

/// One trade route, resolved against the current state (query, no mutation);
/// used for the map layer, the faction panel and [`resolve_trade`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRouteView {
    pub id: String,
    pub from_hub: String,
    pub from_settlement: SettlementId,
    pub to_hub: String,
    pub to_settlement: SettlementId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_faction: Option<FactionId>,
    pub goods: Vec<ResourceId>,
    pub mode: TradeMode,
    /// Settlements walked from `from_settlement` to `to_settlement`
    /// (inclusive), for the map layer; empty when no path exists.
    pub path: Vec<SettlementId>,
    /// A formal trade agreement raises this route's value (only meaningful
    /// when the two hubs belong to different, non-hostile factions).
    pub agreement: bool,
    /// 0 (cut) to 1 (fully secure).
    pub security: f64,
    pub cut: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_reason: Option<String>,
    /// Livres credited to `from_faction` this season (0 when cut).
    pub value_from: i64,
    /// Livres credited to `to_faction` this season (0 when cut, and when
    /// the same faction holds both hubs: `value_from` already covers it).
    pub value_to: i64,
}

impl TradeRouteView {
    /// Total value actually flowing on the route (`value_from + value_to`).
    pub fn total_value(&self) -> i64 {
        self.value_from + self.value_to
    }
}

/// Every trade route of `data/economy/trade.json`, resolved against `state`
/// (income, security, cuts). Deterministic and side-effect free.
pub fn trade_routes(state: &CampaignState, data: &GameData) -> Vec<TradeRouteView> {
    let Some(catalog) = &data.trade else {
        return Vec::new();
    };
    catalog
        .routes
        .iter()
        .filter_map(|route| resolve_route(state, data, route))
        .collect()
}

fn resolve_route(
    state: &CampaignState,
    data: &GameData,
    route: &TradeRouteDef,
) -> Option<TradeRouteView> {
    let catalog = data.trade.as_ref()?;
    let from_hub = catalog.hub(&route.from_hub)?;
    let to_hub = catalog.hub(&route.to_hub)?;
    let from_faction = state
        .settlements
        .get(&from_hub.settlement)
        .map(|s| s.controller.clone());
    let to_faction = state
        .settlements
        .get(&to_hub.settlement)
        .map(|s| s.controller.clone());

    // Precomputed at load (`GameData::build_trade_paths`, review point 18c).
    let (path, cost) = data
        .trade_path(&from_hub.settlement, &to_hub.settlement)
        .map(|(cost, path)| (path, cost))
        .unwrap_or_default();

    let mut view = TradeRouteView {
        id: route.id.clone(),
        from_hub: route.from_hub.clone(),
        from_settlement: from_hub.settlement.clone(),
        to_hub: route.to_hub.clone(),
        to_settlement: to_hub.settlement.clone(),
        from_faction: from_faction.clone(),
        to_faction: to_faction.clone(),
        goods: route.goods.clone(),
        mode: route_mode(data, &path),
        path: path.clone(),
        agreement: false,
        security: 0.0,
        cut: true,
        cut_reason: None,
        value_from: 0,
        value_to: 0,
    };

    if path.is_empty() {
        view.cut_reason = Some("aucun chemin".to_owned());
        return Some(view);
    }
    let (Some(from_faction), Some(to_faction)) = (from_faction, to_faction) else {
        view.cut_reason = Some("comptoir sans contrôleur".to_owned());
        return Some(view);
    };
    if from_faction != to_faction {
        if state.is_at_war(&from_faction, &to_faction) {
            view.cut_reason = Some("guerre".to_owned());
            return Some(view);
        }
        if state
            .factions
            .get(&from_faction)
            .is_some_and(|f| f.embargoes.contains(&to_faction))
            || state
                .factions
                .get(&to_faction)
                .is_some_and(|f| f.embargoes.contains(&from_faction))
        {
            view.cut_reason = Some("embargo".to_owned());
            return Some(view);
        }
    }
    view.agreement =
        from_faction != to_faction && state.has_trade_agreement(&from_faction, &to_faction);

    let (security, threat_reason) = path_security(state, data, &path, &from_faction, &to_faction);
    // Lot SL1: sea legs (enemy squadrons, blockades, gales).
    let sea = crate::sea_lanes::trade_sea_legs(state, data, &path, &from_faction, &to_faction);
    if let Some(reason) = sea.cut_reason {
        view.cut_reason = Some(reason);
        return Some(view);
    }
    let security = security * sea.security;
    let threat_reason = threat_reason.or(sea.threat_reason);
    view.security = security;
    if security <= 0.0 {
        view.cut = true;
        view.cut_reason = threat_reason.or_else(|| Some("route coupée".to_owned()));
        return Some(view);
    }

    let distance = distance_factor(cost);
    let declining = if route.declining {
        DECLINING_FACTOR
    } else {
        1.0
    };
    let price_factor = if from_faction == to_faction {
        coinage_factor(state, &from_faction)
    } else {
        (coinage_factor(state, &from_faction) + coinage_factor(state, &to_faction)) / 2.0
    };
    let mut value = route.base_value as f64
        * distance
        * declining
        * security
        * price_factor
        * sea.season_factor;
    if view.agreement {
        value *= 1.0 + AGREEMENT_BONUS_PERCENT / 100.0;
    }
    let value = value.round().max(0.0) as i64;
    view.cut = false;
    if from_faction == to_faction {
        view.value_from = value;
    } else {
        view.value_from = value / 2;
        view.value_to = value - value / 2;
    }
    Some(view)
}

/// Value factor from the path's total movement cost (a v1 season's march is
/// worth [`REFERENCE_COST`]: a route that long keeps half its value).
fn distance_factor(cost: u32) -> f64 {
    REFERENCE_COST / (REFERENCE_COST + f64::from(cost))
}

/// Coinage (H5) factor of a faction's trade: a debased currency trades less
/// well, floored at [`COINAGE_FACTOR_MIN`].
fn coinage_factor(state: &CampaignState, faction: &FactionId) -> f64 {
    let price = state.factions.get(faction).map_or(100, |f| f.price_level);
    (1.0 - (f64::from(price) - 100.0) / 400.0).clamp(COINAGE_FACTOR_MIN, 1.0)
}

/// Security of a path (0 cut, 1 fully secure): a siege on any settlement of
/// it cuts the route; a hostile army camped on it halves the value per
/// threatened settlement (spec C5); EQ1: a hostile army in the field within
/// its zone of control of a settlement threatens it as well.
fn path_security(
    state: &CampaignState,
    data: &GameData,
    path: &[SettlementId],
    from_faction: &FactionId,
    to_faction: &FactionId,
) -> (f64, Option<String>) {
    let mut security = 1.0;
    let mut reason = None;
    for settlement_id in path {
        let Some(settlement) = state.settlements.get(settlement_id) else {
            continue;
        };
        if settlement.siege.is_some() {
            return (0.0, Some(format!("siège de {settlement_id}")));
        }
        let hostile = |faction: &FactionId| {
            state.is_at_war(faction, from_faction) || state.is_at_war(faction, to_faction)
        };
        let mut threatened = state.armies.values().any(|army| {
            matches!(&army.position, ArmyPosition::Settlement(id) if id == settlement_id)
                && hostile(&army.faction)
        });
        // EQ1: an enemy army in the open within its zone of control of the
        // settlement (free movement, `zoc_radius_km`) threatens it too.
        if !threatened {
            if let Some(point) = data.settlement_point(settlement_id) {
                let radius = data.free_movement_rules().zoc_radius_km;
                threatened = state
                    .armies_near(data, point, radius)
                    .iter()
                    .filter_map(|id| state.armies.get(id))
                    .any(|army| {
                        matches!(army.position, ArmyPosition::Field { .. })
                            && hostile(&army.faction)
                    });
            }
        }
        if threatened {
            security *= THREAT_SECURITY_FACTOR;
            reason.get_or_insert_with(|| format!("armée ennemie près de {settlement_id}"));
        }
    }
    (security, reason)
}

/// `Land` if every edge of the path is a land step, `Sea` if every edge
/// crosses water, `Mixed` otherwise.
fn route_mode(data: &GameData, path: &[SettlementId]) -> TradeMode {
    let mut land = false;
    let mut sea = false;
    for pair in path.windows(2) {
        if data
            .movement_graph
            .edge(&pair[0], &pair[1])
            .is_some_and(|edge| edge.sea)
        {
            sea = true;
        } else {
            land = true;
        }
    }
    match (land, sea) {
        (true, true) => TradeMode::Mixed,
        (false, true) => TradeMode::Sea,
        _ => TradeMode::Land,
    }
}

/// Number of catalogue routes linking a hub held by `a` to a hub held by
/// `b` (either way), whatever their state this season: what a trade
/// agreement between them would raise. Cheap (hub controllers only, no
/// path search): the treaty evaluation calls it for every offer.
pub fn common_routes(
    state: &CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
) -> usize {
    let Some(catalog) = &data.trade else {
        return 0;
    };
    let controller = |hub: &str| {
        catalog
            .hub(hub)
            .and_then(|h| state.settlements.get(&h.settlement))
            .map(|s| &s.controller)
    };
    catalog
        .routes
        .iter()
        .filter(
            |route| match (controller(&route.from_hub), controller(&route.to_hub)) {
                (Some(f), Some(t)) => (f == a && t == b) || (f == b && t == a),
                _ => false,
            },
        )
        .count()
}

/// Income `faction` collects this season from every route touching one of
/// its hubs (query, used by [`crate::economy::FactionEconomy`] and by
/// [`resolve_trade`]).
pub fn faction_trade_income(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    trade_routes(state, data)
        .iter()
        .map(|route| {
            let mut total = 0;
            if route.from_faction.as_ref() == Some(faction) {
                total += route.value_from;
            }
            if route.to_faction.as_ref() == Some(faction) {
                total += route.value_to;
            }
            total
        })
        .sum()
}

/// Phase 6b (after `economy::resolve_economy`): credits every faction's
/// trade income and reports the player's cut routes (spec C5, season report).
pub(crate) fn resolve_trade(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let routes = trade_routes(state, data);
    let mut income: BTreeMap<FactionId, i64> = BTreeMap::new();
    let player = state.player_faction.clone();
    let mut player_cut: Vec<String> = Vec::new();
    for route in &routes {
        if let Some(f) = &route.from_faction {
            *income.entry(f.clone()).or_default() += route.value_from;
        }
        if let Some(f) = &route.to_faction {
            *income.entry(f.clone()).or_default() += route.value_to;
        }
        let touches_player = route.from_faction.as_ref() == Some(&player)
            || route.to_faction.as_ref() == Some(&player);
        if touches_player && route.cut && route.from_faction != route.to_faction {
            let catalog = data.trade.as_ref();
            let from_name = catalog
                .and_then(|c| c.hub(&route.from_hub))
                .map_or(route.from_hub.as_str(), |h| h.name.display.as_str());
            let to_name = catalog
                .and_then(|c| c.hub(&route.to_hub))
                .map_or(route.to_hub.as_str(), |h| h.name.display.as_str());
            player_cut.push(format!("{from_name} – {to_name}"));
        }
    }
    for faction_id in state.factions.keys().cloned().collect::<Vec<_>>() {
        let amount = income.get(&faction_id).copied().unwrap_or(0);
        if let Some(faction) = state.factions.get_mut(&faction_id) {
            faction.treasury += amount;
            faction.last_budget.trade_income = amount;
        }
    }
    if !player_cut.is_empty() {
        let text = format!("Routes commerciales coupées : {}.", player_cut.join(", "));
        events.push(GameEvent::new(EventKind::Trade, text).faction(&player));
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<TradeMode>();
    }
}
