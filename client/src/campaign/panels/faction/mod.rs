//! Campaign panel: faction (lot faction, ADR 0357). Ports `game/scripts/ui/faction_panel.gd`,
//! `budget_table.gd`, `treasury_chart.gd` and `trade_panel.gd`.
//!
//! `OpenPanel::Faction(id)` opens a central, stacked panel (`StackedPanel`, id `faction`) with
//! three tabs: the sheet (ruler, titles, liege and vassals, objectives, relations), the budget
//! (tax brackets, the table of receipts / expenses, the treasury chart drawn with UI nodes) and
//! trade (routes by value, trade agreements). Data: `game_api::faction_panel`; the tax bracket and
//! the end of a trade agreement go through `CampaignState::submit_order` as in the Godot bridge.
//! Closes on Escape, the close button, or `OpenPanel::Diplomacy` / `Tech` / `CloseAll`.
//!
//! Widget names (agent `click`): `faction:close`, `faction:tab:sheet|budget|trade`,
//! `faction:tax:low|normal|high`, `faction:mine_only`, `faction:break:<faction id>`.
//!
//! Fourth tab « Cour » (player only): order of chivalry and, for the crusaders, fervour. The
//! Budget tab also carries the Monnaie section and a treasury curve drawn in segments; the Fiche
//! tab the feudal obligations and the goods by category; the Commerce tab the partners (agreements
//! in force, new routes with their projection). See `sections.rs`.
//!
//! More widget names: `faction:coinage:<level>`, `faction:found:<order id>`, `faction:preach`,
//! `faction:propose:<faction id>` (`faction:break:<id>` asks for a confirmation first).
//!
//! Also: forecast and balance drivers (Budget), Table detail in the tooltip of its line, « Voir »
//! on routes (`faction:route_view:<id>`, `faction:show_map`: `FocusCamera`), clickable Codex links,
//! icons of goods / agreements / orders, the papal call (`faction:join_crusade`, two presses, and
//! `faction:crusade_target`) in the Court tab.
//!
//! Not ported: the ransoms window and the routes overlay of the map.

pub(crate) mod chart;
pub(crate) mod sections;
pub(crate) mod view;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use game_api::faction_panel::{
    FactionPanelView, break_trade_agreement, faction_panel, found_order, join_papal_crusade,
    preach_passage, propose_trade_agreement, set_coinage, set_tax_rate,
};
use serde_json::{Value, json};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::codex::{CodexOverlay, OpenCodex, text as codex_text};
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::RichLink;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::panel_stack::{PanelClosed, PanelStackRequest};
use crate::ui::kit::{ButtonPressed, TabSelected, ToggleChanged};

/// Id of the panel in the panel stack.
pub const PANEL_ID: &str = "faction";
const TAB_GROUP: &str = "faction_tab";
const TABS: [&str; 4] = [
    "faction:tab:sheet",
    "faction:tab:budget",
    "faction:tab:trade",
    "faction:tab:court",
];

pub struct PanelFactionPlugin;

impl Plugin for PanelFactionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FactionPanel>()
            .register_dump(dump_faction)
            .add_systems(
                Update,
                (
                    dev_open,
                    open_close,
                    handle_input,
                    codex_links,
                    watch_turn,
                    view::rebuild,
                )
                    .chain()
                    .before(crate::menu::pause::pause_escape)
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit);
    }
}

/// State of the panel, kept across rebuilds.
#[derive(Resource)]
pub struct FactionPanel {
    pub open: bool,
    pub faction: Option<String>,
    pub tab: String,
    pub mine_only: bool,
    pub result: Option<(bool, String)>,
    /// Faction whose trade agreement « Rompre » asked to confirm.
    pub break_armed: Option<String>,
    /// « Rejoindre la croisade » asked its confirmation.
    pub join_armed: bool,
    dirty: bool,
    last_turn: u32,
    pub view: Option<FactionPanelView>,
}

