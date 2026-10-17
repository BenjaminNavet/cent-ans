//! Campaign panel: feudal tree (lot `feudal`, ADR 0357).
//!
//! Ports `game/scripts/ui/feudal_section.gd` (position, obligations, objectives),
//! `game/scripts/map/feudal_controller.gd` (the « Arbre féodal » panel: suzerain chain, vassals
//! with loyalty pills, feudal calls, actions) and `game/scripts/ui/house_arms.gd` (house shield,
//! see `heraldry.rs`). Data comes from `game_api::feudal`; orders go through
//! `CampaignState::submit_order` like `godot-bridge` (`feudal_revolt`, `feudal_switch_allegiance`,
//! `feudal_declare_commise`, `feudal_grant_title`, `feudal_arbitrate`, `answer_offer`).
//!
//! Opens on `OpenPanel::Feudal` (toggle) or `CENT_ANS_OPEN_FEUDAL=1` (dev hook, first campaign
//! frame); closes on Escape, the close button, `OpenPanel::CloseAll`. The « Événements féodaux »
//! list and the toasts of the end of turn come from `TurnAdvanced` (`record_events`); the
//! three-step guide is `guide.rs`. Not ported: the map
//! filter, the breadcrumb link from the province sheet (province lot), the Menu entry
//! (HUD lot sends `OpenPanel::Feudal`), the possession tooltips of `holdings_controller.gd`
//! (the Colonies list is another screen).

mod format;
mod guide;
mod heraldry;
mod view;

use std::collections::HashSet;

use bevy::prelude::*;
use data_model::FactionId;
use game_api::feudal::{self, DutiesView, FeudalPanel, FeudalSheet, OrderResult};
use serde_json::{Value, json};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::panel_stack::displaces;

pub struct PanelFeudalPlugin;

impl Plugin for PanelFeudalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeudalUi>()
            .register_dump(dump_feudal)
            .add_plugins(guide::plugin)
            .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
            .add_systems(
                Update,
                (
                    open_close.before(crate::menu::pause::pause_escape),
                    handle_presses,
                    record_events,
                    view::rebuild,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit);
    }
}

const ENV_OPEN: &str = "CENT_ANS_OPEN_FEUDAL";

/// Outcome of the last order, shown under the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastResult {
    pub ok: bool,
    pub text: String,
}

/// One feudal event kept for the « Événements féodaux » list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentEvent {
    pub date: String,
    pub text: String,
}

/// What the panel shows, recomputed from the sim each time it is rebuilt.
#[derive(Default)]
pub struct Snapshot {
    pub panel: Option<FeudalPanel>,
    /// The player's own sheet (position and objectives).
    pub player: Option<FeudalSheet>,
    pub selected: Option<FeudalSheet>,
    /// Duties of the player (also in `panel.duties`).
    pub duties: Option<DutiesView>,
    pub arms: heraldry::HouseArms,
}

#[derive(Resource, Default)]
pub struct FeudalUi {
    pub open: bool,
    /// Faction the tree is opened on (the player by default).
    pub focus: Option<String>,
    pub selected: Option<String>,
    /// Folded-out nodes of the tree.
    pub expanded: HashSet<String>,
    pub homage_index: usize,
    pub grant_index: usize,
    pub result: Option<LastResult>,
    /// Feudal events of the latest turns, newest first (`FeudalController.recent`).
    pub recent: Vec<RecentEvent>,
    pub(crate) dirty: bool,
    env_checked: bool,
    arms: Option<heraldry::HouseArms>,
    pub snapshot: Snapshot,
}

impl FeudalUi {
    /// Opens the tree on `faction` (the player when `None`).
    pub fn open_on(&mut self, faction: Option<String>) {
        self.open = true;
        self.selected = faction.clone();
        self.focus = faction;
        self.expanded.clear();
        self.homage_index = 0;
        self.grant_index = 0;
        self.result = None;
        self.dirty = true;
    }
}

fn reset_on_enter(mut ui: ResMut<FeudalUi>) {
    crate::menu::save_extras::restore_feudal(&mut ui);
}

