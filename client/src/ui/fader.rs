//! Scene fader (port of game/scripts/ui/scene_fader.gd + scene_fader_layer.gd, ADR 0097).
//!
//! A full-screen parchment-black veil above everything (`GlobalZIndex` 128, above the loading
//! card at 110 and the tooltips). Two ways in:
//! - [`FadeToState`] message: fade to black (0,25 s), switch [`AppState`], hold the veil two frames
//!   while the new screen builds, fade back in. Repeated requests during a fade are ignored
//!   (double click on a button).
//! - Any [`AppState`] transition made without the message (scripts, `--state`, flow code) is
//!   covered after the fact: the veil snaps to opaque and fades out, so no state change pops.
//!
//! [`ReduceMotion`] shortens every fade to 0,06 s; `CENT_ANS_NO_FADE=1` makes them instant (proofs).
//! While the veil is visible it blocks the pointer. Improvement over Godot: the fade-in after
//! unrequested transitions is automatic; no caller needs to pass through a facade.

use bevy::prelude::*;
use bevy::state::state::StateTransitionEvent;
use serde_json::json;

use super::kit::modal::ReduceMotion;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;

/// Veil colour: near-black brown ink, never pure black (`COVER_COLOR`).
pub const COVER_COLOR: Color = Color::srgb(0.075, 0.056, 0.038);
pub const FADE_SECONDS: f32 = 0.25;
pub const REDUCED_SECONDS: f32 = 0.06;
/// Frames the veil stays opaque after a switch, so the new screen builds under it.
const HOLD_FRAMES: u8 = 2;
const FADER_Z: i32 = 128;

/// Asks for a fade-out, a switch to the state, and a fade-in.
#[derive(Message, Debug, Clone, Copy)]
pub struct FadeToState(pub AppState);

/// Veil state (pure; stepped by [`Fader::step`]).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Fader {
    pub alpha: f32,
    pub target: f32,
    pub pending: Option<AppState>,
    hold: u8,
    instant: bool,
    pub completed: usize,
    pub last_state: Option<AppState>,
}

impl Default for Fader {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            target: 0.0,
            pending: None,
            hold: 0,
            instant: false,
            completed: 0,
            last_state: None,
        }
    }
}

/// What a step asks the caller to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaderAction {
    None,
    Switch(AppState),
}

impl Fader {
    /// True while the veil is visible, even partly.
    pub fn is_covered(&self) -> bool {
        self.alpha > 0.0
    }

    /// True during a requested fade (new requests are ignored).
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Starts a fade towards `state`; false when one is already running.
    pub fn request(&mut self, state: AppState) -> bool {
        if self.busy() {
            return false;
        }
        self.pending = Some(state);
        self.target = 1.0;
        true
    }

    /// A transition happened: cover now, fade in after the hold.
    pub fn transitioned(&mut self, state: AppState) {
        self.last_state = Some(state);
        self.alpha = 1.0;
        self.hold = HOLD_FRAMES;
        self.target = 0.0;
        self.pending = None;
    }

    /// Advances by `dt` seconds with fades of `seconds` (full range).
    pub fn step(&mut self, dt: f32, seconds: f32) -> FaderAction {
        if self.hold > 0 {
            self.hold -= 1;
            return FaderAction::None;
        }
        if self.instant || seconds <= 0.0 {
            self.alpha = self.target;
        } else {
            let delta = dt / seconds;
            self.alpha = if self.target > self.alpha {
                (self.alpha + delta).min(self.target)
            } else {
                (self.alpha - delta).max(self.target)
            };
        }
        if self.alpha >= 1.0
            && let Some(state) = self.pending.take()
        {
            self.completed += 1;
            return FaderAction::Switch(state);
        }
        FaderAction::None
    }
}

#[derive(Component)]
struct FaderVeil;

pub fn plugin(app: &mut App) {
    app.init_resource::<Fader>()
        .add_message::<FadeToState>()
        .register_dump(dump)
        .add_systems(Startup, (spawn_veil, read_env))
        .add_systems(
            Update,
            (collect_requests, track_transitions, advance, paint).chain(),
        );
}

