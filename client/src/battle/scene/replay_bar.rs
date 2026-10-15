//! Replay bar and « Revoir la bataille » (port of `battle_replay_bar.gd`).
//!
//! Top of the battle screen while a replay is shown: play / pause, speeds ×1 to ×8, the time bar
//! (drag then release to jump), elapsed / duration, « Quitter le rejeu » and the core's notice
//! when the re-simulation no longer follows the recording (rules changed since). Pure view: the
//! bar sends [`ReplayControl`] messages that `replay::apply_controls` relays to the player.
//!
//! The « Revoir la bataille » button is added under « Retour à la campagne » of the result
//! screen (a child of its `ResultPage`), a campaign battle already settled by flow.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui_widgets::SliderValue;

use super::BattleSession;
use super::replay::{ReplayControl, WatchAgain, clock};
use crate::app::AppState;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_DECISION};
use crate::ui::kit::style::{self, UiTier};
use crate::ui::kit::{
    ButtonPressed, SliderChanged, Toggled, UiTheme, UiType, button_primary, button_small, caption,
    panel_sized, slider, toggle_button, typed,
};

/// Speeds of the bar.
pub const SPEEDS: [f64; 4] = [1.0, 2.0, 4.0, 8.0];

const PLAY: &str = "replay:play";
const QUIT: &str = "replay:quit";
const TIMELINE: &str = "replay:timeline";
const WATCH: &str = "replay:watch";
const BAR_WIDTH: f32 = 760.0;

/// A replay that could not be read: shown on the main menu.
#[derive(Resource, Debug, Clone)]
pub struct ReplayFailure(pub String);

/// The bar and its live parts.
#[derive(Component)]
pub struct ReplayBar;
#[derive(Component)]
struct TimeLabel;
#[derive(Component)]
struct DivergenceLabel;
#[derive(Component)]
struct SpeedButton(f64);

/// What the bar shows (read by the dump).
#[derive(Resource, Debug, Default, Clone)]
pub struct BarStats {
    pub shown: bool,
    pub time_text: String,
    pub dragging: bool,
    pub watch_button: bool,
    pub seeks: u32,
}

pub struct ReplayBarPlugin;

impl Plugin for ReplayBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BarStats>()
            .add_systems(OnEnter(AppState::Battle), reset_stats)
            .add_systems(
                Update,
                (spawn_bar, bar_buttons, drag_timeline, refresh_bar)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                (add_watch_button, watch_button_pressed)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                show_failure
                    .run_if(in_state(AppState::MainMenu))
                    .run_if(resource_exists::<ReplayFailure>),
            );
    }
}

fn reset_stats(mut stats: ResMut<BarStats>) {
    *stats = BarStats::default();
}

fn spawn_bar(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    theme: Res<UiTheme>,
    bars: Query<(), With<ReplayBar>>,
    mut stats: ResMut<BarStats>,
) {
    if !session.replaying || !bars.is_empty() {
        return;
    }
    let (title, duration) = session
        .playback()
        .map(|p| (p.title.clone(), p.duration))
        .unwrap_or_default();
    let heading = if title.is_empty() {
        "Rejeu".to_owned()
    } else {
        format!("Rejeu — {title}")
    };
    commands
        .spawn((
            Name::new("ReplayBar"),
            ReplayBar,
            DespawnOnExit(AppState::Battle),
            UiTier::Banner.index(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-BAR_WIDTH / 2.0)),
                width: Val::Px(BAR_WIDTH),
                ..default()
            },
        ))
        .with_children(|outer| {
            outer
                .spawn(panel_sized(&theme, BAR_WIDTH))
                .with_children(|bar| {
                    bar.spawn(Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|top| {
                        top.spawn((
                            typed(&theme, &heading, UiType::Body),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                        ));
                        top.spawn(button_small(&theme, PLAY, "Pause"));
                        for speed in SPEEDS {
                            top.spawn((
                                SpeedButton(speed),
                                toggle_button(
                                    &theme,
                                    &format!("replay:speed:{}", speed as u32),
                                    &format!("×{}", speed as u32),
                                    speed == 1.0,
                                ),
                            ));
                        }
                        top.spawn(button_small(&theme, QUIT, "Quitter le rejeu"));
                    });
                    bar.spawn(Node {
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|bottom| {
                        bottom.spawn(slider(
                            &theme,
                            TIMELINE,
                            0.0,
                            duration.max(0.1) as f32,
                            0.0,
                            0.1,
                            BAR_WIDTH - 220.0,
                        ));
                        bottom.spawn((
                            TimeLabel,
                            caption(&theme, &format!("0:00 / {}", clock(duration))),
                            Node {
                                min_width: Val::Px(110.0),
                                ..default()
                            },
                        ));
                    });
                    bar.spawn((
                        DivergenceLabel,
                        caption(&theme, ""),
                        Node {
                            display: Display::None,
                            max_width: Val::Px(BAR_WIDTH - 24.0),
                            ..default()
                        },
                    ))
                    .insert(TextColor(style::RUBRIC));
                });
        });
    stats.shown = true;
}

