//! Campaign panels: the army panel (docked, bottom centre) and the recruitment screen (docked,
//! right), with the mercenary market.
//!
//! Ports: game/scripts/ui/army_strip.gd (regiment cards, split / garrison / merge / recruit /
//! mercenaries), game/scripts/ui/mercenary_panel.gd, game/scripts/ui/stance_bar.gd (stance row),
//! game/scripts/map/recruit_basket.gd (UX5-R basket, `basket.rs`), the recruitment and queue part
//! of game/scripts/map/settlement_panel.gd and settlement_controller.gd, and the order wiring of
//! game/scripts/map/hud_controller.gd. Data views come from `game_api::army` and
//! `game_api::recruit`; every order goes through `CampaignState::submit_order_outcome` exactly as
//! `godot-bridge` does. No rule lives here.
//!
//! Opens on `OpenPanel::Army(id)`, `OpenPanel::Recruitment(province_or_settlement)` and when
//! `SelectedArmy` changes to a player army. Dev hooks: `CENT_ANS_OPEN_RECRUIT=<province id>` and
//! `CENT_ANS_OPEN_ARMY=<army id|player>` open them on the first campaign frame (script proofs).
//! Button names: `army:close|stance:<key>|unit:<i>|split|garrison|disband|recruit|mercs|merge:<id>`,
//! `merc:hire:<unit>`, `rec:close|tab:<cat|all>|sort|target:<id|garrison>|unit:<id>|minus:<id>|
//! clear|seal|cancel:<i>`.

mod basket;
mod detail;
mod view;

use std::collections::BTreeSet;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use data_model::GameData;
use serde_json::{Value, json};
use sim_campaign::{ArmyId, Order, Place, Stance};

use self::basket::{Basket, Sort};
use self::detail::{ArmyTexts, RegimentCard};
use self::view::{ArmyPanelData, IconCache};
use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::SelectedArmy;
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION, ToastRequest};
use crate::ui::kit::tooltip::TooltipState;
use crate::ui::kit::tooltip_spec::TooltipTexts;

const ENV_RECRUIT: &str = "CENT_ANS_OPEN_RECRUIT";
const ENV_ARMY: &str = "CENT_ANS_OPEN_ARMY";
const DISBAND_KEY: &str = "army:disband";

/// Root of the army panel tree.
#[derive(Component)]
pub struct ArmyPanelRoot;
/// Root of the recruitment tree.
#[derive(Component)]
pub struct RecruitPanelRoot;

/// Experience tiers (`data/ui/unit_ranks.json`, `UnitRank`).
#[derive(Resource, Debug, Clone)]
pub struct Ranks {
    max_level: u8,
    tiers: Vec<(u8, String)>,
}

impl Default for Ranks {
    fn default() -> Self {
        Self {
            max_level: 10,
            tiers: vec![(0, "Levée".to_owned())],
        }
    }
}

impl Ranks {
    pub fn parse(text: &str) -> Option<Self> {
        let json: Value = serde_json::from_str(text).ok()?;
        let tiers = json
            .get("tiers")?
            .as_array()?
            .iter()
            .filter_map(|t| {
                Some((
                    u8::try_from(t.get("min_level")?.as_u64()?).ok()?,
                    t.get("name")?.as_str()?.to_owned(),
                ))
            })
            .collect();
        Some(Self {
            max_level: u8::try_from(json.get("max_level")?.as_u64()?).ok()?.max(1),
            tiers,
        })
    }

    /// Highest tier reached by `level`.
    pub fn tier_name(&self, level: u8) -> &str {
        self.tiers
            .iter()
            .filter(|(min, _)| *min <= level)
            .max_by_key(|(min, _)| *min)
            .map_or("", |(_, name)| name.as_str())
    }

    /// « Expérience : 4/10 (Éprouvé) ».
    pub fn summary(&self, level: u8) -> String {
        let level = level.min(self.max_level);
        format!(
            "Expérience : {level}/{} ({})",
            self.max_level,
            self.tier_name(level)
        )
    }
}

/// Last order the panels submitted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderRecord {
    pub kind: String,
    pub ok: bool,
    pub error: String,
}

