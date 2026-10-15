//! Army markers (pole + banner in the faction colour) and 2D plates (men count, status).
//! Port of `army_markers.gd` / `army_marker.gd` / `army_plate.gd`, simplified: no animated
//! figures, no plate declutter (see wip note).

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_campaign::ArmyId;

use super::figures::{ArmyFigure, FIGURE_HEIGHT, FigureSpec, Livery, Look, layout};
use super::plate_layout::{PlateInput, VIEWPORT_MARGIN, place_plates, too_far_for_close_view};
use super::projection::{Ground, px_to_world};
use super::scale::CLOSE_KNEE_DISTANCE;
use super::scale::{DEFAULT_SCALE_EXPONENT, scale_for_distance};
use super::sea::Glides;
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::app::AppState;
use crate::campaign::markers::places::Places;
use crate::campaign::provinces::labels::{LabelPlacer, RegionLabelObstacles};
use crate::campaign::terrain::surface::GroundHeight;

/// Height of the figures (leader and banner) in marker-local units, scaled by the scale law.
pub const POLE_HEIGHT: f32 = FIGURE_HEIGHT;
const PLATE_SIZE: Vec2 = Vec2::new(66.0, 20.0);
const PLAYER_BORDER: Color = Color::srgb(0.85, 0.66, 0.2);
const OTHER_BORDER: Color = Color::srgb(0.30, 0.20, 0.12);
const ENEMY_BORDER: Color = Color::srgb(0.75, 0.12, 0.1);

#[derive(Component)]
pub struct ArmyMarker {
    pub id: ArmyId,
    /// Ground point of the army (map pixel).
    pub point: Vec2,
    /// Slot among the armies sharing the point: (index, count).
    pub slot: (usize, usize),
    /// Figures currently spawned under the marker, and the livery they wear.
    pub specs: Vec<FigureSpec>,
    pub color: [f32; 3],
    /// Radius of the settlement the army garrisons plus margin (ADR 0160); `None` in the field.
    pub standoff: Option<f32>,
    /// 0 while the army glides along a voyage, 1 at rest: eased every frame.
    pub standoff_weight: f32,
    /// Point (map pixel) the figures stand on now: slides to `point` after an order, so that the
    /// troop walks instead of jumping.
    pub shown: Vec2,
    /// Yaw (rad, Bevy convention) of the troop frame: towards the last walking direction.
    pub heading: f32,
    /// The troop is walking (the shown point has not reached `point`).
    pub walking: bool,
    /// Ground speed (world units per second) of the shown point, smoothed.
    pub ground_speed: f32,
    /// Height of the figures in marker-local units (plate anchor, picking).
    pub height: f32,
}

impl ArmyMarker {
    /// Plate under the figures instead of above them (`ArmyMarker.plate_below`, always false).
    pub fn plate_below(&self) -> bool {
        false
    }
}

/// Walking speed of a troop that slides to a new point (map pixels per second), and the longest
/// slide in seconds (faster beyond that).
pub const WALK_PX_PER_S: f32 = 40.0;
pub const WALK_MAX_SECONDS: f32 = 2.5;
/// Slides longer than this snap (loading a save, teleports).
pub const WALK_SNAP_PX: f32 = 600.0;
/// Interval of the plate placement when nothing else asks for it (`PLACEMENT_INTERVAL`).
pub const PLATE_INTERVAL_S: f32 = 0.25;

/// New shown point after `dt` seconds of sliding from `shown` to `target`, and the speed (px/s)
/// of that step (0 when the slide is over or snaps).
pub fn slide_step(shown: Vec2, target: Vec2, dt: f32) -> (Vec2, f32) {
    let delta = target - shown;
    let distance = delta.length();
    if distance < 0.01 || distance > WALK_SNAP_PX {
        return (target, 0.0);
    }
    let speed = WALK_PX_PER_S.max(distance / WALK_MAX_SECONDS);
    let step = speed * dt;
    if step >= distance {
        (target, distance / dt.max(1e-4))
    } else {
        (shown + delta / distance * step, speed)
    }
}

/// Direction (map pixels) in which a garrison stands beside its town (`STANDOFF_DIRECTION`).
pub const STANDOFF_DIRECTION: Vec2 = Vec2::new(0.86, 0.51);
/// Margin beyond the town radius (`STANDOFF_MARGIN`).
pub const STANDOFF_MARGIN: f32 = 0.4;
/// Half footprint of the host at scale 1 (`ArmyMarker.FOOTPRINT_RADIUS`).
pub const FOOTPRINT_RADIUS: f32 = 4.6;
/// Easing rate (per second) of the stand-off when a march starts or ends.
const STANDOFF_EASE: f32 = 8.0;

