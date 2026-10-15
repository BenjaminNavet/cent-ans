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
//! Opens on `OpenPanel::Diplomacy` (toggle) or the `map_toggle_diplomacy` key (F3), closes on
//! Escape, the close button, or an `OpenPanel::Tech` / `CloseAll`. The chancery line and the
//! donation to the Church sit in the header, feudal arbitration calls offer the choice of the
//! side to take, the attitude arrows compare with the previous turn, and gold / tribute are typed
//! in numeric fields (click, digits, Enter). While open the map is put in Diplomacy mode seen
//! from the chosen faction (`SetMapMode` + `MapModeState::focus_faction`; the previous mode comes
//! back on close). Not ported: the diplomatic map in the centre (`diplomacy_map_view.gd`; the
//! panel covers the campaign map, which is coloured instead); the search field is typed on the
//! keyboard (no kit text input).

mod format;
pub mod notify;
pub mod stance_badge;
mod view;
pub mod war;

pub use war::AskWarDeclaration;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::collections::HashMap;

use game_api::diplomacy::{
    ClauseSlot, FactionRow, OfferView, RelationDetail, ReligionView, TreatyVerdict, ask_counter,
    chancery_text, court_banner, evaluate_draft, faction_list, offers, relation_detail,
    religion_view, submit_order,
};
use game_api::feudal::FeudalCall;
use serde_json::{Value, json};
use sim_campaign::negotiation::{Article, Party};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::provinces::{MapMode, MapModeState, SetMapMode};
use crate::menu::KeyEdges;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::{ButtonPressed, TabSelected};

pub struct PanelDiplomacyPlugin;

impl Plugin for PanelDiplomacyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            war::WarDialogPlugin,
            stance_badge::StanceBadgePlugin,
            notify::NotifyPlugin,
        ))
        .init_resource::<DiplomacyUi>()
        .register_dump(dump_diplomacy)
        .add_systems(
            Update,
            (
                shortcut,
                open_close.before(crate::menu::pause::pause_escape),
                type_query,
                handle_presses,
                sync_map_mode,
                view::rebuild,
            )
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(
            Update,
            pick_lord
                .before(crate::campaign::provinces::ProvincePickSet)
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
    /// Chancery line (faith, papal favour, interdict, crusade) and donation.
    pub religion: Option<ReligionView>,
    /// Feudal calls waiting (arbitration offers carry the two sides).
    pub calls: Vec<FeudalCall>,
    /// Attitude trend arrow of each faction (`""` without a reference).
    pub trends: HashMap<String, &'static str>,
    /// « Qui peut entrer en guerre » chain if the player declared war on the selected faction.
    pub escalation: Vec<game_api::diplomacy::EscalationView>,
}

/// Upper bound of a typed tribute per season (`TRIBUTE_MAX`).
pub const TRIBUTE_MAX: i64 = 20_000;

/// The numeric field being typed in: gold or tribute, on the offer or the demand side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmountField {
    pub tribute: bool,
    pub offer: bool,
}

impl AmountField {
    pub fn name(self) -> String {
        format!(
            "{}:{}",
            if self.tribute { "tribute" } else { "gold" },
            if self.offer { "offer" } else { "demand" }
        )
    }
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
    /// Focused numeric field and its digits.
    pub field: Option<AmountField>,
    pub field_text: String,
    /// Attitudes seen this turn / at the previous turn (trend arrows, `_track_turn`).
    turn: Option<u32>,
    seen: HashMap<String, i32>,
    previous: HashMap<String, i32>,
    /// Faction a confirmation of war was asked for (sent as a message by `handle_presses`).
    ask_war: Option<String>,
    /// Lords selected by a click on the map (dump).
    pub picked_by_map: u32,
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
            field: None,
            field_text: String::new(),
            turn: None,
            seen: HashMap::new(),
            previous: HashMap::new(),
            ask_war: None,
            picked_by_map: 0,
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
            self.field = None;
        }
    }

    /// New turn: the attitudes seen at the previous one become the reference of the arrows.
    fn track_turn(&mut self, turn: u32) {
        if self.turn != Some(turn) {
            self.previous = match self.turn {
                Some(old) if turn > old => std::mem::take(&mut self.seen),
                _ => HashMap::new(),
            };
            self.seen.clear();
            self.turn = Some(turn);
        }
        for row in &self.snapshot.rows {
            self.seen.insert(row.id.clone(), row.attitude);
        }
        self.snapshot.trends = self
            .snapshot
            .rows
            .iter()
            .filter_map(|row| {
                let before = *self.previous.get(&row.id)?;
                Some((row.id.clone(), format::trend(before, row.attitude)))
            })
            .collect();
    }
}

