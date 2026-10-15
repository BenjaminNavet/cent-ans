//! Objectives panel (port of the panel of `victory_controller.gd`): the faction's historical
//! objectives with their progress, the deadline and score, the missions in progress, and the
//! choice between a short and a long campaign (ADR 0332, `game_api::progress::set_victory_length`).
//!
//! Opens on [`OpenObjectives`] (toggle) or key O; closes on Escape, `objectives:close`,
//! `OpenPanel::CloseAll`. Widget names: `objectives:close`, `objectives:length:short|long`.
//! Dev hook: `CENT_ANS_OPEN_OBJECTIVES=1` opens it on entering the campaign.

use bevy::prelude::*;
use game_api::progress::{ObjectivesView, objectives_view, set_victory_length};
use serde_json::{Value, json};

use super::{OpenObjectives, ProgressTexts};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::campaign::hud::missions::{Missions, left_text};
use crate::campaign::panels::OpenPanel;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{
    button_small, caption, gauge, panel_sized, scroll_list, separator, toggle_button,
};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const PANEL_WIDTH: f32 = 640.0;
const LIST_HEIGHT: f32 = 380.0;

#[derive(Resource, Default)]
pub struct ObjectivesPanel {
    pub open: bool,
    root: Option<Entity>,
    dirty: bool,
    /// Turn the panel was built for.
    built_turn: u32,
    /// Seasons the panel has been built (dump).
    pub rebuilds: u32,
    /// `short` / `long` changes accepted (dump).
    pub length_changes: u32,
    missions_len: usize,
}

#[derive(Component)]
struct ObjectivesRoot;