/// Current gap from the town centre: town radius plus the host's half footprint at `scale`.
pub fn standoff_distance(base: f32, scale: f32, weight: f32) -> f32 {
    (base + FOOTPRINT_RADIUS * scale) * weight
}

/// Stand-off base of an army in `settlement` (`_standoff_base`).
fn standoff_base(places: &Places, settlement: Option<&str>) -> Option<f32> {
    let place = places.get(settlement?)?;
    Some(place.radius + STANDOFF_MARGIN)
}

/// Plate placement measures (dump).
#[derive(Resource, Default, Debug)]
pub struct PlateStats {
    pub visible: usize,
    pub displaced: usize,
    pub hidden: usize,
    /// Pairs of shown plates that still overlap after placement.
    pub overlaps: usize,
    /// Placements computed since the start (the rest of the frames reuse the offsets).
    pub recomputes: usize,
    /// Frames that reused the previous offsets.
    pub reused: usize,
}

/// Offsets kept between two placements (`ArmyPlateLayout._offsets`, `_due`).
#[derive(Default)]
pub struct PlateCache {
    offsets: HashMap<String, Vec2>,
    camera: Option<GlobalTransform>,
    viewport: Vec2,
    timer: f32,
    ids: Vec<String>,
    blocked: Vec<Rect>,
}

#[derive(Component)]
pub struct ArmyPlate {
    pub id: ArmyId,
}

/// The text of a plate.
#[derive(Component)]
pub struct PlateText {
    pub id: ArmyId,
}

/// Plate text: `12 400 »` style, men count with a thin space and the status glyph.
pub fn plate_text(men: u32, status: &str) -> String {
    let digits = men.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push('\u{202f}');
        }
        grouped.push(c);
    }
    match status {
        "moving" => format!("{grouped} »"),
        "siege" => format!("{grouped} siège"),
        _ => grouped,
    }
}

/// Seed of an army for the escort jitter.
fn seed_of(id: &ArmyId) -> u32 {
    id.as_str()
        .bytes()
        .fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)))
}

/// Figures an army shows now: a fleet while it glides along a voyage.
pub fn specs_for(view: &super::views::ArmyView, at_sea: bool) -> Vec<FigureSpec> {
    layout(
        &Look {
            categories: &view.categories,
            men: view.men,
            sieging: view.status == "siege",
            at_sea,
            camped: !view.in_settlement && view.status == "idle",
        },
        seed_of(&view.id),
    )
}

fn spawn_figures(
    root: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    specs: &[FigureSpec],
    color: Color,
) {
    for spec in specs {
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(spec.path));
        root.spawn((
            Name::new("Figure"),
            ArmyFigure { path: spec.path },
            Livery(color),
            WorldAssetRoot(scene),
            Transform {
                translation: spec.offset,
                rotation: Quat::from_rotation_y(spec.yaw),
                scale: Vec3::splat(spec.scale),
            },
        ));
    }
}

