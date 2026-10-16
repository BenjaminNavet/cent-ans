//! Campaign panel: province (docked, right side). Ports game/scripts/map/province_panel.gd and
//! its sections (province_tax_section.gd, classes_section.gd, settlement_buildings_view.gd,
//! settlements_section.gd). Data: `game_api::province_sheet` and `game_api::pprovince`
//! (building tree, cancel quotes, demolition previews); orders go through
//! `CampaignState::submit_order_outcome` exactly as `godot-bridge` does.
//!
//! Opens on [`ProvinceClicked`] (left button) and `OpenPanel::Province(id)`; closes with Escape,
//! the close button, `OpenPanel::CloseAll` or `OpenPanel::Army`. Dev hook:
//! `CENT_ANS_OPEN_PROVINCE=<province id | capital>` opens a province once the campaign starts.
//!
//! Widget names (script `click <Name>`): `prov:close`, `prov:tab_city|buildings|settlements|garrison`,
//! `prov:set:<settlement>`, `prov:build:<building>`, `prov:cancel:<queue row>`,
//! `prov:raze:<building>`, `prov:tax:<low|normal|high|default>`, `prov:colony:<settlement>`, `prov:slot:<settlement>:<root>`,
//! `prov:recruit`, `prov:crumb:<faction>`, `prov:choose:<table|edict|vocation>`,
//! `prov:table:<diet>`, `prov:edict:<edict>`, `prov:vocation:<vocation>`,
//! `prov:garrison_pick:<index>` (checkbox), `prov:form_army`.

mod choices;
mod images;
mod labels;
mod mates;
mod view;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use data_model::{BuildingId, DietId, EdictId, ProvinceId, SettlementId};
use serde_json::{Value, json};
use sim_campaign::{ArmyId, Order, Place, TaxRate};

use self::labels::UiLabels;
use self::view::{SETTLEMENT_GROUP, Snapshot, TAB_GROUP, TABS};
use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::provinces::ProvinceClicked;
use crate::codex::store::CodexStore;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::panel_stack::{PanelClosed, PanelTier, StackedPanel};
use crate::ui::kit::style::UiTier;
use crate::ui::kit::widgets::{ListSelected, TabSelected, ToggleChanged};
use crate::ui::kit::{ButtonPressed, UiTheme};

const ENV_OPEN: &str = "CENT_ANS_OPEN_PROVINCE";
const PANEL_WIDTH: f32 = 400.0;
const STACK_ID: &str = "province";
const POLL_SECONDS: f32 = 0.5;

pub struct PanelProvincePlugin;

impl Plugin for PanelProvincePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(mates::plugin)
            .init_resource::<ProvincePanel>()
            .insert_resource(UiLabels::load())
            .insert_resource(images::BuildingImages::load())
            .register_dump(dump_panel)
            .add_systems(
                Update,
                (
                    dev_open,
                    open_requests,
                    track_closed,
                    track_selection,
                    handle_buttons,
                    refresh,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), reset_panel);
    }
}

/// State of the province panel.
#[derive(Resource, Default)]
pub struct ProvincePanel {
    /// Province shown (`None` = closed).
    pub open: Option<String>,
    tab: usize,
    /// Settlement of the Buildings tab (`None` = the province's city).
    settlement: Option<String>,
    dirty: bool,
    /// Serialized snapshot of the last build, to rebuild only on change.
    last: String,
    poll: f32,
    error: String,
    rebuilds: u32,
    orders_issued: u32,
    orders_accepted: u32,
    last_order: Option<String>,
    /// Unfolded choice list (`table`, `edict`, `vocation`).
    open_choice: String,
    /// Garrison indices ticked for the new army.
    picked: std::collections::BTreeSet<usize>,
    /// Building the clicked slot cell points at (first step of its chain): the Buildings tab
    /// marks its row, like Godot's focus on the matching "Construire" button.
    focus_building: String,
}

#[derive(Component)]
struct ProvincePanelRoot;

fn reset_panel(mut panel: ResMut<ProvincePanel>) {
    *panel = ProvincePanel::default();
}