pub fn plugin(app: &mut App) {
    app.init_resource::<ObjectivesPanel>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
        .add_systems(
            Update,
            (
                open_close.before(crate::menu::pause::pause_escape),
                handle_buttons,
                watch,
                rebuild,
            )
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn reset_on_enter(mut panel: ResMut<ObjectivesPanel>) {
    let hook = std::env::var("CENT_ANS_OPEN_OBJECTIVES").is_ok();
    *panel = ObjectivesPanel {
        open: hook,
        dirty: hook,
        ..default()
    };
}

fn close_on_exit(mut commands: Commands, mut panel: ResMut<ObjectivesPanel>) {
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    panel.open = false;
}

fn open_close(
    mut panel: ResMut<ObjectivesPanel>,
    mut open: MessageReader<OpenObjectives>,
    mut panels: MessageReader<OpenPanel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut edges: ResMut<KeyEdges>,
) {
    for _ in open.read() {
        panel.open = !panel.open;
        panel.dirty = true;
    }
    for message in panels.read() {
        if matches!(message, OpenPanel::CloseAll) {
            panel.open = false;
        }
    }
    if keys.just_pressed(KeyCode::KeyO) {
        panel.open = !panel.open;
        panel.dirty = true;
    }
    if panel.open && edges.escape {
        panel.open = false;
        edges.escape = false;
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut panel: ResMut<ObjectivesPanel>,
    mut sim: ResMut<Sim>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("objectives:") else {
            continue;
        };
        if !panel.open {
            continue;
        }
        if name == "close" {
            panel.open = false;
        } else if let Some(length) = name.strip_prefix("length:")
            && let Some(state) = sim.state.as_mut()
            && set_victory_length(state, length)
        {
            panel.length_changes += 1;
            panel.dirty = true;
        }
    }
}

/// Rebuilds when a turn passes, or the missions change, under an open panel.
fn watch(
    mut panel: ResMut<ObjectivesPanel>,
    mut advanced: MessageReader<TurnAdvanced>,
    missions: Res<Missions>,
) {
    if advanced.read().count() > 0 || missions.active.len() != panel.missions_len {
        panel.dirty = true;
    }
}

fn rebuild(
    mut commands: Commands,
    mut panel: ResMut<ObjectivesPanel>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    missions: Res<Missions>,
    texts: Res<ProgressTexts>,
) {
    if !panel.open {
        if let Some(root) = panel.root.take() {
            commands.entity(root).despawn();
        }
        return;
    }
    if !panel.dirty && panel.root.is_some() {
        return;
    }
    panel.dirty = false;
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    let view = objectives_view(data, state);
    panel.built_turn = state.turn();
    panel.missions_len = missions.active.len();
    panel.rebuilds += 1;
    panel.root = Some(spawn(
        &mut commands,
        &theme,
        &view,
        &missions,
        &texts.plain("campaign_score"),
    ));
}

fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    view: &ObjectivesView,
    missions: &Missions,
    score_tip: &str,
) -> Entity {
    let theme = theme.clone();
    commands
        .spawn((
            Name::new("ObjectivesPanel"),
            ObjectivesRoot,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            UiTier::Panel.index(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|page| {
                    page.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn(typed(&theme, "Objectifs de campagne", UiType::Title));
                        bar.spawn(button_small(&theme, "objectives:close", "Fermer"));
                    });
                    // The deadline and score line explains the score on hover (`campaign_score`).
                    page.spawn((
                        Name::new("objectives:score"),
                        Node::default(),
                        Tooltip::new(score_tip),
                        children![caption(&theme, &view.header_line())],
                    ));
                    page.spawn(Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(caption(&theme, "Durée de la campagne :"));
                        row.spawn(toggle_button(
                            &theme,
                            "objectives:length:short",
                            "Courte",
                            view.length == "short",
                        ));
                        row.spawn(toggle_button(
                            &theme,
                            "objectives:length:long",
                            "Longue",
                            view.length == "long",
                        ));
                    });
                    page.spawn(separator(&theme));
                    page.spawn(scroll_list(&theme, "objectives:list", LIST_HEIGHT))
                        .with_children(|list| {
                            if view.objectives.is_empty() {
                                list.spawn(caption(
                                    &theme,
                                    "Cette faction n’a pas d’objectifs historiques : survivre et prospérer.",
                                ));
                            }
                            for objective in &view.objectives {
                                list.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    margin: UiRect::bottom(Val::Px(8.0)),
                                    ..default()
                                })
                                .with_children(|row| {
                                    row.spawn((
                                        Name::new(format!("objective:{}", objective.id)),
                                        Text::new(format!(
                                            "{} {} — {}",
                                            if objective.done { "✔" } else { "☐" },
                                            objective.title,
                                            objective.progress
                                        )),
                                        theme.text_font(true, false, UiType::Body.size()),
                                        TextColor(if objective.done {
                                            style::GOOD
                                        } else {
                                            style::INK_SOFT
                                        }),
                                    ));
                                    row.spawn(caption(&theme, &objective.description));
                                });
                            }
                            list.spawn(separator(&theme));
                            list.spawn(typed(&theme, "Missions", UiType::Heading));
                            if missions.active.is_empty() {
                                list.spawn(caption(
                                    &theme,
                                    "Aucune mission en cours : de nouvelles viendront selon la situation du royaume.",
                                ));
                            }
                            for mission in &missions.active {
                                list.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    margin: UiRect::bottom(Val::Px(8.0)),
                                    ..default()
                                })
                                .with_children(|row| {
                                    row.spawn((
                                        Name::new(format!("mission:{}", mission.id)),
                                        Text::new(format!(
                                            "☐ {} — {}",
                                            mission.title, mission.progress
                                        )),
                                        theme.text_font(true, false, UiType::Body.size()),
                                        TextColor(style::INK_SOFT),
                                    ));
                                    row.spawn(caption(&theme, &mission.objective));
                                    row.spawn(gauge(
                                        &theme,
                                        &format!("mission_bar:{}", mission.id),
                                        mission.progress_ratio as f32,
                                        PANEL_WIDTH - 120.0,
                                    ));
                                    let mut terms = format!(
                                        "Échéance : {} ({}) — Récompense : {}",
                                        mission.deadline,
                                        left_text(mission.turns_left),
                                        mission.reward
                                    );
                                    if !mission.source.is_empty() {
                                        terms.push_str(&format!("\nSource : {}", mission.source));
                                    }
                                    row.spawn(caption(&theme, &terms));
                                });
                            }
                        });
                });
        })
        .id()
}

fn dump(world: &World) -> (String, Value) {
    let panel = world.resource::<ObjectivesPanel>();
    let sim = world.resource::<Sim>();
    // Live view, so the evidence does not depend on the panel being open.
    let live = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => Some(objectives_view(data, state)),
        _ => None,
    };
    (
        "progress_objectives".to_owned(),
        json!({
            "open": panel.open,
            "visible": panel.root.is_some(),
            "built_turn": panel.built_turn,
            "rebuilds": panel.rebuilds,
            "length_changes": panel.length_changes,
            "score_tip": world.resource::<ProgressTexts>().plain("campaign_score"),
            "view": live.as_ref().map(|v| json!({
                "state": v.state,
                "length": v.length,
                "end_year": v.end_year,
                "score": v.score,
                "hold_turns": v.hold_turns,
                "streak": v.streak,
                "done": v.done_count,
                "header": v.header_line(),
                "objectives": v.objectives.iter().map(|o| json!({
                    "id": o.id, "done": o.done, "progress": o.progress,
                })).collect::<Vec<_>>(),
            })),
        }),
    )
}
