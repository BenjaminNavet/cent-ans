//! « Les cours d'Europe délibèrent… » cartouche shown while the core resolves the end of turn on
//! its thread (port of game/scripts/map/turn_wait_indicator.gd, PO5).
//!
//! Discreet: it only appears after `SHOW_DELAY` seconds (no flicker on a fast turn), fades in over
//! `FADE`, sits at the top centre under the top bar and never takes the mouse. When the turn ends
//! it fades out (at once with « Réduire les animations » or if it never appeared). The hourglass
//! empties its sand then turns over (2 s cycle).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::HudSet;
use super::flow::TurnFlow;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::ui::kit::UiTheme;
use crate::ui::kit::modal::ReduceMotion;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::panel_illuminated;

pub const TEXT: &str = "Les cours d’Europe délibèrent…";
/// Delay before the cartouche appears (s) and fade duration (s).
pub const SHOW_DELAY: f32 = 0.15;
pub const FADE: f32 = 0.25;
/// Distance to the top edge of the screen, under the top bar.
const TOP_OFFSET: f32 = 64.0;
/// Hourglass cycle (s): sand runs for the first 80 %, the glass turns over in the last 20 %.
const CYCLE: f32 = 2.0;
const GLASS_HEIGHT: f32 = 18.0;

/// State of the indicator; pure and unit-tested.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct TurnWait {
    pub active: bool,
    pub elapsed: f32,
    pub alpha: f32,
    /// Times the cartouche became visible (dump).
    pub shown: u32,
}

impl TurnWait {
    pub fn begin(&mut self) {
        self.active = true;
        self.elapsed = 0.0;
        self.alpha = 0.0;
    }

    /// Ends the wait: the alpha then decays by `fade_out` (see [`TurnWait::tick`]).
    pub fn end(&mut self, instant: bool) {
        self.active = false;
        if instant || self.alpha <= 0.0 {
            self.alpha = 0.0;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.active {
            let was = self.alpha;
            self.elapsed += dt;
            self.alpha = ((self.elapsed - SHOW_DELAY) / FADE).clamp(0.0, 1.0);
            if was <= 0.0 && self.alpha > 0.0 {
                self.shown += 1;
            }
        } else if self.alpha > 0.0 {
            self.alpha = (self.alpha - dt / FADE).max(0.0);
        }
    }

    pub fn is_shown(&self) -> bool {
        self.alpha > 0.0
    }

    /// Sand already run (0-1) and turn-over progress (0-1) of the hourglass at `elapsed`.
    pub fn glass(elapsed: f32) -> (f32, f32) {
        let t = (elapsed % CYCLE) / CYCLE;
        ((t / 0.8).clamp(0.0, 1.0), ((t - 0.8) / 0.2).clamp(0.0, 1.0))
    }
}

#[derive(Component)]
struct WaitRoot;

#[derive(Component)]
struct WaitLabel;

#[derive(Component)]
struct WaitGlass;

#[derive(Component)]
struct WaitSand;

pub fn plugin(app: &mut App) {
    app.init_resource::<TurnWait>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), (reset, spawn))
        .add_systems(Update, (track, animate).chain().in_set(HudSet::Ui));
}

fn reset(mut wait: ResMut<TurnWait>) {
    *wait = TurnWait::default();
}

fn spawn(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Name::new("hud:turn_wait"),
            WaitRoot,
            DespawnOnExit(AppState::Campaign),
            UiTier::Banner.index(),
            panel_illuminated(&theme),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(TOP_OFFSET),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-130.0)),
            width: Val::Px(260.0),
            min_height: Val::Px(0.0),
            padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                WaitGlass,
                Pickable::IGNORE,
                UiTransform::default(),
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(GLASS_HEIGHT),
                    border: UiRect::all(Val::Px(1.2)),
                    align_items: AlignItems::FlexEnd,
                    ..default()
                },
                BorderColor::all(style::INK),
            ))
            .with_child((
                WaitSand,
                Pickable::IGNORE,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.78, 0.58, 0.25)),
            ));
            row.spawn((
                WaitLabel,
                Pickable::IGNORE,
                Text::new(TEXT),
                theme.text_font(false, true, UiType::Body.size() + 1.0),
                TextColor(style::RUBRIC),
            ));
        });
}

fn track(
    flow: Res<TurnFlow>,
    time: Res<Time>,
    reduce: Option<Res<ReduceMotion>>,
    mut wait: ResMut<TurnWait>,
) {
    let running = flow.running();
    if running && !wait.active {
        wait.begin();
    } else if !running && wait.active {
        wait.end(reduce.is_some_and(|r| r.0));
    }
    wait.tick(time.delta_secs());
}

#[allow(clippy::type_complexity)]
fn animate(
    wait: Res<TurnWait>,
    mut root: Query<(&mut Visibility, Option<&mut ImageNode>), With<WaitRoot>>,
    mut label: Query<&mut TextColor, With<WaitLabel>>,
    mut glass: Query<&mut UiTransform, With<WaitGlass>>,
    mut sand: Query<&mut Node, With<WaitSand>>,
) {
    let Ok((mut visibility, image)) = root.single_mut() else {
        return;
    };
    *visibility = if wait.is_shown() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Some(mut image) = image {
        image.color = image.color.with_alpha(wait.alpha);
    }
    if let Ok(mut color) = label.single_mut() {
        color.0 = style::RUBRIC.with_alpha(wait.alpha);
    }
    let (flow, turn) = TurnWait::glass(wait.elapsed);
    if let Ok(mut transform) = glass.single_mut() {
        transform.rotation = Rot2::radians(turn * std::f32::consts::PI);
    }
    if let Ok(mut node) = sand.single_mut() {
        node.height = Val::Px((GLASS_HEIGHT - 2.4) * flow);
    }
}

fn dump(world: &World) -> (String, Value) {
    let wait = world.resource::<TurnWait>();
    (
        "turn_wait".to_owned(),
        json!({
            "active": wait.active, "shown": wait.shown, "visible": wait.is_shown(),
            "alpha": wait.alpha, "elapsed": wait.elapsed,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appears_after_the_delay_and_fades_in() {
        let mut wait = TurnWait::default();
        wait.begin();
        wait.tick(0.1);
        assert!(!wait.is_shown(), "a fast turn never flashes the cartouche");
        wait.tick(0.1);
        assert!(wait.is_shown());
        wait.tick(1.0);
        assert_eq!(wait.alpha, 1.0);
        assert_eq!(wait.shown, 1);
    }

    #[test]
    fn fades_out_unless_instant_or_never_shown() {
        let mut wait = TurnWait::default();
        wait.begin();
        wait.tick(1.0);
        wait.end(false);
        wait.tick(FADE / 2.0);
        assert!(wait.is_shown() && wait.alpha < 1.0);
        wait.tick(FADE);
        assert!(!wait.is_shown());
        wait.begin();
        wait.tick(1.0);
        wait.end(true);
        assert!(!wait.is_shown());
    }

    #[test]
    fn hourglass_runs_then_turns() {
        assert_eq!(TurnWait::glass(0.0), (0.0, 0.0));
        let (flow, turn) = TurnWait::glass(1.6);
        assert!((flow - 1.0).abs() < 1e-5 && turn.abs() < 1e-5);
        let (_, turn) = TurnWait::glass(1.8);
        assert!((turn - 0.5).abs() < 1e-4);
    }
}
