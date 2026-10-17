//! Court panels: character sheet (`OpenPanel::Character`) and council chamber
//! (`OpenPanel::Council`).
//!
//! Ports `game/scripts/ui/character_sheet.gd` (portrait + stats on the left; tabs Fiche /
//! Compétences (skill tree by branch and tier) / Famille (family tree by generation) / Ordres),
//! `court_panel.gd` (court list), `council_chamber.gd` (offices, candidates, appoint / dismiss)
//! and the royal acts, captain and chivalric-order screens. Data comes from `game_api::court`;
//! every action is an order submitted through it (`learn_skill`, `assign_governor`,
//! `assign_general`, `marry`, `designate_heir`, `transfer_companion`, `appoint_councillor`,
//! `perform_royal_act`, `hire_captain`, `found_order`, `pay_ransom`...), as the Godot UI did
//! through `CampaignSim.submit_order`.
//!
//! Also ported: living portraits (`portrait.rs`), family tree with ink links (`family_layout.rs`),
//! skill tree with prerequisite links (`skill_layout.rs`), retinue medallions, ransom screen
//! (installments, parole, terms, execution) and the advice lines of the councillors.
//!
//! The 3D council room (`council_room.gd`) lives in `campaign/council/`: its image is inserted
//! at the top of the « Charges » tab (`view.rs`), hover / click on a councillor select the office
//! of this panel.
//!
//! The next-step hint (`NextHint`) joins the councillors' advice (`hint_alert`, a click is relayed
//! to the HUD card); `portrait_marks.wesl` (`PortraitMarksMaterial`) greys the dead, pales the
//! sick, shades the captive and mirrors the picture; the family tree zooms (`+`/`−`/Ctrl + wheel,
//! 45 % to 160 %) and scrolls by dragging (`tree_input`).

mod family_layout;
pub mod portrait;
mod skill_layout;
mod view;

pub use view::sheet_subject;

use bevy::prelude::*;
use game_api::court::{
    CandidateView, CaptainInfo, CharacterSheet, Chivalry, CourtRow, FamilyTree, OfficeView,
    OrderResult, RoyalActView, SkillNode,
};
use serde_json::{Value, json};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::{ButtonPressed, TabSelected};

pub struct PanelCourtPlugin;

impl Plugin for PanelCourtPlugin {
    fn build(&self, app: &mut App) {
        let bank = crate::paths::find_root()
            .map(|root| portrait::PortraitBank::load(&root))
            .unwrap_or_default();
        app.init_resource::<CourtUi>()
            .add_plugins(portrait::PortraitMarksPlugin)
            .insert_resource(bank)
            .register_dump(dump_court)
            .add_systems(
                Update,
                (
                    dev_open,
                    shortcuts,
                    open_close.before(crate::menu::pause::pause_escape),
                    handle_presses,
                    forward_hint,
                    tree_input,
                    view::rebuild,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit);
    }
}

/// Which screen is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    None,
    Sheet,
    Council,
}

pub const SHEET_TABS: [(&str, &str); 4] = [
    ("court:tab:sheet", "Fiche"),
    ("court:tab:skills", "Compétences"),
    ("court:tab:family", "Famille"),
    ("court:tab:orders", "Ordres"),
];
pub const COUNCIL_TABS: [(&str, &str); 4] = [
    ("court:tab:council", "Conseil"),
    ("court:tab:court", "Cour"),
    ("court:tab:acts", "Actes royaux"),
    ("court:tab:chivalry", "Chevalerie"),
];

/// Sub-list of the « Ordres » tab opened by a menu button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderMenu {
    Marriage,
    Governor,
    General,
    Transfer,
}

/// What the screen shows, recomputed from the sim each time the UI is rebuilt.
#[derive(Default)]
pub struct Snapshot {
    pub sheet: Option<CharacterSheet>,
    pub skills: Vec<SkillNode>,
    pub family: Option<FamilyTree>,
    pub offices: Vec<OfficeView>,
    pub candidates: Vec<CandidateView>,
    pub court: Vec<CourtRow>,
    pub acts: Vec<RoyalActView>,
    pub captain: Option<CaptainInfo>,
    pub chivalry: Option<Chivalry>,
    /// Treasury of the player's faction (livres), shown next to the costs of the acts.
    pub treasury: i64,
    /// Alerts of the player's faction, the raw material of the councillors' advice.
    pub alerts: Vec<game_api::turn::Alert>,
    /// Sheet of the holder of the selected office (portrait).
    pub holder: Option<CharacterSheet>,
}