/// End of turn (`FeudalController.after_end_turn`): the feudal events are noted for the panel
/// and announced when they concern the player; pending feudal calls are announced too.
fn record_events(
    mut advanced: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    mut ui: ResMut<FeudalUi>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for turn in advanced.read() {
        let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
            continue;
        };
        let date = state.date_label();
        let my_name = data.faction_name(&sim.player);
        let mut mine = 0;
        let mut fresh = Vec::new();
        for event in &turn.events {
            let kind = serde_json::to_value(event.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            if !format::is_feudal_event(&kind, &event.text_fr) {
                continue;
            }
            fresh.push(RecentEvent {
                date: date.clone(),
                text: event.text_fr.clone(),
            });
            if event.faction.as_ref() == Some(&sim.player)
                || (!my_name.is_empty() && event.text_fr.contains(&my_name))
            {
                mine += 1;
            }
        }
        fresh.extend(std::mem::take(&mut ui.recent));
        fresh.truncate(format::RECENT_MAX);
        ui.recent = fresh;
        let calls = feudal::calls(data, state);
        if let Some(call) = calls.first() {
            toasts.write(ToastRequest::new(&format!("Appel féodal : {}", call.text)));
        } else if mine > 0 {
            toasts.write(ToastRequest::new(&format!(
                "{} vous concernant : voir l’arbre féodal.",
                format::event_count(mine)
            )));
        }
        ui.dirty = true;
    }
}

fn close_on_exit(mut ui: ResMut<FeudalUi>) {
    ui.open = false;
    ui.dirty = true;
}

/// `OpenPanel::Feudal`, the dev hook and Escape.
fn open_close(
    mut requests: MessageReader<OpenPanel>,
    mut edges: ResMut<KeyEdges>,
    mut ui: ResMut<FeudalUi>,
) {
    if !ui.env_checked {
        ui.env_checked = true;
        if std::env::var(ENV_OPEN).is_ok_and(|v| !v.is_empty() && v != "0") {
            ui.open_on(None);
        }
    }
    for request in requests.read() {
        match request {
            OpenPanel::Feudal => {
                if ui.open {
                    ui.open = false;
                    ui.dirty = true;
                } else {
                    ui.open_on(None);
                }
            }
            other if ui.open && displaces(other, "feudal") => {
                ui.open = false;
                ui.dirty = true;
            }
            _ => {}
        }
    }
    if ui.open && edges.escape {
        edges.escape = false;
        ui.open = false;
        ui.dirty = true;
    }
}

fn handle_presses(
    mut pressed: MessageReader<ButtonPressed>,
    mut ui: ResMut<FeudalUi>,
    mut sim: ResMut<Sim>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    if !ui.open {
        return;
    }
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("feudal:") else {
            continue;
        };
        if let Some(event) = crate::audio::uisound::panel_sound(
            &mut ui,
            |ui| &mut ui.result,
            |result| result.ok,
            |ui| press_button(name, ui, &mut sim),
        ) {
            sfx.write(crate::audio::PlaySfx::new(event));
        }
        ui.dirty = true;
    }
}

fn run<F>(sim: &mut Sim, order: F) -> OrderResult
where
    F: FnOnce(&data_model::GameData, &mut sim_campaign::CampaignState) -> OrderResult,
{
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return OrderResult {
            ok: false,
            message: "aucune campagne en cours".to_owned(),
        };
    };
    order(&data, state)
}

fn report(ui: &mut FeudalUi, result: OrderResult, success: &str) {
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            success.to_owned()
        } else {
            result.message
        },
    });
}

