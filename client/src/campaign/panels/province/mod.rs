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
//! `prov:raze:<building>`, `prov:tax:<low|normal|high|default>`, `prov:colony:<settlement>`,
//! `prov:recruit`.

mod labels;
mod view;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use data_model::{BuildingId, ProvinceId, SettlementId};
use serde_json::{Value, json};
use sim_campaign::{Order, Place, TaxRate};

use self::labels::UiLabels;
use self::view::{SETTLEMENT_GROUP, Snapshot, TAB_GROUP, TABS};
use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::provinces::ProvinceClicked;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::tooltip::TooltipState;
use crate::ui::kit::widgets::{ListSelected, TabSelected};
use crate::ui::kit::{ButtonPressed, UiTheme};

const ENV_OPEN: &str = "CENT_ANS_OPEN_PROVINCE";
const PANEL_WIDTH: f32 = 400.0;
const POLL_SECONDS: f32 = 0.5;

pub struct PanelProvincePlugin;

impl Plugin for PanelProvincePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProvincePanel>()
            .insert_resource(UiLabels::load())
            .register_dump(dump_panel)
            .add_systems(
                PreUpdate,
                close_on_escape.run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                Update,
                (
                    dev_open,
                    open_requests,
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
}

#[derive(Component)]
struct ProvincePanelRoot;

fn reset_panel(mut panel: ResMut<ProvincePanel>) {
    *panel = ProvincePanel::default();
}

impl ProvincePanel {
    fn open_province(&mut self, id: &str) {
        if self.open.as_deref() != Some(id) {
            self.settlement = None;
            self.error.clear();
        }
        self.open = Some(id.to_owned());
        self.dirty = true;
    }

    fn close(&mut self) {
        self.open = None;
        self.dirty = true;
        self.last.clear();
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

fn close_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    tooltips: Res<TooltipState>,
    mut panel: ResMut<ProvincePanel>,
) {
    if keys.just_pressed(KeyCode::Escape) && panel.open.is_some() && tooltips.chain.is_empty() {
        panel.close();
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
            OpenPanel::CloseAll | OpenPanel::Army(_) => panel.close(),
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
) {
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
            panel.dirty = true;
        }
    }
}

/// Builds the snapshot of `province` from `game-api`; `None` for an unknown province.
fn snapshot(sim: &Sim, province: &str, selected: Option<&str>, error: &str) -> Option<Snapshot> {
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
    Some(Snapshot {
        mine: sheet.controller == sim.player.as_str(),
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
    theme: Res<UiTheme>,
    labels: Res<UiLabels>,
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
    let Some(snap) = snapshot(&sim, &province, panel.settlement.as_deref(), &panel.error) else {
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
            DespawnOnExit(AppState::Campaign),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(56.0),
                width: Val::Px(PANEL_WIDTH),
                max_height: Val::Percent(88.0),
                padding: theme.padding("panel"),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| view::build(root, &theme, &labels, &snap, tab));
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
        match submit(&mut sim, order) {
            Ok(()) => {
                panel.orders_accepted += 1;
                panel.error.clear();
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
    let snap = panel
        .open
        .as_deref()
        .and_then(|id| snapshot(sim, id, panel.settlement.as_deref(), ""));
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
        json!({
            "province": s.sheet.id, "name": s.sheet.name, "owner": s.sheet.owner,
            "controller": s.sheet.controller, "mine": s.mine,
            "settlement": s.selected_settlement,
            "settlements": s.sheet.settlements.len(),
            "unrest": s.sheet.unrest, "income": s.sheet.income,
            "tax": s.sheet.tax.rate, "population": s.sheet.population_total,
            "tree_nodes": nodes, "buildable_now": available, "queue": queue,
            "slots": s.tree.as_ref().map(|t| json!({"used": t.slots_used, "max": t.slots_max})),
            "garrison_units": s.sheet.garrison.len(),
        })
    });
    (
        "campaign_panel_province".into(),
        json!({
            "open": panel.open, "tab": TABS[panel.tab.min(TABS.len() - 1)].0,
            "rebuilds": panel.rebuilds, "build_buttons": build_buttons,
            "orders_issued": panel.orders_issued, "orders_accepted": panel.orders_accepted,
            "last_order": panel.last_order, "error": panel.error,
            "sim": state, "widgets": names.len(),
            "widget_names": names.iter().take(60).collect::<Vec<_>>(),
        }),
    )
}