impl ProvincePanel {
    /// Province and selected colony the map overlay [`mates`] marks: set while the Buildings or
    /// Colonies tab is showing.
    fn mates_view(&self) -> Option<(&str, Option<&str>)> {
        let province = self.open.as_deref()?;
        (self.tab == 1 || self.tab == 2).then_some((province, self.settlement.as_deref()))
    }

    fn open_province(&mut self, id: &str) {
        if self.open.as_deref() != Some(id) {
            self.settlement = None;
            self.error.clear();
            self.open_choice.clear();
            self.picked.clear();
            self.focus_building.clear();
        }
        self.open = Some(id.to_owned());
        self.dirty = true;
    }

    fn close(&mut self) {
        self.open = None;
        self.dirty = true;
        self.last.clear();
        self.open_choice.clear();
        self.picked.clear();
    }
}

fn dev_open(sim: Res<Sim>, mut done: Local<bool>, mut requests: MessageWriter<OpenPanel>) {
    if *done {
        return;
    }
    let (Ok(wanted), Some(state)) = (std::env::var(ENV_OPEN), sim.state.as_ref()) else {
        *done = std::env::var(ENV_OPEN).is_err();
        return;
    };
    *done = true;
    let id = if wanted == "capital" {
        state
            .factions
            .get(&sim.player)
            .map(|f| f.capital.as_str().to_owned())
    } else {
        Some(wanted)
    };
    if let Some(id) = id {
        requests.write(OpenPanel::Province(id));
    }
}

/// The panel stack closed the panel (Escape, a central panel…): follow it.
fn track_closed(mut closed: MessageReader<PanelClosed>, mut panel: ResMut<ProvincePanel>) {
    for message in closed.read() {
        if message.id == STACK_ID && panel.open.is_some() {
            panel.close();
        }
    }
}

fn open_requests(
    mut panel: ResMut<ProvincePanel>,
    mut requests: MessageReader<OpenPanel>,
    mut clicks: MessageReader<ProvinceClicked>,
) {
    for request in requests.read() {
        match request {
            OpenPanel::Province(id) => panel.open_province(id),
            OpenPanel::Army(_) => panel.close(),
            other if displaces(other, "province") => panel.close(),
            _ => {}
        }
    }
    for click in clicks.read() {
        if click.button == MouseButton::Left
            && let Some(id) = &click.id
        {
            panel.open_province(id);
        }
    }
}

fn track_selection(
    mut panel: ResMut<ProvincePanel>,
    mut tabs: MessageReader<TabSelected>,
    mut lists: MessageReader<ListSelected>,
    mut toggles: MessageReader<ToggleChanged>,
) {
    for toggle in toggles.read() {
        if let Some(index) = toggle
            .name
            .strip_prefix("prov:garrison_pick:")
            .and_then(|i| i.parse::<usize>().ok())
        {
            if toggle.on {
                panel.picked.insert(index);
            } else {
                panel.picked.remove(&index);
            }
            panel.dirty = true;
        }
    }
    for tab in tabs.read() {
        if tab.group == TAB_GROUP
            && let Some(index) = TABS.iter().position(|(name, _)| *name == tab.name)
        {
            panel.tab = index;
        }
    }
    for item in lists.read() {
        if item.group == SETTLEMENT_GROUP
            && let Some(id) = item.name.strip_prefix("prov:set:")
        {
            panel.settlement = Some(id.to_owned());
            panel.focus_building.clear();
            panel.dirty = true;
        }
    }
}