/// What both panels remember between rebuilds.
#[derive(Resource, Default)]
pub struct PanelState {
    pub army: Option<ArmyId>,
    /// Settlement of the open recruitment screen.
    pub recruit: Option<String>,
    /// Army a recruit joins; `None` = garrison.
    pub target: Option<ArmyId>,
    pub basket: Basket,
    pub category: String,
    pub sort: Sort,
    pub mercs: bool,
    pub selection: BTreeSet<usize>,
    pub error: String,
    pub dirty: bool,
    pub issued: u32,
    pub accepted: u32,
    pub last: Option<OrderRecord>,
    summary: Summary,
    /// Right clicks that pinned a regiment sheet (dump).
    pub sheets_pinned: u32,
}

/// Counts at the last rebuild (dump).
#[derive(Default, Clone)]
struct Summary {
    regiments: usize,
    men: u32,
    max_men: u32,
    upkeep: u32,
    movement_left: u32,
    stance: String,
    merge_targets: Vec<String>,
    mercenary_cards: usize,
    mercenary_ids: Vec<String>,
    recruit_cards: usize,
    recruit_available: usize,
    soon: usize,
    queue: usize,
    slots: usize,
    slots_free: usize,
    treasury: i64,
    settlement: String,
    armies_to_join: usize,
    read_only: bool,
    bubbles: usize,
    detailed_lines: usize,
    chained_links: usize,
}

pub struct PanelArmyPlugin;

impl Plugin for PanelArmyPlugin {
    fn build(&self, app: &mut App) {
        let ranks = paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/ui/unit_ranks.json")).ok())
            .and_then(|text| Ranks::parse(&text))
            .unwrap_or_default();
        app.insert_resource(ranks)
            .insert_resource(ArmyTexts(TooltipTexts::load().unwrap_or_default()))
            .init_resource::<PanelState>()
            .add_systems(Startup, detail::register_rules)
            .init_resource::<IconCache>()
            .register_dump(dump_panel)
            .add_systems(
                Update,
                (
                    intake,
                    pin_regiment_sheet,
                    handle_buttons,
                    handle_modal,
                    rebuild,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), reset);
    }
}

/// Right click on a hovered regiment card: pins its detailed sheet (`unit_details_requested`).
fn pin_regiment_sheet(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cards: Query<(&Hovered, &RegimentCard)>,
    mut tooltips: ResMut<TooltipState>,
    mut state: ResMut<PanelState>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(card) = cards
        .iter()
        .find_map(|(hovered, card)| hovered.get().then_some(card))
    else {
        return;
    };
    if card.detailed.is_empty() {
        return;
    }
    let cursor = window
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .unwrap_or_default();
    tooltips.open_at(&card.detailed, cursor, true);
    state.sheets_pinned += 1;
}

fn reset(mut state: ResMut<PanelState>) {
    *state = PanelState::default();
}

// ---- opening -----------------------------------------------------------------------------

fn open_recruit(state: &mut PanelState, sim: &Sim, place: &str, target: Option<ArmyId>) -> bool {
    let Some(game_state) = &sim.state else {
        return false;
    };
    let Some(id) = game_api::recruit::settlement_or_city(game_state, place) else {
        state.error = format!("Colonie inconnue : {place}");
        return false;
    };
    let id = id.as_str().to_owned();
    if state.recruit.as_deref() != Some(&id) {
        state.basket.clear();
    }
    state.recruit = Some(id);
    state.target = target;
    state.error.clear();
    state.dirty = true;
    true
}

fn open_army(state: &mut PanelState, sim: &Sim, id: &ArmyId) {
    if sim.state.as_ref().is_some_and(|s| s.army(id).is_some()) && state.army.as_ref() != Some(id) {
        state.army = Some(id.clone());
        state.selection.clear();
        state.mercs = false;
        state.error.clear();
        state.dirty = true;
    }
}

fn close_army(state: &mut PanelState) {
    if state.army.take().is_some() {
        state.dirty = true;
    }
    state.selection.clear();
    state.mercs = false;
}

fn close_recruit(state: &mut PanelState) {
    if state.recruit.take().is_some() {
        state.dirty = true;
    }
    state.basket.clear();
    state.target = None;
}

