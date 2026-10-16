//! Bombard muzzle flash (port of `cannon_fire` and the `flash` burst pool of battle_effects.gd).
//!
//! A shot of a field bombard (`Volleys::muzzle_flashes`) or of a siege bombard
//! ([`MuzzleFlashes`], filled when its ball leaves) throws a few additive flame sprites out of the
//! muzzle and lights a short orange point light (9 at the muzzle, falling to 0 in `flash_s`). The
//! duration comes from `bombard.flash_s` of `data/fx/siege_engines.json` (measured on a video).

use bevy::prelude::*;

use super::glow::lumens;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::lighting::LightingLook;
use crate::battle::units::volleys::Volleys;
use crate::fx::flipbook::{FlipbookKind, FlipbookLooks, SmokeSprite, spawn_fire_sprite};

/// Light energy at the muzzle and range (`_flash`).
const FLASH_ENERGY: f32 = 9.0;
const FLASH_RANGE_M: f32 = 18.0;
/// Flame sprites thrown per shot, and the room kept free in the sprite budget.
const SPRITES_PER_FLASH: u64 = 4;
const SPRITE_ROOM: usize = 700;

/// Muzzles (position, direction) of siege bombards that fired, drained by this module.
#[derive(Resource, Default)]
pub struct MuzzleFlashes(pub Vec<(Vec3, Vec3)>);

#[derive(Resource)]
pub struct FlashState {
    flash_s: f32,
    energy: f32,
    pub flashes: usize,
    pub peak_energy: f32,
}

impl Default for FlashState {
    fn default() -> Self {
        Self {
            flash_s: 0.15,
            energy: 0.0,
            flashes: 0,
            peak_energy: 0.0,
        }
    }
}

/// `bombard.flash_s` of `data/fx/siege_engines.json`.
pub fn load_flash_seconds() -> f32 {
    crate::paths::find_root()
        .ok()
        .and_then(|root| {
            std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/siege_engines.json"))
                .ok()
        })
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| v["bombard"]["flash_s"].as_f64())
        .map_or(0.15, |s| s as f32)
}

#[derive(Component)]
struct FlashLight;

fn spawn_light(mut commands: Commands, mut state: ResMut<FlashState>) {
    *state = FlashState {
        flash_s: load_flash_seconds(),
        ..default()
    };
    commands.spawn((
        Name::new("BombardFlash"),
        PointLight {
            color: Color::srgb(1.0, 0.75, 0.4),
            intensity: 0.0,
            range: FLASH_RANGE_M,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        Visibility::Hidden,
        FlashLight,
        DespawnOnExit(AppState::Battle),
    ));
}

#[allow(clippy::too_many_arguments)]
fn update_flash(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    time: Res<Time>,
    look: Option<Res<LightingLook>>,
    mut volleys: ResMut<Volleys>,
    mut siege: ResMut<MuzzleFlashes>,
    mut state: ResMut<FlashState>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut light: Query<(&mut PointLight, &mut Transform, &mut Visibility), With<FlashLight>>,
    mut demo_frames: Local<u32>,
) {
    // Dev hook (proofs): `CENT_ANS_FLASH_DEMO=1` fires a flash from the first regiment every
    // 100 frames, for battles without a bombard to dump.
    if std::env::var_os("CENT_ANS_FLASH_DEMO").is_some() {
        *demo_frames += 1;
        if (*demo_frames).is_multiple_of(100) {
            let sim = session.sim_mut();
            if let Some(unit) = sim.units().iter().find(|u| u.present()) {
                let y = sim.field().height(unit.x, unit.z) as f32 + 0.85;
                siege
                    .0
                    .push((Vec3::new(unit.x as f32, y, unit.z as f32), Vec3::Z));
            }
        }
    }
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs().min(0.1) * session.speed as f32
    };
    let muzzles: Vec<(Vec3, Vec3)> = volleys
        .muzzle_flashes
        .drain(..)
        .chain(siege.0.drain(..))
        .collect();
    let Ok((mut point, mut transform, mut visibility)) = light.single_mut() else {
        return;
    };
    let mut looks = looks;
    for (muzzle, dir) in muzzles {
        state.flashes += 1;
        state.energy = FLASH_ENERGY;
        state.peak_energy = state.peak_energy.max(state.energy);
        transform.translation = muzzle + dir * 1.0 + Vec3::Y * 0.5;
        let Some(looks) = looks.as_deref_mut() else {
            continue;
        };
        for k in 0..SPRITES_PER_FLASH {
            if looks.live() >= SPRITE_ROOM {
                break;
            }
            let h = super::hash01(state.flashes as u64 * 8 + k);
            // A cone of 20 degrees around the shot at 4-12 m/s (the flash particle process).
            let sideways = Vec3::new(dir.z, 0.0, -dir.x) * (h - 0.5) * 0.7;
            let sprite = SmokeSprite {
                position: muzzle + dir * 0.4,
                velocity: (dir + sideways + Vec3::Y * 0.1) * (4.0 + 8.0 * h),
                accel: Vec3::ZERO,
                damping: 1.0,
                life: state.flash_s + 0.1,
                size: 0.8 + 1.4 * h,
                spin: h * 6.0,
                spin_rate: 0.0,
                tint: None,
            };
            if let Some(entity) =
                spawn_fire_sprite(&mut commands, looks, FlipbookKind::Flame, &sprite, h)
            {
                commands
                    .entity(entity)
                    .insert(DespawnOnExit(AppState::Battle));
            }
        }
    }
    if state.energy > 0.0 {
        state.energy = (state.energy - dt * FLASH_ENERGY / state.flash_s.max(0.01)).max(0.0);
    }
    let unit = look.map_or(2650.0, |l| l.unit());
    point.intensity = lumens(state.energy, unit);
    *visibility = if state.energy > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

fn dump_flash(world: &World) -> (String, serde_json::Value) {
    let Some(s) = world.get_resource::<FlashState>() else {
        return ("battle_bombard_flash".into(), serde_json::Value::Null);
    };
    (
        "battle_bombard_flash".into(),
        serde_json::json!({
            "flashes": s.flashes, "flash_s": s.flash_s, "peak_energy": s.peak_energy,
        }),
    )
}

pub struct FlashPlugin;

impl Plugin for FlashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MuzzleFlashes>()
            .init_resource::<FlashState>()
            .register_dump(dump_flash)
            .add_systems(OnEnter(AppState::Battle), spawn_light)
            .add_systems(
                Update,
                update_flash
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flash_lasts_what_the_data_measured() {
        let seconds = load_flash_seconds();
        assert!((seconds - 0.15).abs() < 1e-6, "{seconds}");
    }
}
