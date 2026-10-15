//! River boats and sea ships (`life_ambient.gd`, `dn_water_models.gd`): glb models going back
//! and forth along the big rivers and the routed sea lanes (`sea_lanes_px.json`).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::Value;

use super::data::{LifeData, text_seed};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_instance};
use super::stream::{Cat, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::terrain::surface::GroundHeight;

#[derive(Component)]
pub struct Boat {
    pub route: usize,
}

/// Wake of a ship (`SeaLife.make_wake`): a triangular ribbon behind the stern, in the units of the
/// marker (a ship is `SHIP_UNITS` long), scaled to the ship.
#[derive(Component)]
pub struct Wake;

/// Length of the marker ship the wake data is expressed for (stern at `stern_x` = -6.5).
pub const SHIP_UNITS: f32 = 13.0;

/// Ribbon of the wake (`+X` forward), alpha fading from the stern, from `fx/sea_life.json`.
pub fn wake_mesh(wake: &Value) -> Mesh {
    let f = |k: &str, d: f32| wake[k].as_f64().map_or(d, |x| x as f32);
    let length = f("length", 34.0);
    let (w0, w1) = (f("width_start", 4.0), f("width_end", 15.0));
    let stern = f("stern_x", -6.5);
    let steps = 8;
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let half = (w0 + (w1 - w0) * t) * 0.5;
        let x = stern - t * length;
        let alpha = (1.0 - t).powf(1.4);
        positions.push([x, 0.0, -half]);
        positions.push([x, 0.0, half]);
        colors.push([1.0, 1.0, 1.0, alpha]);
        colors.push([1.0, 1.0, 1.0, alpha]);
        if i < steps {
            let a = (i * 2) as u32;
            indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Wake material: unlit, blended, the colour of the data with the alpha of a moving ship.
pub fn wake_material(wake: &Value) -> StandardMaterial {
    let c = |i: usize, d: f32| wake["color"][i].as_f64().map_or(d, |x| x as f32);
    let alpha = wake["alpha_moving"].as_f64().map_or(0.6, |x| x as f32);
    StandardMaterial {
        base_color: Color::srgba(c(0, 0.8), c(1, 0.86), c(2, 0.86), alpha),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

pub fn spawn_boats(
    mut commands: Commands,
    assets: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut index: ResMut<LifeIndex>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wake_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    if !view.ready || view.zoom_strategic >= 1.0 {
        return;
    }
    // Same zone as the fades (`edge_target`), so nothing is placed just to be retired.
    let radius = view.radius * 0.98;
    for (i, route) in data.river_routes.iter().enumerate() {
        let key = text_seed("life:boat") ^ (i as u64).wrapping_mul(0x9E37_79B9);
        if index.0.contains(&key) {
            continue;
        }
        let (p, heading) = route.pose(view.time);
        if p.distance(view.focus) > radius {
            continue;
        }
        let Some(model) = data.water_models.models.get(&route.model) else {
            continue;
        };
        let Some(path) = data.water_models.scene_path(&route.model) else {
            continue;
        };
        let scale = if route.river {
            data.water_models.river_boat_scale
        } else {
            data.water_models.lane_ship_scale
        };
        let cat = if route.river {
            Cat::RiverBoat
        } else {
            Cat::SeaShip
        };
        index.0.insert(key);
        let length_world = model.length * scale;
        let root = spawn_instance(
            &mut commands,
            &assets,
            &metrics,
            &path,
            FitAxis::Length,
            model.length * scale,
            model.yaw_deg.to_radians(),
            Vec3::new(p.x, 0.0, p.y),
            Quat::from_rotation_y(-heading.y.atan2(heading.x)),
            (
                Boat { route: i },
                LifeItem {
                    key,
                    cat,
                    anchor: p,
                    h: 0.0,
                },
                DespawnOnExit(AppState::Campaign),
            ),
        );
        // Wake of the ships at sea (the river boats have none).
        let wake = &data.sea_life["wake"];
        if !route.river && wake["enabled"].as_bool().unwrap_or(false) {
            let (mesh, material) = wake_assets
                .get_or_insert_with(|| {
                    (
                        meshes.add(wake_mesh(wake)),
                        materials.add(wake_material(wake)),
                    )
                })
                .clone();
            let k = length_world / SHIP_UNITS;
            let y = wake["y"].as_f64().map_or(0.12, |v| v as f32) * k;
            commands.entity(root).with_child((
                Name::new("life:wake"),
                Wake,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, y, 0.0).with_scale(Vec3::splat(k)),
            ));
        }
    }
}

pub fn move_boats(
    data: Res<LifeData>,
    view: Res<LifeView>,
    ground: GroundHeight,
    mut boats: Query<(&Boat, &mut LifeItem, &mut Transform), With<Fade>>,
) {
    for (boat, mut item, mut t) in &mut boats {
        let Some(route) = data.river_routes.get(boat.route) else {
            continue;
        };
        let (p, heading) = route.pose(view.time);
        item.anchor = p;
        let y = ground.at(p.x, p.y).unwrap_or(0.0).max(0.0);
        t.translation = Vec3::new(p.x, y + 0.05, p.y);
        t.rotation = Quat::from_rotation_y(-heading.y.atan2(heading.x));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wake_spec() -> Value {
        let root = crate::paths::find_root().expect("root");
        let text = std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/sea_life.json"))
            .unwrap();
        serde_json::from_str::<Value>(&text).unwrap()["wake"].clone()
    }

    #[test]
    fn wake_ribbon_widens_and_fades_behind_the_stern() {
        let spec = wake_spec();
        assert_eq!(spec["enabled"], Value::Bool(true));
        let mesh = wake_mesh(&spec);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(c)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colours")
        };
        assert_eq!(p.len(), 18);
        assert!((p[0][0] - -6.5).abs() < 1e-4, "starts at the stern");
        assert!(
            (p[16][0] - (-6.5 - 34.0)).abs() < 1e-3,
            "ends one length behind"
        );
        assert!(p[17][2] - p[16][2] > p[1][2] - p[0][2], "wider at the end");
        assert_eq!(c[0][3], 1.0);
        assert!(c[17][3] < 0.01);
    }

    #[test]
    fn wake_material_uses_the_data_colour() {
        let m = wake_material(&wake_spec());
        assert!(m.unlit && m.alpha_mode == AlphaMode::Blend);
    }
}