fn is_player_army(sim: &Sim, id: &ArmyId) -> bool {
    sim.state
        .as_ref()
        .and_then(|s| s.army(id).map(|a| &a.faction == s.player_faction()))
        .unwrap_or(false)
}

fn intake(
    mut open: MessageReader<OpenPanel>,
    selected: Res<SelectedArmy>,
    sim: Res<Sim>,
    mut state: ResMut<PanelState>,
    mut hook: Local<bool>,
    mut last_selected: Local<Option<ArmyId>>,
) {
    if sim.state.is_none() {
        return;
    }
    if !*hook {
        *hook = true;
        if let Ok(place) = std::env::var(ENV_RECRUIT) {
            open_recruit(&mut state, &sim, &place, None);
        }
        if let Ok(value) = std::env::var(ENV_ARMY) {
            // `player` = the player's most numerous army.
            let id = if value == "player" {
                sim.state.as_ref().and_then(|s| {
                    s.armies()
                        .iter()
                        .filter(|(_, a)| &a.faction == s.player_faction())
                        .max_by_key(|(_, a)| a.units.len())
                        .map(|(id, _)| id.clone())
                })
            } else {
                ArmyId::parse(&value)
            };
            if let Some(id) = id {
                open_army(&mut state, &sim, &id);
            }
        }
    }
    for message in open.read() {
        match message {
            OpenPanel::Army(id) => {
                if let Some(id) = ArmyId::parse(id) {
                    open_army(&mut state, &sim, &id);
                }
            }
            OpenPanel::Recruitment(place) => {
                open_recruit(&mut state, &sim, place, None);
            }
            OpenPanel::Province(province) => {
                close_army(&mut state);
                let same = sim.state.as_ref().is_some_and(|s| {
                    state.recruit.as_deref().is_some_and(|r| {
                        game_api::recruit::settlement_or_city(s, province)
                            .is_some_and(|id| id.as_str() == r)
                    })
                });
                if !same {
                    close_recruit(&mut state);
                }
            }
            // Central screens, the siege panel and CloseAll replace the docked army panels.
            _ => {
                close_army(&mut state);
                close_recruit(&mut state);
            }
        }
    }
    if selected.0 != *last_selected {
        last_selected.clone_from(&selected.0);
        match &selected.0 {
            // A foreign army opens the read-only panel (`show_army(.., is_player = false)`).
            Some(id) => open_army(&mut state, &sim, id),
            None => close_army(&mut state),
        }
    }
}

// ---- orders ------------------------------------------------------------------------------

fn submit(sim: &mut Sim, order: Order) -> Result<(), String> {
    let data = sim.data.clone().ok_or("données non chargées")?;
    let game_state = sim.state.as_mut().ok_or("aucune campagne en cours")?;
    game_state
        .submit_order_outcome(&data, order)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Submits one order, records it and returns the French refusal if any.
fn order(state: &mut PanelState, sim: &mut Sim, kind: &str, order: Order) -> Result<(), String> {
    state.issued += 1;
    let result = submit(sim, order);
    state.last = Some(OrderRecord {
        kind: kind.to_owned(),
        ok: result.is_ok(),
        error: result.clone().err().unwrap_or_default(),
    });
    state.dirty = true;
    match &result {
        Ok(()) => {
            state.accepted += 1;
            state.error.clear();
        }
        Err(message) => state.error.clone_from(message),
    }
    result
}

fn indices(selection: &BTreeSet<usize>) -> Vec<usize> {
    selection.iter().copied().collect()
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut sim: ResMut<Sim>,
    mut state: ResMut<PanelState>,
    mut selected: ResMut<SelectedArmy>,
    mut modals: ResMut<ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if let Some(rest) = name.strip_prefix("army:") {
            army_button(
                rest,
                &mut sim,
                &mut state,
                &mut selected,
                &mut modals,
                &mut toasts,
            );
        } else if let Some(unit) = name.strip_prefix("merc:hire:") {
            let (Some(army), Ok(unit)) = (
                state.army.clone(),
                data_model::UnitTypeId::new(unit.to_owned()),
            ) else {
                continue;
            };
            if order(
                &mut state,
                &mut sim,
                "hire_mercenary",
                Order::HireMercenary { army, unit },
            )
            .is_ok()
            {
                toasts.write(ToastRequest::new("Compagnie engagée : elle rejoint l'ost."));
            }
        } else if let Some(rest) = name.strip_prefix("rec:") {
            recruit_button(rest, &mut sim, &mut state, &mut toasts);
        }
    }
}

