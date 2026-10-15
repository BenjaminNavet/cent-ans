//! Campaign panel: diplomacy (V2 lot; owned by a single agent lot, ADR 0357).
//!
//! Ports `game/scripts/ui/diplomacy_panel.gd` and `ui/diplomacy/*` (UX5 lot D layout): the
//! faction list on the left (filter, sort, relation / stance / attitude, reasons in a tooltip),
//! the sheet of the chosen faction on the right (head with the opinion breakdown and links) with
//! three tabs: negotiation (draft treaty, clause menus, acceptance preview and their reasoning,
//! counter-offer, unilateral actions with consequences), war, treaties. Offers received sit on
//! top. Data comes from `game_api::diplomacy`; orders go through `CampaignState::submit_order`
//! exactly like the Godot bridge (`propose_treaty`, `declare_war`, `answer_offer`, ...).
//!
//! Opens on `OpenPanel::Diplomacy` (toggle) or key F3, closes on Escape, the close button, or an
//! `OpenPanel::Tech` / `CloseAll`. Not ported yet: the diplomatic map in the centre
//! (`diplomacy_map_view.gd`), the chancery line
//! (`get_religion_state`), the Church donation, feudal arbitration verdicts, the search field is typed on
//! the keyboard (no kit text input), the free gold/tribute spin boxes (preset amounts instead).

mod format;
mod view;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use game_api::diplomacy::{
    ClauseSlot, FactionRow, OfferView, RelationDetail, TreatyVerdict, ask_counter, court_banner,
    evaluate_draft, faction_list, offers, relation_detail, submit_order,
};
use serde_json::{Value, json};
use sim_campaign::negotiation::{Article, Party};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::{ButtonPressed, TabSelected};

pub struct PanelDiplomacyPlugin;

impl Plugin for PanelDiplomacyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiplomacyUi>()
            .register_dump(dump_diplomacy)
            .add_systems(
                Update,
                (open_close, type_query, handle_presses, view::rebuild)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit);
    }
}

/// Amounts of the preset gold clauses (livres) and of the preset tributes (per season).
pub const GOLD_STEPS: [i64; 3] = [500, 1000, 5000];
pub const TRIBUTE_STEPS: [i64; 2] = [100, 500];
pub const TRIBUTE_SEASONS: u32 = 8;

/// Outcome of the last order sent from the screen, shown under the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastResult {
    pub ok: bool,
    pub text: String,
}

/// What the screen shows, recomputed from the sim each time the UI is rebuilt.
#[derive(Default)]
pub struct Snapshot {
    pub rows: Vec<FactionRow>,
    pub detail: Option<RelationDetail>,
    pub offers: Vec<OfferView>,
    /// Lines of the court banner.
    pub court: Vec<String>,
    pub verdict: Option<TreatyVerdict>,
    /// Widget name of each clause of `detail.options.clauses`, in the same order.
    pub clause_names: Vec<String>,
}

#[derive(Resource)]
pub struct DiplomacyUi {
    pub open: bool,
    pub selected: Option<String>,
    pub filter: usize,
    pub sort: usize,
    /// Search text of the faction list (name contains, case-insensitive).
    pub query: String,
    /// Name of the selected tab of group `dip_tab`.
    pub tab: String,
    pub draft: Vec<Article>,
    draft_for: Option<String>,
    pub open_menu: Option<ClauseSlot>,
    pub open_group: Option<String>,
    pub result: Option<LastResult>,
    dirty: bool,
    pub snapshot: Snapshot,
}

impl Default for DiplomacyUi {
    fn default() -> Self {
        Self {
            open: false,
            selected: None,
            filter: 0,
            sort: 0,
            query: String::new(),
            tab: "dip:tab:negotiation".to_owned(),
            draft: Vec::new(),
            draft_for: None,
            open_menu: None,
            open_group: None,
            result: None,
            dirty: false,
            snapshot: Snapshot::default(),
        }
    }
}

impl DiplomacyUi {
    fn select(&mut self, id: &str) {
        if self.selected.as_deref() != Some(id) {
            self.selected = Some(id.to_owned());
            self.draft.clear();
            self.draft_for = None;
            self.open_menu = None;
            self.open_group = None;
            self.result = None;
        }
    }
}

fn close_on_exit(mut ui: ResMut<DiplomacyUi>) {
    ui.open = false;
    ui.dirty = true;
}

