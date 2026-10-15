//! Chronicle decision window (port of the basics of the Godot event popup): title, text and one
//! button per option; the choice is the core's `Order::ChooseEventOption`.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_campaign::Order;
use sim_campaign::chronicle::DecisionView;

use super::HudSet;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{UiTier, UiType};
use crate::ui::kit::widgets::{button_small, caption, panel_sized};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

/// Opens the decision window.
#[derive(Message, Debug, Clone)]
pub enum OpenDecision {
    /// A given pending decision (id of the core).
    Id(u32),
    /// The first blocking (dialog) decision, if any.
    FirstBlocking,
}

#[derive(Component)]
struct DecisionWindow {
    id: u32,
}

pub fn plugin(app: &mut App) {
    app.add_message::<OpenDecision>().add_systems(
        Update,
        (open_window, handle_buttons)
            .chain()
            .in_set(HudSet::Flow)
            .run_if(in_state(AppState::Campaign)),
    );
}

/// Pending decisions of the player (cheap enough to call on demand).
pub fn pending(sim: &Sim) -> Vec<DecisionView> {
    match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => state.decision_views(data, &sim.player),
        _ => Vec::new(),
    }
}

fn open_window(
    mut requests: MessageReader<OpenDecision>,
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    existing: Query<Entity, With<DecisionWindow>>,
) {
    let Some(request) = requests.read().last().cloned() else {
        return;
    };
    let views = pending(&sim);
    let view = match request {
        OpenDecision::Id(id) => views.into_iter().find(|v| v.id == id),
        OpenDecision::FirstBlocking => views
            .into_iter()
            .find(|v| !matches!(v.presentation, data_model::EventPresentation::Map)),
    };
    let Some(view) = view else { return };
    for window in &existing {
        commands.entity(window).despawn();
    }
    let body = RichStyle::body(&theme);
    commands
        .spawn((
            Name::new("hud:decision_window"),
            DecisionWindow { id: view.id },
            DespawnOnExit(AppState::Campaign),
            UiTier::Banner.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((panel_sized(&theme, 560.0), Pickable::default()))
                .with_children(|card| {
                    if let Some(path) = crate::campaign::panels::chronicle::art::decision_art(
                        crate::campaign::panels::chronicle::art::assets_root().as_deref(),
                        view.event.as_str(),
                    ) {
                        card.spawn((
                            Name::new("hud:decision_art"),
                            Node {
                                height: Val::Px(
                                    crate::campaign::panels::chronicle::art::ART_HEIGHT,
                                ),
                                aspect_ratio: Some(16.0 / 9.0),
                                align_self: AlignSelf::Center,
                                ..default()
                            },
                            ImageNode::new(assets.load(path)),
                        ));
                    }
                    card.spawn(typed(&theme, &view.title, UiType::Heading));
                    if !view.province_name.is_empty() {
                        card.spawn(caption(&theme, &view.province_name));
                    }
                    card.spawn(rich_text(&theme, &assets, &view.text, &body));
                    for option in &view.options {
                        let name = format!("hud:decision_option:{}", option.index);
                        card.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|entry| {
                            let mut button = entry.spawn(button_small(&theme, &name, &option.text));
                            if !option.allowed {
                                button.insert(bevy::ui::InteractionDisabled);
                            }
                            if !option.effects_text.is_empty() {
                                entry.spawn(caption(&theme, &option.effects_text));
                            }
                            if !option.allowed {
                                entry.spawn(caption(&theme, &option.reason));
                            }
                        });
                    }
                    card.spawn(button_small(&theme, "hud:decision_later", "Plus tard"));
                });
        });
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut commands: Commands,
    mut sim: ResMut<Sim>,
    windows: Query<(Entity, &DecisionWindow)>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Ok((window, decision)) = windows.single() else {
            continue;
        };
        if press.name == "hud:decision_later" {
            commands.entity(window).despawn();
        } else if let Some(index) = press.name.strip_prefix("hud:decision_option:") {
            let Ok(option) = index.parse::<usize>() else {
                continue;
            };
            let Some(data) = sim.data.clone() else {
                continue;
            };
            let Some(state) = sim.state.as_mut() else {
                continue;
            };
            match state.submit_order_outcome(
                &data,
                Order::ChooseEventOption {
                    decision: decision.id,
                    option,
                },
            ) {
                Ok(_) => commands.entity(window).despawn(),
                Err(error) => {
                    toasts.write(ToastRequest::new(&error.to_string()));
                }
            }
        }
    }
}
