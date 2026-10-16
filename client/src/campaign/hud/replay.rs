//! Replay of the AI turn (port of game/scripts/map/ai_turn_replay.gd, ADR 0073).
//!
//! The core resolves the whole AI turn inside `end_turn`; with recording on
//! (`CampaignState::set_ai_replay_recording`) it keeps every AI army move (real path, outcome,
//! visibility, notability). This module replays them between the resolution and the season
//! report: the camera visits the moves that concern the player (`max_followed_moves` at most, the
//! most important ones), walking along each real path, then goes back where it was. Space,
//! Escape, Enter or a click skip the rest. Staging comes from `data/ui/ai_turn_replay.json`.
//!
//! Differences with Godot: the armies' markers are not animated (that belongs to the armies lot):
//! only the camera and a caption move.

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_campaign::{AiMoveKind, AiMoveRecord};

use super::HudSet;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::campaign::camera::rig::CampaignRig;
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_REPORT};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::text_field::{TextInputFocus, keyboard_free};
use crate::ui::kit::{UiTheme, widgets::panel_note};

/// Moves that make the camera stop a moment on their end point.
const HOLD_KINDS: [AiMoveKind; 4] = [
    AiMoveKind::Battle,
    AiMoveKind::SiegeStarted,
    AiMoveKind::SettlementTaken,
    AiMoveKind::Landing,
];

/// Staging, `data/ui/ai_turn_replay.json` (defaults when the file is missing).
#[derive(Debug, Clone, PartialEq)]
pub struct Tuning {
    pub max_followed_moves: usize,
    pub notable_radius_km: f64,
    pub march_speed_px_per_s: f32,
    pub min_move_duration_s: f32,
    pub max_move_duration_s: f32,
    pub camera_pan_duration_s: f32,
    pub camera_hold_s: f32,
    pub camera_return_duration_s: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            max_followed_moves: 6,
            notable_radius_km: 60.0,
            march_speed_px_per_s: 55.0,
            min_move_duration_s: 0.8,
            max_move_duration_s: 3.0,
            camera_pan_duration_s: 0.7,
            camera_hold_s: 0.5,
            camera_return_duration_s: 0.6,
        }
    }
}

impl Tuning {
    pub fn load() -> Self {
        let mut tuning = Self::default();
        let Ok(root) = crate::paths::find_root() else {
            return tuning;
        };
        let Ok(text) = std::fs::read_to_string(root.join("data/ui/ai_turn_replay.json")) else {
            return tuning;
        };
        let Ok(tree) = serde_json::from_str::<Value>(&text) else {
            return tuning;
        };
        let num =
            |key: &str, default: f64| tree.get(key).and_then(Value::as_f64).unwrap_or(default);
        tuning.max_followed_moves = num("max_followed_moves", 6.0) as usize;
        tuning.notable_radius_km = num("notable_radius_km", 60.0);
        tuning.march_speed_px_per_s = num("march_speed_px_per_s", 55.0) as f32;
        tuning.min_move_duration_s = num("min_move_duration_s", 0.8) as f32;
        tuning.max_move_duration_s = num("max_move_duration_s", 3.0) as f32;
        tuning.camera_pan_duration_s = num("camera_pan_duration_s", 0.7) as f32;
        tuning.camera_hold_s = num("camera_hold_s", 0.5) as f32;
        tuning.camera_return_duration_s = num("camera_return_duration_s", 0.6) as f32;
        tuning
    }
}

/// The moves the player sees, in play order.
pub fn shown_moves(moves: &[AiMoveRecord]) -> Vec<&AiMoveRecord> {
    moves
        .iter()
        .filter(|m| m.visible && m.path.len() >= 2)
        .collect()
}

/// Sequences of the moves the camera follows: the most important notable ones, `max` at most
/// (priority, then play order), returned in play order.
pub fn followed_sequences(shown: &[&AiMoveRecord], max: usize) -> Vec<u32> {
    let mut notable: Vec<&&AiMoveRecord> = shown.iter().filter(|m| m.notable.is_some()).collect();
    notable.sort_by_key(|m| {
        (
            std::cmp::Reverse(m.notable.map_or(0, |n| n.priority())),
            m.sequence,
        )
    });
    let mut chosen: Vec<u32> = notable.iter().take(max).map(|m| m.sequence).collect();
    chosen.sort_unstable();
    chosen
}