fn army_button(
    name: &str,
    sim: &mut Sim,
    state: &mut PanelState,
    selected: &mut SelectedArmy,
    modals: &mut ModalQueue,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    let Some(army) = state.army.clone() else {
        return;
    };
    if name == "close" {
        close_army(state);
    } else if !is_player_army(sim, &army) {
        // Foreign armies are read-only: no order leaves this panel.
    } else if let Some(key) = name.strip_prefix("stance:") {
        let Ok(stance) = serde_json::from_value::<Stance>(json!(key)) else {
            return;
        };
        if order(state, sim, "set_stance", Order::SetStance { army, stance }).is_ok() {
            toasts.write(ToastRequest::new("Posture modifiée."));
        }
    } else if let Some(index) = name
        .strip_prefix("unit:")
        .and_then(|i| i.parse::<usize>().ok())
    {
        if !state.selection.remove(&index) {
            state.selection.insert(index);
        }
        state.dirty = true;
    } else if name == "split" {
        let unit_indices = indices(&state.selection);
        if order(
            state,
            sim,
            "split_army",
            Order::SplitArmy { army, unit_indices },
        )
        .is_ok()
        {
            state.selection.clear();
            toasts.write(ToastRequest::new("Armée séparée."));
        }
    } else if name == "garrison" {
        let unit_indices = indices(&state.selection);
        let many = unit_indices.len() > 1;
        if order(
            state,
            sim,
            "garrison_units",
            Order::GarrisonUnits { army, unit_indices },
        )
        .is_ok()
        {
            state.selection.clear();
            toasts.write(ToastRequest::new(if many {
                "Régiments laissés en garnison."
            } else {
                "Régiment laissé en garnison."
            }));
        }
    } else if name == "disband" {
        if !state.selection.is_empty() {
            let count = state.selection.len();
            modals.request(
                DISBAND_KEY,
                PRIORITY_DECISION,
                ModalSpec::confirm(
                    "Licencier",
                    &format!(
                        "Congédier [b]{}[/b] ? Les hommes rentrent chez eux, sans remboursement.",
                        crate::ui::kit::fr::count(count as i64, "régiment", None)
                    ),
                ),
            );
        }
    } else if name == "recruit" {
        let province = sim.state.as_ref().and_then(|s| {
            let sheet = game_api::army_sheet(sim.data.as_deref()?, s, &army)?;
            sheet.location_province
        });
        if let Some(province) = province {
            open_recruit(state, sim, &province, Some(army));
        }
    } else if name == "mercs" {
        state.mercs = !state.mercs;
        state.dirty = true;
    } else if let Some(target) = name.strip_prefix("merge:").and_then(ArmyId::parse) {
        let merged = order(
            state,
            sim,
            "merge_armies",
            Order::MergeArmies {
                source: army,
                target: target.clone(),
            },
        );
        if merged.is_ok() {
            state.army = None;
            state.selection.clear();
            selected.0 = Some(target);
            toasts.write(ToastRequest::new("Armées fusionnées."));
        }
    }
}

fn recruit_view(sim: &Sim, state: &PanelState) -> Option<game_api::recruit::RecruitView> {
    let place = state.recruit.as_deref()?;
    game_api::recruit::recruit_view(sim.data.as_deref()?, sim.state.as_ref()?, place)
}