/// Zoom bounds of the family tree (`family_tree_view.gd`).
pub const ZOOM_MIN: f32 = 0.45;
pub const ZOOM_MAX: f32 = 1.6;

#[derive(Resource)]
pub struct CourtUi {
    pub screen: Screen,
    pub character: Option<String>,
    /// Characters visited before the current one (« Retour »).
    pub history: Vec<String>,
    pub sheet_tab: String,
    pub council_tab: String,
    pub office: Option<String>,
    pub menu: Option<OrderMenu>,
    /// Companion chosen for a transfer.
    pub companion: Option<String>,
    pub result: Option<OrderResult>,
    /// The « Exécuter » button was pressed once and waits for its confirmation.
    pub armed_execution: bool,
    pub(crate) dirty: bool,
    /// A hint advice was clicked: [`forward_hint`] relays it to the HUD card.
    hint_go: bool,
    /// Hint advices activated (dump).
    pub hint_activations: u32,
    /// Zoom of the family tree and its scroll offset (kept across rebuilds).
    pub tree_zoom: f32,
    pub tree_scroll: Vec2,
    pub snapshot: Snapshot,
}

impl Default for CourtUi {
    fn default() -> Self {
        Self {
            screen: Screen::None,
            character: None,
            history: Vec::new(),
            sheet_tab: SHEET_TABS[0].0.to_owned(),
            council_tab: COUNCIL_TABS[0].0.to_owned(),
            office: None,
            menu: None,
            companion: None,
            result: None,
            armed_execution: false,
            dirty: false,
            hint_go: false,
            hint_activations: 0,
            tree_zoom: 1.0,
            tree_scroll: Vec2::ZERO,
            snapshot: Snapshot::default(),
        }
    }
}

impl CourtUi {
    /// Sets the family-tree zoom (clamped); the scroll offset follows the scale.
    pub fn set_tree_zoom(&mut self, value: f32) {
        let zoom = value.clamp(ZOOM_MIN, ZOOM_MAX);
        if (zoom - self.tree_zoom).abs() > f32::EPSILON {
            self.tree_scroll *= zoom / self.tree_zoom;
            self.tree_zoom = zoom;
            self.dirty = true;
        }
    }

    /// Opens the sheet of `id`; `push` keeps the previous one for « Retour ».
    pub fn open_character(&mut self, id: &str, push: bool) {
        if push
            && self.screen == Screen::Sheet
            && let Some(previous) = self.character.clone()
            && previous != id
        {
            self.history.push(previous);
        }
        if !push {
            self.history.clear();
        }
        self.screen = Screen::Sheet;
        self.character = Some(id.to_owned());
        self.menu = None;
        self.companion = None;
        self.result = None;
        self.armed_execution = false;
        self.dirty = true;
    }

    pub fn open_council(&mut self) {
        self.screen = Screen::Council;
        self.result = None;
        self.dirty = true;
    }

    pub fn close(&mut self) {
        self.screen = Screen::None;
        self.history.clear();
        self.menu = None;
        self.dirty = true;
    }
}

/// Dev hook for agent scripts: `CENT_ANS_OPEN_COURT=character:<id>[@tab]|character:ruler[@tab]|council`
/// opens the screen once the campaign runs.
fn dev_open(mut done: Local<bool>, mut frames: Local<u32>, sim: Res<Sim>, mut ui: ResMut<CourtUi>) {
    if *done {
        return;
    }
    // After the first frames: the campaign start sends a `CloseAll` that would shut the screen.
    *frames += 1;
    if *frames < 30 {
        return;
    }
    *done = true;
    let Ok(request) = std::env::var("CENT_ANS_OPEN_COURT") else {
        return;
    };
    if request == "council" {
        ui.open_council();
    } else if let Some(id) = request.strip_prefix("character:") {
        // `character:<id>[@<tab>]`, tab = sheet | skills | family | orders.
        let (id, tab) = id.split_once('@').map_or((id, None), |(i, t)| (i, Some(t)));
        if let Some(tab) = tab {
            ui.sheet_tab = format!("court:tab:{tab}");
        }
        let id = if id == "ruler" {
            sim.state
                .as_ref()
                .and_then(|s| s.factions.get(&sim.player))
                .and_then(|f| f.ruler.as_ref())
                .map_or_else(String::new, ToString::to_string)
        } else {
            id.to_owned()
        };
        ui.open_character(&id, false);
    }
}

