//! Lights of the siege fires (port of `_assign_lights` and `_process` of siege_fire_fx.gd).
//!
//! The `max_lights` (8) strongest burning houses and gate carry a flickering orange point light
//! 7 m above the hearth: three incommensurable sines of the `flicker_speed` phase shift the energy
//! and yellow the colour at the peaks, and the light dances by a few decimetres. Everything comes
//! from `data/fx/siege_fire.json` (`light`, `max_lights`, `flame_height_m`). The hearths are the
//! flame emitters of `siege::fx` (their ratio follows the intensity).

use bevy::prelude::*;

use super::glow::lumens;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::lighting::LightingLook;
use crate::battle::siege::fx::FireEmitter;
use crate::fx::flipbook::{FlipbookEmitter, FlipbookKind};

/// `light` and `max_lights` of `data/fx/siege_fire.json`.
#[derive(Resource, Clone, Debug)]
pub struct SiegeLightParams {
    pub max_lights: usize,
    pub color: [f32; 3],
    pub energy: f32,
    pub range_m: f32,
    pub height_m: f32,
    pub flicker_speed: f32,
    pub flicker_amount: f32,
    pub flame_height_m: f32,
}

impl Default for SiegeLightParams {
    fn default() -> Self {
        Self {
            max_lights: 8,
            color: [1.0, 0.52, 0.18],
            energy: 5.0,
            range_m: 30.0,
            height_m: 7.0,
            flicker_speed: 9.0,
            flicker_amount: 0.35,
            flame_height_m: 7.5,
        }
    }
}

impl SiegeLightParams {
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut p = Self::default();
        let num = |v: &serde_json::Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        let light = &value["light"];
        p.max_lights = value["max_lights"]
            .as_u64()
            .map_or(p.max_lights, |n| n as usize);
        p.energy = num(&light["energy"], p.energy);
        p.range_m = num(&light["range_m"], p.range_m);
        p.height_m = num(&light["height_m"], p.height_m);
        p.flicker_speed = num(&light["flicker_speed"], p.flicker_speed);
        p.flicker_amount = num(&light["flicker_amount"], p.flicker_amount);
        p.flame_height_m = num(&value["flame_height_m"], p.flame_height_m);
        if let Some(c) = light["color"].as_array() {
            for (slot, v) in p.color.iter_mut().zip(c) {
                *slot = num(v, *slot);
            }
        }
        p
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/siege_fire.json"))
                    .ok()
            })
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .map_or_else(Self::default, |v| Self::from_json(&v))
    }
}

/// Flicker factor of light `slot` at `seconds`: `1 + amount * wave` and the wave itself
/// (`wave` is in `[-1, 1]`; the colour yellows with its positive part).
pub fn flicker(params: &SiegeLightParams, slot: usize, seconds: f32) -> (f32, f32) {
    let phase = slot as f32 * 1.37;
    let t = seconds * params.flicker_speed;
    let wave = 0.5 * (t + phase).sin()
        + 0.3 * (t * 2.31 + phase * 2.0).sin()
        + 0.2 * (t * 5.17 + phase * 3.3).sin();
    (1.0 + params.flicker_amount * wave, wave)
}

#[derive(Component)]
struct SiegeLight {
    slot: usize,
}

#[derive(Resource, Default)]
pub struct SiegeLightStats {
    pub lit: usize,
    pub strongest_lumens: f32,
    pub highest_m: f32,
}