fn close_on_exit(mut ui: ResMut<DiplomacyUi>) {
    ui.open = false;
    ui.dirty = true;
}

/// Key while closed: asks for the panel through the message bus so the other central panels
/// close (`displaces`); the `OpenPanel::Diplomacy` request in `open_close` opens this one.
fn shortcut(
    ui: Res<DiplomacyUi>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<KeyMap>,
    mut outgoing: MessageWriter<OpenPanel>,
) {
    if !ui.open && keymap.just_pressed("map_toggle_diplomacy", &keys) {
        outgoing.write(OpenPanel::Diplomacy);
    }
}

/// `OpenPanel`, F3 (testing) and Escape.
fn open_close(
    mut requests: MessageReader<OpenPanel>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<KeyMap>,
    mut edges: ResMut<KeyEdges>,
    mut ui: ResMut<DiplomacyUi>,
) {
    // Opening by key goes through [`shortcut`] (message bus); only the close is handled here.
    let mut toggle = ui.open && keymap.just_pressed("map_toggle_diplomacy", &keys);
    for request in requests.read() {
        match request {
            OpenPanel::Diplomacy => toggle = true,
            other if ui.open && displaces(other, "diplomacy") => {
                ui.open = false;
                ui.dirty = true;
            }
            _ => {}
        }
    }
    if toggle {
        ui.open = !ui.open;
        ui.dirty = true;
        ui.field = None;
        if ui.open {
            ui.result = None;
        }
    } else if ui.open && edges.escape {
        edges.escape = false;
        if ui.field.take().is_none() {
            ui.open = false;
        }
        ui.dirty = true;
    }
}

/// Central relations map (`diplomacy_map_view.gd`, DZ): the campaign map itself shows the
/// Diplomacy mode while the screen is open ([`sync_map_mode`]); a click on a province selects the
/// lord who holds it (the province panel stays shut, the click is swallowed). Own lands and the
/// sea do nothing.
fn pick_lord(
    mouse: Res<ButtonInput<MouseButton>>,
    hovered: Res<crate::campaign::provinces::HoveredProvince>,
    mut guard: ResMut<crate::campaign::provinces::ProvinceClickGuard>,
    sim: Res<Sim>,
    mut ui: ResMut<DiplomacyUi>,
) {
    if !ui.open || !mouse.just_released(MouseButton::Left) || guard.blocked {
        return;
    }
    let Some(id) = hovered.id.as_deref() else {
        return;
    };
    let (Some(state), Ok(province)) = (&sim.state, data_model::ProvinceId::new(id.to_owned()))
    else {
        return;
    };
    guard.blocked = true;
    let Some(lord) = state.province_controller(&province).cloned() else {
        return;
    };
    if lord != sim.player && state.factions.contains_key(&lord) {
        ui.select(lord.as_str());
        ui.picked_by_map += 1;
        ui.dirty = true;
    }
}

/// While the screen is open the campaign map shows the Diplomacy mode from the chosen faction;
/// the mode in force before comes back on close.
fn sync_map_mode(
    ui: Res<DiplomacyUi>,
    mut modes: ResMut<MapModeState>,
    mut requests: MessageWriter<SetMapMode>,
    mut before: Local<Option<MapMode>>,
    sim: Res<Sim>,
) {
    if ui.open {
        if before.is_none() {
            *before = Some(modes.mode);
            requests.write(SetMapMode(MapMode::Diplomacy));
        }
        let focus = ui
            .selected
            .clone()
            .filter(|id| id.as_str() != sim.player.as_str());
        if modes.focus_faction != focus {
            modes.focus_faction = focus;
        }
    } else if let Some(mode) = before.take() {
        requests.write(SetMapMode(mode));
    }
}