/// J (`map_toggle_council`) toggles the council hall; C (`map_toggle_court`) toggles the ruler's
/// character sheet (the court and its characters). Both through the message bus, so the other
/// central panels close (`displaces`); no key while a text field has the focus.
fn shortcuts(
    keymap: Res<crate::settings::KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    typing: Res<crate::ui::kit::text_field::TextInputFocus>,
    sim: Res<Sim>,
    ui: Res<CourtUi>,
    mut outgoing: MessageWriter<OpenPanel>,
) {
    if typing.is_active() {
        return;
    }
    if keymap.just_pressed("map_toggle_council", &keys) {
        outgoing.write(OpenPanel::Council);
    } else if keymap.just_pressed("map_toggle_court", &keys) {
        if ui.screen == Screen::Sheet {
            outgoing.write(OpenPanel::CloseAll);
        } else if let Some(ruler) = sim
            .state
            .as_ref()
            .and_then(|s| s.factions.get(&sim.player))
            .and_then(|f| f.ruler.as_ref())
        {
            outgoing.write(OpenPanel::Character(ruler.to_string()));
        }
    }
}

fn close_on_exit(mut ui: ResMut<CourtUi>) {
    ui.close();
}

/// `OpenPanel` and Escape (consumed before the pause menu sees it).
fn open_close(
    mut requests: MessageReader<OpenPanel>,
    mut edges: ResMut<KeyEdges>,
    mut ui: ResMut<CourtUi>,
) {
    for request in requests.read() {
        match request {
            OpenPanel::Character(id) => ui.open_character(id, false),
            OpenPanel::Council => {
                if ui.screen == Screen::Council {
                    ui.close();
                } else {
                    ui.open_council();
                }
            }
            other if ui.screen != Screen::None && displaces(other, "court") => ui.close(),
            _ => {}
        }
    }
    if edges.escape && ui.screen != Screen::None {
        edges.escape = false;
        if let Some(previous) = ui.history.pop()
            && ui.screen == Screen::Sheet
        {
            ui.character = Some(previous);
            ui.dirty = true;
        } else {
            ui.close();
        }
    }
}

fn handle_presses(
    mut pressed: MessageReader<ButtonPressed>,
    mut tabs: MessageReader<TabSelected>,
    mut ui: ResMut<CourtUi>,
    mut sim: ResMut<Sim>,
    mut panels: MessageWriter<OpenPanel>,
    mut select: MessageWriter<crate::campaign::provinces::SelectProvince>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    for tab in tabs.read() {
        match tab.group.as_str() {
            "court_sheet_tab" => ui.sheet_tab = tab.name.clone(),
            "court_council_tab" => ui.council_tab = tab.name.clone(),
            _ => {}
        }
    }
    if ui.screen == Screen::None {
        // Drain so a stale press is not replayed when the screen opens.
        pressed.clear();
        return;
    }
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("court:") else {
            continue;
        };
        if let Some(id) = name.strip_prefix("advice:") {
            if id.starts_with("hint:") {
                // `next_hint.activate(hint)`: the HUD card performs the action of its hint.
                ui.hint_go = true;
                ui.hint_activations += 1;
                continue;
            }
            if let Some(alert) = ui.snapshot.alerts.iter().find(|a| a.id == id).cloned() {
                for panel in advice_route(&alert, &mut select) {
                    panels.write(panel);
                }
            }
            continue;
        }
        if let Some(event) = crate::audio::uisound::panel_sound(
            &mut ui,
            |ui| &mut ui.result,
            |result| result.ok,
            |ui| press_button(name, ui, &mut sim),
        ) {
            sfx.write(crate::audio::PlaySfx::new(event));
        }
    }
}