/// Builds the snapshot of `province` from `game-api`; `None` for an unknown province.
fn snapshot(
    sim: &Sim,
    panel: &ProvincePanel,
    codex: &CodexStore,
    province: &str,
    error: &str,
) -> Option<Snapshot> {
    let selected = panel.settlement.as_deref();
    let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
    let sheet = game_api::province_sheet(data, state, &ProvinceId::new(province).ok()?)?;
    let selected_settlement = selected
        .filter(|id| sheet.settlements.iter().any(|s| s.id == *id))
        .map_or_else(|| sheet.city.clone(), str::to_owned);
    let settlement = SettlementId::new(&selected_settlement).ok()?;
    let tree = game_api::pprovince::building_tree(data, state, &settlement);
    let demolitions = game_api::pprovince::demolition_previews(data, state, &settlement);
    let quotes = tree
        .iter()
        .flat_map(|t| t.queue.iter().enumerate())
        .map(|(index, _)| {
            let queue_index = if index == 0 { -1 } else { index as i64 - 1 };
            game_api::pprovince::cancel_quote(data, state, &settlement, queue_index)
        })
        .collect();
    let province_id = ProvinceId::new(province).ok()?;
    let possession = game_api::pprovince::possession_view(data, state, &sim.player, &province_id);
    let table = game_api::pprovince::table_view(data, state, &province_id);
    let edict = game_api::pprovince::edict_view(data, state, &province_id);
    let vocation = game_api::pprovince::vocation_view(data, state, &settlement);
    let mine = sheet.controller == sim.player.as_str();
    // A regional edict supposes full authority: every place held by the controller.
    let edict_editable = mine
        && possession
            .as_ref()
            .is_some_and(|p| p.whole_holder == sim.player.as_str());
    let bars = sheet
        .settlements
        .iter()
        .filter_map(|place| {
            game_api::pprovince::slot_bar(data, state, &SettlementId::new(&place.id).ok()?)
        })
        .collect();
    let lent_text = table.as_ref().is_some_and(|t| t.lent).then(|| {
        let constants = sim_campaign::rule_constants::rule_constants(data)
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect();
        choices::lent_text(codex, &constants)
    });
    let mut linked = std::collections::BTreeMap::new();
    for (key, text) in [
        ("table", table.as_ref().map(|t| t.description.as_str())),
        ("edict", edict.as_ref().map(|e| e.description.as_str())),
    ] {
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            linked.insert(key.to_owned(), crate::codex::text::format_auto(codex, text));
        }
    }
    let tier = state.settlement_tier(data, &settlement);
    let tier_progress = state.settlement_tier_progress(data, &settlement);
    Some(Snapshot {
        tier,
        tier_progress,
        linked,
        bars,
        lent_text,
        focus_building: panel.focus_building.clone(),
        mine,
        possession,
        table,
        edict,
        vocation,
        edict_editable,
        army_cap: data.army_rules.max_units,
        picked: panel.picked.iter().copied().collect(),
        open_choice: panel.open_choice.clone(),
        sheet,
        tree,
        demolitions,
        quotes,
        selected_settlement,
        error: error.to_owned(),
    })
}

