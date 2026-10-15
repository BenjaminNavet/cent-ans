//! Economy window (lot econ, ADR 0357). Ports `game/scripts/ui/economy/economy_window.gd`
//! (+ overview, trade and currency tabs, `economy_icons.gd`) and `map/economy_controller.gd`.
//!
//! `OpenPanel::Economy` (HUD button « Économie », key `map_toggle_economy` = M, key
//! `campaign_commerce` = I straight on the Commerce tab) opens a central stacked panel (id
//! `economy`) with three tabs: overview (budget table, forecast, drivers, balance history), trade
//! (agreements, new routes, active routes, goods) and currency (the four coinage levels, read-only;
//! the change itself stays in the faction panel). Data: `game_api::faction_panel` for the player;
//! the only orders are the trade agreements (`propose_trade_agreement`, `break_trade_agreement`),
//! as in the Godot window. Widget names: `economy:close`, `economy:tab:overview|trade|currency`,
//! `economy:propose:<id>`, `economy:break:<id>` (two presses), `economy:show_map`,
//! `economy:route_view:<id>`. Dev hook: `CENT_ANS_OPEN_ECONOMY=<overview|trade|currency>`.
//!
//! Royal acts (`royal_acts_section.gd`) live in the court panel (`panels/court`, Actes royaux
//! and captain), where Godot shows them; they are not duplicated here.

mod view;

use bevy::prelude::*;
use game_api::faction_panel::{
    FactionPanelView, break_trade_agreement, faction_panel, propose_trade_agreement,
};
use serde_json::{Value, json};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::menu::KeyEdges;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::panel_stack::{PanelClosed, PanelStackRequest};
use crate::ui::kit::{ButtonPressed, TabSelected};

/// Id of the panel in the panel stack.
pub const PANEL_ID: &str = "economy";
const TAB_GROUP: &str = "economy_tab";
pub(crate) const TABS: [(&str, &str); 3] = [
    ("overview", "Vue d’ensemble"),
    ("trade", "Commerce"),
    ("currency", "Monnaie"),
];

pub struct PanelEconomyPlugin;