/// Family tree input (`family_tree_view.gd::_gui_input`): Ctrl + wheel zooms by 1.1, a drag with
/// the left or middle button scrolls; the scroll offset is kept for the next rebuild.
fn tree_input(
    mut ui: ResMut<CourtUi>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut scrollers: Query<
        (&bevy::picking::hover::Hovered, &mut ScrollPosition),
        With<view::FamilyScroller>,
    >,
) {
    let Ok((hovered, mut position)) = scrollers.single_mut() else {
        wheel.clear();
        motion.clear();
        return;
    };
    let hovered = hovered.get();
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    for event in wheel.read() {
        if hovered && ctrl && event.y != 0.0 {
            let factor = if event.y > 0.0 { 1.1 } else { 1.0 / 1.1 };
            let zoom = ui.tree_zoom * factor;
            ui.set_tree_zoom(zoom);
        }
    }
    let dragging = mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Middle);
    for event in motion.read() {
        if hovered && dragging {
            position.0 -= event.delta;
            position.0 = position.0.max(Vec2::ZERO);
        }
    }
    if ui.tree_scroll != position.0 && !ui.dirty {
        ui.tree_scroll = position.0;
    }
}

/// Relays a click on the hint advice to the card of the HUD (`hud:hint_go`), which owns the
/// action (`NextHintController.activate`).
fn forward_hint(mut ui: ResMut<CourtUi>, mut out: MessageWriter<ButtonPressed>) {
    if ui.hint_go {
        ui.hint_go = false;
        out.write(ButtonPressed {
            entity: Entity::PLACEHOLDER,
            name: "hud:hint_go".to_owned(),
        });
    }
}

/// The next-step hint as an advice item (`hint:<id>`, kind of its alert or its own id).
pub(crate) fn hint_alert(hint: &crate::campaign::hud::hint::Hint) -> game_api::turn::Alert {
    game_api::turn::Alert {
        id: format!("hint:{}", hint.id),
        kind: hint
            .alert
            .as_ref()
            .map_or_else(|| hint.id.clone(), |alert| alert.kind.clone()),
        severity: game_api::turn::Severity::Info,
        text: if hint.text.is_empty() {
            hint.title.clone()
        } else {
            hint.text.clone()
        },
        province: hint.alert.as_ref().and_then(|a| a.province.clone()),
        army: hint.army.clone(),
        blocking: false,
    }
}

/// Where an advice leads (`alert_routes.gd`): its army, its province or the screen concerned.
fn advice_route(
    alert: &game_api::turn::Alert,
    select: &mut MessageWriter<crate::campaign::provinces::SelectProvince>,
) -> Vec<OpenPanel> {
    if let Some(army) = &alert.army {
        return vec![OpenPanel::Army(army.clone())];
    }
    if let Some(province) = &alert.province {
        select.write(crate::campaign::provinces::SelectProvince(Some(
            province.clone(),
        )));
        return vec![OpenPanel::Province(province.clone())];
    }
    match alert.kind.as_str() {
        "research_idle" => vec![OpenPanel::Tech],
        "diplomacy_offer" => vec![OpenPanel::Diplomacy],
        _ => Vec::new(),
    }
}

fn press_button(name: &str, ui: &mut CourtUi, sim: &mut Sim) {
    ui.dirty = true;
    if let Some(action) = name.strip_prefix("tree:") {
        match action {
            "in" => ui.set_tree_zoom(ui.tree_zoom * 1.2),
            "out" => ui.set_tree_zoom(ui.tree_zoom / 1.2),
            "home" => {
                ui.set_tree_zoom(1.0);
                ui.tree_scroll = Vec2::ZERO;
            }
            _ => {}
        }
        return;
    }
    if name != "ransom:execute" {
        ui.armed_execution = false;
    }
    let Some(character) = ui.character.clone() else {
        if name == "close" {
            ui.close();
        }
        run_council_press(name, ui, sim);
        return;
    };
    match name {
        "close" => ui.close(),
        "back" => {
            if let Some(previous) = ui.history.pop() {
                ui.character = Some(previous);
            } else {
                ui.close();
            }
        }
        "council" => ui.open_council(),
        _ => {
            if let Some(id) = name.strip_prefix("char:") {
                ui.open_character(id, true);
            } else if let Some(menu) = name.strip_prefix("menu:") {
                let menu = match menu {
                    "marriage" => OrderMenu::Marriage,
                    "governor" => OrderMenu::Governor,
                    "general" => OrderMenu::General,
                    _ => OrderMenu::Transfer,
                };
                ui.menu = if ui.menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
            } else if let Some(companion) = name.strip_prefix("companion:") {
                ui.companion = Some(companion.to_owned());
                ui.menu = Some(OrderMenu::Transfer);
            } else if ui.screen == Screen::Sheet {
                run_sheet_order(name, &character, ui, sim);
            } else {
                run_council_press(name, ui, sim);
            }
        }
    }
}

