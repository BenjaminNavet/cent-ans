//! Night glow of the camp fires (port of `_update_glows` and `_process` of battle_smoke.gd):
//! the strongest glowing smoke sources carry a flickering orange point light when the light of
//! the hour falls below 0.75. The light level is `energy_mul` of the day keyframes of
//! `data/fx/battle_staging.json` (`time_of_day`), interpolated with a smoothstep between two
//! keyframes as `battle_time_of_day.gd` does, at the simulated hour.

use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::lighting::LightingLook;
use crate::battle::units::smoke::SmokeSources;

/// Lights at most (`MAX_GLOWS`).
const MAX_GLOWS: usize = 4;
/// Range (m) and height above the fire (m) of a glow, base energy at full darkness.
const RANGE_M: f32 = 16.0;
const HEIGHT_M: f32 = 1.2;
const BASE_ENERGY: f32 = 2.5;
/// Candela multiplier turning a Godot omni energy into lumens: Godot's attenuation keeps a light
/// strong over its range while Bevy's falls with the square of the distance, so a unit of
/// energy is given the punch of ~8 m. Uncalibrated by eye (no captures in this lot).
const REACH_FACTOR: f32 = 8.0;

/// Day keyframes: `(hour, energy_mul)` sorted by hour.
#[derive(Resource, Clone, Debug, Default)]
pub struct Daylight {
    keyframes: Vec<(f32, f32)>,
}

impl Daylight {
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut keyframes: Vec<(f32, f32)> = value["time_of_day"]["keyframes"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|k| {
                        Some((k["hour"].as_f64()? as f32, k["energy_mul"].as_f64()? as f32))
                    })
                    .collect()
            })
            .unwrap_or_default();
        keyframes.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { keyframes }
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read_to_string(
                    crate::paths::data_dir(&root).join("fx/battle_staging.json"),
                )
                .ok()
            })
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .map_or_else(Self::default, |v| Self::from_json(&v))
    }

    /// Light level (`light_level`) at `hour` (0-24); 1 without keyframes.
    pub fn level(&self, hour: f32) -> f32 {
        let count = self.keyframes.len();
        if count == 0 {
            return 1.0;
        }
        let hour = hour.rem_euclid(24.0);
        let mut previous = self.keyframes[count - 1];
        let mut next = self.keyframes[0];
        for (i, key) in self.keyframes.iter().enumerate() {
            if key.0 > hour {
                next = *key;
                previous = self.keyframes[(i + count - 1) % count];
                break;
            }
            previous = *key;
            next = self.keyframes[(i + 1) % count];
        }
        let span = (next.0 - previous.0).rem_euclid(24.0);
        let t = if span <= 0.0 {
            0.0
        } else {
            ((hour - previous.0).rem_euclid(24.0) / span).clamp(0.0, 1.0)
        };
        let t = t * t * (3.0 - 2.0 * t);
        previous.1 + (next.1 - previous.1) * t
    }
}

/// How dark the camps are: 0 above a light level of 0.75, 1 below 0.25.
pub fn darkness(light_level: f32) -> f32 {
    ((0.75 - light_level) / 0.5).clamp(0.0, 1.0)
}

/// Lumens of a point light of Godot energy `energy` (see `REACH_FACTOR`).
pub fn lumens(energy: f32, unit: f32) -> f32 {
    energy * unit * std::f32::consts::TAU * 2.0 * REACH_FACTOR
}

#[derive(Component)]
struct CampGlow {
    slot: usize,
}

#[derive(Resource, Default)]
pub struct GlowStats {
    pub light_level: f32,
    pub darkness: f32,
    pub lit: usize,
    pub strongest_lumens: f32,
}