impl Default for FactionPanel {
    fn default() -> Self {
        Self {
            open: false,
            faction: None,
            tab: TABS[0].to_owned(),
            mine_only: true,
            result: None,
            break_armed: None,
            join_armed: false,
            dirty: false,
            last_turn: 0,
            view: None,
        }
    }
}

/// Dev hook: `CENT_ANS_OPEN_FACTION=<faction id>` opens the panel once the campaign is running
/// (agent scripts have no way to send an `OpenPanel`).
fn dev_open(sim: Res<Sim>, mut done: Local<bool>, mut open: MessageWriter<OpenPanel>) {
    if *done || sim.state.is_none() {
        return;
    }
    *done = true;
    if let Ok(id) = std::env::var("CENT_ANS_OPEN_FACTION")
        && !id.is_empty()
    {
        open.write(OpenPanel::Faction(id));
    }
}

fn close_on_exit(mut panel: ResMut<FactionPanel>) {
    panel.open = false;
    panel.dirty = true;
}

fn refresh(panel: &mut FactionPanel, sim: &Sim) {
    panel.view = match (&sim.data, &sim.state, &panel.faction) {
        (Some(data), Some(state), Some(id)) => data_model::FactionId::new(id.clone())
            .ok()
            .and_then(|id| faction_panel(data, state, &id, &sim.player)),
        _ => None,
    };
}

#[allow(clippy::too_many_arguments)]
fn open_close(
    mut requests: MessageReader<OpenPanel>,
    mut closed: MessageReader<PanelClosed>,
    mut edges: ResMut<KeyEdges>,
    mut stack: MessageWriter<PanelStackRequest>,
    mut panel: ResMut<FactionPanel>,
    sim: Res<Sim>,
) {
    for request in requests.read() {
        match request {
            OpenPanel::Faction(id) => {
                if !panel.open || panel.faction.as_deref() != Some(id) {
                    panel.faction = Some(id.clone());
                    panel.result = None;
                    panel.break_armed = None;
                    panel.join_armed = false;
                }
                panel.open = true;
                panel.dirty = true;
                panel.last_turn = sim.state.as_ref().map_or(0, |state| state.turn);
            }
            other if panel.open && displaces(other, "faction") => {
                panel.open = false;
                panel.dirty = true;
            }
            _ => {}
        }
    }
    for message in closed.read() {
        if message.id == PANEL_ID && panel.open {
            panel.open = false;
            panel.dirty = true;
        }
    }
    if panel.open && edges.escape {
        edges.escape = false;
        stack.write(PanelStackRequest::Close(PANEL_ID.to_owned()));
        panel.open = false;
        panel.dirty = true;
    }
}