/// Spawns, updates and removes the markers (figures + plate) to match the army list. A marker
/// whose figures and livery did not change is kept (its glTF scenes are not reloaded).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_markers(
    mut commands: Commands,
    views: Res<ArmyViews>,
    glides: Res<Glides>,
    places: Res<Places>,
    assets: Res<AssetServer>,
    skinned: Res<super::skinned::SkinnedArmies>,
    mut markers: Query<(Entity, &mut ArmyMarker)>,
    plates: Query<(Entity, &ArmyPlate)>,
) {
    if !(views.is_changed() || glides.is_changed() || places.is_changed()) {
        return;
    }
    let mut counts: std::collections::HashMap<(i32, i32), usize> = Default::default();
    for view in &views.0 {
        *counts.entry(key(view.point)).or_default() += 1;
    }
    let mut seen: std::collections::HashMap<(i32, i32), usize> = Default::default();
    let mut kept: std::collections::HashSet<ArmyId> = Default::default();
    let mut have_plate: std::collections::HashSet<ArmyId> =
        plates.iter().map(|(_, p)| p.id.clone()).collect();
    let mut existing: std::collections::HashMap<ArmyId, (Entity, usize)> = markers
        .iter()
        .enumerate()
        .map(|(i, (e, m))| (m.id.clone(), (e, i)))
        .collect();
    for view in &views.0 {
        let k = key(view.point);
        let index = *seen.entry(k).and_modify(|i| *i += 1).or_insert(0);
        let slot = (index, counts[&k]);
        let point = Vec2::new(view.point[0], view.point[1]);
        let mut specs = specs_for(view, glides.active.contains_key(&view.id));
        if skinned.enabled {
            // The troop is drawn by the skinned batches; camps and ships stay glTF.
            specs.retain(|s| !s.path.contains("models/army"));
        }
        let color = Color::srgb(view.color[0], view.color[1], view.color[2]);
        kept.insert(view.id.clone());
        let standoff = standoff_base(&places, view.settlement.as_deref());
        if let Some((entity, _)) = existing.remove(&view.id)
            && let Ok((_, mut marker)) = markers.get_mut(entity)
        {
            marker.point = point;
            marker.slot = slot;
            marker.standoff = standoff;
            if marker.specs == specs && marker.color == view.color {
                continue;
            }
            marker.specs = specs.clone();
            marker.color = view.color;
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|root| spawn_figures(root, &assets, &specs, color));
            continue;
        }
        commands
            .spawn((
                Name::new(format!("Army_{}", view.id.as_str())),
                ArmyMarker {
                    id: view.id.clone(),
                    point,
                    slot,
                    specs: specs.clone(),
                    color: view.color,
                    standoff,
                    standoff_weight: 1.0,
                    shown: point,
                    heading: 0.0,
                    walking: false,
                    ground_speed: 0.0,
                    height: if skinned.enabled {
                        super::skinned::SKINNED_HEIGHT
                    } else {
                        POLE_HEIGHT
                    },
                },
                Transform::default(),
                Visibility::default(),
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|root| spawn_figures(root, &assets, &specs, color));
        if have_plate.insert(view.id.clone()) {
            spawn_plate(&mut commands, view);
        }
    }
    for (id, (entity, _)) in existing {
        if !kept.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, plate) in &plates {
        if !kept.contains(&plate.id) {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_plate(commands: &mut Commands, view: &super::views::ArmyView) {
    let border = if view.player {
        PLAYER_BORDER
    } else if view.hostile {
        ENEMY_BORDER
    } else {
        OTHER_BORDER
    };
    commands
        .spawn((
            Name::new(format!("Plate_{}", view.id.as_str())),
            ArmyPlate {
                id: view.id.clone(),
            },
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(PLATE_SIZE.x),
                height: Val::Px(PLATE_SIZE.y),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.94, 0.89, 0.76, 0.94)),
            BorderColor::all(border),
            GlobalZIndex(-1),
        ))
        .with_children(|plate| {
            plate.spawn((
                PlateText {
                    id: view.id.clone(),
                },
                Text::new(plate_text(view.men, view.status)),
                TextFont {
                    font_size: bevy::text::FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.16, 0.10, 0.05)),
            ));
        });
}

/// Keeps the plate texts at the current men count and status.
pub fn refresh_plate_text(views: Res<ArmyViews>, mut texts: Query<(&PlateText, &mut Text)>) {
    if !views.is_changed() {
        return;
    }
    for (plate, mut text) in &mut texts {
        if let Some(view) = views.get(&plate.id) {
            let wanted = plate_text(view.men, view.status);
            if text.0 != wanted {
                text.0 = wanted;
            }
        }
    }
}

fn key(point: [f32; 2]) -> (i32, i32) {
    (point[0] as i32, point[1] as i32)
}

/// World position of a marker's base: ground point plus a ring offset for stacked armies.
pub fn marker_base(point: Vec2, slot: (usize, usize), scale: f32, ground: &impl Ground) -> Vec3 {
    let (index, count) = slot;
    let offset = if count > 1 {
        let angle = std::f32::consts::TAU * index as f32 / count as f32;
        Vec2::from_angle(angle) * 3.0 * scale
    } else {
        Vec2::ZERO
    };
    px_to_world(point + offset, ground)
}

pub fn camera_distance(camera: &GlobalTransform, target: Vec3) -> f32 {
    camera.translation().distance(target)
}