fn recruit_button(
    name: &str,
    sim: &mut Sim,
    state: &mut PanelState,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    let Some(settlement) = state.recruit.clone() else {
        return;
    };
    let Some(view) = recruit_view(sim, state) else {
        return;
    };
    if name == "close" {
        close_recruit(state);
    } else if let Some(category) = name.strip_prefix("tab:") {
        state.category = if category == "all" {
            String::new()
        } else {
            category.to_owned()
        };
        state.dirty = true;
    } else if name == "sort" {
        state.sort = state.sort.next();
        state.dirty = true;
    } else if let Some(target) = name.strip_prefix("target:") {
        state.target = ArmyId::parse(target);
        state.dirty = true;
    } else if let Some(unit) = name.strip_prefix("unit:") {
        state.error.clear();
        state.basket.add(&view, unit, 1);
        state.dirty = true;
    } else if let Some(unit) = name.strip_prefix("minus:") {
        state.basket.remove(unit, 1);
        state.dirty = true;
    } else if name == "clear" {
        state.basket.clear();
        state.dirty = true;
    } else if let Some(index) = name.strip_prefix("cancel:").and_then(|i| i.parse().ok()) {
        let Ok(id) = data_model::SettlementId::new(settlement) else {
            return;
        };
        if order(
            state,
            sim,
            "cancel_recruit",
            Order::CancelRecruit {
                settlement: Place::Settlement(id),
                index,
            },
        )
        .is_ok()
        {
            toasts.write(ToastRequest::new("Recrue annulée."));
        }
    } else if name == "seal" {
        seal(sim, state, &view, &settlement, toasts);
    }
}

/// « Sceller la levée »: one core order per recruit of the basket.
fn seal(
    sim: &mut Sim,
    state: &mut PanelState,
    view: &game_api::recruit::RecruitView,
    settlement: &str,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    if !state.basket.can_seal(view) {
        return;
    }
    let (Ok(site), batch) = (
        data_model::SettlementId::new(settlement.to_owned()),
        state.basket.order(view),
    ) else {
        return;
    };
    state.basket.clear();
    let target = state.target.clone();
    let mut done = 0;
    for unit in batch {
        let Ok(unit_type) = data_model::UnitTypeId::new(unit) else {
            continue;
        };
        let settlement = Place::Settlement(site.clone());
        let (kind, built) = match &target {
            Some(army) => (
                "recruit_into",
                Order::RecruitInto {
                    settlement,
                    unit_type,
                    army: army.clone(),
                },
            ),
            None => (
                "recruit",
                Order::Recruit {
                    settlement,
                    unit_type,
                },
            ),
        };
        if order(state, sim, kind, built).is_err() {
            break;
        }
        done += 1;
    }
    if done > 0 {
        toasts.write(ToastRequest::new(if target.is_some() {
            "Recrutement lancé : l'unité rejoindra l'armée au prochain tour."
        } else {
            "Recrutement lancé : l'unité rejoindra la garnison au prochain tour."
        }));
    }
}

fn handle_modal(
    mut results: MessageReader<ModalResult>,
    mut sim: ResMut<Sim>,
    mut state: ResMut<PanelState>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for result in results.read() {
        if result.key != DISBAND_KEY || !result.confirmed {
            continue;
        }
        let Some(army) = state.army.clone() else {
            continue;
        };
        // Highest index first: removing a unit shifts the ones after it.
        let chosen: Vec<usize> = state.selection.iter().rev().copied().collect();
        state.selection.clear();
        let mut count = 0;
        for unit_index in chosen {
            let built = Order::DisbandUnit {
                army: Some(army.clone()),
                settlement: None,
                unit_index,
            };
            if order(&mut state, &mut sim, "disband_unit", built).is_err() {
                break;
            }
            count += 1;
        }
        if count > 0 {
            toasts.write(ToastRequest::new(if count > 1 {
                "Régiments licenciés."
            } else {
                "Régiment licencié."
            }));
        }
    }
}

