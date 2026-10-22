//! Province under the cursor. Ports game/scripts/map/province_picker.gd: camera ray, intersection
//! with the plane `Y = 0` refined by fixed-point iterations against the terrain height (when a
//! [`SurfaceHeight`] is installed by the terrain lot), then a lookup in the province raster at
//! `(x, z)`.

use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::data::ProvinceMap;
use crate::campaign::armies::pointer::PointerOverUi;

/// Same constants as the Godot picker.
const REFINE_ITERATIONS: usize = 8;
const REFINE_EPSILON: f32 = 0.02;
/// A press and a release closer than this (screen px) form a click, not a drag.
const CLICK_MAX_DRAG_PX: f32 = 6.0;

/// Terrain height at a map position `(x, z)` (world units). Optional: installed by the terrain
/// lot (`app.insert_resource(SurfaceHeight(...))`); without it the picker uses the plane `Y = 0`.
#[derive(Resource, Clone)]
pub struct SurfaceHeight(pub Arc<dyn Fn(f32, f32) -> f32 + Send + Sync>);

/// Province under the cursor (`index` 0 = sea / none).
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct HoveredProvince {
    pub index: u16,
    pub id: Option<String>,
}

/// Selected province (`index` 0 = none).
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct SelectedProvince {
    pub index: u16,
    pub id: Option<String>,
}

/// A click (press and release within 6 px) on the map. `index` 0 = sea / off map.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ProvinceClicked {
    pub index: u16,
    pub id: Option<String>,
    pub button: MouseButton,
}

/// Request to select a province by id (`None` deselects), for other lots and scripts.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SelectProvince(pub Option<String>);