fn bar_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut controls: MessageWriter<ReplayControl>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            PLAY => {
                controls.write(ReplayControl::TogglePlay);
            }
            QUIT => {
                controls.write(ReplayControl::Quit);
            }
            name => {
                if let Some(speed) = name
                    .strip_prefix("replay:speed:")
                    .and_then(|v| v.parse::<f64>().ok())
                {
                    controls.write(ReplayControl::Speed(speed));
                }
            }
        }
    }
}

/// A drag only moves the label; the jump happens on release (`is_final`).
fn drag_timeline(
    mut changes: MessageReader<SliderChanged>,
    mut controls: MessageWriter<ReplayControl>,
    mut stats: ResMut<BarStats>,
    mut dragged: Local<Option<f32>>,
) {
    for change in changes.read() {
        if change.name != TIMELINE {
            continue;
        }
        if change.is_final {
            *dragged = None;
            stats.dragging = false;
            stats.seeks += 1;
            controls.write(ReplayControl::Seek(f64::from(change.value)));
        } else {
            *dragged = Some(change.value);
            stats.dragging = true;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh_bar(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut stats: ResMut<BarStats>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TimeLabel>>,
        Query<(&mut Text, &mut Node), With<DivergenceLabel>>,
    )>,
    speeds: Query<(Entity, &SpeedButton, &Toggled)>,
    play_buttons: Query<(&Name, &Children)>,
    mut child_texts: Query<&mut Text, (Without<TimeLabel>, Without<DivergenceLabel>)>,
    slider_query: Query<(Entity, &Name, &SliderValue)>,
) {
    if !session.replaying {
        return;
    }
    let elapsed = session.snapshot.elapsed_s;
    let (duration, message) = {
        let Some(playback) = session.playback() else {
            return;
        };
        (
            playback.duration,
            playback
                .player
                .divergence()
                .map(|d| d.message.clone())
                .unwrap_or_default(),
        )
    };
    let shown = if stats.dragging {
        slider_query
            .iter()
            .find(|(_, name, _)| name.as_str() == TIMELINE)
            .map_or(elapsed, |(_, _, value)| f64::from(value.0))
    } else {
        elapsed
    };
    let time_text = format!("{} / {}", clock(shown), clock(duration));
    for mut text in &mut texts.p0() {
        if text.0 != time_text {
            text.0.clone_from(&time_text);
        }
    }
    stats.time_text = time_text;
    for (mut text, mut node) in &mut texts.p1() {
        if text.0 != message {
            text.0.clone_from(&message);
        }
        node.display = if message.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    // « Pause » while playing, « Lecture » while paused.
    let label = if session.paused { "Lecture" } else { "Pause" };
    for (name, children) in &play_buttons {
        if name.as_str() == PLAY {
            for child in children.iter() {
                if let Ok(mut text) = child_texts.get_mut(child)
                    && text.0 != label
                {
                    text.0 = label.to_owned();
                }
            }
        }
    }
    for (entity, speed, toggled) in &speeds {
        let on = (speed.0 - session.speed).abs() < 1e-9;
        if toggled.0 != on {
            commands.entity(entity).insert(Toggled(on));
        }
    }
    if !stats.dragging {
        for (entity, name, value) in &slider_query {
            if name.as_str() == TIMELINE && (f64::from(value.0) - elapsed).abs() > 0.05 {
                commands.entity(entity).insert(SliderValue(elapsed as f32));
            }
        }
    }
}

/// « Revoir la bataille » under the return button of the result screen.
fn add_watch_button(
    mut commands: Commands,
    session: Res<BattleSession>,
    theme: Res<UiTheme>,
    pages: Query<(Entity, &Name)>,
    existing: Query<&Name>,
    mut stats: ResMut<BarStats>,
) {
    if session.replaying || stats.watch_button {
        return;
    }
    let Some((page, _)) = pages.iter().find(|(_, name)| name.as_str() == "ResultPage") else {
        return;
    };
    if existing.iter().any(|name| name.as_str() == WATCH) {
        return;
    }
    commands.spawn((
        button_primary(&theme, WATCH, "Revoir la bataille"),
        ChildOf(page),
    ));
    stats.watch_button = true;
}

fn watch_button_pressed(
    mut pressed: MessageReader<ButtonPressed>,
    mut watch: MessageWriter<WatchAgain>,
) {
    for press in pressed.read() {
        if press.name == WATCH {
            watch.write(WatchAgain);
        }
    }
}

/// The replay could not be read: a message on the main menu.
fn show_failure(
    mut commands: Commands,
    failure: Res<ReplayFailure>,
    mut queue: ResMut<ModalQueue>,
) {
    queue.request(
        "replay:failed",
        PRIORITY_DECISION,
        ModalSpec::info(
            "Rejeu",
            &format!("Ce rejeu ne peut être revu : {}.", failure.0),
        ),
    );
    commands.remove_resource::<ReplayFailure>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_are_one_to_eight() {
        assert_eq!(SPEEDS, [1.0, 2.0, 4.0, 8.0]);
    }
}