fn handle_input(
    mut pressed: MessageReader<ButtonPressed>,
    mut tabs: MessageReader<TabSelected>,
    mut toggles: MessageReader<ToggleChanged>,
    mut stack: MessageWriter<PanelStackRequest>,
    mut panel: ResMut<FactionPanel>,
    mut sim: ResMut<Sim>,
    mut focus: MessageWriter<FocusCamera>,
) {
    for tab in tabs.read() {
        if tab.group == TAB_GROUP {
            panel.tab = tab.name.clone();
        }
    }
    for toggle in toggles.read() {
        if toggle.name == "faction:mine_only" {
            panel.mine_only = toggle.on;
            panel.dirty = true;
        }
    }
    if !panel.open {
        return;
    }
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("faction:") else {
            continue;
        };
        if name == "close" {
            stack.write(PanelStackRequest::Close(PANEL_ID.to_owned()));
            panel.open = false;
            panel.dirty = true;
        } else if name == "show_map" {
            let target = panel.view.as_ref().and_then(|view| {
                view.trade
                    .routes
                    .iter()
                    .find(|r| r.mine && !r.from_province.is_empty())
                    .map(|r| r.from_province.clone())
            });
            if let Some(province) = target {
                focus.write(FocusCamera::Province(province));
            }
        } else if let Some(id) = name.strip_prefix("route_view:") {
            let target = panel.view.as_ref().and_then(|view| {
                view.trade
                    .routes
                    .iter()
                    .find(|r| r.id == id && !r.from_province.is_empty())
                    .map(|r| r.from_province.clone())
            });
            if let Some(province) = target {
                focus.write(FocusCamera::Province(province));
            }
        } else if name == "crusade_target" {
            let target = panel
                .view
                .as_ref()
                .and_then(|view| view.papal_call.as_ref())
                .map(|call| call.target.clone());
            if let Some(province) = target {
                focus.write(FocusCamera::Province(province));
            }
        } else if name == "join_crusade" {
            if !panel.join_armed {
                panel.join_armed = true;
                panel.dirty = true;
                continue;
            }
            panel.join_armed = false;
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = join_papal_crusade(data, state);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if let Some(rate) = name.strip_prefix("tax:") {
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = set_tax_rate(data, state, rate);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if let Some(level) = name.strip_prefix("coinage:") {
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = set_coinage(data, state, level);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if let Some(order) = name.strip_prefix("found:") {
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = found_order(data, state, order);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if name == "preach" {
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = preach_passage(data, state);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if let Some(target) = name.strip_prefix("propose:") {
            let sim = &mut *sim;
            if let (Some(data), Some(state), Ok(target)) = (
                &sim.data,
                &mut sim.state,
                data_model::FactionId::new(target.to_owned()),
            ) {
                let result = propose_trade_agreement(data, state, &target);
                panel.result = Some((
                    result.ok,
                    if result.ok {
                        "Accord commercial signé.".to_owned()
                    } else {
                        result.message
                    },
                ));
                panel.dirty = true;
            }
        } else if let Some(target) = name.strip_prefix("break:") {
            if panel.break_armed.as_deref() != Some(target) {
                panel.break_armed = Some(target.to_owned());
                panel.dirty = true;
                continue;
            }
            panel.break_armed = None;
            let sim = &mut *sim;
            if let (Some(data), Some(state), Ok(target)) = (
                &sim.data,
                &mut sim.state,
                data_model::FactionId::new(target.to_owned()),
            ) {
                let result = break_trade_agreement(data, state, &target);
                panel.result = Some((
                    result.ok,
                    if result.ok {
                        "Accord commercial rompu.".to_owned()
                    } else {
                        result.message
                    },
                ));
                panel.dirty = true;
            }
        }
    }
}

/// A click on a `[[cdx_…]]` link of the panel opens that Codex entry.
fn codex_links(
    mouse: Res<ButtonInput<MouseButton>>,
    overlay: Res<CodexOverlay>,
    panel: Res<FactionPanel>,
    links: Query<(&RichLink, &Hovered)>,
    mut open: MessageWriter<OpenCodex>,
) {
    if !panel.open || overlay.open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let target = links
        .iter()
        .find(|(_, hovered)| hovered.get())
        .and_then(|(link, _)| codex_text::target_id(&link.0).map(str::to_owned));
    if let Some(entry) = target {
        open.write(OpenCodex { entry: Some(entry) });
    }
}

/// A new season changes every figure: rebuild.
fn watch_turn(sim: Res<Sim>, mut panel: ResMut<FactionPanel>) {
    let turn = sim.state.as_ref().map_or(0, |state| state.turn);
    if turn != panel.last_turn {
        panel.last_turn = turn;
        if panel.open {
            panel.dirty = true;
        }
    }
}

fn dump_faction(world: &World) -> (String, Value) {
    let panel = world.resource::<FactionPanel>();
    let mut texts = 0;
    let mut bars = 0;
    let mut rows = 0;
    let count = |world: &World, marker: &str| -> usize {
        match marker {
            "segments" => world
                .try_query::<&view::ChartSegment>()
                .map_or(0, |mut q| q.iter(world).count()),
            "goods" => world
                .try_query::<&sections::GoodsChip>()
                .map_or(0, |mut q| q.iter(world).count()),
            "fervor_ticks" => world
                .try_query::<&sections::FervorTick>()
                .map_or(0, |mut q| q.iter(world).count()),
            "fervor_fill" => world
                .try_query::<&sections::FervorFill>()
                .map_or(0, |mut q| q.iter(world).count()),
            _ => world
                .try_query::<&sections::PriceFill>()
                .map_or(0, |mut q| q.iter(world).count()),
        }
    };
    if let Some(mut query) = world.try_query::<&view::ChartBar>() {
        bars = query.iter(world).count();
    }
    if let Some(mut query) = world.try_query::<&view::TradeRowMarker>() {
        rows = query.iter(world).count();
    }
    if let Some(mut query) = world.try_query::<&view::FactionRoot>() {
        texts = query.iter(world).count();
    }
    let summary = panel.view.as_ref().map(|view| {
        json!({
            "faction": view.sheet.id,
            "name": view.sheet.name,
            "ruler": view.sheet.ruler,
            "titles": view.sheet.titles.len(),
            "objectives": view.sheet.objectives.len(),
            "treasury": view.economy.treasury,
            "net_income": view.economy.net_income,
            "budget_lines": view.economy.budget_lines.len(),
            "income_sources": view.economy.income_lines.len(),
            "tax": view.tax.iter().find(|t| t.selected).map(|t| t.key.clone()),
            "chart_points": view.chart.len(),
            "chart_turns": view.chart.iter().map(|p| p.turn).collect::<Vec<_>>(),
            "chart_treasury": view.chart.iter().map(|p| p.treasury).collect::<Vec<_>>(),
            "forecast": view.forecast.as_ref().map(|f| json!({"prestige": [f.prestige.current, f.prestige.next], "men": [f.men.current, f.men.next], "unrest": [f.unrest.current, f.unrest.next]})),
            "top_provinces": view.top_provinces.len(),
            "table_provinces": view.table.len(),
            "routes_with_province": view.trade.routes.iter().filter(|r| !r.from_province.is_empty()).count(),
            "papal_call": view.papal_call.as_ref().map(|c| json!({"target": c.target, "can_join": c.can_join, "turns_left": c.turns_left})),
            "trade_income": view.trade.income,
            "trade_routes": view.trade.routes.len(),
            "trade_agreements": view.trade.agreements.len(),
            "is_player": view.is_player,
            "partners": view.partners.len(),
            "partners_signed": view.partners.iter().filter(|p| p.signed).count(),
            "goods_categories": view.goods.len(),
            "goods": view.goods.iter().map(|c| c.items.len()).sum::<usize>(),
            "duties_lines": view.feudal.as_ref().map(|s| sections::feudal_lines(s, &view.duties).len()),
            "coinage": view.coinage.as_ref().map(|c| json!({
                "level": c.level,
                "price_level": c.price_level,
                "options": c.options.len(),
                "changed_this_year": c.changed_this_year,
            })),
            "chivalry": view.chivalry.as_ref().map(|c| json!({
                "founded": c.founded.as_ref().map(|f| f.name.clone()),
                "options": c.options.len(),
            })),
            "crusade": view.crusade.as_ref().map(|c| json!({
                "fervor": c.fervor,
                "status": sections::crusade_status_text(c),
            })),
        })
    });
    (
        "panel_faction".into(),
        json!({
            "open": panel.open,
            "faction": panel.faction,
            "tab": panel.tab,
            "mine_only": panel.mine_only,
            "roots": texts,
            "chart_bars": bars,
            "trade_rows": rows,
            "chart_segments": count(world, "segments"),
            "goods_chips": count(world, "goods"),
            "price_fills": count(world, "price"),
            "fervor_fills": count(world, "fervor_fill"),
            "fervor_ticks": count(world, "fervor_ticks"),
            "break_armed": panel.break_armed,
            "result": panel.result.as_ref().map(|(ok, text)| json!({"ok": ok, "text": text})),
            "view": summary,
        }),
    )
}
