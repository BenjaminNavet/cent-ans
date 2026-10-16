//! Delayed Codex bubble over the natural decor (port of game/scripts/ui/decor_hover.gd and
//! `CampaignMap.pick_decor`, ADR 0219).
//!
//! The decor (trees, herds) has no collider: after the pointer stayed still for the delay of the
//! `interface/decor_hover_delay` setting (1,5 s by default, 0 = off), a single CPU pick reads the
//! instances already in memory around the ground point under the cursor, ranks them in screen
//! space (nearest to its projected centre, inside its projected radius, never less than a
//! minimum aimed radius) and opens the ordinary Codex bubble of the species
//! (`CodexStore::entry_for_decor`) near the cursor. Armies, interface and the parchment view come
//! first (no pick). One attempt per stillness: nothing repeats before the pointer moved. The
//! bubble closes when the pointer leaves it by more than `leave_px`, unless the pointer is on the
//! bubble, or the player pinned it (T, right click: it then belongs to [`super::pinned`]).
//!
//! Sources: campaign trees ([`crate::campaign::vegetation::TreeDecor`]) and herd animals
//! ([`crate::campaign::life::fauna::Animal`]). Not ported: rocks (the countryside rocks keep no
//! queryable instance list) and the battle trees (plantings are not kept after spawning).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::Deserialize;
use serde_json::{Value, json};

use super::store::CodexStore;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::HoveredArmy;
use crate::campaign::armies::pointer::PointerOverUi;
use crate::campaign::armies::projection::ray_ground;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::life::fauna::Animal;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::vegetation::TreeDecor;
use crate::settings::Settings;
use crate::ui::kit::tooltip::{TooltipPanel, TooltipRegistry, TooltipState};

/// Default delay (s) without the settings resource.
pub const DEFAULT_DELAY: f32 = 1.5;
/// Distance (px) at which the screen scale of the ground is measured (`DECOR_SEARCH_PX`).
pub const SEARCH_PX: f32 = 40.0;
/// Strategic weight above which the parchment view hides the decor.
pub const MAX_STRATEGIC: f32 = 0.3;
/// Frames a bubble requested from the kit may take to appear.
const OPEN_FRAMES: u32 = 4;

/// `decor_hover` block of `data/ui/tooltip_style.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DecorStyle {
    /// The pointer is still while it moves less than this (px).
    pub still_px: f32,
    /// The bubble closes when the pointer is farther than this from where it opened (px).
    pub leave_px: f32,
}

impl Default for DecorStyle {
    fn default() -> Self {
        Self {
            still_px: 3.0,
            leave_px: 12.0,
        }
    }
}

impl DecorStyle {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        match value.get("decor_hover") {
            Some(block) => serde_json::from_value(block.clone()).map_err(|e| e.to_string()),
            None => Ok(Self::default()),
        }
    }

    pub fn load() -> Result<Self, String> {
        let root = crate::paths::find_root()?;
        let path = crate::paths::data_dir(&root).join("ui/tooltip_style.json");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes)
    }
}

/// Minimum aimed radius (px) by decor kind (`DECOR_MIN_RADIUS_PX`).
pub fn min_radius_px(kind: &str) -> f32 {
    match kind {
        "tree" => 10.0,
        "rock" => 12.0,
        _ => 16.0,
    }
}

/// A decor element near the ground point under the cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// `tree`, `fauna`, `rock` (the `kind` of `CodexStore::entry_for_decor`).
    pub kind: &'static str,
    pub species: String,
    pub foot: Vec3,
    pub height: f32,
    pub radius: f32,
}

/// The candidate under `cursor`: smallest `distance to its projected centre / projected radius`
/// below 1. `project` maps a world point to the screen (None behind the camera); `right` is the
/// camera's right vector (projects the radius).
pub fn best_candidate<'a>(
    candidates: &'a [Candidate],
    cursor: Vec2,
    project: impl Fn(Vec3) -> Option<Vec2>,
    right: Vec3,
) -> Option<&'a Candidate> {
    let mut best = None;
    let mut best_score = 1.0;
    for candidate in candidates {
        let centre = candidate.foot + Vec3::Y * candidate.height * 0.5;
        let (Some(at), Some(edge)) = (project(centre), project(centre + right * candidate.radius))
        else {
            continue;
        };
        let radius = at.distance(edge).max(min_radius_px(candidate.kind));
        let score = at.distance(cursor) / radius;
        if score < best_score {
            best_score = score;
            best = Some(candidate);
        }
    }
    best
}