/// `OpenPanel`, F3 (testing) and Escape.
fn open_close(
    mut requests: MessageReader<OpenPanel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<DiplomacyUi>,
    mut held: Local<(bool, bool)>,
) {
    // Edge detection on `pressed`: the agent script presses keys in `Last`, so `just_pressed`
    // is already cleared when `Update` runs.
    let f3 = keys.pressed(KeyCode::F3);
    let escape = keys.pressed(KeyCode::Escape);
    let f3_edge = f3 && !held.0;
    let escape_edge = escape && !held.1;
    *held = (f3, escape);
    let mut toggle = f3_edge;
    for request in requests.read() {
        match request {
            OpenPanel::Diplomacy => toggle = true,
            OpenPanel::Tech | OpenPanel::CloseAll if ui.open => {
                ui.open = false;
                ui.dirty = true;
            }
            _ => {}
        }
    }
    if toggle {
        ui.open = !ui.open;
        ui.dirty = true;
        if ui.open {
            ui.result = None;
        }
    } else if ui.open && escape_edge {
        ui.open = false;
        ui.dirty = true;
    }
}

/// Typing edits the search field of the faction list (the kit has no text input).
fn type_query(mut keys: MessageReader<KeyboardInput>, mut ui: ResMut<DiplomacyUi>) {
    if !ui.open {
        keys.clear();
        return;
    }
    let before = ui.query.clone();
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Character(chars) if chars.chars().all(|c| !c.is_control()) => {
                if ui.query.chars().count() < 40 {
                    ui.query.push_str(chars);
                }
            }
            Key::Space if !ui.query.is_empty() && ui.query.chars().count() < 40 => {
                ui.query.push(' ');
            }
            Key::Backspace => {
                ui.query.pop();
            }
            _ => {}
        }
    }
    if ui.query != before {
        ui.dirty = true;
    }
}

/// Handles the buttons of the screen (`dip:*` names) and the tab messages.
fn handle_presses(
    mut pressed: MessageReader<ButtonPressed>,
    mut tabs: MessageReader<TabSelected>,
    mut ui: ResMut<DiplomacyUi>,
    mut sim: ResMut<Sim>,
) {
    for tab in tabs.read() {
        if tab.group == "dip_tab" {
            ui.tab = tab.name.clone();
        }
    }
    if !ui.open {
        return;
    }
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("dip:") else {
            continue;
        };
        press_button(name, &mut ui, &mut sim);
    }
}

fn press_button(name: &str, ui: &mut DiplomacyUi, sim: &mut Sim) {
    let selected = ui.selected.clone();
    match name {
        "close" => ui.open = false,
        "filter" => ui.filter = (ui.filter + 1) % format::FILTERS.len(),
        "sort" => ui.sort = (ui.sort + 1) % format::SORTS.len(),
        "clear_search" => ui.query.clear(),
        "clear" => {
            ui.draft.clear();
            ui.result = None;
        }
        "adopt_counter" => {
            if let Some(verdict) = ui
                .snapshot
                .verdict
                .as_ref()
                .filter(|v| !v.counter.is_empty())
            {
                ui.draft = verdict.counter.clone();
            }
        }
        "ask_counter" => ask(ui, sim),
        "send" => send_treaty(ui, sim),
        _ => {
            if let Some(id) = name.strip_prefix("faction:").or(name.strip_prefix("link:")) {
                ui.select(id);
            } else if let Some(slot) = name.strip_prefix("menu:") {
                let slot = match slot {
                    "common" => ClauseSlot::Common,
                    "offer" => ClauseSlot::Offer,
                    _ => ClauseSlot::Demand,
                };
                ui.open_menu = if ui.open_menu == Some(slot) {
                    None
                } else {
                    Some(slot)
                };
                ui.open_group = None;
            } else if let Some(group) = name.strip_prefix("group:") {
                let group = group.to_owned();
                ui.open_group = if ui.open_group.as_ref() == Some(&group) {
                    None
                } else {
                    Some(group)
                };
            } else if name.starts_with("clause:") {
                add_clause(ui, name);
            } else if let Some(rest) = name.strip_prefix("gold:") {
                add_amount(ui, rest, false);
            } else if let Some(rest) = name.strip_prefix("tribute:") {
                add_amount(ui, rest, true);
            } else if let Some(index) = name.strip_prefix("remove:") {
                if let Ok(index) = index.parse::<usize>()
                    && index < ui.draft.len()
                {
                    ui.draft.remove(index);
                }
            } else if let Some(key) = name.strip_prefix("action:") {
                run_action(ui, sim, key, selected.as_deref());
            } else if let Some(rest) = name.strip_prefix("offer:") {
                answer(ui, sim, rest);
            }
        }
    }
    ui.dirty = true;
}

fn add_clause(ui: &mut DiplomacyUi, name: &str) {
    let Some(detail) = &ui.snapshot.detail else {
        return;
    };
    let full = format!("dip:{name}");
    let Some(index) = ui.snapshot.clause_names.iter().position(|n| *n == full) else {
        return;
    };
    let article = detail.options.clauses[index].article.clone();
    push_article(&mut ui.draft, article);
}

