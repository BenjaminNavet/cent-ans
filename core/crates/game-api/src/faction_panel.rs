//! View of the faction panel (`faction_panel.gd`, `trade_panel.gd`): the sheet, the detailed
//! budget, the treasury series of the chart, the tax brackets and the trade routes and
//! agreements of one faction. Everything is read from `core`; the client only draws it.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ResourceCategory, ResourceId};
use serde::Serialize;
use sim_campaign::coinage::{self, CoinageLevel};
use sim_campaign::crusade::{crusade_view, CrusadeView};
use sim_campaign::economy::TaxRate;
use sim_campaign::negotiation::Article;
use sim_campaign::trade::{trade_partner_options, trade_routes, TradeRouteView};
use sim_campaign::{CampaignState, Order};

use crate::common::FactionRef;
use crate::court::PersonRef;
use crate::diplomacy::{submit_order, OrderResult};
use crate::faction::{faction_sheet, FactionSheet};
use crate::feudal::{duties, feudal_sheet, DutiesView, FeudalSheet};
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

/// A trade partner (`get_trade_partner_options`): the projection before signing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartnerRow {
    pub id: String,
    pub name: String,
    /// Per-season change for this faction if an agreement were signed now.
    pub gain_self: i64,
    pub gain_partner: i64,
    /// What this faction already earns from the routes shared with the partner.
    pub current_self: i64,
    pub signed: bool,
}

/// One coinage level of the selector (`get_coinage().options`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoinageOption {
    /// `strong`, `sound`, `debased`, `heavily_debased`.
    pub level: String,
    pub label: String,
    /// Forecasts for this season, in livres.
    pub seigniorage: i64,
    pub recoinage: i64,
    pub inflation: u32,
    pub deflation: u32,
    pub burgher_unrest: f64,
    pub prestige: i32,
    pub current: bool,
}

/// The « Monnaie » section (`coinage_section.gd`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoinageSection {
    pub level: String,
    pub label: String,
    pub price_level: u32,
    pub changed_this_year: bool,
    pub seigniorage: i64,
    pub recoinage: i64,
    pub seigniorage_last_turn: i64,
    pub recoinage_last_turn: i64,
    pub options: Vec<CoinageOption>,
}

/// An order of chivalry the faction may found, with its bonuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChivalryOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: i64,
    pub prestige_required: i32,
    pub members: u32,
    pub historical_members: String,
    pub member_loyalty: u8,
    pub member_morale: u8,
    pub founder_prestige: i32,
    pub yearly_prestige: i32,
    pub min_year: i32,
    pub available: bool,
    pub reason: String,
}

/// The order the faction founded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChivalryFounded {
    pub id: String,
    pub name: String,
    pub collapsed: bool,
    pub members: Vec<PersonRef>,
    /// Size of the order once complete (0 if the definition is unknown).
    pub members_max: u32,
    pub member_loyalty: u8,
    pub member_morale: u8,
    pub yearly_prestige: i32,
}

/// The « Ordre de chevalerie » section (`chivalry_section.gd`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChivalrySection {
    pub founded: Option<ChivalryFounded>,
    /// Orders that can be founded (empty once one is founded).
    pub options: Vec<ChivalryOption>,
}

/// One resource of the goods list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoodsItem {
    pub id: String,
    pub name: String,
    pub count: u32,
}

/// Goods of one category, in the order `food`, `raw_material`, `manufactured`, `luxury`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoodsCategory {
    pub key: String,
    pub label: String,
    pub items: Vec<GoodsItem>,
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
    /// Trade partners (agreements in force and possible ones), best gain first.
    pub partners: Vec<PartnerRow>,
    pub goods: Vec<GoodsCategory>,
    /// `None` when the faction has no feudal position.
    pub feudal: Option<FeudalSheet>,
    pub duties: DutiesView,
    /// The sections below belong to the player's own panel only.
    pub coinage: Option<CoinageSection>,
    pub chivalry: Option<ChivalrySection>,
    /// `None` unless the player leads the crusader faction.
    pub crusade: Option<CrusadeView>,
}

