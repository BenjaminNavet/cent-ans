//! View of the faction panel (`faction_panel.gd`, `trade_panel.gd`): the sheet, the detailed
//! budget, the treasury series of the chart, the tax brackets and the trade routes and
//! agreements of one faction. Everything is read from `core`; the client only draws it.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::economy::TaxRate;
use sim_campaign::trade::{trade_routes, TradeRouteView};
use sim_campaign::{CampaignState, Order};

use crate::common::FactionRef;
use crate::diplomacy::{submit_order, OrderResult};
use crate::faction::{faction_sheet, FactionSheet};
use crate::hud::{economy, EconomyView};

/// Seasons kept by the treasury chart (Godot `budget_history` window).
pub const CHART_SEASONS: usize = 12;

/// One tax bracket a faction can pick.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaxOption {
    /// `low`, `normal`, `high`.
    pub key: String,
    pub multiplier: f64,
    /// Share of the population's wealth taken (0-1).
    pub burden: f64,
    pub selected: bool,
}

/// One point of the treasury chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChartPoint {
    /// Season index (0 = spring 1337) the treasury is the balance of.
    pub turn: u32,
    pub treasury: i64,
    pub change: i64,
    /// The point is the current treasury, not a resolved season.
    pub now: bool,
}

/// One trade route touching the faction (or any route, `mine` false).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradeRow {
    pub id: String,
    pub from_hub: String,
    pub to_hub: String,
    /// `land`, `sea`, `mixed`.
    pub mode: String,
    pub total_value: i64,
    /// Livres credited to the faction.
    pub my_value: i64,
    pub mine: bool,
    pub cut: bool,
    pub cut_reason: String,
    pub agreement: bool,
}

/// Trade section of the panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradeSection {
    /// Income of the faction's routes this season.
    pub income: i64,
    pub income_last_turn: i64,
    /// Routes sorted by value, highest first.
    pub routes: Vec<TradeRow>,
    /// Factions with a trade agreement with this one.
    pub agreements: Vec<FactionRef>,
}

/// Everything the faction panel shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactionPanelView {
    pub sheet: FactionSheet,
    pub economy: EconomyView,
    /// The faction is the player's: tax brackets and treaties can be edited.
    pub is_player: bool,
    pub tax: Vec<TaxOption>,
    pub chart: Vec<ChartPoint>,
    pub trade: TradeSection,
}

fn trade_row(data: &GameData, route: &TradeRouteView, faction: &FactionId) -> TradeRow {
    let hub_name = |hub: &str| -> String {
        data.trade
            .as_ref()
            .and_then(|catalog| catalog.hub(hub))
            .map_or_else(|| hub.to_owned(), |h| h.name.display.clone())
    };
    TradeRow {
        id: route.id.clone(),
        from_hub: hub_name(&route.from_hub),
        to_hub: hub_name(&route.to_hub),
        mode: route.mode.key().to_owned(),
        total_value: route.total_value(),
        my_value: route.value_for(faction),
        mine: route.touches(faction),
        cut: route.cut,
        cut_reason: route.cut_reason.clone().unwrap_or_default(),
        agreement: route.agreement,
    }
}

/// The panel of `faction`; `None` for an unknown faction. `player` is the player's faction.
pub fn faction_panel(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    player: &FactionId,
) -> Option<FactionPanelView> {
    let sheet = faction_sheet(data, state, faction)?;
    let economy = economy(data, state, faction)?;
    let current = state.factions.get(faction)?.tax_rate;
    let tax = [TaxRate::Low, TaxRate::Normal, TaxRate::High]
        .into_iter()
        .map(|rate| {
            let bracket = rate.bracket(&data.economy_rules);
            TaxOption {
                key: rate.key().to_owned(),
                multiplier: bracket.multiplier,
                burden: bracket.burden,
                selected: rate == current,
            }
        })
        .collect();
    let mut chart: Vec<ChartPoint> = economy
        .budget_history
        .iter()
        .map(|record| ChartPoint {
            turn: record.turn + 1,
            treasury: record.treasury,
            change: record.change,
            now: false,
        })
        .collect();
    let skip = chart.len().saturating_sub(CHART_SEASONS);
    chart.drain(..skip);
    if chart
        .last()
        .is_none_or(|last| last.treasury != economy.treasury)
    {
        let change = chart
            .last()
            .map_or(0, |last| economy.treasury - last.treasury);
        chart.push(ChartPoint {
            turn: state.turn,
            treasury: economy.treasury,
            change,
            now: true,
        });
    }
    let mut routes = trade_routes(state, data);
    routes.sort_by(|a, b| b.total_value().cmp(&a.total_value()).then(a.id.cmp(&b.id)));
    let trade = TradeSection {
        income: economy.trade_income,
        income_last_turn: economy.trade_income_last_turn,
        routes: routes
            .iter()
            .map(|route| trade_row(data, route, faction))
            .collect(),
        agreements: sheet
            .relations
            .entries
            .iter()
            .filter(|entry| entry.trade_agreement)
            .map(|entry| FactionRef {
                id: entry.id.clone(),
                name: entry.name.clone(),
            })
            .collect(),
    };
    Some(FactionPanelView {
        is_player: faction == player,
        sheet,
        economy,
        tax,
        chart,
        trade,
    })
}

/// Picks the player's tax bracket (`SetTaxRate`).
pub fn set_tax_rate(data: &GameData, state: &mut CampaignState, key: &str) -> OrderResult {
    let rate = match key {
        "low" => TaxRate::Low,
        "normal" => TaxRate::Normal,
        "high" => TaxRate::High,
        _ => {
            return OrderResult {
                ok: false,
                message: format!("Taux inconnu : {key}"),
            }
        }
    };
    submit_order(data, state, Order::SetTaxRate { rate })
}

/// Ends the player's trade agreement with `target` (`BreakTradeAgreement`).
pub fn break_trade_agreement(
    data: &GameData,
    state: &mut CampaignState,
    target: &FactionId,
) -> OrderResult {
    submit_order(
        data,
        state,
        Order::BreakTradeAgreement {
            target: target.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn france() -> FactionId {
        FactionId::new("fac_france").unwrap()
    }

    #[test]
    fn france_panel_at_start() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let view = faction_panel(data, &state, &france(), &france()).expect("france");
        assert!(view.is_player);
        assert_eq!(view.tax.len(), 3);
        assert_eq!(view.tax.iter().filter(|t| t.selected).count(), 1);
        assert!(!view.chart.is_empty());
        assert!(view.chart.len() <= CHART_SEASONS + 1);
        assert_eq!(view.chart.last().unwrap().treasury, view.economy.treasury);
        assert_eq!(
            view.trade.routes.iter().map(|r| r.my_value).sum::<i64>(),
            view.trade.income
        );
        assert!(serde_json::to_string(&view).is_ok());
        let nowhere = FactionId::new("fac_nowhere").unwrap();
        assert!(faction_panel(data, &state, &nowhere, &france()).is_none());
    }

    #[test]
    fn tax_rate_order_changes_the_bracket() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(set_tax_rate(data, &mut state, "high").ok);
        let view = faction_panel(data, &state, &france(), &france()).unwrap();
        assert!(view.tax.iter().any(|t| t.key == "high" && t.selected));
        assert!(!set_tax_rate(data, &mut state, "bogus").ok);
    }
}
