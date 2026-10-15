//! Campaign top bar and economy views.
//!
//! Replaces `CampaignSim.get_turn`, `get_date_label`, `get_faction_summary`, `get_faction_economy`,
//! `get_income_breakdown`, `get_faction_forecast`, `get_income_top_provinces`, `get_difficulty*`.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::difficulty::effect_summary;
use sim_campaign::CampaignState;

use crate::common::KeyedAmount;

/// What the top bar of the campaign map shows about the player's (or any) faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopBar {
    pub faction_id: String,
    pub faction_name: String,
    /// Zero-based turn counter.
    pub turn: u32,
    /// French date, e.g. `"Printemps 1337"`.
    pub date_label: String,
    pub season: String,
    pub year: i32,
    /// Livres tournois; may be negative.
    pub treasury: i64,
    /// Projected net balance of the coming season (receipts minus all charges).
    pub net_income: i64,
    /// Booked net balance of the season just resolved.
    pub net_income_last_turn: i64,
    /// Tax income of the coming season (`projected_income` of the economy).
    pub projected_income: i64,
    pub army_upkeep: i64,
    pub building_upkeep: i64,
    pub tax_rate: String,
    /// Men under arms (head count of every army).
    pub manpower: u64,
    pub prestige: i32,
    /// Population-weighted mean unrest, 0-100.
    pub mean_unrest: u8,
    pub provinces_count: usize,
    pub armies_count: usize,
    pub at_war_with: Vec<String>,
    pub allies: Vec<String>,
    pub alive: bool,
    pub difficulty: String,
}

/// Top bar of `faction`; `None` for an unknown faction.
pub fn top_bar(data: &GameData, state: &CampaignState, faction: &FactionId) -> Option<TopBar> {
    let summary = state.faction_summary(faction)?;
    let economy = state.faction_economy(data, faction)?;
    Some(TopBar {
        faction_id: faction.as_str().to_owned(),
        faction_name: data.faction_name(faction),
        turn: state.turn(),
        date_label: state.date_label(),
        season: format!("{:?}", state.season()).to_lowercase(),
        year: state.year(),
        treasury: summary.treasury,
        net_income: economy.net_income(),
        net_income_last_turn: state.faction_net_last_turn(faction).unwrap_or(0),
        projected_income: summary.projected_income,
        army_upkeep: summary.army_upkeep,
        building_upkeep: summary.building_upkeep,
        tax_rate: summary.tax_rate.key().to_owned(),
        manpower: summary.soldiers,
        prestige: summary.prestige,
        mean_unrest: summary.mean_unrest,
        provinces_count: summary.provinces_count,
        armies_count: summary.armies_count,
        at_war_with: summary
            .at_war_with
            .iter()
            .map(|f| f.as_str().to_owned())
            .collect(),
        allies: summary
            .allies
            .iter()
            .map(|f| f.as_str().to_owned())
            .collect(),
        alive: summary.alive,
        difficulty: state.difficulty().key().to_owned(),
    })
}

/// One signed row of the budget table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetLineView {
    /// `receipts`, `armies`, `buildings`, `table`, `administration`, `other`.
    pub key: String,
    pub projected: i64,
    pub last: Option<i64>,
    pub delta: Option<i64>,
    pub charge: bool,
}

/// One season of the purse history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetRecordView {
    pub turn: u32,
    pub treasury: i64,
    pub net: i64,
    pub change: i64,
    pub other: i64,
}