fn type_query(mut keys: MessageReader<KeyboardInput>, mut ui: ResMut<DiplomacyUi>) {
    if !ui.open {
        keys.clear();
        return;
    }
    let before = ui.query.clone();
    let mut touched = false;
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        if ui.field.is_some() {
            // A numeric field has the focus: digits go to it, Enter adds the clause.
            match &key.logical_key {
                Key::Character(chars) => {
                    for c in chars.chars().filter(char::is_ascii_digit) {
                        if ui.field_text.len() < 7 {
                            ui.field_text.push(c);
                        }
                    }
                }
                Key::Backspace => {
                    ui.field_text.pop();
                }
                Key::Enter => commit_field(&mut ui),
                Key::Unidentified(_) => {
                    if let Some(digit) = key_code_digit(key.key_code)
                        && ui.field_text.len() < 7
                    {
                        ui.field_text.push(digit);
                    } else if key.key_code == KeyCode::Enter {
                        commit_field(&mut ui);
                    }
                }
                _ => {}
            }
            touched = true;
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
            // Agent scripts send key codes without a logical key: letters still type.
            Key::Unidentified(_) => {
                if let Some(letter) = key_code_letter(key.key_code)
                    && ui.query.chars().count() < 40
                {
                    ui.query.push(letter);
                }
            }
            _ => {}
        }
    }
    if ui.query != before || touched {
        ui.dirty = true;
    }
}

/// Adds the typed amount as a gold / tribute clause and leaves the field.
fn commit_field(ui: &mut DiplomacyUi) {
    let Some(field) = ui.field else {
        return;
    };
    let value = ui.field_text.parse::<i64>().unwrap_or(0);
    add_amount_value(ui, field.tribute, field.offer, value);
    ui.field_text.clear();
}

fn key_code_digit(code: KeyCode) -> Option<char> {
    let name = format!("{code:?}");
    let digit = name
        .strip_prefix("Digit")
        .or_else(|| name.strip_prefix("Numpad"))?;
    let mut chars = digit.chars();
    let c = chars.next()?;
    (chars.next().is_none() && c.is_ascii_digit()).then_some(c)
}

fn key_code_letter(code: KeyCode) -> Option<char> {
    let name = format!("{code:?}");
    let letter = name.strip_prefix("Key")?;
    let mut chars = letter.chars();
    let c = chars.next()?;
    chars.next().is_none().then(|| c.to_ascii_lowercase())
}

/// Handles the buttons of the screen (`dip:*` names) and the tab messages.
fn handle_presses(
    mut pressed: MessageReader<ButtonPressed>,
    mut tabs: MessageReader<TabSelected>,
    mut ui: ResMut<DiplomacyUi>,
    mut sim: ResMut<Sim>,
    mut wars: MessageWriter<AskWarDeclaration>,
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
        if let Some(target) = ui.ask_war.take() {
            wars.write(AskWarDeclaration {
                target,
                target_label: String::new(),
                then: None,
            });
        }
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
        "donate" => donate(ui, sim),
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
            } else if let Some(rest) = name.strip_prefix("field:") {
                let field = AmountField {
                    tribute: rest.starts_with("tribute"),
                    offer: rest.ends_with("offer"),
                };
                ui.field = if ui.field == Some(field) {
                    None
                } else {
                    Some(field)
                };
                ui.field_text.clear();
            } else if let Some(rest) = name.strip_prefix("addamt:") {
                let (tribute, offer) = (rest.starts_with("tribute"), rest.ends_with("offer"));
                let field = AmountField { tribute, offer };
                if ui.field != Some(field) {
                    ui.field = Some(field);
                }
                commit_field(ui);
            } else if let Some(rest) = name.strip_prefix("arbitrate:") {
                arbitrate(ui, sim, rest);
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
    if let Ok(amount) = amount.parse::<i64>() {
        add_amount_value(ui, tribute, side == "offer", amount);
    }
}