fn press_button(name: &str, ui: &mut FeudalUi, sim: &mut Sim) {
    match name {
        "close" => ui.open = false,
        "revolt" => {
            let result = run(sim, feudal::revolt);
            report(ui, result, "Vous vous révoltez contre votre suzerain.");
        }
        "homage:next" => ui.homage_index += 1,
        "grant:next" => ui.grant_index += 1,
        "homage" => {
            let Some(lord) = ui
                .snapshot
                .duties
                .as_ref()
                .and_then(|d| pick(&d.homage_candidates, ui.homage_index))
                .map(|f| f.id.clone())
            else {
                return;
            };
            let result = run(sim, |d, s| feudal::switch_allegiance(d, s, &lord));
            report(ui, result, "Hommage prêté.");
        }
        "commise" => {
            let Some(target) = ui.selected.clone() else {
                return;
            };
            let result = run(sim, |d, s| feudal::declare_commise(d, s, &target));
            report(
                ui,
                result,
                "Commise prononcée : la guerre est déclarée au félon.",
            );
        }
        "grant" => {
            let (Some(target), Some(title)) = (
                ui.selected.clone(),
                ui.snapshot
                    .duties
                    .as_ref()
                    .and_then(|d| pick(&d.grantable_titles, ui.grant_index))
                    .map(|t| t.id.clone()),
            ) else {
                return;
            };
            let result = run(sim, |d, s| feudal::grant_title(d, s, &title, &target));
            report(ui, result, "Titre concédé.");
        }
        _ => {
            if let Some(id) = name.strip_prefix("node:") {
                ui.selected = Some(id.to_owned());
                ui.grant_index = 0;
            } else if let Some(id) = name.strip_prefix("fold:") {
                if !ui.expanded.remove(id) {
                    ui.expanded.insert(id.to_owned());
                }
            } else if let Some(rest) = name.strip_prefix("call:") {
                call_answer(ui, sim, rest);
            }
        }
    }
}

/// Element `index` of `list`, wrapping (the Godot `OptionButton` cycled by the client).
pub fn pick<T>(list: &[T], index: usize) -> Option<&T> {
    (!list.is_empty()).then(|| &list[index % list.len()])
}

/// `call:<offer>:<verb>` with verb `yes`, `no`, `peace`, `side:<faction>`, `letbe`.
fn call_answer(ui: &mut FeudalUi, sim: &mut Sim, rest: &str) {
    let mut parts = rest.splitn(3, ':');
    let (Some(id), Some(verb)) = (parts.next(), parts.next()) else {
        return;
    };
    let side = parts.next().unwrap_or_default().to_owned();
    let Ok(id) = id.parse::<u32>() else {
        return;
    };
    let kind = ui
        .snapshot
        .panel
        .as_ref()
        .and_then(|p| p.calls.iter().find(|c| c.id == id))
        .map(|c| c.kind.clone())
        .unwrap_or_default();
    let (result, text) = match verb {
        "yes" | "no" => {
            let accept = verb == "yes";
            let text = match (kind.as_str(), accept) {
                ("peace_summons", true) => "Vous faites la paix.",
                ("peace_summons", false) => "Vous passez outre.",
                (_, true) => "Vous intervenez.",
                (_, false) => "Vous vous dérobez.",
            };
            (run(sim, |d, s| feudal::answer_call(d, s, id, accept)), text)
        }
        "peace" => (
            run(sim, |d, s| feudal::arbitrate(d, s, id, "impose_peace", "")),
            "Arbitrage rendu.",
        ),
        "letbe" => (
            run(sim, |d, s| feudal::arbitrate(d, s, id, "let_be", "")),
            "Arbitrage rendu.",
        ),
        "side" => (
            run(sim, |d, s| feudal::arbitrate(d, s, id, "take_side", &side)),
            "Arbitrage rendu.",
        ),
        _ => return,
    };
    report(ui, result, text);
}

