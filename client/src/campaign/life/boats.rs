//! River boats and sea ships (`life_ambient.gd`, `dn_water_models.gd`): glb models going back
//! and forth along the big rivers and the routed sea lanes (`sea_lanes_px.json`).

use bevy::prelude::*;

use super::data::{LifeData, text_seed};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_instance};
use super::stream::{Cat, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::terrain::surface::GroundHeight;

#[derive(Component)]
pub struct Boat {
    pub route: usize,
}

pub fn spawn_boats(
    mut commands: Commands,
    assets: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut index: ResMut<LifeIndex>,
) {
    if !view.ready || view.distance > 1400.0 {
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
        spawn_instance(
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