fn category_label(category: ResourceCategory) -> &'static str {
    match category {
        ResourceCategory::Food => "Nourriture",
        ResourceCategory::RawMaterial => "Matières premières",
        ResourceCategory::Manufactured => "Manufacturé",
        ResourceCategory::Luxury => "Luxe",
    }
}

fn goods_by_category(data: &GameData, goods: &BTreeMap<ResourceId, u32>) -> Vec<GoodsCategory> {
    use ResourceCategory::{Food, Luxury, Manufactured, RawMaterial};
    let mut out = Vec::new();
    for category in [Food, RawMaterial, Manufactured, Luxury] {
        let mut items: Vec<GoodsItem> = goods
            .iter()
            .filter_map(|(id, count)| {
                let resource = data.resources.get(id)?;
                (resource.category == category).then(|| GoodsItem {
                    id: id.as_str().to_owned(),
                    name: resource.name.display.clone(),
                    count: *count,
                })
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        items.sort_by(|a, b| a.name.cmp(&b.name));
        out.push(GoodsCategory {
            key: category.key().to_owned(),
            label: category_label(category).to_owned(),
            items,
        });
    }
    out
}

fn coinage_section(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<CoinageSection> {
    let f = state.faction_state(faction)?;
    let options = CoinageLevel::ALL
        .into_iter()
        .map(|level| {
            let params = level.params();
            CoinageOption {
                level: level.key().to_owned(),
                label: level.label_fr().to_owned(),
                seigniorage: coinage::seigniorage_for(state, data, faction, level),
                recoinage: coinage::recoinage_for(state, data, faction, level),
                inflation: params.inflation,
                deflation: params.deflation,
                burgher_unrest: params.burgher_unrest,
                prestige: params.prestige,
                current: level == f.coinage,
            }
        })
        .collect();
    Some(CoinageSection {
        level: f.coinage.key().to_owned(),
        label: f.coinage.label_fr().to_owned(),
        price_level: f.price_level,
        changed_this_year: f.coinage_changed_year == Some(state.year()),
        seigniorage: coinage::seigniorage(state, data, faction),
        recoinage: coinage::recoinage(state, data, faction),
        seigniorage_last_turn: f.last_budget.seigniorage,
        recoinage_last_turn: f.last_budget.recoinage,
        options,
    })
}

fn chivalry_section(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> ChivalrySection {
    let base = crate::court::chivalric_orders(data, state, faction);
    let definition = |id: &str| {
        data.chivalric_orders
            .values()
            .find(|order| order.id.as_str() == id)
    };
    let founded = base.founded.map(|order| {
        let def = definition(&order.id);
        ChivalryFounded {
            members_max: def.map_or(0, |d| d.members),
            member_loyalty: def.map_or(0, |d| d.member_loyalty),
            member_morale: def.map_or(0, |d| d.member_morale),
            yearly_prestige: def.map_or(0, |d| d.yearly_prestige),
            id: order.id,
            name: order.name,
            collapsed: order.collapsed,
            members: order.members,
        }
    });
    let options = if founded.is_some() {
        Vec::new()
    } else {
        base.options
            .into_iter()
            .map(|option| {
                let def = definition(&option.id);
                ChivalryOption {
                    member_loyalty: def.map_or(0, |d| d.member_loyalty),
                    member_morale: def.map_or(0, |d| d.member_morale),
                    founder_prestige: def.map_or(0, |d| d.founder_prestige),
                    yearly_prestige: def.map_or(0, |d| d.yearly_prestige),
                    min_year: def.and_then(|d| d.min_year).unwrap_or(0),
                    id: option.id,
                    name: option.name,
                    description: option.description,
                    cost: option.cost,
                    prestige_required: option.prestige_required,
                    members: option.members,
                    historical_members: option.historical_members,
                    available: option.available,
                    reason: option.reason,
                }
            })
            .collect()
    };
    ChivalrySection { founded, options }
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
    let is_player = faction == player;
    let partners = trade_partner_options(state, data, faction)
        .iter()
        .map(|option| PartnerRow {
            id: option.faction.as_str().to_owned(),
            name: data
                .factions
                .get(&option.faction)
                .map_or(option.faction.as_str(), |f| f.short_or_display_name())
                .to_owned(),
            gain_self: option.gain_self,
            gain_partner: option.gain_partner,
            current_self: option.current_self,
            signed: option.signed,
        })
        .collect();
    Some(FactionPanelView {
        goods: goods_by_category(data, &state.factions.get(faction)?.goods),
        feudal: feudal_sheet(data, state, faction),
        duties: duties(data, state, faction),
        coinage: coinage_section(data, state, faction),
        chivalry: is_player.then(|| chivalry_section(data, state, faction)),
        crusade: if is_player {
            crusade_view(state, data, faction)
        } else {
            None
        },
        partners,
        is_player,
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

/// Strikes the player's money at `level` (`SetCoinage`, one change per year).
pub fn set_coinage(data: &GameData, state: &mut CampaignState, level: &str) -> OrderResult {
    match CoinageLevel::ALL.into_iter().find(|l| l.key() == level) {
        Some(level) => submit_order(data, state, Order::SetCoinage { level }),
        None => OrderResult {
            ok: false,
            message: format!("Monnaie inconnue : {level}"),
        },
    }
}

/// Founds the player's chivalric order (`FoundChivalricOrder`).
pub fn found_order(data: &GameData, state: &mut CampaignState, order: &str) -> OrderResult {
    crate::court::found_order(data, state, order)
}

/// Preaches the passage of the crusade (`PreachPassage`).
pub fn preach_passage(data: &GameData, state: &mut CampaignState) -> OrderResult {
    submit_order(data, state, Order::PreachPassage)
}

/// Proposes a trade agreement to `target` (`ProposeTreaty` with one article).
pub fn propose_trade_agreement(
    data: &GameData,
    state: &mut CampaignState,
    target: &FactionId,
) -> OrderResult {
    submit_order(
        data,
        state,
        Order::ProposeTreaty {
            target: target.clone(),
            articles: vec![Article::TradeAgreement],
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
    fn new_sections_are_filled_for_the_player() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let view = faction_panel(data, &state, &france(), &france()).unwrap();
        let coinage = view.coinage.as_ref().expect("coinage");
        assert_eq!(coinage.options.len(), 4);
        assert_eq!(coinage.options.iter().filter(|o| o.current).count(), 1);
        let chivalry = view.chivalry.as_ref().expect("chivalry");
        assert!(chivalry.founded.is_some() || !chivalry.options.is_empty());
        assert!(chivalry
            .options
            .iter()
            .all(|o| o.member_loyalty > 0 || o.members == 0));
        assert!(!view.goods.is_empty());
        let total: u32 = view
            .goods
            .iter()
            .flat_map(|c| &c.items)
            .map(|i| i.count)
            .sum();
        assert_eq!(total, state.factions[&france()].goods.values().sum::<u32>());
        assert!(view.crusade.is_none());
        assert!(view.feudal.is_some());
        let other = FactionId::new("fac_england").unwrap();
        let theirs = faction_panel(data, &state, &other, &france()).unwrap();
        assert!(theirs.chivalry.is_none() && theirs.crusade.is_none());
    }

    #[test]
    fn coinage_and_trade_orders_go_through() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(!set_coinage(data, &mut state, "bogus").ok);
        assert!(set_coinage(data, &mut state, "debased").ok);
        let view = faction_panel(data, &state, &france(), &france()).unwrap();
        assert!(view
            .coinage
            .unwrap()
            .options
            .iter()
            .any(|o| o.level == "debased" && o.current));
        assert!(
            !set_coinage(data, &mut state, "strong").ok,
            "one change per year"
        );
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