fn run_sheet_order(name: &str, character: &str, ui: &mut CourtUi, sim: &mut Sim) {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return;
    };
    use game_api::court as api;
    let result = if let Some(skill) = name.strip_prefix("learn:") {
        api::learn_skill(&data, state, character, skill)
    } else if let Some(spouse) = name.strip_prefix("marry:") {
        api::marry(&data, state, character, spouse)
    } else if let Some(province) = name.strip_prefix("gov:") {
        api::assign_governor(&data, state, character, province)
    } else if let Some(army) = name.strip_prefix("gen:") {
        api::assign_general(&data, state, character, army)
    } else if name == "heir" {
        api::designate_heir(&data, state, character)
    } else if let Some(to) = name.strip_prefix("transfer:") {
        let Some(companion) = ui.companion.clone() else {
            return;
        };
        api::transfer_companion(&data, state, character, to, &companion)
    } else if let Some(installments) = name.strip_prefix("ransom:pay:") {
        api::pay_ransom_in(&data, state, character, installments.parse().unwrap_or(0))
    } else if name == "ransom:pay" {
        api::pay_ransom(&data, state, character)
    } else if name == "ransom:release" {
        api::release_captive(&data, state, character)
    } else if name == "ransom:parole" {
        api::release_on_parole(&data, state, character)
    } else if let Some(terms) = name.strip_prefix("ransom:terms:") {
        let (kind, province) = terms.split_once(':').unwrap_or((terms, ""));
        api::set_ransom_terms(&data, state, character, kind, province)
    } else if name == "ransom:execute" {
        if !ui.armed_execution {
            ui.armed_execution = true;
            return;
        }
        ui.armed_execution = false;
        api::execute_captive(&data, state, character)
    } else {
        return;
    };
    if result.ok {
        ui.menu = None;
        ui.companion = None;
    }
    ui.result = Some(result);
}

fn run_council_press(name: &str, ui: &mut CourtUi, sim: &mut Sim) {
    if let Some(id) = name.strip_prefix("char:") {
        ui.open_character(id, false);
        return;
    }
    if let Some(office) = name.strip_prefix("office:") {
        ui.office = Some(office.to_owned());
        return;
    }
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return;
    };
    use game_api::court as api;
    let result = if let Some(rest) = name.strip_prefix("appoint:") {
        let Some((office, character)) = rest.split_once(':') else {
            return;
        };
        api::appoint_councillor(&data, state, office, character)
    } else if let Some(office) = name.strip_prefix("dismiss:") {
        api::dismiss_councillor(&data, state, office)
    } else if let Some(act) = name.strip_prefix("act:") {
        api::perform_royal_act(&data, state, act)
    } else if let Some(settlement) = name.strip_prefix("captain:") {
        api::hire_captain(&data, state, settlement)
    } else if let Some(order) = name.strip_prefix("found:") {
        api::found_order(&data, state, order)
    } else {
        return;
    };
    ui.result = Some(result);
}

