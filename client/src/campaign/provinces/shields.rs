//! Party shields of the Feudal map mode: one shield per province in double allegiance (the
//! holder's arms on the left half, its second lord's on the right), fixed in screen size above
//! the province centre. Port of `FeudalMapLens.place_shields` / `party_shield`
//! (game/scripts/map/feudal_map_lens.gd), as UI nodes projected on the map like the incident
//! seals: the arms are the player's `heraldry/<faction>.png` assets, halved.

use std::collections::HashMap;

use crate::ui::kit::viewport_to_ui;
use bevy::prelude::*;
use bevy::ui::UiScale;

use super::data::ProvinceMap;
use super::modes::{MapMode, MapModeState};
use super::overlay::OverlayState;
use crate::app::AppState;
use crate::campaign::terrain::surface::GroundHeight;

/// Screen size (px) of a shield (48 x 56 texture of Godot, shown at `pixel_size` 0.0006).
pub const SHIELD_SIZE: Vec2 = Vec2::new(26.0, 30.0);
/// Lift above the ground at the province centre (map units, Godot `+ 0.6`).
const SHIELD_LIFT: f32 = 0.6;
const EDGE: Color = Color::srgb(0.12, 0.08, 0.05);

/// Provinces in double allegiance: `(raster index, holder, second lord)`.
type Doubles = Vec<(u16, String, String)>;

/// A placed shield.
#[derive(Component)]
pub struct FeudalShield {
    pub index: u16,
    pub holder: String,
    pub second_lord: String,
}

/// Shields currently on the map, for the dump.
#[derive(Resource, Default)]
pub struct ShieldStats {
    pub spawned: usize,
    pub visible: usize,
    pub rebuilds: u32,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<ShieldStats>().add_systems(
        Update,
        (sync_shields, place_shields)
            .chain()
            .run_if(in_state(AppState::Campaign))
            .run_if(resource_exists::<OverlayState>),
    );
}

/// Keeps the shields in step with the doubles of the overlay (Feudal mode only).
fn sync_shields(
    mut commands: Commands,
    mode: Res<MapModeState>,
    overlay: Res<OverlayState>,
    assets: Res<AssetServer>,
    existing: Query<Entity, With<FeudalShield>>,
    mut stats: ResMut<ShieldStats>,
    mut built: Local<Option<Doubles>>,
) {
    let wanted = (mode.mode == MapMode::Feudal).then(|| overlay.doubles.clone());
    if wanted.as_ref() == built.as_ref() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    stats.spawned = 0;
    if let Some(doubles) = wanted {
        for (index, holder, second_lord) in &doubles {
            spawn_shield(
                &mut commands,
                &assets,
                &overlay.heraldic,
                *index,
                holder,
                second_lord,
            );
        }
        stats.spawned = doubles.len();
        stats.rebuilds += 1;
        *built = Some(doubles);
    } else {
        *built = None;
    }
}

fn spawn_shield(
    commands: &mut Commands,
    assets: &AssetServer,
    heraldic: &HashMap<String, Srgba>,
    index: u16,
    holder: &str,
    second_lord: &str,
) {
    let half = SHIELD_SIZE.x / 2.0;
    let arms = |faction: &str, offset: f32| {
        // The arms are drawn at the full width of the shield and clipped to their half.
        let backing = heraldic
            .get(faction)
            .map_or(Color::srgb(0.62, 0.60, 0.56), |c| Color::from(*c));
        (
            Node {
                width: Val::Px(half),
                height: Val::Px(SHIELD_SIZE.y),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(backing),
            Pickable::IGNORE,
            children![(
                ImageNode::new(assets.load(format!("heraldry/{faction}.png"))),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(SHIELD_SIZE.x),
                    height: Val::Px(SHIELD_SIZE.y),
                    left: Val::Px(offset),
                    ..default()
                },
                Pickable::IGNORE,
            )],
        )
    };
    commands.spawn((
        Name::new(format!("FeudalShield_{index}")),
        FeudalShield {
            index,
            holder: holder.to_owned(),
            second_lord: second_lord.to_owned(),
        },
        DespawnOnExit(AppState::Campaign),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(SHIELD_SIZE.x),
            height: Val::Px(SHIELD_SIZE.y),
            border: UiRect::all(Val::Px(1.5)),
            // "French ancient" shield: straight top, rounded point.
            border_radius: BorderRadius::bottom(Val::Px(SHIELD_SIZE.x * 0.5)),
            overflow: Overflow::clip(),
            ..default()
        },
        BorderColor::all(EDGE),
        GlobalZIndex(-1),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![arms(holder, 0.0), arms(second_lord, -half)],
    ));
}

/// Projects the shields on the screen each frame.
fn place_shields(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    map: Res<ProvinceMap>,
    ground: GroundHeight,
    mut shields: Query<(&FeudalShield, &mut Node, &mut Visibility)>,
    mut stats: ResMut<ShieldStats>,
    ui_scale: Res<UiScale>,
) {
    let Some((camera, transform)) = camera.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let mut visible = 0;
    for (shield, mut node, mut visibility) in &mut shields {
        let Some(province) = map.province(shield.index) else {
            continue;
        };
        let (x, z) = (province.centroid[0], province.centroid[1]);
        let world = Vec3::new(x, ground.at(x, z).unwrap_or(0.0) + SHIELD_LIFT, z);
        match camera.world_to_viewport(transform, world) {
            Ok(screen) => {
                let screen = viewport_to_ui(screen, &ui_scale);
                node.left = Val::Px(screen.x - SHIELD_SIZE.x * 0.5);
                node.top = Val::Px(screen.y - SHIELD_SIZE.y * 0.5);
                *visibility = Visibility::Inherited;
                visible += 1;
            }
            Err(_) => *visibility = Visibility::Hidden,
        }
    }
    stats.visible = visible;
}