/// Adds an article unless already present; one gold payment and one tribute per giver.
fn push_article(draft: &mut Vec<Article>, article: Article) {
    if draft.contains(&article) {
        return;
    }
    draft.retain(|old| match (old, &article) {
        (Article::Gold { giver: a, .. }, Article::Gold { giver: b, .. })
        | (Article::Tribute { giver: a, .. }, Article::Tribute { giver: b, .. }) => a != b,
        _ => true,
    });
    draft.push(article);
}

/// `rest` = « offer:1000 » / « demand:500 ».
fn add_amount(ui: &mut DiplomacyUi, rest: &str, tribute: bool) {
    let Some((side, amount)) = rest.split_once(':') else {
        return;
    };
    let (Ok(amount), giver) = (
        amount.parse::<i64>(),
        if side == "offer" {
            Party::Proposer
        } else {
            Party::Recipient
        },
    ) else {
        return;
    };
    let article = if tribute {
        Article::Tribute {
            giver,
            per_season: amount,
            seasons: TRIBUTE_SEASONS,
        }
    } else {
        // Gold is bounded by the treasury of the giver.
        let treasury = ui.snapshot.detail.as_ref().map_or(0, |d| {
            if giver == Party::Proposer {
                d.options.ours.treasury
            } else {
                d.options.theirs.treasury
            }
        });
        let amount = amount.min(treasury.max(0));
        if amount <= 0 {
            return;
        }
        Article::Gold { giver, amount }
    };
    push_article(&mut ui.draft, article);
}

fn ask(ui: &mut DiplomacyUi, sim: &Sim) {
    let (Some(data), Some(state), Some(target)) = (&sim.data, &sim.state, ui.selected.clone())
    else {
        return;
    };
    let Ok(target) = data_model::FactionId::new(target) else {
        return;
    };
    match ask_counter(data, state, &sim.player, &target, &ui.draft) {
        Ok(counter) => {
            ui.draft = counter;
            ui.result = None;
        }
        Err(message) => {
            ui.result = Some(LastResult {
                ok: false,
                text: message,
            });
        }
    }
}

fn send_treaty(ui: &mut DiplomacyUi, sim: &mut Sim) {
    let Some(target) = ui.selected.clone() else {
        return;
    };
    if ui.draft.is_empty() {
        return;
    }
    let Ok(target) = data_model::FactionId::new(target) else {
        return;
    };
    let order = sim_campaign::Order::ProposeTreaty {
        target,
        articles: ui.draft.clone(),
    };
    let result = run_order(sim, order);
    if result.ok {
        ui.draft.clear();
    }
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            "Le traité est signé.".to_owned()
        } else {
            result.message
        },
    });
}

fn run_action(ui: &mut DiplomacyUi, sim: &mut Sim, key: &str, selected: Option<&str>) {
    let Some(detail) = &ui.snapshot.detail else {
        return;
    };
    if selected != Some(detail.row.id.as_str()) {
        return;
    }
    let Some(action) = detail.actions.iter().find(|a| a.key == key).cloned() else {
        return;
    };
    let result = run_order(sim, action.order);
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            action.success_text
        } else {
            result.message
        },
    });
}

/// `rest` = « 12:accept » / « 12:refuse ».
fn answer(ui: &mut DiplomacyUi, sim: &mut Sim, rest: &str) {
    let Some((id, verb)) = rest.split_once(':') else {
        return;
    };
    let Ok(id) = id.parse::<u32>() else {
        return;
    };
    let accept = verb == "accept";
    let result = run_order(sim, sim_campaign::Order::AnswerOffer { offer: id, accept });
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            if accept {
                "Proposition acceptée.".to_owned()
            } else {
                "Proposition refusée.".to_owned()
            }
        } else {
            result.message
        },
    });
}

fn run_order(sim: &mut Sim, order: sim_campaign::Order) -> game_api::diplomacy::OrderResult {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return game_api::diplomacy::OrderResult {
            ok: false,
            message: "aucune campagne en cours".to_owned(),
        };
    };
    submit_order(&data, state, order)
}

/// Removes from the draft the clauses already in force (`_prune_in_force`).
fn prune_in_force(draft: &mut Vec<Article>, detail: &RelationDetail) {
    draft.retain(|article| match article {
        Article::TradeAgreement => !detail.options.trade,
        Article::NonAggression { .. } => !detail.options.non_aggression,
        Article::MilitaryAccess { giver } => {
            !(if *giver == Party::Proposer {
                detail.options.access_given
            } else {
                detail.options.access_received
            })
        }
        _ => true,
    });
}