/// One followed move of the camera tour.
#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
    pub faction: String,
    pub kind: AiMoveKind,
    /// Path in world units (x, z).
    pub path: Vec<Vec2>,
    pub march_seconds: f32,
}

impl Stop {
    fn point_at(&self, share: f32) -> Vec2 {
        let share = share.clamp(0.0, 1.0);
        let lengths: Vec<f32> = self.path.windows(2).map(|w| w[0].distance(w[1])).collect();
        let total: f32 = lengths.iter().sum();
        if total <= f32::EPSILON {
            return self.path[0];
        }
        let mut left = share * total;
        for (i, length) in lengths.iter().enumerate() {
            if left <= *length || i == lengths.len() - 1 {
                let t = if *length > 0.0 {
                    (left / length).min(1.0)
                } else {
                    1.0
                };
                return self.path[i].lerp(self.path[i + 1], t);
            }
            left -= length;
        }
        self.path[self.path.len() - 1]
    }

    fn holds(&self) -> bool {
        HOLD_KINDS.contains(&self.kind)
    }
}

/// Builds the camera tour from the recorded moves.
pub fn build_tour(
    moves: &[AiMoveRecord],
    data: &data_model::GameData,
    tuning: &Tuning,
) -> Vec<Stop> {
    let shown = shown_moves(moves);
    let followed = followed_sequences(&shown, tuning.max_followed_moves);
    shown
        .iter()
        .filter(|m| followed.contains(&m.sequence))
        .map(|m| {
            let path: Vec<Vec2> = m.path.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            let length: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
            Stop {
                faction: data.faction_name(&m.faction),
                kind: m.kind,
                march_seconds: (length / tuning.march_speed_px_per_s.max(1.0))
                    .clamp(tuning.min_move_duration_s, tuning.max_move_duration_s),
                path,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Pan,
    March,
    Hold,
}

/// Request held back until the replay ends: the season report.
#[derive(Debug, Clone)]
pub struct HeldReport {
    pub key: String,
    pub spec: ModalSpec,
}

#[derive(Resource, Default)]
pub struct AiReplay {
    pub tuning: Tuning,
    stops: Vec<Stop>,
    index: usize,
    phase: Option<Phase>,
    elapsed: f32,
    saved_focus: Option<Vec2>,
    returning: f32,
    held: Option<HeldReport>,
    /// Moves recorded by the last turn / shown / followed / replays played / replays skipped.
    pub moves: usize,
    pub shown: usize,
    pub followed: usize,
    pub played: u32,
    pub skipped: u32,
}

impl AiReplay {
    pub fn playing(&self) -> bool {
        self.phase.is_some() || self.returning > 0.0
    }

    /// Starts the replay of a resolved turn. Returns `true` when it plays (the caller then holds
    /// the report, see `hold_report`).
    pub fn begin(
        &mut self,
        moves: &[AiMoveRecord],
        data: &data_model::GameData,
        focus: Option<Vec2>,
        speed_setting: f32,
    ) -> bool {
        self.moves = moves.len();
        self.shown = shown_moves(moves).len();
        self.stops = if speed_setting > 0.0 {
            build_tour(moves, data, &self.tuning)
        } else {
            Vec::new()
        };
        self.followed = self.stops.len();
        if self.stops.is_empty() {
            self.phase = None;
            return false;
        }
        self.index = 0;
        self.elapsed = 0.0;
        self.phase = Some(Phase::Pan);
        self.saved_focus = focus;
        self.played += 1;
        true
    }

    pub fn hold_report(&mut self, report: HeldReport) {
        self.held = Some(report);
    }

    fn finish(&mut self, skipped: bool) -> Option<HeldReport> {
        self.phase = None;
        self.stops.clear();
        if skipped {
            self.skipped += 1;
        }
        self.returning = if self.saved_focus.is_some() {
            self.tuning.camera_return_duration_s
        } else {
            0.0
        };
        self.held.take()
    }

    /// Advances by `dt` seconds (already divided by the speed); returns the camera point to show.
    fn advance(&mut self, dt: f32) -> Option<Vec2> {
        let phase = self.phase?;
        let stop = self.stops.get(self.index)?.clone();
        self.elapsed += dt;
        let (duration, point) = match phase {
            Phase::Pan => (self.tuning.camera_pan_duration_s, stop.path[0]),
            Phase::March => (
                stop.march_seconds,
                stop.point_at(self.elapsed / stop.march_seconds),
            ),
            Phase::Hold => (self.tuning.camera_hold_s, stop.point_at(1.0)),
        };
        if self.elapsed >= duration {
            self.elapsed = 0.0;
            self.phase = match phase {
                Phase::Pan => Some(Phase::March),
                Phase::March if stop.holds() => Some(Phase::Hold),
                _ => {
                    self.index += 1;
                    if self.index < self.stops.len() {
                        Some(Phase::Pan)
                    } else {
                        None
                    }
                }
            };
        }
        Some(point)
    }

    fn caption(&self) -> String {
        match self.stops.get(self.index) {
            Some(stop) => format!(
                "Tour de {} ({}/{}) — Espace pour passer",
                stop.faction,
                self.index + 1,
                self.stops.len()
            ),
            None => String::new(),
        }
    }
}

#[derive(Component)]
struct ReplayCaption;

pub fn plugin(app: &mut App) {
    app.init_resource::<AiReplay>()
        .register_dump(dump_replay)
        .add_systems(OnEnter(AppState::Campaign), spawn_caption)
        .add_systems(
            Update,
            run_replay
                .in_set(HudSet::Flow)
                .after(super::flow::poll_turn)
                .run_if(in_state(AppState::Campaign)),
        );
    app.world_mut().resource_mut::<AiReplay>().tuning = Tuning::load();
}

fn spawn_caption(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Name::new("hud:replay_caption"),
            DespawnOnExit(AppState::Campaign),
            ReplayCaption,
            UiTier::Banner.index(),
            panel_note(&theme),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(90.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-190.0)),
            width: Val::Px(380.0),
            justify_content: JustifyContent::Center,
            padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
            display: Display::None,
            ..default()
        })
        .with_children(|bar| {
            bar.spawn((
                Text::new(""),
                theme.text_font(false, true, UiType::Body.size()),
                TextColor(style::INK),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn run_replay(
    mut replay: ResMut<AiReplay>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Option<Res<Settings>>,
    text_focus: Option<Res<TextInputFocus>>,
    rig: Option<Res<CampaignRig>>,
    mut focus: MessageWriter<FocusCamera>,
    mut queue: ResMut<ModalQueue>,
    mut captions: Query<(&mut Node, &Children), With<ReplayCaption>>,
    mut texts: Query<&mut Text>,
) {
    let speed = settings.map_or(1.0, |s| s.ai_moves_speed).max(0.1);
    let skip = (keyboard_free(text_focus)
        && (keys.just_pressed(KeyCode::Space)
            || keys.just_pressed(KeyCode::Escape)
            || keys.just_pressed(KeyCode::Enter)))
        || mouse.just_pressed(MouseButton::Left);
    let playing = replay.phase.is_some();
    if playing {
        if skip {
            if let Some(report) = replay.finish(true) {
                queue.request(&report.key, PRIORITY_REPORT, report.spec);
            }
            if let Some(home) = replay.saved_focus {
                focus.write(FocusCamera::Point(home));
            }
        } else if let Some(point) = replay.advance(time.delta_secs() * speed) {
            focus.write(FocusCamera::Point(point));
            if replay.phase.is_none() {
                // The tour ended on this frame.
                if let Some(report) = replay.finish(false) {
                    queue.request(&report.key, PRIORITY_REPORT, report.spec);
                }
            }
        }
    } else if replay.returning > 0.0 {
        replay.returning -= time.delta_secs();
        if let Some(home) = replay.saved_focus.take() {
            focus.write(FocusCamera::Point(home));
        }
        replay.returning = 0.0;
    }
    let _ = rig;
    if let Ok((mut node, kids)) = captions.single_mut() {
        let want = if replay.phase.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
        let line = replay.caption();
        for kid in kids.iter() {
            if let Ok(mut text) = texts.get_mut(kid)
                && text.0 != line
            {
                text.0 = line.clone();
            }
        }
    }
}

/// Camera focus (x, z) to come back to after the replay.
pub fn current_focus(rig: Option<&CampaignRig>) -> Option<Vec2> {
    rig.map(|r| Vec2::new(r.target_focus.x, r.target_focus.z))
}

/// Dummy use so the helper is shared by `flow.rs`.
pub fn recording_wanted(settings: Option<&Settings>, sim: &Sim) -> bool {
    sim.state.is_some() && settings.is_none_or(|s| s.ai_moves_speed > 0.0)
}

fn dump_replay(world: &World) -> (String, Value) {
    let replay = world.resource::<AiReplay>();
    (
        "ai_replay".to_owned(),
        json!({
            "playing": replay.playing(),
            "moves": replay.moves, "shown": replay.shown, "followed": replay.followed,
            "played": replay.played, "skipped": replay.skipped,
            "stop": replay.index, "stops": replay.stops.len(),
            "held_report": replay.held.is_some(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::{AiMoveNotability, ArmyId};

    fn record(sequence: u32, notable: Option<AiMoveNotability>, visible: bool) -> AiMoveRecord {
        AiMoveRecord {
            sequence,
            army: ArmyId::parse("army_1").expect("army id"),
            faction: data_model::FactionId::new("fac_england").expect("faction"),
            path: vec![[0.0, 0.0], [30.0, 40.0]],
            kind: AiMoveKind::March,
            settlement: None,
            target_army: None,
            target_faction: None,
            settlement_controller: None,
            visible,
            visible_from: 0,
            visible_to: 1,
            notable,
        }
    }

    #[test]
    fn follows_the_most_important_in_play_order() {
        let moves = vec![
            record(0, Some(AiMoveNotability::NearPlayer), true),
            record(1, Some(AiMoveNotability::Battle), true),
            record(2, None, true),
            record(3, Some(AiMoveNotability::Siege), true),
            record(4, Some(AiMoveNotability::Battle), false),
        ];
        let shown = shown_moves(&moves);
        assert_eq!(shown.len(), 4);
        assert_eq!(followed_sequences(&shown, 2), vec![1, 3]);
        assert_eq!(followed_sequences(&shown, 9), vec![0, 1, 3]);
    }

    #[test]
    fn stop_walks_its_path() {
        let stop = Stop {
            faction: String::new(),
            kind: AiMoveKind::March,
            path: vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)],
            march_seconds: 1.0,
        };
        assert_eq!(stop.point_at(0.0), Vec2::ZERO);
        assert_eq!(stop.point_at(0.5), Vec2::new(10.0, 0.0));
        assert_eq!(stop.point_at(1.0), Vec2::new(10.0, 10.0));
    }

    #[test]
    fn tour_phases_run_to_the_end() {
        let tuning = Tuning::default();
        let mut replay = AiReplay {
            tuning: tuning.clone(),
            stops: vec![Stop {
                faction: "x".into(),
                kind: AiMoveKind::Battle,
                path: vec![Vec2::ZERO, Vec2::new(5.0, 0.0)],
                march_seconds: 1.0,
            }],
            phase: Some(Phase::Pan),
            ..default()
        };
        let mut guard = 0;
        while replay.phase.is_some() && guard < 1000 {
            replay.advance(0.1);
            guard += 1;
        }
        assert!(replay.phase.is_none());
        assert!(guard > 15, "pan + march + hold last about 2.2 s");
    }

    #[test]
    fn ai_moves_speed_zero_plays_no_replay() {
        let root = crate::paths::find_root().expect("root");
        let (data, _) =
            data_model::GameData::load(&crate::paths::data_dir(&root)).expect("game data");
        let moves = vec![record(0, Some(AiMoveNotability::Battle), true)];
        let mut off = AiReplay::default();
        assert!(!off.begin(&moves, &data, None, 0.0));
        assert!(off.phase.is_none() && off.stops.is_empty());
    }
}
