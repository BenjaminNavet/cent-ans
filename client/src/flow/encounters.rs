//! Map encounters (`encounter_controller.gd`, `encounter_window.gd`): site markers on the
//! campaign map and the choice window opened when a player army meets a site.
//!
//! Sites, options, availability and reasons all come from the core
//! (`game_api::flow::{encounter_sites, pending_encounters, choose_encounter_option}`). A marker
//! is a small gold button projected on the map; clicking it with one of the player's armies
//! selected marches that army to the site (an ordinary move order). The window is a modal
//! with one button per option (effects underneath, unavailable options greyed with their
//! reason) and « Plus tard ».
//! Not ported: encounter illustrations (the `assets/encounters/` folder does not exist), the
//! hover bubble of the marker, the readability layer rules (`MapReadability.sign_shown`: markers
//! are always shown).

use std::collections::HashSet;

use crate::ui::kit::viewport_to_ui;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::UiScale;
use game_api::flow::{EncounterSiteView, PendingEncounterView};

use super::{FlowCommand, FlowState, Stage};
use crate::app::AppState;
use crate::campaign::armies::ArmyCommand;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::modal::{ModalQueue, ToastRequest};
use crate::ui::kit::style::{UiTier, UiType};
use crate::ui::kit::widgets::{button_icon, button_small, caption, panel_sized};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const MARKER_ICON: &str = "icons/ink/encounter.png";
pub const MARKER_PREFIX: &str = "flow:site:";
pub const OPTION_PREFIX: &str = "flow:encounter_option:";
pub const LATER: &str = "flow:encounter_later";

/// Marker of a site on the map.
#[derive(Component, Debug, Clone)]
pub struct SiteMarker {
    pub id: u32,
    /// Map pixel of the site (world x, z).
    pub point: Vec2,
    pub title: String,
}

/// The open choice window.
#[derive(Component, Debug, Clone)]
pub struct EncounterWindow {
    pub site: u32,
    pub army: String,
    pub title: String,
    pub options: usize,
}

#[derive(Component)]
pub struct SiteLayer;

/// Key of a pending encounter, as `EncounterController.key_of`.
pub fn key_of(army: &str, site: u32) -> String {
    format!("{army}:{site}")
}