/// Recomputes the snapshot from the sim (called by the view just before it rebuilds).
pub(crate) fn refresh_snapshot(
    ui: &mut CourtUi,
    sim: &Sim,
    hint: Option<&crate::campaign::hud::hint::Hint>,
) {
    ui.snapshot = Snapshot::default();
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    use game_api::court as api;
    let player = state.player_faction().clone();
    match ui.screen {
        Screen::Sheet => {
            let Some(id) = ui
                .character
                .as_deref()
                .and_then(|id| data_model::CharacterId::new(id).ok())
            else {
                return;
            };
            ui.snapshot.sheet = api::character_sheet(data, state, &id);
            ui.snapshot.skills = api::skill_tree(data, state, &id);
            ui.snapshot.family = api::family_tree(data, state, &id, 2, 2);
        }
        Screen::Council => {
            ui.snapshot.offices = api::council(data, state, &player);
            ui.snapshot.court = api::court_rows(data, state, &player);
            ui.snapshot.acts = api::royal_acts(data, state, &player);
            ui.snapshot.captain = api::captain_info(state, &player);
            ui.snapshot.chivalry = Some(api::chivalric_orders(data, state, &player));
            ui.snapshot.treasury = state.faction_summary(&player).map_or(0, |s| s.treasury);
            ui.snapshot.alerts = api::advice_alerts(data, state, &player);
            // `council_controller.gd::_advice_items`: the « que faire maintenant » hint joins
            // the alerts the councillors comment on.
            ui.snapshot.alerts.extend(hint.map(hint_alert));
            let valid = |id: &str| ui.snapshot.offices.iter().any(|o| o.id == id);
            if ui.office.as_deref().is_none_or(|id| !valid(id)) {
                ui.office = ui.snapshot.offices.first().map(|o| o.id.clone());
            }
            if let Some(office) = ui.office.clone() {
                ui.snapshot.candidates = api::council_candidates(data, state, &player, &office);
                ui.snapshot.holder = ui
                    .snapshot
                    .offices
                    .iter()
                    .find(|o| o.id == office)
                    .and_then(|o| data_model::CharacterId::new(&o.holder_id).ok())
                    .and_then(|id| api::character_sheet(data, state, &id));
            }
        }
        Screen::None => {}
    }
}

fn portrait_dump(bank: &portrait::PortraitBank, s: &CharacterSheet) -> Value {
    let subject = view::sheet_subject(s);
    let resolved = bank.resolve(&subject);
    let marks = bank.marks(&subject, &resolved);
    json!({
        "path": resolved.path, "kind": resolved.kind, "rank": resolved.rank,
        "band": resolved.band, "culture": resolved.culture,
        "mirrored": resolved.mirrored,
        "frame": bank.frame_style(&resolved.rank),
        "marks": portrait::marks_text(&marks, s.sex == "female"),
        "crown": marks.crown, "mitre": marks.mitre,
    })
}