/// The lights are made the first time a camp fire burns in the dark: a battle by day (or
/// without camp fires) has no point light at all.
fn spawn_glows(commands: &mut Commands) {
    for slot in 0..MAX_GLOWS {
        commands.spawn((
            Name::new("CampGlow"),
            PointLight {
                color: Color::srgb(1.0, 0.55, 0.22),
                intensity: 0.0,
                range: RANGE_M,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
            CampGlow { slot },
            DespawnOnExit(AppState::Battle),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn update_glows(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    daylight: Res<Daylight>,
    sources: Res<SmokeSources>,
    look: Option<Res<LightingLook>>,
    time: Res<Time>,
    mut stats: ResMut<GlowStats>,
    mut lights: Query<(&CampGlow, &mut PointLight, &mut Transform, &mut Visibility)>,
) {
    let level = daylight.level(session.sim_mut().hour() as f32);
    let dark = darkness(level);
    let fires = sources.glow_points(MAX_GLOWS);
    if dark > 0.0 && !fires.is_empty() && lights.is_empty() {
        spawn_glows(&mut commands);
    }
    let unit = look.map_or(2650.0, |l| l.unit());
    stats.light_level = level;
    stats.darkness = dark;
    stats.lit = 0;
    stats.strongest_lumens = 0.0;
    let seconds = time.elapsed_secs();
    for (glow, mut light, mut transform, mut visibility) in &mut lights {
        match fires.get(glow.slot) {
            Some(&(position, intensity)) if dark > 0.0 => {
                let energy = BASE_ENERGY * dark * intensity;
                // The flicker of `_process`: 0.85 + 0.15 sin(t * 11 rad/s + x).
                let flicker = 0.85 + 0.15 * (seconds * 11.0 + position.x).sin();
                light.intensity = lumens(energy * flicker, unit);
                transform.translation = position + Vec3::Y * HEIGHT_M;
                *visibility = Visibility::Inherited;
                stats.lit += 1;
                stats.strongest_lumens = stats.strongest_lumens.max(light.intensity);
            }
            _ => {
                light.intensity = 0.0;
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn dump_glow(world: &World) -> (String, serde_json::Value) {
    let Some(s) = world.get_resource::<GlowStats>() else {
        return ("battle_glow".into(), serde_json::Value::Null);
    };
    (
        "battle_glow".into(),
        serde_json::json!({
            "light_level": s.light_level, "darkness": s.darkness,
            "lit": s.lit, "strongest_lumens": s.strongest_lumens,
            "keyframes": world.get_resource::<Daylight>().map_or(0, |d| d.keyframes.len()),
        }),
    )
}

pub struct GlowPlugin;

impl Plugin for GlowPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Daylight::load())
            .init_resource::<GlowStats>()
            .register_dump(dump_glow)
            .add_systems(
                Update,
                update_glows
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn daylight() -> Daylight {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/fx/battle_staging.json"
        ))
        .unwrap();
        Daylight::from_json(&serde_json::from_str(&text).unwrap())
    }

    #[test]
    fn keyframes_give_a_dark_night_and_a_bright_noon() {
        let d = daylight();
        assert!(d.keyframes.len() >= 8);
        assert!((d.level(12.0) - 1.0).abs() < 1e-4);
        assert!((d.level(0.0) - 0.14).abs() < 1e-4);
        assert!((d.level(24.0) - d.level(0.0)).abs() < 1e-4, "wraps");
        let dusk = d.level(19.3);
        assert!(dusk > 0.26 && dusk < 0.7, "{dusk}");
        let after_midnight = d.level(22.0);
        assert!(
            after_midnight < 0.26 && after_midnight > 0.13,
            "{after_midnight}"
        );
    }

    #[test]
    fn only_camp_fires_glow_the_strongest_first() {
        let mut sources = SmokeSources::default();
        sources.add(Vec3::new(1.0, 0.0, 0.0), 0.4, "campfire");
        sources.add(Vec3::new(2.0, 0.0, 0.0), 1.5, "column");
        sources.add(Vec3::new(3.0, 0.0, 0.0), 0.9, "campfire");
        let points = sources.glow_points(MAX_GLOWS);
        assert_eq!(points.len(), 2, "the burning house does not light a camp");
        assert_eq!(points[0].0.x, 3.0);
        assert_eq!(sources.glow_points(1).len(), 1);
    }

    #[test]
    fn camps_glow_only_in_the_dark() {
        assert_eq!(darkness(1.0), 0.0);
        assert_eq!(darkness(0.75), 0.0);
        assert!((darkness(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(darkness(0.14), 1.0);
    }
}