/// Recomputes the snapshot from the sim (called by the view just before it rebuilds).
pub(crate) fn refresh_snapshot(ui: &mut DiplomacyUi, sim: &Sim) {
    ui.snapshot = Snapshot::default();
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    ui.snapshot.rows = faction_list(data, state, &sim.player);
    ui.snapshot.offers = offers(data, state, &sim.player);
    ui.snapshot.court = court_banner(data, state, &ui.snapshot.rows, &ui.snapshot.offers);
    let valid = |id: &str| ui.snapshot.rows.iter().any(|r| r.id == id);
    if ui.selected.as_deref().is_none_or(|id| !valid(id)) {
        let first = format::visible_rows(&ui.snapshot.rows, 0, 0, "")
            .first()
            .map(|r| r.id.clone());
        ui.selected = first;
        ui.draft.clear();
        ui.draft_for = None;
    }
    let Some(selected) = ui.selected.clone() else {
        return;
    };
    let Ok(target) = data_model::FactionId::new(selected.clone()) else {
        return;
    };
    ui.snapshot.detail = relation_detail(data, state, &sim.player, &target);
    let Some(detail) = ui.snapshot.detail.clone() else {
        return;
    };
    if ui.draft_for.as_deref() != Some(selected.as_str()) {
        ui.draft_for = Some(selected);
        ui.draft = if detail.row.status == "war" {
            vec![Article::Peace]
        } else {
            Vec::new()
        };
    }
    prune_in_force(&mut ui.draft, &detail);
    ui.snapshot.verdict = evaluate_draft(data, state, &sim.player, &target, &ui.draft);
    ui.snapshot.clause_names = clause_names(&detail);
}

/// Unique, script-friendly widget names of the menu clauses: `dip:clause:<slot>:<key>` for a
/// top-level entry, `dip:clause:<slot>:<key>:<n>` inside a submenu (n-th of that submenu).
fn clause_names(detail: &RelationDetail) -> Vec<String> {
    let mut counts: std::collections::HashMap<(String, String), usize> = Default::default();
    detail
        .options
        .clauses
        .iter()
        .map(|c| {
            let slot = match c.slot {
                ClauseSlot::Common => "common",
                ClauseSlot::Offer => "offer",
                ClauseSlot::Demand => "demand",
            };
            if c.group.is_empty() {
                format!("dip:clause:{slot}:{}", c.article.key())
            } else {
                let n = counts
                    .entry((slot.to_owned(), c.group.clone()))
                    .or_insert(0);
                *n += 1;
                format!("dip:clause:{slot}:{}:{}", c.article.key(), n)
            }
        })
        .collect()
}

fn dump_diplomacy(world: &World) -> (String, Value) {
    let ui = world.resource::<DiplomacyUi>();
    let snap = &ui.snapshot;
    let detail = snap.detail.as_ref().map(|d| {
        json!({
            "faction": d.row.id,
            "name": d.row.name,
            "ruler": d.row.ruler,
            "status": d.row.status,
            "stance": d.row.stance_label,
            "attitude": d.row.attitude,
            "attitude_band": d.row.attitude_band,
            "opinion_reasons": d.row.reasons,
            "allies": d.row.allies.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "enemies": d.row.enemies.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "truce_turns_left": d.row.truce_turns_left,
            "casus_belli": d.row.casus_belli,
            "war": d.war,
            "history": d.history.len(),
            "clauses_available": d.options.clauses.len(),
            "actions": d.actions.iter().map(|a| json!({
                "key": a.key, "label": a.label,
                "preview": a.preview.as_ref().map(|p| json!({
                    "accept": p.accept, "score": p.score,
                    "reasons": p.reasons.iter().map(|r| format!("{} {}", r.value, r.text)).collect::<Vec<_>>(),
                    "allies": p.allies.iter().map(|a| format!("{}: {}", a.id, a.answer)).collect::<Vec<_>>(),
                })),
            })).collect::<Vec<_>>(),
        })
    });
    let verdict = snap.verdict.as_ref().map(|v| {
        json!({
            "accept": v.accept, "score": v.score, "chance": v.chance,
            "summary": v.summary, "blocked": v.blocked,
            "clauses": v.clause_labels,
            "clause_values": v.clause_values,
            "lines": v.lines.iter().take(8).map(|l| format!("{} {}", l.value, l.text)).collect::<Vec<_>>(),
            "counter_text": v.counter_text,
            "counter_clauses": v.counter.len(),
        })
    });
    (
        "diplomacy".to_owned(),
        json!({
            "open": ui.open,
            "selected": ui.selected,
            "tab": ui.tab,
            "filter": format::FILTERS[ui.filter],
            "sort": format::SORTS[ui.sort],
            "rows": snap.rows.len(),
            "query": ui.query,
            "court": snap.court,
            "visible": format::visible_rows(&snap.rows, ui.filter, ui.sort, &ui.query).len(),
            "offers": snap.offers.len(),
            "draft": ui.draft.iter().map(|a| a.key()).collect::<Vec<_>>(),
            "detail": detail,
            "verdict": verdict,
            "last_result": ui.result.as_ref().map(|r| json!({"ok": r.ok, "text": r.text})),
        }),
    )
}
