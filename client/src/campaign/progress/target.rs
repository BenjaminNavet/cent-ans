//! Arrow and halo of the tutorial (port of `resolve_target` of `tutorial_controller.gd` and of the
//! gold frame of `ui/tutorial.gd::_draw`).
//!
//! Each step names its target (`data/tutorial/steps.json`, `target`). [`TargetLookup::resolve`]
//! turns it into a screen shape in layout pixels: the rectangle of a widget (found by its kit
//! name, `Name`), or a point projected from the map (the royal army, the capital). The overlay
//! draws a pulsing gold frame (corner brackets included) or a ring around it, and a bouncing arrow
//! above; the pulse stops with `ReduceMotion`. The overlay never takes a click (`Pickable::IGNORE`).
//!
//! [`focus_for`] is the camera focus of the steps that need one (`_focus_for_step`).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::{Value, json};

use super::tutorial::Tutorial;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::ArmyViews;
use crate::campaign::camera::{CampaignCamera, FocusCamera};
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::modal::ReduceMotion;
use crate::ui::kit::style::UiTier;

const GOLD: Color = Color::srgb(0.95, 0.78, 0.25);
const SHADOW: Color = Color::srgba(0.1, 0.05, 0.0, 0.7);
/// Radius of the ring around a map point (`POINT_RADIUS`).
const POINT_RADIUS: f32 = 34.0;
/// Seconds of one pulse.
const PULSE_PERIOD: f32 = 1.2;
/// Height above the ground (world units) the arrow of a map point aims at.
const POINT_LIFT: f32 = 1.5;

/// A widget to look for by its kit `Name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// Exactly this name.
    Name(&'static str),
    /// Every widget whose name starts so (their union).
    Prefix(&'static str),
}

/// Widgets that carry `target`, by preference (the first one visible wins). The table of
/// `resolve_target`.
pub fn candidates(target: &str) -> &'static [Candidate] {
    use Candidate::{Name, Prefix};
    match target {
        "city_tab" => &[Name("prov:tab_buildings")],
        "buildable" => &[Prefix("prov:build:"), Name("prov:tab_buildings")],
        "research" => &[Name("TechPanel"), Name("hud:open_tech")],
        "diplomacy" => &[Name("hud:open_diplomacy")],
        "end_turn" => &[Name("hud:end_turn")],
        "season_report" => &[Name("Modal hud:season_report")],
        "chronicle" => &[Name("hud:open_chronicle")],
        "tax" => &[
            Prefix("faction:tax:"),
            Name("FactionPanel"),
            Name("hud:open_faction"),
        ],
        "governor" => &[
            Prefix("court:appoint:"),
            Name("court:menu:governor"),
            Name("CourtPanel"),
            Name("hud:open_council"),
        ],
        // The army marker is a map point; the strip is the fallback.
        "royal_army" => &[Name("hud:army_strip")],
        _ => &[],
    }
}

/// Targets drawn from a map point (the widget table, if any, is the fallback).
pub fn is_world_target(target: &str) -> bool {
    matches!(target, "royal_army" | "capital")
}

/// Targets that fall back on the capital when their widget is closed.
fn falls_back_on_capital(target: &str) -> bool {
    matches!(target, "city_tab" | "buildable")
}

/// Camera focus at the start of a step (`_focus_for_step`): the royal army at « Votre armée »,
/// the capital at « La province ». `Point` is used for the army, not `Army`, which would select
/// it and complete the step.
pub fn focus_for(
    step_id: &str,
    army_point: Option<Vec2>,
    capital: Option<&str>,
) -> Option<FocusCamera> {
    match step_id {
        "select_army" => army_point.map(FocusCamera::Point),
        "open_province" => capital.map(|c| FocusCamera::Province(c.to_owned())),
        _ => None,
    }
}

/// Where the arrow points, in layout pixels (the space of `Node` positions).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Rect { min: Vec2, max: Vec2 },
    Point(Vec2),
}

