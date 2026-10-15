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
//! Not ported: the 3D council room (`council_room.gd`, SubViewport with seated figures: the
//! offices are a list), connector lines of the family tree (generations are rows of cards), the
//! living / aged portrait frame (`PortraitFrame`: a static `portraits/<id>.png`, else the
//! faction arms), the advice lines of the offices (`advice_provider`).

mod view;

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
        app.init_resource::<CourtUi>()
            .register_dump(dump_court)
            .add_systems(
                Update,
                (
                    dev_open,
                    open_close.before(crate::menu::pause::pause_escape),
                    handle_presses,
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
}

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
    pub(crate) dirty: bool,
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
            dirty: false,
            snapshot: Snapshot::default(),
        }
    }
}

impl CourtUi {
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

/// Dev hook for agent scripts: `CENT_ANS_OPEN_COURT=character:<id>|character:ruler|council`
/// opens the screen once the campaign runs.
fn dev_open(mut done: Local<bool>, sim: Res<Sim>, mut ui: ResMut<CourtUi>) {
    if *done {
        return;
    }
    *done = true;
    let Ok(request) = std::env::var("CENT_ANS_OPEN_COURT") else {
        return;
    };
    if request == "council" {
        ui.open_council();
    } else if let Some(id) = request.strip_prefix("character:") {
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
        press_button(name, &mut ui, &mut sim);
    }
}

fn press_button(name: &str, ui: &mut CourtUi, sim: &mut Sim) {
    ui.dirty = true;
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
    } else if name == "ransom:pay" {
        api::pay_ransom(&data, state, character)
    } else if name == "ransom:release" {
        api::release_captive(&data, state, character)
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
pub(crate) fn refresh_snapshot(ui: &mut CourtUi, sim: &Sim) {
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
            let valid = |id: &str| ui.snapshot.offices.iter().any(|o| o.id == id);
            if ui.office.as_deref().is_none_or(|id| !valid(id)) {
                ui.office = ui.snapshot.offices.first().map(|o| o.id.clone());
            }
            if let Some(office) = ui.office.clone() {
                ui.snapshot.candidates = api::council_candidates(data, state, &player, &office);
            }
        }
        Screen::None => {}
    }
}

fn dump_court(world: &World) -> (String, Value) {
    let ui = world.resource::<CourtUi>();
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
            "portrait": view::portrait_path(&s.id),
        })
    });
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