fn read_env(mut fader: ResMut<Fader>) {
    fader.instant = std::env::var_os("CENT_ANS_NO_FADE").is_some_and(|v| v != "0");
}

fn spawn_veil(mut commands: Commands) {
    commands.spawn((
        Name::new("SceneFader"),
        FaderVeil,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(COVER_COLOR.with_alpha(0.0)),
        GlobalZIndex(FADER_Z),
        Visibility::Hidden,
        // Blocks clicks while visible (the veil is hidden when transparent).
        Pickable::default(),
    ));
}

fn collect_requests(mut requests: MessageReader<FadeToState>, mut fader: ResMut<Fader>) {
    for FadeToState(state) in requests.read() {
        fader.request(*state);
    }
}

fn track_transitions(
    mut events: MessageReader<StateTransitionEvent<AppState>>,
    mut fader: ResMut<Fader>,
) {
    for event in events.read() {
        if let Some(entered) = event.entered
            && event.exited != Some(entered)
        {
            fader.transitioned(entered);
        }
    }
}

fn advance(
    time: Res<Time<Real>>,
    reduce: Res<ReduceMotion>,
    mut fader: ResMut<Fader>,
    mut next: ResMut<NextState<AppState>>,
) {
    let seconds = if reduce.0 {
        REDUCED_SECONDS
    } else {
        FADE_SECONDS
    };
    if let FaderAction::Switch(state) = fader.step(time.delta_secs(), seconds) {
        next.set(state);
        // Hold until the transition event confirms the switch (same frame in practice).
        fader.alpha = 1.0;
    }
}

fn paint(
    fader: Res<Fader>,
    mut veil: Query<(&mut BackgroundColor, &mut Visibility), With<FaderVeil>>,
) {
    for (mut color, mut visibility) in &mut veil {
        color.0 = COVER_COLOR.with_alpha(fader.alpha);
        let wanted = if fader.alpha > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

fn dump(world: &World) -> (String, serde_json::Value) {
    let fader = world.resource::<Fader>();
    (
        "fader".into(),
        json!({
            "alpha": fader.alpha,
            "covered": fader.is_covered(),
            "busy": fader.busy(),
            "fades_completed": fader.completed,
            "last_state": fader.last_state.map(|s| format!("{s:?}")),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_fade_switches_once_then_fades_in() {
        let mut fader = Fader::default();
        assert!(fader.request(AppState::Campaign));
        assert!(!fader.request(AppState::Battle), "double click ignored");
        let mut switched = None;
        for _ in 0..40 {
            if let FaderAction::Switch(state) = fader.step(0.016, FADE_SECONDS) {
                switched = Some(state);
                break;
            }
        }
        assert_eq!(switched, Some(AppState::Campaign));
        assert_eq!(fader.alpha, 1.0);
        fader.transitioned(AppState::Campaign);
        for _ in 0..HOLD_FRAMES {
            assert_eq!(fader.step(0.016, FADE_SECONDS), FaderAction::None);
            assert_eq!(fader.alpha, 1.0);
        }
        for _ in 0..40 {
            fader.step(0.016, FADE_SECONDS);
        }
        assert_eq!(fader.alpha, 0.0);
        assert!(!fader.is_covered());
    }

    #[test]
    fn unrequested_transition_is_covered_then_revealed() {
        let mut fader = Fader::default();
        fader.transitioned(AppState::MainMenu);
        assert!(fader.is_covered());
        for _ in 0..60 {
            fader.step(0.016, FADE_SECONDS);
        }
        assert!(!fader.is_covered());
    }

    #[test]
    fn instant_mode_and_reduced_motion() {
        let mut fader = Fader {
            instant: true,
            ..Fader::default()
        };
        fader.request(AppState::Battle);
        assert_eq!(
            fader.step(0.016, FADE_SECONDS),
            FaderAction::Switch(AppState::Battle)
        );
        let mut quick = Fader::default();
        quick.request(AppState::Battle);
        let frames = (0..20)
            .take_while(|_| quick.step(0.016, REDUCED_SECONDS) == FaderAction::None)
            .count();
        assert!(frames <= 4, "reduced fade takes {frames} frames");
    }
}