impl Shape {
    pub fn center(&self) -> Vec2 {
        match self {
            Self::Rect { min, max } => (*min + *max) * 0.5,
            Self::Point(p) => *p,
        }
    }

    /// Box of the halo (the ring of a point is a square box).
    pub fn frame(&self, grow: f32) -> (Vec2, Vec2) {
        match self {
            Self::Rect { min, max } => (*min - Vec2::splat(grow), *max + Vec2::splat(grow)),
            Self::Point(p) => (
                *p - Vec2::splat(POINT_RADIUS + grow),
                *p + Vec2::splat(POINT_RADIUS + grow),
            ),
        }
    }

    fn json(&self) -> Value {
        match self {
            Self::Rect { min, max } => json!({
                "rect": [min.x.round(), min.y.round(), max.x.round(), max.y.round()]
            }),
            Self::Point(p) => json!({"point": [p.x.round(), p.y.round()]}),
        }
    }
}

/// Last shape resolved (dump, overlay).
#[derive(Resource, Default)]
pub struct TutorialTarget {
    pub shape: Option<Shape>,
    /// Name of the target of the current step.
    pub name: String,
    /// What resolved it: `ui:<name>`, `world`, `fallback` or empty.
    pub source: String,
}

/// Everything `resolve` reads.
#[derive(SystemParam)]
pub struct TargetLookup<'w, 's> {
    ui: Query<
        'w,
        's,
        (
            &'static Name,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
    >,
    camera: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<CampaignCamera>>,
    ground: GroundHeight<'w>,
    views: Res<'w, ArmyViews>,
    map: Option<Res<'w, ProvinceMap>>,
    sim: Res<'w, Sim>,
    scale: Res<'w, UiScale>,
}

impl TargetLookup<'_, '_> {
    /// Layout-pixel rectangle of a visible widget (at least 2 px).
    fn widget_rect(&self, wanted: Candidate) -> Option<(Vec2, Vec2)> {
        let mut union: Option<(Vec2, Vec2)> = None;
        for (name, node, transform, visibility) in &self.ui {
            let matches = match wanted {
                Candidate::Name(n) => name.as_str() == n,
                Candidate::Prefix(p) => name.as_str().starts_with(p),
            };
            if !matches || !visibility.get() {
                continue;
            }
            let center = transform.translation * node.inverse_scale_factor;
            let half = node.size() * node.inverse_scale_factor * 0.5;
            if half.x < 1.0 || half.y < 1.0 {
                continue;
            }
            let (min, max) = (center - half, center + half);
            union = Some(union.map_or((min, max), |(a, b)| (a.min(min), b.max(max))));
        }
        union
    }

    /// Screen position (layout pixels) of the map point `px`, `None` behind the camera or off
    /// screen.
    fn map_point(&self, px: Vec2) -> Option<Vec2> {
        let (camera, transform) = self.camera.single().ok()?;
        let height = self.ground.at(px.x, px.y).unwrap_or(0.0) + POINT_LIFT;
        let screen = camera
            .world_to_viewport(transform, Vec3::new(px.x, height, px.y))
            .ok()?;
        let size = camera.logical_viewport_size()?;
        if screen.x < 0.0 || screen.y < 0.0 || screen.x > size.x || screen.y > size.y {
            return None;
        }
        Some(screen / self.scale.0.max(0.1))
    }

    fn capital(&self) -> Option<String> {
        let state = self.sim.state.as_ref()?;
        state
            .factions
            .get(state.player_faction())
            .map(|f| f.capital.to_string())
    }

    /// Map pixel of the capital (centroid of its province).
    pub fn capital_px(&self) -> Option<Vec2> {
        let map = self.map.as_ref()?;
        let id = self.capital()?;
        let province = map.province(map.index_of_id(&id))?;
        Some(Vec2::new(province.centroid[0], province.centroid[1]))
    }

    /// Map pixel of an army of the player.
    pub fn army_px(&self, army: &str) -> Option<Vec2> {
        self.views
            .0
            .iter()
            .find(|v| v.id.as_str() == army)
            .map(|v| Vec2::new(v.point[0], v.point[1]))
    }

