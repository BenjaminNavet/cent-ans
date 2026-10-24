//! Act transition screen (ADR 0784): when an act closes, a vellum card gives the bilan of the act
//! (score, objectives, pressures, the three main causes of the journal), announces the next act
//! and lets the player choose the policy of the household. All figures and texts come from
//! `game_api::acts` (data `rules/acts.json`).
//!
//! Widget names: `acts:policy:<id>` (choose a policy), `acts:continue`. Dev hook:
//! `CENT_ANS_FORCE_ACT=1` jumps the calendar past the first act after the first frame. Dump
//! section: `progress_acts`.

use bevy::prelude::*;
use game_api::acts::{TransitionView, choose_policy, mark_transition_seen, transition_view};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_primary, caption, panel_illuminated};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

#[derive(Resource, Default)]
pub struct ActsScreen {
    root: Option<Entity>,
    pub shown: u32,
    pub view: Option<TransitionView>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<ActsScreen>()
        .register_dump(dump)
        .add_systems(
            Update,
            (force_transition, watch, handle_buttons)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn close_on_exit(mut commands: Commands, mut screen: ResMut<ActsScreen>) {
    if let Some(root) = screen.root.take() {
        commands.entity(root).despawn();
    }
}

/// Dev hook: the calendar jumps past the first act (see the module doc).
fn force_transition(mut sim: ResMut<Sim>, mut done: Local<bool>) {
    if *done {
        return;
    }
    if std::env::var("CENT_ANS_FORCE_ACT").is_err() {
        *done = true;
        return;
    }
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return;
    };
    if !state.acts.started {
        return;
    }
    *done = true;
    state.year = data.acts_rules.acts[0].end_year + 1;
    state.end_turn(&data);
}

fn watch(
    mut commands: Commands,
    mut screen: ResMut<ActsScreen>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
) {
    if !sim.is_changed() || screen.root.is_some() {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    // The end screen takes over once the campaign has an outcome.
    if state.outcome.is_some() {
        return;
    }
    let Some(view) = transition_view(data, state) else {
        return;
    };
    screen.root = Some(spawn(&mut commands, &theme, &view));
    screen.shown += 1;
    screen.view = Some(view);
}

fn spawn(commands: &mut Commands, theme: &UiTheme, view: &TransitionView) -> Entity {
    let theme = theme.clone();
    let objectives: String = view
        .objectives
        .iter()
        .map(|(title, met)| format!("{} {title}", if *met { "[x]" } else { "[ ]" }))
        .collect::<Vec<_>>()
        .join("\n");
    commands
        .spawn((
            Name::new("ActTransition"),
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(style::FE_NIGHT.with_alpha(0.85)),
            UiTier::Modal.index(),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: Val::Px(760.0),
                ..default()
            })
            .with_children(|slot| {
                slot.spawn(panel_illuminated(&theme)).with_children(|card| {
                    card.spawn((
                        Name::new("ActTitle"),
                        Text::new(format!("{} — {}", view.title, view.act_name)),
                        theme.title_font(32.0),
                        TextColor(style::RUBRIC),
                    ));
                    card.spawn(typed(
                        &theme,
                        &format!("{}\n{}", view.closing, view.ended_line),
                        UiType::Body,
                    ));
                    card.spawn(caption(
                        &theme,
                        &format!("{}\n{}", view.score_line, view.grade_line),
                    ));
                    card.spawn(caption(&theme, &objectives));
                    if !view.pressures.is_empty() {
                        card.spawn(caption(&theme, &view.pressures.join("\n")));
                    }
                    if !view.causes.is_empty() {
                        card.spawn(typed(&theme, &view.causes_title, UiType::Body));
                        card.spawn(caption(&theme, &view.causes.join("\n")));
                    }
                    if let Some(next) = &view.next_name {
                        card.spawn(typed(
                            &theme,
                            &format!("{next} — {}\n{}", view.next_subtitle, view.next_intro),
                            UiType::Body,
                        ));
                        card.spawn(caption(&theme, &view.policy_prompt));
                        card.spawn(Node {
                            column_gap: Val::Px(12.0),
                            justify_content: JustifyContent::Center,
                            flex_wrap: FlexWrap::Wrap,
                            ..default()
                        })
                        .with_children(|row| {
                            for policy in &view.policies {
                                let label = if policy.chosen {
                                    format!("> {}", policy.name)
                                } else {
                                    policy.name.clone()
                                };
                                row.spawn(button_primary(
                                    &theme,
                                    &format!("acts:policy:{}", policy.id),
                                    &label,
                                ));
                            }
                        });
                        let chosen = view
                            .policies
                            .iter()
                            .find(|p| p.chosen)
                            .map_or_else(|| view.policy_prompt.clone(), |p| p.description.clone());
                        card.spawn(caption(&theme, &chosen));
                    }
                    card.spawn(button_primary(&theme, "acts:continue", "Continuer"));
                });
            });
        })
        .id()
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut commands: Commands,
    mut screen: ResMut<ActsScreen>,
    mut sim: ResMut<Sim>,
    theme: Res<UiTheme>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if let Some(policy) = name.strip_prefix("acts:policy:") {
            let Some(data) = sim.data.clone() else {
                continue;
            };
            let Some(state) = sim.state.as_mut() else {
                continue;
            };
            if choose_policy(&data, state, policy) {
                if let Some(root) = screen.root.take() {
                    commands.entity(root).despawn();
                }
                if let Some(view) = transition_view(&data, state) {
                    screen.root = Some(spawn(&mut commands, &theme, &view));
                    screen.view = Some(view);
                }
            }
        } else if name == "acts:continue" {
            if let Some(root) = screen.root.take() {
                commands.entity(root).despawn();
            }
            if let Some(state) = sim.state.as_mut() {
                mark_transition_seen(state);
            }
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let screen = world.resource::<ActsScreen>();
    (
        "progress_acts".to_owned(),
        json!({
            "visible": screen.root.is_some(),
            "shown": screen.shown,
            "view": screen.view.as_ref().map(|v| json!({
                "title": v.title, "act_name": v.act_name, "ended_by": v.ended_by,
                "score": v.score, "reign_score": v.reign_score,
                "objectives": v.objectives, "pressures": v.pressures,
                "causes": v.causes, "next_name": v.next_name,
                "policies": v.policies.iter().map(|p| json!({"id": p.id, "chosen": p.chosen})).collect::<Vec<_>>(),
            })),
        }),
    )
}