/// Recomputes the snapshot from the sim; picks defaults for the focus and the selection.
pub(crate) fn refresh_snapshot(ui: &mut FeudalUi, sim: &Sim) {
    let arms = ui
        .arms
        .take()
        .unwrap_or_else(|| match crate::paths::find_root() {
            Ok(root) => heraldry::HouseArms::load(&crate::paths::data_dir(&root)),
            Err(_) => heraldry::HouseArms::default(),
        });
    ui.arms = Some(arms.clone());
    ui.snapshot = Snapshot {
        arms,
        ..Snapshot::default()
    };
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let player: FactionId = sim.player.clone();
    let focus = ui
        .focus
        .as_deref()
        .and_then(|f| FactionId::new(f.to_owned()).ok())
        .filter(|f| state.factions.contains_key(f))
        .unwrap_or_else(|| player.clone());
    let Some(panel) = feudal::feudal_panel(data, state, &focus) else {
        return;
    };
    if ui.expanded.is_empty() {
        // Folded out on the focused position: the root, its lieges and itself (Godot: `chain`).
        ui.expanded.insert(panel.tree.id.clone());
        ui.expanded.insert(focus.as_str().to_owned());
        ui.expanded
            .extend(panel.focus.liege_chain.iter().map(|f| f.id.clone()));
    }
    let selected = ui
        .selected
        .clone()
        .unwrap_or_else(|| focus.as_str().to_owned());
    ui.selected = Some(selected.clone());
    ui.snapshot.selected = FactionId::new(selected)
        .ok()
        .and_then(|f| feudal::feudal_sheet(data, state, &f));
    ui.snapshot.player = feudal::feudal_sheet(data, state, &player);
    ui.snapshot.duties = Some(panel.duties.clone());
    ui.snapshot.panel = Some(panel);
}

fn count_nodes(node: &feudal::TreeNodeView) -> usize {
    1 + node.vassals.iter().map(count_nodes).sum::<usize>()
}

fn dump_feudal(world: &World) -> (String, Value) {
    let ui = world.resource::<FeudalUi>();
    let snap = &ui.snapshot;
    let panel = snap.panel.as_ref().map(|p| {
        json!({
            "focus": p.focus.id,
            "sovereign": p.focus.sovereign.id,
            "liege": p.focus.liege.as_ref().map(|l| l.id.clone()),
            "loyalty": p.focus.loyalty,
            "status": p.focus.status,
            "titles": p.focus.titles.iter().map(|t| json!({
                "name": t.name, "rank": t.rank, "primary": t.primary, "provinces": t.provinces.len(),
            })).collect::<Vec<_>>(),
            "direct_vassals": p.focus.direct_vassals.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
            "title_vassals": p.focus.title_vassals.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
            "objectives": p.focus.objectives.len(),
            "house": p.focus.house,
            "house_arms": snap.arms.asset_path(&p.focus.house),
            "tree_root": p.tree.id,
            "tree_nodes": count_nodes(&p.tree),
            "calls": p.calls.iter().map(|c| json!({"id": c.id, "kind": c.kind})).collect::<Vec<_>>(),
            "duties": {
                "tribute_percent": p.duties.tribute_percent,
                "felons": p.duties.felons.len(),
                "grantable_titles": p.duties.grantable_titles.len(),
                "homage_candidates": p.duties.homage_candidates.len(),
            },
        })
    });
    (
        "feudal".into(),
        json!({
            "open": ui.open,
            "selected": ui.selected,
            "expanded": ui.expanded.len(),
            "recent": ui.recent.iter().map(|e| json!({"date": e.date, "text": e.text})).collect::<Vec<_>>(),
            "houses_known": snap.arms.len(),
            "result": ui.result.as_ref().map(|r| json!({"ok": r.ok, "text": r.text})),
            "panel": panel,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_wraps() {
        let list = [1, 2, 3];
        assert_eq!(pick(&list, 0), Some(&1));
        assert_eq!(pick(&list, 4), Some(&2));
        assert_eq!(pick::<i32>(&[], 3), None);
    }

    #[test]
    fn open_on_resets_state() {
        let mut ui = FeudalUi {
            homage_index: 3,
            ..FeudalUi::default()
        };
        ui.expanded.insert("x".into());
        ui.open_on(Some("fac_france".into()));
        assert!(ui.open && ui.dirty);
        assert_eq!(ui.selected.as_deref(), Some("fac_france"));
        assert_eq!(ui.homage_index, 0);
        assert!(ui.expanded.is_empty());
    }
}
