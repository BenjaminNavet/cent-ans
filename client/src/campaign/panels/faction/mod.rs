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
//! Not ported: the sections Monnaie, Ordre de chevalerie, Ferveur, Obligations féodales and the
//! ransoms window (their own lots / core views), the goods list by category (`goods` is not in
//! `game-api` yet) and the per-province Table detail tooltip.

mod chart;
mod view;

use bevy::prelude::*;
use game_api::faction_panel::{
    FactionPanelView, break_trade_agreement, faction_panel, set_tax_rate,
};
use serde_json::{Value, json};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::panel_stack::{PanelClosed, PanelStackRequest};
use crate::ui::kit::{ButtonPressed, TabSelected, ToggleChanged};

/// Id of the panel in the panel stack.
pub const PANEL_ID: &str = "faction";
const TAB_GROUP: &str = "faction_tab";
const TABS: [&str; 3] = [
    "faction:tab:sheet",
    "faction:tab:budget",
    "faction:tab:trade",
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
        } else if let Some(rate) = name.strip_prefix("tax:") {
            let sim = &mut *sim;
            if let (Some(data), Some(state)) = (&sim.data, &mut sim.state) {
                let result = set_tax_rate(data, state, rate);
                panel.result = Some((result.ok, result.message));
                panel.dirty = true;
            }
        } else if let Some(target) = name.strip_prefix("break:") {
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
            "trade_income": view.trade.income,
            "trade_routes": view.trade.routes.len(),
            "trade_agreements": view.trade.agreements.len(),
            "is_player": view.is_player,
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
            "result": panel.result.as_ref().map(|(ok, text)| json!({"ok": ok, "text": text})),
            "view": summary,
        }),
    )
}