pub fn place_markers(
    ground: GroundHeight,
    glides: Res<Glides>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut markers: Query<(&mut ArmyMarker, &mut Transform)>,
    selected: Res<SelectedArmy>,
    time: Res<Time>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    let dt = time.delta_secs();
    let ease = 1.0 - (-STANDOFF_EASE * dt).exp();
    let smooth = 1.0 - (-5.0 * dt).exp();
    for (mut marker, mut transform) in &mut markers {
        let glide = glides.active.get(&marker.id).and_then(|g| g.pose());
        let target_weight = if glide.is_some() { 0.0 } else { 1.0 };
        marker.standoff_weight += (target_weight - marker.standoff_weight) * ease;
        // Slide of the troop towards its point (walking animation, heading of the march).
        let (shown, speed) = if glide.is_some() {
            (marker.point, 0.0)
        } else {
            slide_step(marker.shown, marker.point, dt)
        };
        let walking = glide.is_none() && speed > 0.0;
        if walking {
            let direction = marker.point - marker.shown;
            if direction.length_squared() > 1e-6 {
                let wanted = (-direction.y).atan2(direction.x);
                let turn = (wanted - marker.heading + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                marker.heading += turn * (1.0 - (-8.0 * dt).exp());
            }
        }
        marker.shown = shown;
        marker.walking = walking;
        marker.ground_speed += (speed - marker.ground_speed) * smooth;
        if !walking && marker.ground_speed < 0.01 {
            marker.ground_speed = 0.0;
        }
        let point = glide.map_or(shown, |(p, _)| p);
        let base = px_to_world(point, &ground);
        let distance = camera_distance(camera, base);
        let mut scale = scale_for_distance(distance, 0.0, DEFAULT_SCALE_EXPONENT);
        if selected.0.as_ref() == Some(&marker.id) {
            scale *= 1.15;
        }
        let slot = if glide.is_some() { (0, 1) } else { marker.slot };
        let aside = marker.standoff.map_or(Vec2::ZERO, |base| {
            STANDOFF_DIRECTION.normalize() * standoff_distance(base, scale, marker.standoff_weight)
        });
        transform.translation = marker_base(point + aside, slot, scale, &ground);
        transform.scale = Vec3::splat(scale);
        // The troop looks at +X; a fleet heads along its route (`yaw` turns +X towards -Z).
        transform.rotation = glide.map_or(Quat::IDENTITY, |(_, dir)| {
            Quat::from_rotation_y((-dir.y).atan2(dir.x))
        });
    }
}

/// Screen rectangle of the marker silhouette (pole + banner).
pub fn screen_rect(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    marker: &Transform,
    height: f32,
) -> Option<Rect> {
    let base = marker.translation;
    let top = base + Vec3::Y * height * marker.scale.y;
    let a = camera.world_to_viewport(camera_transform, base).ok()?;
    let b = camera.world_to_viewport(camera_transform, top).ok()?;
    let height = (a.y - b.y).abs().max(6.0);
    let width = (height * 0.55).max(6.0);
    let center_x = a.x + width * 0.2;
    Some(Rect::new(
        center_x - width * 0.5,
        a.y.min(b.y),
        center_x + width * 0.5,
        a.y.max(b.y),
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn update_plates(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    mut plates: Query<(&ArmyPlate, &mut Node, &mut Visibility)>,
    hovered: Res<HoveredArmy>,
    selected: Res<SelectedArmy>,
    mut borders: Query<(&ArmyPlate, &mut BorderColor)>,
    views: Res<ArmyViews>,
    obstacles: Res<RegionLabelObstacles>,
    mut placer: Local<LabelPlacer>,
    mut cache: Local<PlateCache>,
    time: Res<Time>,
    mut stats: ResMut<PlateStats>,
) {
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let viewport = camera
        .logical_viewport_rect()
        .unwrap_or(Rect::new(0.0, 0.0, 1.0e5, 1.0e5));
    let shown_area = Rect::new(
        viewport.min.x - VIEWPORT_MARGIN,
        viewport.min.y - VIEWPORT_MARGIN,
        viewport.max.x + VIEWPORT_MARGIN,
        viewport.max.y + VIEWPORT_MARGIN,
    );
    let mut inputs = Vec::new();
    let mut hidden = 0usize;
    let mut anchors: HashMap<ArmyId, Rect> = HashMap::new();
    for (plate, _, _) in &plates {
        let Some((marker, transform)) = markers.iter().find(|(m, _)| m.id == plate.id) else {
            continue;
        };
        let top = transform.translation + Vec3::Y * (marker.height + 1.0) * transform.scale.y;
        let is_selected = selected.0.as_ref() == Some(&plate.id);
        let anchor_distance = camera_transform.translation().distance(top);
        let camera_dist = camera_distance(camera_transform, transform.translation);
        let Ok(screen) = camera.world_to_viewport(camera_transform, top) else {
            continue;
        };
        if !shown_area.contains(screen)
            || too_far_for_close_view(
                camera_dist,
                anchor_distance,
                is_selected,
                CLOSE_KNEE_DISTANCE,
            )
        {
            continue;
        }
        let lift = if marker.plate_below() {
            -2.0
        } else {
            PLATE_SIZE.y + 2.0
        };
        let min = Vec2::new(screen.x - PLATE_SIZE.x * 0.5, screen.y - lift).round();
        let base = Rect::from_corners(min, min + PLATE_SIZE);
        anchors.insert(plate.id.clone(), base);
        let view = views.get(&marker.id);
        inputs.push(PlateInput {
            id: plate.id.as_str().to_owned(),
            base,
            selected: is_selected,
            player: view.is_some_and(|v| v.player),
            men: view.map_or(0, |v| v.men),
        });
    }
    let mut blocked: Vec<Rect> = obstacles.rects.clone();
    if obstacles.top_inset > 0.0 {
        blocked.push(Rect::new(-1.0e4, -1.0e4, 1.0e5, obstacles.top_inset));
    }
    // Placement: when the camera or the view moved, the armies or the selection changed, a plate
    // appeared or vanished, or every `PLATE_INTERVAL_S`; otherwise the plates keep their offsets
    // (no flicker, no useless work).
    cache.timer -= time.delta_secs();
    let mut ids: Vec<String> = inputs.iter().map(|i| i.id.clone()).collect();
    ids.sort();
    let moved = cache
        .camera
        .is_none_or(|old| old.affine() != camera_transform.affine())
        || cache.viewport != viewport.size();
    let due = moved
        || views.is_changed()
        || selected.is_changed()
        || cache.blocked != blocked
        || ids != cache.ids
        || cache.timer <= 0.0;
    if due {
        cache.offsets = place_plates(&mut placer, inputs, &blocked);
        cache.camera = Some(*camera_transform);
        cache.viewport = viewport.size();
        cache.timer = PLATE_INTERVAL_S;
        cache.ids = ids;
        cache.blocked = blocked.clone();
        stats.recomputes += 1;
    } else {
        stats.reused += 1;
    }
    let offsets = cache.offsets.clone();
    stats.visible = anchors.len();
    stats.displaced = offsets.values().filter(|o| **o != Vec2::ZERO).count();
    for (plate, mut node, mut visibility) in &mut plates {
        match anchors.get(&plate.id) {
            Some(base) => {
                let offset = offsets.get(plate.id.as_str()).copied().unwrap_or_default();
                node.left = Val::Px(base.min.x + offset.x);
                node.top = Val::Px(base.min.y + offset.y);
                *visibility = Visibility::Inherited;
            }
            None => {
                hidden += 1;
                *visibility = Visibility::Hidden;
            }
        }
    }
    stats.hidden = hidden;
    let finals: Vec<Rect> = anchors
        .iter()
        .map(|(id, base)| {
            let o = offsets.get(id.as_str()).copied().unwrap_or_default();
            Rect::from_corners(base.min + o, base.max + o)
        })
        .collect();
    stats.overlaps = (0..finals.len())
        .flat_map(|i| (i + 1..finals.len()).map(move |j| (i, j)))
        .filter(|&(i, j)| !finals[i].intersect(finals[j]).is_empty())
        .count();
    for (plate, mut border) in &mut borders {
        let lit = selected.0.as_ref() == Some(&plate.id) || hovered.0.as_ref() == Some(&plate.id);
        let color = if lit {
            Color::srgb(1.0, 0.9, 0.4)
        } else {
            match views.get(&plate.id) {
                Some(v) if v.player => PLAYER_BORDER,
                Some(v) if v.hostile => ENEMY_BORDER,
                _ => OTHER_BORDER,
            }
        };
        *border = BorderColor::all(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plate_text_groups_thousands() {
        assert_eq!(plate_text(12400, "idle"), "12\u{202f}400");
        assert_eq!(plate_text(800, "moving"), "800 »");
        assert_eq!(plate_text(1000, "siege"), "1\u{202f}000 siège");
    }
}