impl Plugin for PanelEconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EconomyPanel>()
            .register_dump(dump_economy)
            .add_systems(
                Update,
                (
                    dev_open,
                    shortcuts.run_if(crate::campaign::hud::flow::orders_open),
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

/// State of the window, kept across rebuilds.
#[derive(Resource)]
pub struct EconomyPanel {
    pub open: bool,
    /// Key of the shown tab (`TABS`).
    pub tab: String,
    /// Tab the next `OpenPanel::Economy` should land on (key I), if any.
    wanted_tab: Option<String>,
    pub result: Option<(bool, String)>,
    /// Faction whose « Rompre » asked for a confirmation.
    pub break_armed: Option<String>,
    dirty: bool,
    last_turn: u32,
    pub view: Option<FactionPanelView>,
}

impl Default for EconomyPanel {
    fn default() -> Self {
        Self {
            open: false,
            tab: TABS[0].0.to_owned(),
            wanted_tab: None,
            result: None,
            break_armed: None,
            dirty: false,
            last_turn: 0,
            view: None,
        }
    }
}

/// Dev hook: `CENT_ANS_OPEN_ECONOMY=<tab>` opens the window once the campaign is running.
fn dev_open(
    sim: Res<Sim>,
    mut done: Local<bool>,
    mut panel: ResMut<EconomyPanel>,
    mut open: MessageWriter<OpenPanel>,
) {
    if *done || sim.state.is_none() {
        return;
    }
    *done = true;
    if let Ok(tab) = std::env::var("CENT_ANS_OPEN_ECONOMY")
        && !tab.is_empty()
    {
        if TABS.iter().any(|(key, _)| *key == tab) {
            panel.wanted_tab = Some(tab);
        }
        open.write(OpenPanel::Economy);
    }
}

fn close_on_exit(mut panel: ResMut<EconomyPanel>) {
    panel.open = false;
    panel.dirty = true;
}

/// M toggles the window; I toggles it on the Commerce tab (`economy_controller.gd`).
fn shortcuts(
    keymap: Res<KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    mut panel: ResMut<EconomyPanel>,
    mut open: MessageWriter<OpenPanel>,
) {
    if keymap.just_pressed("map_toggle_economy", &keys) {
        open.write(OpenPanel::Economy);
    } else if keymap.just_pressed("campaign_commerce", &keys) {
        panel.wanted_tab = Some("trade".to_owned());
        open.write(OpenPanel::Economy);
    }
}

fn refresh(panel: &mut EconomyPanel, sim: &Sim) {
    panel.view = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => faction_panel(data, state, &sim.player, &sim.player),
        _ => None,
    };
}

fn open_close(
    mut requests: MessageReader<OpenPanel>,
    mut closed: MessageReader<PanelClosed>,
    mut edges: ResMut<KeyEdges>,
    mut stack: MessageWriter<PanelStackRequest>,
    mut panel: ResMut<EconomyPanel>,
    sim: Res<Sim>,
) {
    for request in requests.read() {
        match request {
            OpenPanel::Economy => {
                let wanted = panel.wanted_tab.take();
                if panel.open && wanted.as_ref().is_none_or(|tab| *tab == panel.tab) {
                    stack.write(PanelStackRequest::Close(PANEL_ID.to_owned()));
                    panel.open = false;
                } else {
                    if let Some(tab) = wanted {
                        panel.tab = tab;
                    }
                    if !panel.open {
                        panel.result = None;
                        panel.break_armed = None;
                    }
                    panel.open = true;
                    panel.last_turn = sim.state.as_ref().map_or(0, |state| state.turn);
                }
                panel.dirty = true;
            }
            other if panel.open && displaces(other, PANEL_ID) => {
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
    mut stack: MessageWriter<PanelStackRequest>,
    mut panel: ResMut<EconomyPanel>,
    mut sim: ResMut<Sim>,
    mut focus: MessageWriter<FocusCamera>,
    mut open: MessageWriter<OpenPanel>,
) {
    for tab in tabs.read() {
        if tab.group == TAB_GROUP
            && let Some(key) = tab.name.strip_prefix("economy:tab:")
        {
            panel.tab = key.to_owned();
        }
    }
    for press in pressed.read() {
        // The HUD button opens the window even when it is closed.
        if press.name == "hud:open_economy" {
            open.write(OpenPanel::Economy);
            continue;
        }
        if !panel.open {
            continue;
        }
        let Some(name) = press.name.strip_prefix("economy:") else {
            continue;
        };
        if name == "close" {
            stack.write(PanelStackRequest::Close(PANEL_ID.to_owned()));
            panel.open = false;
            panel.dirty = true;
        } else if name == "show_map" || name.starts_with("route_view:") {
            let route = name.strip_prefix("route_view:");
            let target = panel.view.as_ref().and_then(|view| {
                view.trade
                    .routes
                    .iter()
                    .find(|r| {
                        r.mine && !r.from_province.is_empty() && route.is_none_or(|id| r.id == id)
                    })
                    .map(|r| r.from_province.clone())
            });
            if let Some(province) = target {
                focus.write(FocusCamera::Province(province));
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

/// A new season (or an order elsewhere) changes every figure: rebuild.
fn watch_turn(sim: Res<Sim>, mut panel: ResMut<EconomyPanel>) {
    let turn = sim.state.as_ref().map_or(0, |state| state.turn);
    if turn != panel.last_turn {
        panel.last_turn = turn;
        if panel.open {
            panel.dirty = true;
        }
    }
}

fn dump_economy(world: &World) -> (String, Value) {
    let panel = world.resource::<EconomyPanel>();
    let roots = world
        .try_query::<&view::EconomyRoot>()
        .map_or(0, |mut q| q.iter(world).count());
    let rows = world
        .try_query::<&view::EconomyRow>()
        .map_or(0, |mut q| q.iter(world).count());
    let summary = panel.view.as_ref().map(|view| {
        json!({
            "treasury": view.economy.treasury,
            "net_income": view.economy.net_income,
            "budget_lines": view.economy.budget_lines.len(),
            "income_sources": view.economy.income_lines.len(),
            "history": view.chart.iter().filter(|p| !p.now).count(),
            "forecast": view.forecast.as_ref().map(|f| json!({
                "prestige": [f.prestige.current, f.prestige.next],
                "men": [f.men.current, f.men.next],
                "unrest": [f.unrest.current, f.unrest.next],
            })),
            "trade_income": view.trade.income,
            "routes_mine": view.trade.routes.iter().filter(|r| r.mine).count(),
            "partners_signed": view.partners.iter().filter(|p| p.signed).count(),
            "partners_open": view.partners.iter().filter(|p| !p.signed && p.gain_self > 0).count(),
            "goods": view.goods.iter().map(|c| c.items.len()).sum::<usize>(),
            "coinage": view.coinage.as_ref().map(|c| json!({
                "label": c.label,
                "price_level": c.price_level,
                "levels": c.options.iter().map(|o| o.level.clone()).collect::<Vec<_>>(),
            })),
        })
    });
    (
        "panel_economy".into(),
        json!({
            "open": panel.open,
            "tab": panel.tab,
            "roots": roots,
            "rows": rows,
            "break_armed": panel.break_armed,
            "result": panel.result.as_ref().map(|(ok, text)| json!({"ok": ok, "text": text})),
            "view": summary,
        }),
    )
}
