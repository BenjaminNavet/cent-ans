//! Smoke of the camp fires of the armies (`ArmyFigures._make_smoke`, `army_figures.gd`): a thin
//! column over the hearth of a siege camp or of a bivouac, in the player's smoke flipbook
//! (`crate::fx::flipbook`). The emitter is a child of the army marker, so it follows the army and
//! its size, speeds and spread follow the marker's scale (Godot particles use local coordinates);
//! it is switched off beyond the camp's view distance (`SMOKE_DISTANCE` / `BIVOUAC_DISTANCE`).

use bevy::prelude::*;

use super::figures::BIVOUAC_DISTANCE;
use crate::fx::flipbook::{EmitterSpec, FlipbookData, FlipbookEmitter};

/// Camera distance beyond which a siege camp's smoke stops (`SMOKE_DISTANCE`).
pub const SMOKE_DISTANCE: f32 = 420.0;
/// Hearth of the siege camp model, in model units (scaled by the camp scale).
const SIEGE_HEARTH: Vec3 = Vec3::new(-0.1, 0.0, -0.05);
/// Smoke colour of `_make_smoke` with the ramp's start opacity.
const COLOR: [f32; 4] = [0.62, 0.60, 0.57, 0.55];

/// Marks the camp smoke emitter and keeps its unscaled settings.
#[derive(Component, Clone, Debug)]
pub struct CampSmoke {
    base: EmitterSpec,
    /// Distance (to the camera) beyond which it stays quiet.
    range: f32,
}

/// The emitter of a camp (`siege_camp` or `bivouac` model) placed at `offset` with `scale`.
pub fn camp_smoke_bundle(path: &str, offset: Vec3, scale: f32) -> Option<impl Bundle> {
    let siege = path.contains("siege_camp");
    if !siege && !path.contains("bivouac") {
        return None;
    }
    let hearth = if siege {
        SIEGE_HEARTH * scale
    } else {
        Vec3::ZERO
    };
    let base = EmitterSpec {
        amount: 10.0,
        life: 7.0,
        size: [2.0, 3.2],
        velocity: [1.2, 1.8],
        spread_m: 0.4,
        loops: 0.0,
    };
    let mut emitter = FlipbookEmitter::smoke(&FlipbookData::default(), 100.0);
    emitter.spec = base.clone();
    emitter.accel = Vec3::new(0.35, 0.25, 0.1);
    emitter.damping = [0.1, 0.2];
    emitter.tint = Some(COLOR);
    Some((
        Name::new("CampSmoke"),
        CampSmoke {
            base,
            range: if siege {
                SMOKE_DISTANCE
            } else {
                BIVOUAC_DISTANCE
            },
        },
        emitter,
        Transform::from_translation(offset + hearth + Vec3::Y),
        Visibility::Inherited,
    ))
}

/// Follows the marker's scale and the camera distance.
pub fn update_camp_smoke(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut smokes: Query<(&CampSmoke, &mut FlipbookEmitter, &GlobalTransform)>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    for (smoke, mut emitter, at) in &mut smokes {
        let scale = at.scale().x.max(1e-3);
        let near = camera.translation().distance(at.translation()) < smoke.range;
        if emitter.active != near {
            emitter.active = near;
        }
        emitter.spec.size = smoke.base.size.map(|v| v * scale);
        emitter.spec.velocity = smoke.base.velocity.map(|v| v * scale);
        emitter.spec.spread_m = smoke.base.spread_m * scale;
        emitter.accel = Vec3::new(0.35, 0.25, 0.1) * scale;
    }
}

/// Dump section: camp emitters and how many are active.
pub fn dump_camp_smoke(world: &World) -> (String, serde_json::Value) {
    let (mut total, mut active) = (0, 0);
    if let Some(mut query) = world.try_query::<(&CampSmoke, &FlipbookEmitter)>() {
        for (_, emitter) in query.iter(world) {
            total += 1;
            active += usize::from(emitter.active);
        }
    }
    (
        "army_camp_smoke".into(),
        serde_json::json!({"emitters": total, "active": active}),
    )
}