fn spawn_lights(mut commands: Commands, params: Res<SiegeLightParams>) {
    for slot in 0..params.max_lights {
        commands.spawn((
            Name::new("SiegeFireLight"),
            PointLight {
                color: Color::srgb(params.color[0], params.color[1], params.color[2]),
                intensity: 0.0,
                range: params.range_m,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
            SiegeLight { slot },
            DespawnOnExit(AppState::Battle),
        ));
    }
}

fn update_lights(
    params: Res<SiegeLightParams>,
    time: Res<Time>,
    look: Option<Res<LightingLook>>,
    emitters: Query<(&FireEmitter, &FlipbookEmitter, &Transform)>,
    mut lights: Query<
        (
            &SiegeLight,
            &mut PointLight,
            &mut Transform,
            &mut Visibility,
        ),
        Without<FireEmitter>,
    >,
    mut stats: ResMut<SiegeLightStats>,
) {
    // One hearth per flame emitter, strongest (highest ratio) first: intensity from the ratio.
    let mut hearths: Vec<(f32, Vec3, usize)> = emitters
        .iter()
        .filter(|(fire, _, _)| fire.kind == FlipbookKind::Flame)
        .map(|(fire, emitter, transform)| {
            let intensity = ((emitter.ratio - 0.25) / 0.75).clamp(0.0, 1.0);
            let ground_y = transform.translation.y - params.flame_height_m * 0.45;
            (
                intensity,
                Vec3::new(
                    transform.translation.x,
                    ground_y + params.height_m,
                    transform.translation.z,
                ),
                fire.key,
            )
        })
        .collect();
    hearths.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.2.cmp(&b.2)));
    let unit = look.map_or(2650.0, |l| l.unit());
    let seconds = time.elapsed_secs();
    stats.lit = 0;
    stats.strongest_lumens = 0.0;
    stats.highest_m = 0.0;
    for (light, mut point, mut transform, mut visibility) in &mut lights {
        let Some(&(intensity, base, _)) = hearths.get(light.slot) else {
            point.intensity = 0.0;
            *visibility = Visibility::Hidden;
            continue;
        };
        let (factor, wave) = flicker(&params, light.slot, seconds);
        let phase = light.slot as f32 * 1.37;
        let t = seconds * params.flicker_speed;
        point.intensity = lumens(params.energy * intensity * factor, unit);
        let base_color = Vec3::from(params.color);
        let warm = base_color.lerp(Vec3::new(1.0, 0.78, 0.42), wave.clamp(0.0, 1.0) * 0.35);
        point.color = Color::srgb(warm.x, warm.y, warm.z);
        transform.translation = base
            + Vec3::new(
                (t * 0.73 + phase).sin(),
                (t * 1.1 + phase).sin() * 0.5,
                (t * 0.91 + phase).cos(),
            ) * 0.35;
        *visibility = Visibility::Inherited;
        stats.lit += 1;
        stats.strongest_lumens = stats.strongest_lumens.max(point.intensity);
        stats.highest_m = stats.highest_m.max(transform.translation.y);
    }
}

fn dump_lights(world: &World) -> (String, serde_json::Value) {
    let (Some(s), Some(p)) = (
        world.get_resource::<SiegeLightStats>(),
        world.get_resource::<SiegeLightParams>(),
    ) else {
        return ("siege_fire_lights".into(), serde_json::Value::Null);
    };
    (
        "siege_fire_lights".into(),
        serde_json::json!({
            "lit": s.lit, "max_lights": p.max_lights, "strongest_lumens": s.strongest_lumens,
            "highest_m": s.highest_m, "range_m": p.range_m, "energy": p.energy,
        }),
    )
}

pub struct SiegeLightsPlugin;

impl Plugin for SiegeLightsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SiegeLightParams::load())
            .init_resource::<SiegeLightStats>()
            .register_dump(dump_lights)
            .add_systems(OnEnter(AppState::Battle), spawn_lights)
            .add_systems(
                Update,
                update_lights
                    .run_if(in_state(AppState::Battle))
                    .after(crate::battle::scene::StepSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> SiegeLightParams {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/fx/siege_fire.json"
        ))
        .unwrap();
        SiegeLightParams::from_json(&serde_json::from_str(&text).unwrap())
    }

    #[test]
    fn data_gives_eight_lights_seven_metres_up() {
        let p = params();
        assert_eq!(p.max_lights, 8);
        assert_eq!(p.height_m, 7.0);
        assert_eq!(p.energy, 5.0);
        assert!((p.color[1] - 0.52).abs() < 1e-6);
    }

    #[test]
    fn the_flicker_stays_within_its_amount_and_differs_per_light() {
        let p = params();
        for i in 0..200 {
            let (f, wave) = flicker(&p, 0, i as f32 * 0.07);
            assert!(wave.abs() <= 1.0 + 1e-5);
            assert!((f - 1.0).abs() <= p.flicker_amount + 1e-5);
        }
        assert_ne!(flicker(&p, 0, 1.0).0, flicker(&p, 3, 1.0).0);
    }
}