/// Income and expenses breakdown (the economy overview tab).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EconomyView {
    pub treasury: i64,
    pub income: i64,
    pub projected_income: i64,
    pub net_income: i64,
    pub net_income_last_turn: i64,
    /// Change of the projected balance against the season just resolved.
    pub net_change: Option<i64>,
    pub army_upkeep: i64,
    pub army_surcharge: i64,
    pub army_count: u32,
    pub building_upkeep: i64,
    pub administration_upkeep: i64,
    pub table_upkeep: i64,
    pub table_upkeep_last_turn: i64,
    pub coinage: String,
    pub price_level: u32,
    pub seigniorage: i64,
    pub recoinage: i64,
    pub tax_rate: String,
    pub trade_income: i64,
    pub trade_income_last_turn: i64,
    pub mercenary_premium: i64,
    pub mercenary_premium_last_turn: i64,
    pub budget_lines: Vec<BudgetLineView>,
    pub budget_history: Vec<BudgetRecordView>,
    /// Non-zero receipts of the coming season by source.
    pub income_lines: Vec<KeyedAmount>,
}

/// Economy of `faction`; `None` for an unknown faction.
pub fn economy(data: &GameData, state: &CampaignState, faction: &FactionId) -> Option<EconomyView> {
    let economy = state.faction_economy(data, faction)?;
    let history = state.budget_history(faction);
    let last = history.last();
    let breakdown = state.faction_income_breakdown(data, faction);
    let mut lines: Vec<(&'static str, i64)> = breakdown.provinces.named_lines();
    lines.extend([
        ("embargo", breakdown.embargo),
        ("domain", breakdown.domain),
        ("difficulty", breakdown.difficulty),
        ("alms", breakdown.alms),
        ("seigniorage", economy.seigniorage),
        ("trade_routes", economy.trade_income),
    ]);
    lines.retain(|(_, value)| *value != 0);
    Some(EconomyView {
        treasury: economy.treasury,
        income: economy.income,
        projected_income: economy.projected_income,
        net_income: economy.net_income(),
        net_income_last_turn: state.faction_net_last_turn(faction).unwrap_or(0),
        net_change: last.map(|record| economy.net_income() - record.net()),
        army_upkeep: economy.army_upkeep,
        army_surcharge: economy.army_surcharge,
        army_count: economy.army_count,
        building_upkeep: economy.building_upkeep,
        administration_upkeep: economy.administration_upkeep,
        table_upkeep: economy.table_upkeep,
        table_upkeep_last_turn: economy.table_upkeep_last_turn,
        coinage: economy.coinage.key().to_owned(),
        price_level: economy.price_level,
        seigniorage: economy.seigniorage,
        recoinage: economy.recoinage,
        tax_rate: economy.tax_rate.key().to_owned(),
        trade_income: economy.trade_income,
        trade_income_last_turn: economy.trade_income_last_turn,
        mercenary_premium: state.mercenary_premium(data, faction),
        mercenary_premium_last_turn: state
            .mercenaries
            .premium_last_turn
            .get(faction)
            .copied()
            .unwrap_or(0),
        budget_lines: economy
            .budget_lines(last)
            .iter()
            .map(|line| BudgetLineView {
                key: line.kind.key().to_owned(),
                projected: line.projected,
                last: line.last,
                delta: line.delta(),
                charge: line.kind.is_charge(),
            })
            .collect(),
        budget_history: history
            .iter()
            .map(|record| BudgetRecordView {
                turn: record.turn,
                treasury: record.treasury,
                net: record.net(),
                change: record.change(),
                other: record.other,
            })
            .collect(),
        income_lines: lines
            .into_iter()
            .map(|(key, value)| KeyedAmount {
                key: key.to_owned(),
                value,
            })
            .collect(),
    })
}

/// A top-bar gauge now, next season, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GaugeView {
    pub current: i64,
    pub next: i64,
    pub delta: i64,
    pub other: f64,
    /// `(key, label, amount)` sorted by absolute weight; an empty label means the UI maps `key`.
    pub contributions: Vec<ContributionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContributionView {
    pub key: String,
    pub label: String,
    pub amount: f64,
}

/// Next-season forecast of the top-bar gauges.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastView {
    pub prestige: GaugeView,
    pub men: GaugeView,
    pub unrest: GaugeView,
}