/// Set `blocked` to swallow the left click of this frame (an army or a UI element took it), like
/// the Godot `click_interceptor`. Systems that set it run `.before(ProvincePickSet)`.
#[derive(Resource, Default)]
pub struct ProvinceClickGuard {
    pub blocked: bool,
    /// The left press went to an army: its release must not also select the province (or the
    /// town) under the army. Kept until that release, unlike `blocked`.
    pub army_press: bool,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProvincePickSet;

/// Point of the terrain under a ray: `None` if the ray does not go down or leaves the map.
/// `size` is the map size in pixels; `height` the terrain height (0 for a flat map).
pub fn pick_ray(
    origin: Vec3,
    direction: Vec3,
    size: Vec2,
    height: &dyn Fn(f32, f32) -> f32,
) -> Option<Vec3> {
    if direction.y >= -0.001 {
        return None;
    }
    let mut surface = 0.0;
    let mut point = origin;
    let mut previous_t = f32::INFINITY;
    for _ in 0..REFINE_ITERATIONS {
        let t = (surface - origin.y) / direction.y;
        if t < 0.0 {
            return None;
        }
        point = origin + direction * t;
        if point.x < 0.0 || point.z < 0.0 || point.x >= size.x || point.z >= size.y {
            return None;
        }
        surface = height(point.x, point.z).max(0.0);
        if (t - previous_t).abs() < REFINE_EPSILON {
            break;
        }
        previous_t = t;
    }
    Some(Vec3::new(point.x, surface, point.z))
}

/// Province index under a ray (0 = sea / outside).
pub fn pick_province(
    map: &ProvinceMap,
    origin: Vec3,
    direction: Vec3,
    height: &dyn Fn(f32, f32) -> f32,
) -> u16 {
    let size = Vec2::new(map.width as f32, map.height as f32);
    pick_ray(origin, direction, size, height).map_or(0, |hit| map.index_at_world(hit))
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<HoveredProvince>()
        .init_resource::<SelectedProvince>()
        .init_resource::<ProvinceClickGuard>()
        .init_resource::<PressState>()
        .add_message::<ProvinceClicked>()
        .add_message::<SelectProvince>()
        .add_systems(
            Update,
            (update_hover, handle_clicks, apply_select_requests)
                .chain()
                .in_set(ProvincePickSet)
                .after(crate::campaign::armies::pointer::track_pointer_over_ui)
                .run_if(resource_exists::<ProvinceMap>),
        );
}

#[derive(Resource, Default)]
struct PressState {
    position: Vec2,
    button: Option<MouseButton>,
    /// The press began over a window or panel: its release is not a map click.
    over_ui: bool,
}

/// The pointer is over a window, panel or button drawn above the map.
fn over_ui(pointer: Option<&PointerOverUi>) -> bool {
    pointer.is_some_and(|p| p.over)
}

/// Cursor ray of the first active 3D camera.
fn cursor_ray(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<(Vec2, Ray3d)> {
    let cursor = windows.single().ok()?.cursor_position()?;
    let (camera, transform) = cameras.iter().find(|(camera, _)| camera.is_active)?;
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    Some((cursor, ray))
}

fn pick_at(map: &ProvinceMap, surface: Option<&SurfaceHeight>, ray: Ray3d) -> u16 {
    let flat = |_: f32, _: f32| 0.0;
    match surface {
        Some(SurfaceHeight(f)) => pick_province(map, ray.origin, *ray.direction, f.as_ref()),
        None => pick_province(map, ray.origin, *ray.direction, &flat),
    }
}

fn update_hover(
    map: Res<ProvinceMap>,
    surface: Option<Res<SurfaceHeight>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    pointer: Option<Res<PointerOverUi>>,
    mut hovered: ResMut<HoveredProvince>,
) {
    // Under a window the map is hidden: nothing is hovered (no highlight, no click through).
    let index = if over_ui(pointer.as_deref()) {
        0
    } else {
        cursor_ray(&windows, &cameras).map_or(0, |(_, ray)| pick_at(&map, surface.as_deref(), ray))
    };
    if index != hovered.index {
        hovered.index = index;
        hovered.id = map.province(index).map(|p| p.id.clone());
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_clicks(
    map: Res<ProvinceMap>,
    surface: Option<Res<SurfaceHeight>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    pointer: Option<Res<PointerOverUi>>,
    mut press: ResMut<PressState>,
    mut guard: ResMut<ProvinceClickGuard>,
    mut selected: ResMut<SelectedProvince>,
    mut clicks: MessageWriter<ProvinceClicked>,
) {
    let blocked = std::mem::take(&mut guard.blocked);
    let over_ui = over_ui(pointer.as_deref());
    let Some((cursor, ray)) = cursor_ray(&windows, &cameras) else {
        return;
    };
    for button in [MouseButton::Left, MouseButton::Right] {
        if buttons.just_pressed(button) {
            press.position = cursor;
            press.button = Some(button);
            press.over_ui = over_ui;
        } else if buttons.just_released(button)
            && press.button == Some(button)
            && cursor.distance(press.position) <= CLICK_MAX_DRAG_PX
        {
            press.button = None;
            // A click on a window (pressed or released over it) never reaches the map behind.
            if std::mem::take(&mut press.over_ui) || over_ui {
                continue;
            }
            if button == MouseButton::Left && std::mem::take(&mut guard.army_press) {
                continue;
            }
            if blocked && button == MouseButton::Left {
                continue;
            }
            let index = pick_at(&map, surface.as_deref(), ray);
            let id = map.province(index).map(|p| p.id.clone());
            if button == MouseButton::Left {
                selected.index = index;
                selected.id = id.clone();
            }
            clicks.write(ProvinceClicked { index, id, button });
        }
    }
}

fn apply_select_requests(
    map: Res<ProvinceMap>,
    mut requests: MessageReader<SelectProvince>,
    mut selected: ResMut<SelectedProvince>,
) {
    for SelectProvince(id) in requests.read() {
        let index = id.as_deref().map_or(0, |id| map.index_of_id(id));
        selected.index = index;
        selected.id = map.province(index).map(|p| p.id.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::provinces::data::tests::real_map;

    const FLAT: &dyn Fn(f32, f32) -> f32 = &|_, _| 0.0;

    #[test]
    fn ray_hits_the_plane_straight_below() {
        let hit = pick_ray(
            Vec3::new(100.0, 50.0, 200.0),
            Vec3::NEG_Y,
            Vec2::new(500.0, 500.0),
            FLAT,
        )
        .unwrap();
        assert!((hit - Vec3::new(100.0, 0.0, 200.0)).length() < 1e-3);
    }

    #[test]
    fn rising_or_off_map_rays_miss() {
        let size = Vec2::new(500.0, 500.0);
        assert!(pick_ray(Vec3::new(1.0, 5.0, 1.0), Vec3::Y, size, FLAT).is_none());
        assert!(pick_ray(Vec3::new(600.0, 50.0, 1.0), Vec3::NEG_Y, size, FLAT).is_none());
    }

    #[test]
    fn refines_against_terrain_height() {
        // A plateau of height 10 over the whole map: an oblique ray from (0, 110, 0) going
        // down-and-forward at 45 degrees lands at z = 100 on the plateau, not z = 110.
        let dir = Vec3::new(0.0, -1.0, 1.0).normalize();
        let hit = pick_ray(
            Vec3::new(50.0, 110.0, 0.0),
            dir,
            Vec2::new(500.0, 500.0),
            &|_, _| 10.0,
        )
        .unwrap();
        assert!((hit.z - 100.0).abs() < 0.5, "{hit}");
        assert!((hit.y - 10.0).abs() < 1e-3);
    }

    #[test]
    fn picks_paris_province_from_above() {
        let map = real_map();
        let paris = map.index_of_id("prov_ile_de_france");
        let px = map.projection.lonlat_to_px(2.35, 48.85).unwrap();
        let index = pick_province(map, Vec3::new(px.x, 300.0, px.y), Vec3::NEG_Y, FLAT);
        assert_eq!(index, paris);
        // an oblique ray that lands on the same point
        let dir = Vec3::new(0.3, -1.0, 0.2).normalize();
        let origin = Vec3::new(px.x, 0.0, px.y) - dir * (300.0 / -dir.y);
        assert_eq!(pick_province(map, origin, dir, FLAT), paris);
    }

    #[test]
    fn picks_another_known_province() {
        let map = real_map();
        // London: England owns its province; the raster index differs from Paris'.
        let px = map.projection.lonlat_to_px(-0.12, 51.5).unwrap();
        let index = pick_province(map, Vec3::new(px.x, 100.0, px.y), Vec3::NEG_Y, FLAT);
        assert_ne!(index, 0);
        assert_ne!(index, map.index_of_id("prov_ile_de_france"));
        assert_eq!(map.province(index).unwrap().owner, "fac_england");
    }
}
