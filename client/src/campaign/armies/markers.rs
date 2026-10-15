//! Army markers (pole + banner in the faction colour) and 2D plates (men count, status).
//! Port of `army_markers.gd` / `army_marker.gd` / `army_plate.gd`, simplified: no animated
//! figures, no plate declutter (see wip note).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_campaign::ArmyId;

use super::figures::{ArmyFigure, FIGURE_HEIGHT, FigureSpec, Livery, Look, layout};
use super::projection::{Ground, px_to_world};
use super::scale::{DEFAULT_SCALE_EXPONENT, scale_for_distance};
use super::sea::Glides;
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::app::AppState;
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
    assets: Res<AssetServer>,
    mut markers: Query<(Entity, &mut ArmyMarker)>,
    plates: Query<(Entity, &ArmyPlate)>,
) {
    if !(views.is_changed() || glides.is_changed()) {
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
        let specs = specs_for(view, glides.active.contains_key(&view.id));
        let color = Color::srgb(view.color[0], view.color[1], view.color[2]);
        kept.insert(view.id.clone());
        if let Some((entity, _)) = existing.remove(&view.id)
            && let Ok((_, mut marker)) = markers.get_mut(entity)
        {
            marker.point = point;
            marker.slot = slot;
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
    mut markers: Query<(&ArmyMarker, &mut Transform)>,
    selected: Res<SelectedArmy>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    for (marker, mut transform) in &mut markers {
        let glide = glides.active.get(&marker.id).and_then(|g| g.pose());
        let point = glide.map_or(marker.point, |(p, _)| p);
        let base = px_to_world(point, &ground);
        let distance = camera_distance(camera, base);
        let mut scale = scale_for_distance(distance, 0.0, DEFAULT_SCALE_EXPONENT);
        if selected.0.as_ref() == Some(&marker.id) {
            scale *= 1.15;
        }
        let slot = if glide.is_some() { (0, 1) } else { marker.slot };
        transform.translation = marker_base(point, slot, scale, &ground);
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
) -> Option<Rect> {
    let base = marker.translation;
    let top = base + Vec3::Y * POLE_HEIGHT * marker.scale.y;
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

pub fn update_plates(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    mut plates: Query<(&ArmyPlate, &mut Node, &mut Visibility)>,
    hovered: Res<HoveredArmy>,
    selected: Res<SelectedArmy>,
    mut borders: Query<(&ArmyPlate, &mut BorderColor)>,
    views: Res<ArmyViews>,
) {
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    for (plate, mut node, mut visibility) in &mut plates {
        let anchor = markers
            .iter()
            .find(|(m, _)| m.id == plate.id)
            .and_then(|(_, t)| {
                let top = t.translation + Vec3::Y * (POLE_HEIGHT + 1.0) * t.scale.y;
                camera.world_to_viewport(camera_transform, top).ok()
            });
        match anchor {
            Some(screen) => {
                node.left = Val::Px(screen.x - PLATE_SIZE.x * 0.5);
                node.top = Val::Px(screen.y - PLATE_SIZE.y);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
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