// ---- waiting state ------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Waiting,
    /// The pick ran (nothing found, or blocked): no new attempt before the pointer moves.
    Fired,
    Shown,
}

/// The stillness timer of `DecorHover`; pure, tested without Bevy.
#[derive(Debug, Clone, Default)]
pub struct DecorWait {
    pub phase: Phase,
    pub anchor: Vec2,
    pub elapsed: f32,
}

impl DecorWait {
    fn begin_wait(&mut self, position: Vec2) {
        self.anchor = position;
        self.elapsed = 0.0;
        self.phase = Phase::Waiting;
    }

    /// The pointer is at `position`. True when the shown bubble must be closed.
    pub fn note_mouse(
        &mut self,
        position: Vec2,
        bubble_alive: bool,
        near_bubble: bool,
        style: &DecorStyle,
    ) -> bool {
        if self.phase == Phase::Shown {
            if !bubble_alive {
                self.begin_wait(position);
            } else if position.distance(self.anchor) > style.leave_px && !near_bubble {
                self.begin_wait(position);
                return true;
            }
            return false;
        }
        if position.distance(self.anchor) > style.still_px || self.phase == Phase::Idle {
            self.begin_wait(position);
        }
        false
    }

    /// A click or a key press: the countdown starts again. True when the shown bubble closes.
    pub fn interrupted(&mut self, position: Vec2, bubble_alive: bool, near_bubble: bool) -> bool {
        let mut close = false;
        if self.phase == Phase::Shown && bubble_alive && !near_bubble {
            close = true;
        }
        if self.phase != Phase::Shown || close {
            self.begin_wait(position);
        }
        close
    }

    /// Advances the countdown; true when the delay elapsed (the pick must run once).
    pub fn tick(&mut self, dt: f32, delay: f32) -> bool {
        if self.phase != Phase::Waiting || delay <= 0.0 {
            return false;
        }
        self.elapsed += dt;
        if self.elapsed >= delay {
            self.phase = Phase::Fired;
            return true;
        }
        false
    }

    pub fn shown(&mut self) {
        self.phase = Phase::Shown;
    }
}

#[derive(Resource, Default)]
pub struct DecorHover {
    wait: DecorWait,
    /// Frames since the bubble was requested (the kit opens it a frame later).
    frames_since_open: u32,
    shown_entry: String,
    shown_total: usize,
    attempts: usize,
    /// Why the last attempt opened nothing.
    last_miss: &'static str,
    last_candidates: usize,
    last_species: String,
    /// Proof hook `CENT_ANS_DECOR_CURSOR` (`center` or `x,y`): scripts cannot move the pointer.
    cursor_override: Option<Option<Vec2>>,
}

pub fn plugin(app: &mut App) {
    let style = DecorStyle::load().unwrap_or_default();
    let cursor_override = std::env::var("CENT_ANS_DECOR_CURSOR").ok().map(|text| {
        let (x, y) = text.split_once(',')?;
        Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
    });
    app.insert_resource(DecorHover {
        cursor_override,
        ..default()
    })
    .insert_resource(style)
    .register_dump(dump)
    .add_systems(Update, decor_hover.run_if(in_state(AppState::Campaign)));
}