    /// Shape and source of `target`; `royal` is the royal army's id.
    pub fn resolve(&self, target: &str, royal: Option<&str>) -> Option<(Shape, String)> {
        if is_world_target(target) {
            let px = match target {
                "capital" => self.capital_px(),
                _ => royal.and_then(|r| self.army_px(r)),
            };
            if let Some(point) = px.and_then(|p| self.map_point(p)) {
                return Some((Shape::Point(point), "world".to_owned()));
            }
        }
        for candidate in candidates(target) {
            if let Some((min, max)) = self.widget_rect(*candidate) {
                let source = match candidate {
                    Candidate::Name(n) | Candidate::Prefix(n) => format!("ui:{n}"),
                };
                return Some((Shape::Rect { min, max }, source));
            }
        }
        if falls_back_on_capital(target)
            && let Some(point) = self.capital_px().and_then(|p| self.map_point(p))
        {
            return Some((Shape::Point(point), "fallback".to_owned()));
        }
        None
    }
}

#[derive(Component)]
struct TargetHalo;
#[derive(Component)]
struct TargetArrow;

pub fn plugin(app: &mut App) {
    app.init_resource::<TutorialTarget>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), spawn_overlay)
        .add_systems(
            Update,
            (update_target, draw_overlay)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        );
}

fn spawn_overlay(mut commands: Commands) {
    commands
        .spawn((
            Name::new("TutorialHalo"),
            TargetHalo,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(GOLD),
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
            Pickable::IGNORE,
            UiTier::Tutorial.index(),
        ))
        .with_children(|halo| {
            // A soft dark shadow ring around the gold one keeps it readable on a busy map.
            halo.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-6.0),
                    top: Val::Px(-6.0),
                    right: Val::Px(-6.0),
                    bottom: Val::Px(-6.0),
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BorderColor::all(SHADOW),
                Pickable::IGNORE,
            ));
        });
    commands.spawn((
        Name::new("TutorialArrow"),
        TargetArrow,
        DespawnOnExit(AppState::Campaign),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Text::new("▼"),
        TextFont {
            font_size: bevy::text::FontSize::Px(30.0),
            ..default()
        },
        TextColor(GOLD),
        Visibility::Hidden,
        Pickable::IGNORE,
        UiTier::Tutorial.index(),
    ));
}

fn update_target(
    tutorial: Res<Tutorial>,
    lookup: TargetLookup,
    mut target: ResMut<TutorialTarget>,
) {
    let (name, show) = tutorial.arrow_target();
    let resolved = if show {
        lookup.resolve(name, tutorial.royal_army())
    } else {
        None
    };
    target.name = name.to_owned();
    match resolved {
        Some((shape, source)) => {
            target.shape = Some(shape);
            target.source = source;
        }
        None => {
            target.shape = None;
            target.source.clear();
        }
    }
}

/// Pulse in 0..1.
pub fn pulse(seconds: f32, reduce_motion: bool) -> f32 {
    if reduce_motion {
        return 0.5;
    }
    0.5 + 0.5 * (seconds * std::f32::consts::TAU / PULSE_PERIOD).sin()
}