#[allow(clippy::too_many_arguments)]
fn refresh(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<Sim>,
    codex: Res<CodexStore>,
    assets: Res<AssetServer>,
    theme: Res<UiTheme>,
    labels: Res<UiLabels>,
    images: Res<images::BuildingImages>,
    mut panel: ResMut<ProvincePanel>,
    existing: Query<Entity, With<ProvincePanelRoot>>,
) {
    panel.poll += time.delta_secs();
    let Some(province) = panel.open.clone() else {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        panel.dirty = false;
        return;
    };
    if !panel.dirty && panel.poll < POLL_SECONDS && !existing.is_empty() {
        return;
    }
    panel.poll = 0.0;
    let Some(snap) = snapshot(&sim, &panel, &codex, &province, &panel.error) else {
        panel.close();
        return;
    };
    let serialized = serde_json::to_string(&snap).unwrap_or_default();
    if !panel.dirty && serialized == panel.last && !existing.is_empty() {
        return;
    }
    panel.dirty = false;
    panel.last = serialized;
    panel.rebuilds += 1;
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let theme = theme.clone();
    let tab = panel.tab;
    commands
        .spawn((
            Name::new("Province Panel"),
            ProvincePanelRoot,
            StackedPanel::new(PanelTier::Docked, STACK_ID),
            crate::ui::kit::layout::FitZone::Side,
            DespawnOnExit(AppState::Campaign),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(PANEL_WIDTH),
                padding: theme.padding("panel"),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| view::build(root, &theme, &assets, &labels, &images, &snap, tab));
}

fn submit(sim: &mut Sim, order: Order) -> Result<(), String> {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return Err("aucune campagne en cours".into());
    };
    state
        .submit_order_outcome(&data, order)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn handle_buttons(
    mut presses: MessageReader<ButtonPressed>,
    mut panel: ResMut<ProvincePanel>,
    mut sim: ResMut<Sim>,
    mut toasts: MessageWriter<ToastRequest>,
    mut requests: MessageWriter<OpenPanel>,
    mut selected_army: ResMut<SelectedArmy>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    for press in presses.read() {
        let Some(action) = press.name.strip_prefix("prov:") else {
            continue;
        };
        let Some(province) = panel.open.clone() else {
            continue;
        };
        let settlement_id = panel.settlement.clone().or_else(|| {
            let state = sim.state.as_ref()?;
            state
                .province_state(&ProvinceId::new(&province).ok()?)
                .map(|p| p.city.as_str().to_owned())
        });
        let settlement = settlement_id
            .as_deref()
            .and_then(|id| SettlementId::new(id).ok());
        let (order, success): (Option<Order>, String) = if action == "close" {
            panel.close();
            continue;
        } else if action == "recruit" {
            requests.write(OpenPanel::Recruitment(province));
            continue;
        } else if let Some(faction) = action.strip_prefix("crumb:") {
            requests.write(OpenPanel::Faction(faction.to_owned()));
            continue;
        } else if let Some(key) = action.strip_prefix("choose:") {
            panel.open_choice = if panel.open_choice == key {
                String::new()
            } else {
                key.to_owned()
            };
            panel.dirty = true;
            continue;
        } else if let Some(diet) = action.strip_prefix("table:") {
            let (Ok(province_id), Ok(diet)) = (ProvinceId::new(&province), DietId::new(diet))
            else {
                continue;
            };
            (
                Some(Order::SetDiet {
                    province: province_id,
                    diet,
                }),
                "Régime modifié (effet à la fin du tour).".to_owned(),
            )
        } else if let Some(edict) = action.strip_prefix("edict:") {
            let (Ok(province_id), Ok(edict)) = (ProvinceId::new(&province), EdictId::new(edict))
            else {
                continue;
            };
            (
                Some(Order::SetEdict {
                    province: province_id,
                    edict,
                }),
                "Édit adopté.".to_owned(),
            )
        } else if let Some(vocation) = action.strip_prefix("vocation:") {
            let Some(settlement) = settlement.clone() else {
                continue;
            };
            (
                Some(Order::SetSettlementVocation {
                    settlement: Place::Settlement(settlement),
                    vocation: vocation.to_owned(),
                }),
                "Vocation modifiée.".to_owned(),
            )
        } else if action == "form_army" {
            let Some(city) = sim.state.as_ref().and_then(|state| {
                state
                    .province_state(&ProvinceId::new(&province).ok()?)
                    .map(|p| p.city.clone())
            }) else {
                continue;
            };
            (
                Some(Order::CreateArmy {
                    settlement: Place::Settlement(city),
                    units_from_garrison: panel.picked.iter().copied().collect(),
                    general: None,
                }),
                "Armée formée.".to_owned(),
            )
        } else if let Some(rest) = action.strip_prefix("slot:") {
            if let Some((id, root)) = rest.split_once(':') {
                // A slot cell (an empty one above all) leads to the building site: the
                // Buildings tab of its colony, its next step marked.
                panel.focus_building = match (sim.data.as_ref(), sim.state.as_ref()) {
                    (Some(data), Some(state)) => SettlementId::new(id)
                        .ok()
                        .and_then(|sid| game_api::pprovince::slot_bar(data, state, &sid))
                        .and_then(|bar| bar.slots.into_iter().find(|c| c.root == root))
                        .map(|cell| cell.next_building)
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                panel.settlement = Some(id.to_owned());
                panel.tab = 1;
                panel.dirty = true;
            }
            continue;
        } else if let Some(id) = action.strip_prefix("colony:") {
            panel.settlement = Some(id.to_owned());
            panel.tab = 1;
            panel.dirty = true;
            continue;
        } else if let Some(rate) = action.strip_prefix("tax:") {
            let rate = match rate {
                "low" => Some(TaxRate::Low),
                "normal" => Some(TaxRate::Normal),
                "high" => Some(TaxRate::High),
                _ => None,
            };
            let Ok(province_id) = ProvinceId::new(&province) else {
                continue;
            };
            (
                Some(Order::SetProvinceTax {
                    province: province_id,
                    rate,
                }),
                "Impôt de la province modifié.".to_owned(),
            )
        } else if let Some(building) = action.strip_prefix("build:") {
            let (Some(settlement), Ok(building)) = (settlement.clone(), BuildingId::new(building))
            else {
                continue;
            };
            let name = sim
                .data
                .as_ref()
                .and_then(|d| d.buildings.get(&building))
                .map_or_else(|| building.to_string(), |b| b.name.display.clone());
            (
                Some(Order::Build {
                    settlement: Place::Settlement(settlement),
                    building,
                }),
                format!("Construction lancée : {name}."),
            )
        } else if let Some(index) = action.strip_prefix("cancel:") {
            let (Some(settlement), Ok(index)) = (settlement.clone(), index.parse::<usize>()) else {
                continue;
            };
            let place = Place::Settlement(settlement);
            let order = if index == 0 {
                Order::CancelBuild { settlement: place }
            } else {
                Order::CancelQueuedBuild {
                    settlement: place,
                    index: index - 1,
                }
            };
            (Some(order), "Chantier annulé.".to_owned())
        } else if let Some(building) = action.strip_prefix("raze:") {
            let (Some(settlement), Ok(building)) = (settlement.clone(), BuildingId::new(building))
            else {
                continue;
            };
            (
                Some(Order::Demolish {
                    settlement: Place::Settlement(settlement),
                    building,
                }),
                "Bâtiment rasé.".to_owned(),
            )
        } else {
            (None, String::new())
        };
        let Some(order) = order else {
            continue;
        };
        panel.orders_issued += 1;
        panel.last_order = Some(format!("{order:?}"));
        // After « Former une armée » the army panel opens on the new army.
        let armies_before: Option<Vec<ArmyId>> = (action == "form_army")
            .then(|| sim.state.as_ref())
            .flatten()
            .map(|state| state.armies().keys().cloned().collect());
        let outcome = submit(&mut sim, order);
        sfx.write(crate::audio::PlaySfx::new(
            crate::audio::uisound::province_action_event(action, outcome.is_ok()),
        ));
        match outcome {
            Ok(()) => {
                if let (Some(before), Some(state)) = (armies_before, sim.state.as_ref())
                    && let Some(new_army) = state.armies().keys().find(|id| !before.contains(id))
                {
                    selected_army.0 = Some(new_army.clone());
                    requests.write(OpenPanel::Army(new_army.to_string()));
                }
                panel.orders_accepted += 1;
                panel.error.clear();
                panel.open_choice.clear();
                panel.picked.clear();
                toasts.write(ToastRequest::new(&success));
            }
            Err(message) => panel.error = message,
        }
        panel.dirty = true;
    }
}

/// Dump section `campaign_panel_province`: panel state, widgets and the construction queue of
/// the selected settlement as the simulation holds it.
fn dump_panel(world: &World) -> (String, Value) {
    let panel = world.resource::<ProvincePanel>();
    let sim = world.resource::<Sim>();
    let mut names: Vec<String> = Vec::new();
    if let Some(mut query) = world.try_query::<&Name>() {
        names = query
            .iter(world)
            .map(|n| n.as_str().to_owned())
            .filter(|n| n.starts_with("prov:"))
            .collect();
        names.sort();
    }
    let build_buttons = names
        .iter()
        .filter(|n| n.starts_with("prov:build:"))
        .count();
    let codex = world.resource::<CodexStore>();
    let snap = panel
        .open
        .as_deref()
        .and_then(|id| snapshot(sim, panel, codex, id, ""));
    let state = snap.as_ref().map(|s| {
        let queue: Vec<Value> = s
            .tree
            .iter()
            .flat_map(|t| t.queue.iter())
            .map(|w| json!({"building": w.building, "turns_left": w.turns_left, "total_turns": w.total_turns}))
            .collect();
        let nodes = s.tree.as_ref().map_or(0, |t| t.nodes.len());
        let available = s.tree.as_ref().map_or(0, |t| {
            t.nodes.iter().filter(|n| n.state == "available" && n.can_build).count()
        });
        let buildable_ids: Vec<&str> = s.tree.as_ref().map_or_else(Vec::new, |t| {
            t.nodes
                .iter()
                .filter(|n| n.state == "available" && n.can_build)
                .map(|n| n.building.as_str())
                .collect()
        });
        let images = world.resource::<images::BuildingImages>();
        json!({
            "images_known": images.image_count(),
            "tier": s.tier, "tier_progress": s.tier_progress,
            "tier_caption": s.tree.as_ref().map(|t| images.tier_caption(&t.kind, s.tier)),
            "bound_images": images::bound_images(images, s).iter()
                .map(|(widget, path)| json!({"widget": widget, "path": path}))
                .collect::<Vec<_>>(),
            "buildable_ids": buildable_ids,
            "province": s.sheet.id, "name": s.sheet.name, "owner": s.sheet.owner,
            "controller": s.sheet.controller, "mine": s.mine,
            "settlement": s.selected_settlement,
            "settlements": s.sheet.settlements.len(),
            "unrest": s.sheet.unrest, "income": s.sheet.income,
            "tax": s.sheet.tax.rate, "population": s.sheet.population_total,
            "tree_nodes": nodes, "buildable_now": available, "queue": queue,
            "slots": s.tree.as_ref().map(|t| json!({"used": t.slots_used, "max": t.slots_max})),
            "slot_bars": s.bars.iter().map(|b| json!({
                "settlement": b.settlement, "cells": b.slots.len(), "used": b.used, "max": b.max,
                "states": b.slots.iter().map(|c| c.state.as_str()).collect::<Vec<_>>(),
                "roots": b.slots.iter().map(|c| c.root.as_str()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "lent_text": s.lent_text,
            "focus_building": s.focus_building,
            "linked_descriptions": s.linked.keys().collect::<Vec<_>>(),
            "garrison_units": s.sheet.garrison.len(),
            "possession": s.possession.as_ref().map(|p| json!({
                "status": p.status, "cue": p.cue, "line": choices::status_line(p),
                "held": p.held, "total": p.total,
            })),
            "table": s.table.as_ref().map(|t| json!({
                "diet": t.diet, "cost": t.cost, "lent": t.lent,
                "changed_this_turn": t.changed_this_turn,
                "options": t.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            })),
            "edict": s.edict.as_ref().map(|e| json!({
                "edict": e.edict, "pending": e.pending,
                "options": e.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            })),
            "edict_editable": s.edict_editable,
            "vocation": s.vocation.as_ref().map(|v| json!({
                "vocation": v.vocation, "count": v.count,
                "options": v.options.iter().filter(|o| o.available).map(|o| o.id.as_str()).collect::<Vec<_>>(),
            })),
            "army_cap": s.army_cap, "picked": s.picked,
            "breadcrumb": s.sheet.breadcrumb.iter().map(|l| l.holder.as_str()).collect::<Vec<_>>(),
        })
    });
    (
        "campaign_panel_province".into(),
        json!({
            "open": panel.open, "open_choice": panel.open_choice, "tab": TABS[panel.tab.min(TABS.len() - 1)].0,
            "rebuilds": panel.rebuilds, "build_buttons": build_buttons,
            "orders_issued": panel.orders_issued, "orders_accepted": panel.orders_accepted,
            "last_order": panel.last_order, "error": panel.error,
            "sim": state, "widgets": names.len(),
            "widget_names": names.iter().take(60).collect::<Vec<_>>(),
        }),
    )
}