/// Candidates around the ground point `ground` within `reach` map units.
fn gather(
    ground: Vec2,
    reach: f32,
    trees: &TreeDecor,
    animals: &Query<(&Animal, &GlobalTransform)>,
) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = trees
        .near(ground.x, ground.y, reach)
        .into_iter()
        .map(|tree| Candidate {
            kind: "tree",
            species: tree.species,
            foot: tree.foot,
            height: tree.height,
            radius: tree.radius,
        })
        .collect();
    for (animal, transform) in animals {
        let position = transform.translation();
        if Vec2::new(position.x, position.z).distance(ground) > reach {
            continue;
        }
        let size = animal.length_world.max(0.1);
        found.push(Candidate {
            kind: "fauna",
            species: animal.species.clone(),
            foot: position,
            height: size * 0.5,
            radius: size * 0.5,
        });
    }
    found
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn decor_hover(
    (time, mouse, keys): (
        Res<Time<Real>>,
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
    ),
    (settings, style, zoom): (
        Option<Res<Settings>>,
        Res<DecorStyle>,
        Option<Res<ZoomView>>,
    ),
    (pointer, hovered_army): (Res<PointerOverUi>, Option<Res<HoveredArmy>>),
    (store, registry): (Res<CodexStore>, Res<TooltipRegistry>),
    mut tooltips: ResMut<TooltipState>,
    mut decor: ResMut<DecorHover>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: GroundHeight,
    trees: TreeDecor,
    animals: Query<(&Animal, &GlobalTransform)>,
    panels: Query<&TooltipPanel>,
) {
    let delay = settings.map_or(DEFAULT_DELAY, |s| s.decor_hover_delay);
    let decor = &mut *decor;
    let real = window.single().ok();
    let cursor = match decor.cursor_override {
        // `center` (no coordinates): the middle of the window.
        Some(None) => real.map(|window| window.size() * 0.5),
        Some(Some(point)) => Some(point),
        None => real.and_then(Window::cursor_position),
    };
    let Some(cursor) = cursor else {
        return;
    };
    // The bubble requested is up (or about to be).
    if decor.frames_since_open < OPEN_FRAMES {
        decor.frames_since_open += 1;
    }
    let bubble_alive = !tooltips.chain.is_empty() || decor.frames_since_open < OPEN_FRAMES;
    let near_bubble = panels.iter().any(|panel| {
        panel.placed
            && Rect::from_corners(
                panel.rect.min - Vec2::splat(style.leave_px),
                panel.rect.max + Vec2::splat(style.leave_px),
            )
            .contains(cursor)
    });
    let pressed =
        mouse.get_just_pressed().next().is_some() || keys.get_just_pressed().next().is_some();
    let close = if pressed {
        decor.wait.interrupted(cursor, bubble_alive, near_bubble)
    } else {
        decor
            .wait
            .note_mouse(cursor, bubble_alive, near_bubble, &style)
    };
    if close {
        tooltips.chain.clear();
        decor.shown_entry.clear();
    }
    if !decor.wait.tick(time.delta_secs(), delay) {
        return;
    }

    // One attempt per stillness.
    decor.attempts += 1;
    decor.last_candidates = 0;
    decor.last_species.clear();
    let blocked = if pointer.over {
        Some("interface")
    } else if hovered_army.is_some_and(|army| army.0.is_some()) {
        Some("army")
    } else if !tooltips.chain.is_empty() {
        Some("bubble")
    } else if zoom.is_some_and(|zoom| zoom.strategic > MAX_STRATEGIC) {
        Some("parchment")
    } else {
        None
    };
    if let Some(reason) = blocked {
        decor.last_miss = reason;
        return;
    }
    let Some((camera, camera_transform)) = camera.iter().next() else {
        decor.last_miss = "no_camera";
        return;
    };
    let Some(point) = camera
        .viewport_to_world(camera_transform, cursor)
        .ok()
        .and_then(|ray| ray_ground(ray, &ground))
    else {
        decor.last_miss = "no_ground";
        return;
    };
    let probe = camera
        .viewport_to_world(camera_transform, cursor + Vec2::new(SEARCH_PX, 0.0))
        .ok()
        .and_then(|ray| ray_ground(ray, &ground));
    let reach = probe.map_or(0.0, |probe| probe.distance(point));
    if reach <= 0.0 {
        decor.last_miss = "no_scale";
        return;
    }
    let candidates = gather(point, reach, &trees, &animals);
    decor.last_candidates = candidates.len();
    let right = camera_transform.right().as_vec3();
    let Some(hit) = best_candidate(
        &candidates,
        cursor,
        |world| camera.world_to_viewport(camera_transform, world).ok(),
        right,
    ) else {
        decor.last_miss = "nothing";
        return;
    };
    decor.last_species = format!("{}:{}", hit.kind, hit.species);
    let Some(entry) = store
        .entry_for_decor(hit.kind, &hit.species)
        .map(str::to_owned)
    else {
        decor.last_miss = "no_entry";
        return;
    };
    open_bubble(decor, &mut tooltips, &registry, &entry, cursor);
}

fn open_bubble(
    decor: &mut DecorHover,
    tooltips: &mut TooltipState,
    registry: &TooltipRegistry,
    entry: &str,
    cursor: Vec2,
) {
    let key = format!("{}{entry}", super::text::META_PREFIX);
    tooltips.open_at(&registry.resolve(&key), cursor, false);
    decor.wait.shown();
    decor.frames_since_open = 0;
    decor.shown_entry = entry.to_owned();
    decor.shown_total += 1;
    decor.last_miss = "";
}