#[allow(clippy::type_complexity)]
fn draw_overlay(
    target: Res<TutorialTarget>,
    time: Res<Time<Real>>,
    reduce: Res<ReduceMotion>,
    mut halo: Query<
        (&mut Node, &mut Visibility, &mut BorderColor),
        (With<TargetHalo>, Without<TargetArrow>),
    >,
    mut arrow: Query<
        (&mut Node, &mut Visibility, &mut TextColor, &mut Text),
        (With<TargetArrow>, Without<TargetHalo>),
    >,
) {
    let strength = pulse(time.elapsed_secs(), reduce.0);
    let Some(shape) = target.shape else {
        for (_, mut visibility, _) in &mut halo {
            visibility.set_if_neq(Visibility::Hidden);
        }
        for (_, mut visibility, _, _) in &mut arrow {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let (min, max) = shape.frame(4.0 + 2.0 * strength);
    for (mut node, mut visibility, mut border) in &mut halo {
        node.left = Val::Px(min.x);
        node.top = Val::Px(min.y);
        node.width = Val::Px(max.x - min.x);
        node.height = Val::Px(max.y - min.y);
        node.border_radius = match shape {
            Shape::Point(_) => BorderRadius::MAX,
            Shape::Rect { .. } => BorderRadius::ZERO,
        };
        *border = BorderColor::all(GOLD.with_alpha(0.65 + 0.35 * strength));
        visibility.set_if_neq(Visibility::Inherited);
    }
    // The arrow hangs above the frame and bobs towards it.
    let bob = if reduce.0 { 0.0 } else { 6.0 * strength };
    let arrow_size = 30.0;
    for (mut node, mut visibility, mut color, mut glyph) in &mut arrow {
        let x = shape.center().x - arrow_size * 0.5;
        let above = min.y - arrow_size - 4.0 + bob;
        // Under the frame when there is no room above (a bar at the top of the screen).
        let (y, pointing_up) = if above < 2.0 {
            (max.y + 4.0 - bob, true)
        } else {
            (above, false)
        };
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        let wanted = if pointing_up { "▲" } else { "▼" };
        if glyph.0 != wanted {
            glyph.0 = wanted.to_owned();
        }
        color.0 = GOLD.with_alpha(0.8 + 0.2 * strength);
        visibility.set_if_neq(Visibility::Inherited);
    }
}

fn dump(world: &World) -> (String, Value) {
    let target = world.resource::<TutorialTarget>();
    (
        "progress_target".to_owned(),
        json!({
            "name": target.name,
            "source": target.source,
            "shape": target.shape.map(|s| s.json()),
            "halo_visible": target.shape.is_some(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_target_of_the_data_has_a_way_to_resolve() {
        let raw = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/tutorial/steps.json"),
        )
        .expect("steps.json");
        let value: Value = serde_json::from_str(&raw).unwrap();
        for (id, step) in value["steps"].as_object().unwrap() {
            let target = step["target"].as_str().unwrap_or_default();
            if target.is_empty() {
                continue;
            }
            assert!(
                is_world_target(target) || !candidates(target).is_empty(),
                "step {id}: target {target:?} unresolved"
            );
        }
    }

    #[test]
    fn focus_follows_the_godot_steps() {
        let point = Vec2::new(10.0, 20.0);
        assert_eq!(
            focus_for("select_army", Some(point), Some("prov_x")),
            Some(FocusCamera::Point(point))
        );
        assert_eq!(
            focus_for("open_province", Some(point), Some("prov_x")),
            Some(FocusCamera::Province("prov_x".to_owned()))
        );
        assert_eq!(focus_for("research", Some(point), Some("prov_x")), None);
        assert_eq!(focus_for("select_army", None, Some("prov_x")), None);
    }

    #[test]
    fn frames_grow_and_pulse_is_bounded() {
        let rect = Shape::Rect {
            min: Vec2::new(10.0, 10.0),
            max: Vec2::new(30.0, 20.0),
        };
        assert_eq!(
            rect.frame(4.0),
            (Vec2::new(6.0, 6.0), Vec2::new(34.0, 24.0))
        );
        let (min, max) = Shape::Point(Vec2::new(100.0, 100.0)).frame(0.0);
        assert_eq!(max - min, Vec2::splat(2.0 * POINT_RADIUS));
        for t in [0.0, 0.3, 0.9, 5.0] {
            assert!((0.0..=1.0).contains(&pulse(t, false)));
        }
        assert_eq!(pulse(0.4, true), 0.5);
    }

    #[test]
    fn widgets_by_name_and_prefix() {
        assert_eq!(candidates("end_turn"), &[Candidate::Name("hud:end_turn")]);
        assert!(candidates("buildable").contains(&Candidate::Prefix("prov:build:")));
        assert!(candidates("nothing").is_empty());
    }
}