// ---- building ----------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<crate::ui::kit::UiTheme>,
    assets: Res<AssetServer>,
    ranks: Res<Ranks>,
    texts: Res<ArmyTexts>,
    mut icons: ResMut<IconCache>,
    mut state: ResMut<PanelState>,
    army_roots: Query<Entity, With<ArmyPanelRoot>>,
    recruit_roots: Query<Entity, With<RecruitPanelRoot>>,
) {
    let open = state.army.is_some() || state.recruit.is_some();
    let present = !army_roots.is_empty() || !recruit_roots.is_empty();
    if !(state.dirty || (sim.is_changed() && (open || present))) {
        return;
    }
    state.dirty = false;
    for root in army_roots.iter().chain(recruit_roots.iter()) {
        commands.entity(root).despawn();
    }
    let (Some(data), Some(game_state)) = (sim.data.as_deref(), sim.state.as_ref()) else {
        return;
    };
    state.summary = Summary::default();
    if let Some(id) = state.army.clone() {
        match build_army(
            &mut commands,
            &theme,
            &assets,
            &mut icons,
            &ranks,
            &texts.0,
            data,
            game_state,
            &id,
            &state,
        ) {
            Some(summary) => state.summary = summary,
            None => {
                state.army = None;
                state.selection.clear();
            }
        }
    }
    if let Some(place) = state.recruit.clone() {
        match game_api::recruit::recruit_view(data, game_state, &place) {
            Some(view) => {
                let mut basket = std::mem::take(&mut state.basket);
                basket.clear_if_unknown(&view);
                basket.clamp_to(&view);
                state.basket = basket;
                if let Some(target) = &state.target
                    && !view.armies.iter().any(|a| a.id == target.to_string())
                {
                    state.target = None;
                }
                view::spawn_recruit_panel(
                    &mut commands,
                    &theme,
                    &assets,
                    &mut icons,
                    &view,
                    &state,
                );
                let summary = &mut state.summary;
                summary.recruit_cards = view
                    .options
                    .iter()
                    .filter(|o| o.group == "ready" || o.group == "blocked")
                    .count();
                summary.recruit_available = view.options.iter().filter(|o| o.available).count();
                summary.soon = view.options.iter().filter(|o| o.group == "soon").count();
                summary.queue = view.queue.len();
                summary.slots = view.slots;
                summary.slots_free = view.slots_free;
                summary.treasury = view.treasury;
                summary.settlement = view.settlement.clone();
                summary.armies_to_join = view.armies.len();
            }
            None => state.recruit = None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_army(
    commands: &mut Commands,
    theme: &crate::ui::kit::UiTheme,
    assets: &AssetServer,
    icons: &mut IconCache,
    ranks: &Ranks,
    texts: &TooltipTexts,
    data: &GameData,
    game_state: &sim_campaign::CampaignState,
    id: &ArmyId,
    state: &PanelState,
) -> Option<Summary> {
    let sheet = game_api::army_sheet(data, game_state, id)?;
    let (extras, upkeep) = game_api::recruit::army_extras(data, game_state, id)?;
    let own = sheet.recruit_blocker.is_some();
    let stances = if own {
        game_api::recruit::stance_options(data, game_state, id)
    } else {
        Vec::new()
    };
    let market = if own && state.mercs {
        game_api::recruit::mercenary_view(data, game_state, id)
    } else {
        None
    };
    let mut bubbles = Vec::new();
    let (mut detailed_lines, mut chained_links) = (0, 0);
    for index in 0..sheet.units.len() {
        let Some(regiment) = game_api::recruit::regiment_detail(data, game_state, id, index) else {
            bubbles.push(Default::default());
            continue;
        };
        let spec = detail::regiment_spec(&regiment, extras.get(index), ranks, texts);
        let (short, long) = (spec.to_bbcode(texts, false), spec.to_bbcode(texts, true));
        detailed_lines += long.lines().count();
        chained_links += long.matches("[url=ib:").count();
        bubbles.push((short, long));
    }
    view::spawn_army_panel(
        commands,
        theme,
        assets,
        icons,
        &ArmyPanelData {
            sheet: &sheet,
            extras: &extras,
            bubbles: &bubbles,
            upkeep,
            stances: &stances,
            mercenaries: market.as_ref(),
        },
        state,
    );
    Some(Summary {
        regiments: sheet.units.len(),
        men: sheet.strength,
        max_men: sheet.max_strength,
        upkeep,
        movement_left: sheet.movement_left,
        stance: sheet.stance.clone(),
        merge_targets: sheet.merge_targets.iter().map(|t| t.id.clone()).collect(),
        mercenary_cards: market.as_ref().map_or(0, |m| m.options.len()),
        mercenary_ids: market.as_ref().map_or_else(Vec::new, |m| {
            m.options
                .iter()
                .filter(|o| o.available)
                .map(|o| o.unit_type.clone())
                .collect()
        }),
        read_only: !own,
        bubbles: bubbles.len(),
        detailed_lines,
        chained_links,
        ..Summary::default()
    })
}

impl Basket {
    /// Drops entries of unit types the new view no longer offers.
    fn clear_if_unknown(&mut self, view: &game_api::recruit::RecruitView) {
        self.counts
            .retain(|unit, _| view.options.iter().any(|o| &o.unit_type == unit));
    }
}

// ---- dump --------------------------------------------------------------------------------

fn dump_panel(world: &World) -> (String, Value) {
    let state = world.resource::<PanelState>();
    let s = &state.summary;
    let sim = world.resource::<Sim>();
    let view = recruit_view(sim, state);
    let basket = view.as_ref().map(|v| {
        json!({
            "count": state.basket.total_count(),
            "cost": state.basket.total_cost(v),
            "upkeep": state.basket.total_upkeep(v),
            "units": state.basket.counts,
            "can_seal": state.basket.can_seal(v),
        })
    });
    let value = json!({
        "army": state.army.as_ref().map(ToString::to_string),
        "army_panel": {
            "regiments": s.regiments, "men": s.men, "max_men": s.max_men, "upkeep": s.upkeep,
            "movement_left": s.movement_left, "stance": s.stance, "merge_targets": s.merge_targets,
            "selection": state.selection, "read_only": s.read_only, "bubbles": s.bubbles,
            "detailed_lines": s.detailed_lines, "chained_links": s.chained_links,
            "sheets_pinned": state.sheets_pinned, "mercenaries_open": state.mercs,
            "mercenary_cards": s.mercenary_cards, "mercenary_available": s.mercenary_ids,
        },
        "recruit": state.recruit,
        "recruit_panel": {
            "settlement": s.settlement, "cards": s.recruit_cards, "available": s.recruit_available,
            "soon": s.soon, "queue": s.queue, "slots": s.slots, "slots_free": s.slots_free,
            "treasury": view.as_ref().map_or(s.treasury, |v| v.treasury),
            "armies_to_join": s.armies_to_join,
            "available_units": view.as_ref().map(|v| v.options.iter().filter(|o| o.available).take(12).map(|o| json!({"id": o.unit_type, "cost": o.cost, "upkeep": o.upkeep, "pool": o.pool_available})).collect::<Vec<_>>()),
            "queue_units": view.as_ref().map(|v| v.queue.iter().map(|q| json!({"unit": q.unit_type, "turns_left": q.turns_left, "refund": q.refund})).collect::<Vec<_>>()),
            "target": state.target.as_ref().map(ToString::to_string),
            "category": state.category, "sort": state.sort.label(),
            "basket": basket,
        },
        "orders": {
            "issued": state.issued, "accepted": state.accepted,
            "last": state.last.as_ref().map(|l| json!({"kind": l.kind, "ok": l.ok, "error": l.error})),
        },
        "error": state.error,
    });
    ("panel_army".to_owned(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_parse_and_summary() {
        let ranks = Ranks::parse(
            r#"{"max_level":10,"tiers":[{"min_level":0,"name":"Levée"},{"min_level":3,"name":"Éprouvé"},{"min_level":10,"name":"Légendaire"}]}"#,
        )
        .unwrap();
        assert_eq!(ranks.summary(4), "Expérience : 4/10 (Éprouvé)");
        assert_eq!(ranks.summary(12), "Expérience : 10/10 (Légendaire)");
        assert_eq!(ranks.tier_name(0), "Levée");
    }

    #[test]
    fn repository_ranks_file_parses() {
        let root = paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("repository root");
        let text = std::fs::read_to_string(root.join("data/ui/unit_ranks.json")).unwrap();
        assert!(Ranks::parse(&text).is_some());
    }

    #[test]
    fn chevrons_follow_level() {
        assert_eq!(view::chevrons(3), "▲▲▲");
    }
}