fn dump(world: &World) -> (String, Value) {
    let decor = world.resource::<DecorHover>();
    (
        "codex_decor".to_owned(),
        json!({
            "phase": format!("{:?}", decor.wait.phase),
            "attempts": decor.attempts,
            "shown_total": decor.shown_total,
            "shown_entry": decor.shown_entry,
            "last_miss": decor.last_miss,
            "last_candidates": decor.last_candidates,
            "last_species": decor.last_species,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(world: Vec3) -> Option<Vec2> {
        // Orthographic top view: x right, z down, 1 unit = 1 px, height lifts up the screen.
        Some(Vec2::new(world.x, world.z - world.y))
    }

    fn tree(species: &str, x: f32, z: f32, height: f32, radius: f32) -> Candidate {
        Candidate {
            kind: "tree",
            species: species.to_owned(),
            foot: Vec3::new(x, 0.0, z),
            height,
            radius,
        }
    }

    #[test]
    fn nearest_to_its_centre_inside_its_radius_wins() {
        let candidates = vec![
            tree("oak", 100.0, 100.0, 40.0, 20.0),
            tree("beech", 130.0, 100.0, 40.0, 20.0),
        ];
        // Centres at (100, 80) and (130, 80).
        let hit = best_candidate(&candidates, Vec2::new(120.0, 80.0), project, Vec3::X).unwrap();
        assert_eq!(hit.species, "beech");
        let hit = best_candidate(&candidates, Vec2::new(105.0, 82.0), project, Vec3::X).unwrap();
        assert_eq!(hit.species, "oak");
        assert!(best_candidate(&candidates, Vec2::new(300.0, 300.0), project, Vec3::X).is_none());
    }

    #[test]
    fn small_decor_keeps_a_minimum_aimed_radius() {
        // A 2 px tree is still aimable within 10 px.
        let candidates = vec![tree("oak", 50.0, 50.0, 2.0, 1.0)];
        assert!(best_candidate(&candidates, Vec2::new(57.0, 49.0), project, Vec3::X).is_some());
        assert!(best_candidate(&candidates, Vec2::new(80.0, 49.0), project, Vec3::X).is_none());
    }

    #[test]
    fn behind_the_camera_is_ignored() {
        let candidates = vec![tree("oak", 0.0, 0.0, 10.0, 5.0)];
        assert!(best_candidate(&candidates, Vec2::ZERO, |_| None, Vec3::X).is_none());
    }

    #[test]
    fn fires_once_after_the_delay_and_not_again_until_the_pointer_moves() {
        let style = DecorStyle::default();
        let mut wait = DecorWait::default();
        wait.note_mouse(Vec2::new(10.0, 10.0), false, false, &style);
        assert!(!wait.tick(1.0, 1.5));
        assert!(wait.tick(0.6, 1.5));
        assert!(!wait.tick(5.0, 1.5), "one attempt per stillness");
        // A jitter within still_px keeps the state; a real move restarts the countdown.
        wait.note_mouse(Vec2::new(11.0, 11.0), false, false, &style);
        assert!(!wait.tick(5.0, 1.5));
        wait.note_mouse(Vec2::new(40.0, 40.0), false, false, &style);
        assert!(wait.tick(1.5, 1.5));
    }

    #[test]
    fn zero_delay_disables_the_pick() {
        let mut wait = DecorWait::default();
        wait.note_mouse(Vec2::ZERO, false, false, &DecorStyle::default());
        assert!(!wait.tick(100.0, 0.0));
    }

    #[test]
    fn shown_bubble_closes_on_leaving_unless_near_or_dead() {
        let style = DecorStyle::default();
        let mut wait = DecorWait::default();
        wait.note_mouse(Vec2::ZERO, false, false, &style);
        wait.shown();
        assert!(!wait.note_mouse(Vec2::new(5.0, 0.0), true, false, &style));
        assert!(
            !wait.note_mouse(Vec2::new(50.0, 0.0), true, true, &style),
            "on the bubble"
        );
        assert!(wait.note_mouse(Vec2::new(50.0, 0.0), true, false, &style));
        assert_eq!(wait.phase, Phase::Waiting);
        wait.shown();
        // The bubble vanished (pinned, Escape): back to waiting without closing anything.
        assert!(!wait.note_mouse(Vec2::new(50.0, 0.0), false, false, &style));
        assert_eq!(wait.phase, Phase::Waiting);
    }

    #[test]
    fn style_reads_the_data_block() {
        let style = DecorStyle::parse(br#"{"decor_hover":{"still_px":5,"leave_px":20}}"#).unwrap();
        assert_eq!((style.still_px, style.leave_px), (5.0, 20.0));
        assert_eq!(DecorStyle::parse(b"{}").unwrap().leave_px, 12.0);
    }
}