/// Forecast for `faction` (simulates the next turn); `None` for an unknown faction.
pub fn forecast(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<ForecastView> {
    let forecast = sim_campaign::forecast::faction_forecast(state, data, faction)?;
    let gauge = |g: &sim_campaign::forecast::GaugeForecast| GaugeView {
        current: g.current,
        next: g.next,
        delta: g.delta(),
        other: g.other,
        contributions: g
            .contributions
            .iter()
            .map(|c| ContributionView {
                key: c.key.clone(),
                label: c.label.clone(),
                amount: c.amount,
            })
            .collect(),
    };
    Some(ForecastView {
        prestige: gauge(&forecast.prestige),
        men: gauge(&forecast.men),
        unrest: gauge(&forecast.unrest),
    })
}

/// A province with its gross tax income.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvinceIncome {
    pub province: String,
    pub name: String,
    pub value: i64,
}

/// The `limit` provinces of `faction` that bring in the most tax, largest first.
pub fn income_top_provinces(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    limit: usize,
) -> Vec<ProvinceIncome> {
    state
        .faction_province_incomes(data, faction)
        .into_iter()
        .take(limit)
        .map(|(province, value)| ProvinceIncome {
            name: data.province_name(&province),
            province: province.as_str().to_owned(),
            value,
        })
        .collect()
}

/// One difficulty level of the faction screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DifficultyLevel {
    pub id: String,
    pub label: String,
    pub description: String,
    pub effects: Vec<String>,
    pub is_default: bool,
}

/// The difficulty levels, easiest first.
pub fn difficulty_levels(data: &GameData) -> Vec<DifficultyLevel> {
    let rules = &data.difficulty;
    rules
        .levels
        .iter()
        .map(|level| DifficultyLevel {
            id: level.id.clone(),
            label: level.label.clone(),
            description: level.description.clone(),
            effects: effect_summary(&level.modifiers),
            is_default: level.id == rules.default,
        })
        .collect()
}

/// Key of the campaign's difficulty (`normal` by default).
pub fn current_difficulty(state: &CampaignState) -> &'static str {
    state.difficulty().key()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (&'static GameData, CampaignState, FactionId) {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        (data, state, FactionId::new("fac_france").unwrap())
    }

    #[test]
    fn top_bar_at_start() {
        let (data, state, id) = setup();
        let bar = top_bar(data, &state, &id).unwrap();
        assert_eq!(bar.turn, 0);
        assert_eq!(bar.date_label, "Printemps 1337");
        assert_eq!(bar.season, "spring");
        assert!(bar.manpower > 0 && bar.provinces_count > 0 && bar.armies_count > 0);
        assert_eq!(bar.difficulty, "normal");
        assert!(top_bar(data, &state, &FactionId::new("fac_nowhere").unwrap()).is_none());
        assert!(serde_json::to_string(&bar).unwrap().contains("manpower"));
    }

    #[test]
    fn economy_balances() {
        let (data, state, id) = setup();
        let view = economy(data, &state, &id).unwrap();
        let lines: i64 = view
            .budget_lines
            .iter()
            .filter(|l| l.key != "other")
            .map(|l| l.projected)
            .sum();
        assert_eq!(lines, view.net_income);
        assert!(!view.income_lines.is_empty());
        assert!(view.net_change.is_none());
    }

    #[test]
    fn forecast_and_top_provinces() {
        let (data, state, id) = setup();
        let f = forecast(data, &state, &id).unwrap();
        assert_eq!(f.men.delta, f.men.next - f.men.current);
        let top = income_top_provinces(data, &state, &id, 3);
        assert!(!top.is_empty() && top.len() <= 3);
        assert!(top.windows(2).all(|w| w[0].value >= w[1].value));
    }

    #[test]
    fn four_difficulty_levels() {
        let (data, state, _) = setup();
        let levels = difficulty_levels(data);
        assert_eq!(levels.len(), 4);
        assert_eq!(levels.iter().filter(|l| l.is_default).count(), 1);
        assert_eq!(current_difficulty(&state), "normal");
    }
}