fn dump_court(world: &World) -> (String, Value) {
    let ui = world.resource::<CourtUi>();
    let bank = world.resource::<portrait::PortraitBank>();
    let snap = &ui.snapshot;
    let sheet = snap.sheet.as_ref().map(|s| {
        json!({
            "id": s.id, "name": s.name, "title": s.title, "activity": s.activity,
            "age": s.age, "own": s.own, "can_act": s.can_act,
            "skills": [s.skills.command, s.skills.governance, s.skills.court],
            "skill_points": s.skill_points, "level": s.level,
            "learned": s.learned.len(), "learnable": s.learnable,
            "traits": s.traits.len(), "children": s.children.len(),
            "retinue": s.retinue.len(),
            "marriage_candidates": s.marriage_candidates.len(),
            "governor_options": s.governor_options.len(),
            "general_options": s.general_options.len(),
            "portrait": portrait_dump(bank, s),
            "retinue_max": s.retinue_max,
            "ransom": s.ransom_detail.as_ref().map(|r| json!({
                "action": s.ransom_action, "terms": r.terms, "plans": r.plans.len(),
                "cedable": r.cedable.len(), "execution": r.execution.is_some(),
            })),
        })
    });
    let family_layout = snap.family.as_ref().map(|tree| {
        let layout = family_layout::layout(tree);
        json!({
            "cards": layout.positions.len(), "segments": layout.segments.len(),
            "couples": layout.couples.len(),
            "generations": family_layout::generations(tree).len(),
            "width": layout.width, "height": layout.height,
        })
    });
    let skill_layout = {
        let layout = skill_layout::layout(&snap.skills);
        json!({
            "nodes": layout.positions.len(), "bands": layout.bands.len(),
            "links": layout.links.len(), "width": layout.width, "height": layout.height,
        })
    };
    let advice = json!({
        "alerts": snap.alerts.len(),
        "hint": snap.alerts.iter().find(|a| a.id.starts_with("hint:")).map(|a| json!({
            "id": a.id, "kind": a.kind, "text": a.text,
        })),
        "hint_activations": ui.hint_activations,
        "top": view::top_advice(snap, 3).iter().map(|(office, a)| json!({
            "office": office, "kind": a.kind, "text": a.text,
        })).collect::<Vec<_>>(),
    });
    let marks_stats = {
        let stats = world.resource::<portrait::PortraitMarksStats>();
        json!({"dressed": stats.dressed, "grey": stats.grey, "pallor": stats.pallor, "dim": stats.dim})
    };
    use game_api::court::SkillState;
    let count = |state: SkillState| snap.skills.iter().filter(|n| n.state == state).count();
    (
        "court".to_owned(),
        json!({
            "screen": format!("{:?}", ui.screen),
            "character": ui.character,
            "history": ui.history,
            "sheet_tab": ui.sheet_tab,
            "council_tab": ui.council_tab,
            "menu": ui.menu.map(|m| format!("{m:?}")),
            "sheet": sheet,
            "skill_tree": {
                "nodes": snap.skills.len(),
                "learned": count(SkillState::Learned),
                "available": count(SkillState::Available),
                "needs_points": count(SkillState::NeedsPoints),
                "locked": count(SkillState::Locked),
            },
            "family_nodes": snap.family.as_ref().map(|f| f.nodes.len()),
            "family_layout": family_layout,
            "skill_layout": skill_layout,
            "advice": advice,
            "tree_zoom": ui.tree_zoom,
            "tree_scroll": [ui.tree_scroll.x, ui.tree_scroll.y],
            "portrait_marks": marks_stats,
            "holder_portrait": snap.holder.as_ref().map(|h| {
                let resolved = bank.resolve(&view::sheet_subject(h));
                json!({"id": h.id, "kind": resolved.kind, "path": resolved.path})
            }),
            "armed_execution": ui.armed_execution,
            "council": {
                "offices": snap.offices.iter().map(|o| json!({
                    "id": o.id, "holder": o.holder_id, "skill": o.holder_skill,
                })).collect::<Vec<_>>(),
                "selected": ui.office,
                "candidates": snap.candidates.len(),
                "court": snap.court.len(),
                "royal_acts": snap.acts.iter().map(|a| json!({
                    "id": a.id, "available": a.available, "reason": a.reason,
                })).collect::<Vec<_>>(),
                "captain": snap.captain.as_ref().map(|c| json!({
                    "count": c.count, "cap": c.cap, "can_hire": c.can_hire, "cost": c.cost,
                })),
                "chivalry_options": snap.chivalry.as_ref().map(|c| c.options.len()),
                "chivalry_founded": snap.chivalry.as_ref().is_some_and(|c| c.founded.is_some()),
            },
            "last_result": ui.result.as_ref().map(|r| json!({"ok": r.ok, "text": r.message})),
        }),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn tree_zoom_is_clamped_and_scales_the_scroll() {
        let mut ui = CourtUi {
            tree_scroll: Vec2::new(100.0, 40.0),
            ..CourtUi::default()
        };
        ui.set_tree_zoom(2.0 * ZOOM_MAX);
        assert_eq!(ui.tree_zoom, ZOOM_MAX);
        assert!((ui.tree_scroll.x - 100.0 * ZOOM_MAX).abs() < 1e-3);
        ui.set_tree_zoom(0.0);
        assert_eq!(ui.tree_zoom, ZOOM_MIN);
    }

    #[test]
    fn hint_becomes_an_info_advice() {
        let hint = crate::campaign::hud::hint::Hint {
            id: "end_turn".into(),
            title: "Fin".into(),
            text: "Entrée".into(),
            action: "end_turn".into(),
            alert: None,
            army: None,
        };
        let alert = hint_alert(&hint);
        assert_eq!(alert.id, "hint:end_turn");
        assert_eq!(alert.kind, "end_turn");
        assert_eq!(alert.text, "Entrée");
        assert_eq!(alert.severity, game_api::turn::Severity::Info);
    }

    use super::*;

    #[test]
    fn history_follows_character_links() {
        let mut ui = CourtUi::default();
        ui.open_character("chr_a", false);
        ui.open_character("chr_b", true);
        ui.open_character("chr_c", true);
        assert_eq!(ui.history, ["chr_a", "chr_b"]);
        assert_eq!(ui.character.as_deref(), Some("chr_c"));
        ui.open_character("chr_x", false);
        assert!(ui.history.is_empty());
        ui.close();
        assert_eq!(ui.screen, Screen::None);
    }

    #[test]
    fn council_screen_toggles() {
        let mut ui = CourtUi::default();
        ui.open_council();
        assert_eq!(ui.screen, Screen::Council);
        ui.close();
        assert_eq!(ui.screen, Screen::None);
    }
}