pub fn spawn_layer(mut commands: Commands) {
    commands.spawn((
        Name::new("flow:site_layer"),
        SiteLayer,
        DespawnOnExit(AppState::Campaign),
        // World markers go under every piece of UI (HUD = tier 0).
        GlobalZIndex(-1),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// Rebuilds the markers when the set of visible sites changes.
pub fn sync_markers(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    layer: Query<Entity, With<SiteLayer>>,
    markers: Query<(Entity, &SiteMarker)>,
) {
    if !sim.is_changed() {
        return;
    }
    let (Some(data), Some(state), Ok(layer)) =
        (sim.data.as_ref(), sim.state.as_ref(), layer.single())
    else {
        return;
    };
    let sites: Vec<EncounterSiteView> = game_api::flow::encounter_sites(data, state);
    let want: HashSet<u32> = sites.iter().map(|s| s.id).collect();
    let have: HashSet<u32> = markers.iter().map(|(_, m)| m.id).collect();
    for (entity, marker) in &markers {
        if !want.contains(&marker.id) {
            commands.entity(entity).despawn();
        }
    }
    for site in sites.iter().filter(|s| !have.contains(&s.id)) {
        let mut marker_commands = commands.spawn((
            button_icon(
                &theme,
                &assets,
                &format!("{MARKER_PREFIX}{}", site.id),
                MARKER_ICON,
                "",
            ),
            SiteMarker {
                id: site.id,
                point: Vec2::new(site.point[0], site.point[1]),
                title: site.title.clone(),
            },
            Visibility::Hidden,
        ));
        marker_commands
            .entry::<Node>()
            .and_modify(|mut node| node.position_type = PositionType::Absolute);
        let marker = marker_commands.id();
        commands.entity(layer).add_child(marker);
    }
}

/// Places each marker on the projected site.
pub fn place_markers(
    camera: Query<(&Camera, &GlobalTransform), With<CampaignCamera>>,
    ground: GroundHeight,
    mut markers: Query<(&SiteMarker, &mut Node, &mut Visibility)>,
    ui_scale: Res<UiScale>,
) {
    let Some((camera, transform)) = camera.iter().next() else {
        return;
    };
    for (marker, mut node, mut visibility) in &mut markers {
        let y = ground.at(marker.point.x, marker.point.y).unwrap_or(0.0);
        let world = Vec3::new(marker.point.x, y, marker.point.y);
        match camera.world_to_viewport(transform, world) {
            Ok(screen) => {
                let screen = viewport_to_ui(screen, &ui_scale);
                node.left = Val::Px(screen.x - 14.0);
                node.top = Val::Px(screen.y - 34.0);
                *visibility = Visibility::Inherited;
            }
            Err(_) => *visibility = Visibility::Hidden,
        }
    }
}

/// Click on a marker: the selected army marches to the site.
pub fn marker_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    markers: Query<&SiteMarker>,
    selected: Res<SelectedArmy>,
    sim: Res<Sim>,
    mut moves: MessageWriter<ArmyCommand>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Some(id) = press
            .name
            .strip_prefix(MARKER_PREFIX)
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        let Some(marker) = markers.iter().find(|m| m.id == id) else {
            continue;
        };
        let own_army = selected.0.as_ref().filter(|army| {
            sim.state
                .as_ref()
                .and_then(|s| s.army(army))
                .is_some_and(|a| a.faction == sim.player)
        });
        match own_army {
            Some(army) => {
                moves.write(ArmyCommand::Move {
                    army: army.clone(),
                    target: [marker.point.x, marker.point.y],
                });
            }
            None => {
                toasts.write(ToastRequest::new(&format!(
                    "Sélectionnez d’abord une de vos armées pour marcher vers « {} ».",
                    marker.title
                )));
            }
        }
    }
}

/// Opens the window on the first pending encounter not put off with « Plus tard »; not during
/// the end of turn, a modal, a battle dialog or a battle launch.
#[allow(clippy::too_many_arguments)]
pub fn offer_pending(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    queue: Res<ModalQueue>,
    turn: Res<crate::campaign::hud::flow::TurnFlow>,
    mut flow: ResMut<FlowState>,
    windows: Query<(Entity, &EncounterWindow)>,
    battle_dialog: Query<(), With<super::dialog::PreBattleDialog>>,
) {
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let pending: Vec<PendingEncounterView> = game_api::flow::pending_encounters(data, state);
    // An open window whose encounter is gone (answered elsewhere) closes.
    for (entity, window) in &windows {
        let key = key_of(&window.army, window.site);
        if !pending
            .iter()
            .any(|p| key_of(p.army.as_str(), p.site) == key)
        {
            commands.entity(entity).despawn();
        }
    }
    if !windows.is_empty() || flow.stage != Stage::Idle {
        return;
    }
    let Some(first) = pending
        .iter()
        .find(|p| !flow.dismissed.contains(&key_of(p.army.as_str(), p.site)))
    else {
        return;
    };
    if turn.running() || queue.is_busy() || !battle_dialog.is_empty() {
        return;
    }
    flow.stats.encounter_windows += 1;
    spawn_window(&mut commands, &theme, first, pending.len());
}

fn spawn_window(
    commands: &mut Commands,
    theme: &UiTheme,
    view: &PendingEncounterView,
    count: usize,
) {
    let window = EncounterWindow {
        site: view.site,
        army: view.army.to_string(),
        title: view.title.clone(),
        options: view.options.len(),
    };
    commands
        .spawn((
            Name::new("flow:encounter_window"),
            window,
            DespawnOnExit(AppState::Campaign),
            UiTier::Modal.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.02, 0.5)),
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(theme, 620.0)).with_children(|card| {
                card.spawn(caption(theme, "✠ Rencontre en chemin"));
                card.spawn(typed(theme, &view.title, UiType::Title));
                let mut meta = Vec::new();
                if !view.army_name.is_empty() {
                    meta.push(view.army_name.clone());
                }
                if !view.province_name.is_empty() {
                    meta.push(view.province_name.clone());
                }
                meta.push("à décider avant la fin du tour".to_owned());
                card.spawn(caption(theme, &meta.join(" — ")));
                card.spawn((
                    Name::new("flow:encounter_text"),
                    Text::new(view.text.clone()),
                    theme.text_font(false, true, UiType::Body.size()),
                    TextColor(theme.text),
                    Node {
                        max_width: Val::Px(580.0),
                        ..default()
                    },
                ));
                for option in &view.options {
                    card.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|entry| {
                        let mut label = option.label.clone();
                        if option.default {
                            label += "  (par défaut)";
                        }
                        let mut button = entry.spawn(button_small(
                            theme,
                            &format!("{OPTION_PREFIX}{}", option.index),
                            &label,
                        ));
                        if !option.available {
                            button.insert(bevy::ui::InteractionDisabled);
                        }
                        let mut lines = Vec::new();
                        if !option.effects_text.is_empty() {
                            lines.push(option.effects_text.replace('\n', " · "));
                        }
                        if !option.available
                            && let Some(reason) = &option.reason
                        {
                            lines.push(format!("Impossible : {reason}"));
                        }
                        if !lines.is_empty() {
                            entry.spawn((
                                caption(theme, &format!("   {}", lines.join(" — "))),
                                Node {
                                    max_width: Val::Px(580.0),
                                    ..default()
                                },
                            ));
                        }
                    });
                }
                if count > 1 {
                    card.spawn(caption(theme, &format!("Rencontre 1 sur {count}")));
                }
                card.spawn(button_small(theme, LATER, "Plus tard"));
            });
        });
}

/// Buttons of the window: an option is the core's choice, « Plus tard » puts it off.
pub fn window_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    windows: Query<&EncounterWindow>,
    mut flow: ResMut<FlowState>,
    mut commands: MessageWriter<FlowCommand>,
) {
    let Ok(window) = windows.single() else {
        pressed.clear();
        return;
    };
    for press in pressed.read() {
        if let Some(index) = press
            .name
            .strip_prefix(OPTION_PREFIX)
            .and_then(|s| s.parse::<usize>().ok())
        {
            commands.write(FlowCommand::ChooseEncounter {
                army: window.army.clone(),
                site: window.site,
                option: index,
            });
        } else if press.name == LATER {
            flow.dismissed.insert(key_of(&window.army, window.site));
            flow.stats.encounters_postponed += 1;
            commands.write(FlowCommand::CloseEncounterWindow);
        }
    }
}
