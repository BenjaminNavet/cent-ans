//! Smoke plumes of the decor types flagged `smoke` (forges, charcoal burners, glassworks; Godot
//! `DecorLayer._write_smoke`: one `life_smoke` quad per instance, `size_m[0] x 0.4` wide,
//! `size_m[1] x 1.8` high, lifted to the top of the volume). Here a light column of the player's
//! smoke flipbook (`crate::fx::flipbook`, the sheet the camps and fires use) rises from the top
//! of the model: same footprint and height, the puffs drift and fade instead of a fixed card.
//!
//! The emitter is a child of the decor instance, whose scale turns metres into world units; the
//! emitter works in world units, so the sizes are rescaled from the global scale every frame (as
//! `armies::camp_smoke`). Quiet when the instance is hidden or the camera is far.

use bevy::prelude::*;

use crate::fx::flipbook::{EmitterSpec, FlipbookData, FlipbookEmitter};

/// Camera distance (world units) beyond which the plumes stop emitting.
pub const SMOKE_DISTANCE: f32 = 160.0;
/// Light grey-brown wood smoke, a little lighter than the siege camps.
const COLOR: [f32; 4] = [0.66, 0.64, 0.60, 0.45];

/// Base (metre) settings of a plume.
#[derive(Component, Clone, Debug)]
pub struct DecorSmoke {
    base: EmitterSpec,
    /// Height of the volume (m): the drift scales with it.
    height: f32,
}

/// Emitter settings (metres) for a type of `size_m` (footprint, height): the column reaches
/// about 1.8 x the height over its life, `0.4 x` the footprint wide at the top.
pub fn plume_spec(size_m: [f32; 2]) -> EmitterSpec {
    let (w, h) = (size_m[0].max(0.5), size_m[1].max(0.5));
    let life = 6.0;
    EmitterSpec {
        amount: 6.0,
        life,
        size: [0.2 * w.max(h), 0.4 * w.max(h)],
        velocity: [1.8 * h / life * 0.8, 1.8 * h / life * 1.2],
        spread_m: 0.08 * w,
        loops: 0.0,
    }
}

/// Plume child bundle for a decor instance of `size_m`, at the top of the volume.
pub fn smoke_bundle(size_m: [f32; 2]) -> impl Bundle {
    let base = plume_spec(size_m);
    let mut emitter = FlipbookEmitter::smoke(&FlipbookData::default(), size_m[0].max(0.5));
    emitter.spec = base.clone();
    emitter.accel = Vec3::new(0.08, 0.05, 0.02) * size_m[1].max(0.5);
    emitter.damping = [0.1, 0.2];
    emitter.tint = Some(COLOR);
    emitter.active = false;
    (
        Name::new("DecorSmoke"),
        DecorSmoke {
            base,
            height: size_m[1].max(0.5),
        },
        emitter,
        Transform::from_xyz(0.0, size_m[1], 0.0),
        Visibility::Inherited,
    )
}

/// Rescales the plumes to their instance and gates them on visibility and camera distance.
pub fn update_decor_smoke(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut smokes: Query<(
        &DecorSmoke,
        &mut FlipbookEmitter,
        &GlobalTransform,
        &InheritedVisibility,
    )>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    for (smoke, mut emitter, at, visible) in &mut smokes {
        let scale = at.scale().x.max(1e-6);
        let on = visible.get()
            && camera.translation().distance(at.translation()) < SMOKE_DISTANCE
            && scale > 1e-5;
        if emitter.active != on {
            emitter.active = on;
        }
        if !on {
            continue;
        }
        let spec = &smoke.base;
        emitter.spec.size = spec.size.map(|v| v * scale);
        emitter.spec.velocity = spec.velocity.map(|v| v * scale);
        emitter.spec.spread_m = spec.spread_m * scale;
        emitter.accel = Vec3::new(0.08, 0.05, 0.02) * smoke.height * scale;
    }
}

/// Plumes in the scene and the emitting ones.
pub fn counts(world: &World) -> (usize, usize) {
    let (mut total, mut active) = (0, 0);
    if let Some(mut query) = world.try_query::<(&DecorSmoke, &FlipbookEmitter)>() {
        for (_, emitter) in query.iter(world) {
            total += 1;
            active += usize::from(emitter.active);
        }
    }
    (total, active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plume_rises_about_twice_the_volume() {
        let spec = plume_spec([6.0, 5.0]);
        let mean = (spec.velocity[0] + spec.velocity[1]) * 0.5;
        assert!((mean * spec.life - 9.0).abs() < 0.5);
        assert!(spec.size[0] > 0.0 && spec.size[1] > spec.size[0]);
    }
}