/// Gold is bounded by the treasury of the giver, tribute by `TRIBUTE_MAX`; nothing if null.
fn add_amount_value(ui: &mut DiplomacyUi, tribute: bool, offer: bool, amount: i64) {
    let giver = if offer {
        Party::Proposer
    } else {
        Party::Recipient
    };
    let treasury = ui.snapshot.detail.as_ref().map_or(0, |d| {
        if offer {
            d.options.ours.treasury
        } else {
            d.options.theirs.treasury
        }
    });
    let amount = format::clamp_amount(amount, if tribute { TRIBUTE_MAX } else { treasury });
    if amount <= 0 {
        return;
    }
    let article = if tribute {
        Article::Tribute {
            giver,
            per_season: amount,
            seasons: TRIBUTE_SEASONS,
        }
    } else {
        Article::Gold { giver, amount }
    };
    push_article(&mut ui.draft, article);
}

/// Gift to the Church (`donate_to_church`).
fn donate(ui: &mut DiplomacyUi, sim: &mut Sim) {
    let result = run_order(
        sim,
        sim_campaign::Order::DonateToChurch {
            amount: game_api::diplomacy::DONATION_AMOUNT,
        },
    );
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            "Don versé à l’Église.".to_owned()
        } else {
            result.message
        },
    });
}

/// `rest` = « <offer id>:<side faction> »: takes the side of one vassal in a private war.
fn arbitrate(ui: &mut DiplomacyUi, sim: &mut Sim, rest: &str) {
    let Some((id, side)) = rest.split_once(':') else {
        return;
    };
    let Ok(id) = id.parse::<u32>() else {
        return;
    };
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return;
    };
    let result = game_api::feudal::arbitrate(&data, state, id, "take_side", side);
    ui.result = Some(LastResult {
        ok: result.ok,
        text: if result.ok {
            "Arbitrage rendu.".to_owned()
        } else {
            result.message
        },
    });
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
    if matches!(action.order, sim_campaign::Order::DeclareWar { .. }) {
        // Confirmed in the war declaration dialog (consequences + escalation chain).
        ui.ask_war = Some(detail.row.id.clone());
        return;
    }
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
    ui.snapshot.religion = religion_view(data, state, &sim.player);
    ui.snapshot.calls = game_api::feudal::calls(data, state);
    ui.track_turn(state.turn());
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
    if detail.actions.iter().any(|a| a.key == "declare_war") {
        ui.snapshot.escalation =
            game_api::diplomacy::war_escalation(data, state, &sim.player, &target);
    }
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
            "picked_by_map": ui.picked_by_map,
            "tab": ui.tab,
            "filter": format::FILTERS[ui.filter],
            "sort": format::SORTS[ui.sort],
            "rows": snap.rows.len(),
            "query": ui.query,
            "court": snap.court,
            "visible": format::visible_rows(&snap.rows, ui.filter, ui.sort, &ui.query).len(),
            "offers": snap.offers.len(),
            "chancery": snap.religion.as_ref().map(chancery_text),
            "can_donate": snap.religion.as_ref().map(|r| r.can_donate),
            "arbitration_calls": snap.calls.iter().filter(|c| c.kind == "arbitration").map(|c| c.id).collect::<Vec<_>>(),
            "trends": trend_lines(&snap.trends),
            "field": ui.field.map(|f| json!({"name": f.name(), "text": ui.field_text})),
            "draft": ui.draft.iter().map(|a| a.key()).collect::<Vec<_>>(),
            "detail": detail,
            "verdict": verdict,
            "escalation": snap.escalation,
            "last_result": ui.result.as_ref().map(|r| json!({"ok": r.ok, "text": r.text})),
        }),
    )
}

/// Up to 12 `<faction><arrow>` entries, sorted, for the dump.
fn trend_lines(trends: &HashMap<String, &'static str>) -> Vec<String> {
    let mut lines: Vec<String> = trends.iter().map(|(k, v)| format!("{k}{v}")).collect();
    lines.sort();
    lines.truncate(12);
    lines
}
