//! Atmosphere: a Bevy `DistanceFog` on the campaign camera whose colour and strength follow the
//! weather under the camera's focus (morning fog that lifts with the turn clock, grey haze in
//! rain and storm, pale haze in snow). Replaces the morning-mist and sky-haze part of
//! `campaign_weather_view.gd` / `map_atmosphere.gd` for objects drawn with Bevy materials; the
//! custom terrain shader has its own fog (look lot).

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::model::{Entry, Kind, lerp};
use super::{WeatherTuning, WeatherView};
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;

/// Seconds-based easing rate of strength and colour changes.
const EASE_PER_SECOND: f32 = 1.5;

/// Smoothed atmosphere state (what is applied to the camera).
#[derive(Resource, Debug, Clone, Default)]
pub struct Atmosphere {
    pub strength: f32,
    pub color: [f32; 3],
    pub density: f32,
    pub applied: bool,
}

/// Haze colour of a weather kind (linear-ish sRGB triple).
pub fn haze_color(kind: Kind) -> [f32; 3] {
    match kind {
        Kind::Fog => [0.80, 0.81, 0.80],
        Kind::Rain => [0.55, 0.59, 0.65],
        Kind::Storm => [0.38, 0.40, 0.46],
        Kind::Snow => [0.88, 0.90, 0.95],
        Kind::Clear => [0.78, 0.80, 0.84],
    }
}

/// Target fog strength (0..1): the morning fog is full at the start of the turn and keeps
/// `lifted_share` once lifted; rain, snow and storm add a light haze.
pub fn target_strength(entry: Entry, morning: f32, lifted_share: f32) -> f32 {
    let k = 0.45 + 0.55 * entry.intensity.clamp(0.0, 1.0);
    match entry.kind {
        Kind::Clear => 0.0,
        Kind::Fog => 0.8 * k * (lifted_share + (1.0 - lifted_share) * morning.clamp(0.0, 1.0)),
        Kind::Rain => 0.28 * k,
        Kind::Storm => 0.42 * k,
        Kind::Snow => 0.34 * k,
    }
}

/// Exponential density so that the fog reaches a visible share about two focus-distances away.
pub fn density_for(distance: f32) -> f32 {
    0.5 / distance.max(1.0)
}

fn update_atmosphere(
    mut commands: Commands,
    time: Res<Time<Real>>,
    view: Res<WeatherView>,
    tuning: Res<WeatherTuning>,
    mut state: ResMut<Atmosphere>,
    cameras: Query<(Entity, Option<&DistanceFog>), With<CampaignCamera>>,
) {
    let target = target_strength(
        view.focus_entry,
        view.morning_mist,
        tuning.clouds.lifted_share,
    );
    let color = haze_color(view.focus_entry.kind);
    let t = (EASE_PER_SECOND * time.delta_secs()).clamp(0.0, 1.0);
    state.strength = lerp(state.strength, target, t);
    for (i, c) in color.iter().enumerate() {
        state.color[i] = lerp(state.color[i], *c, t);
    }
    state.density = density_for(view.distance);
    let fog = DistanceFog {
        color: Color::srgba(
            state.color[0],
            state.color[1],
            state.color[2],
            state.strength,
        ),
        directional_light_color: Color::NONE,
        directional_light_exponent: 30.0,
        falloff: FogFalloff::Exponential {
            density: state.density,
        },
    };
    state.applied = false;
    for (entity, current) in &cameras {
        if current.is_none_or(|c| c.color != fog.color
                || !matches!(c.falloff, FogFalloff::Exponential { density } if (density - state.density).abs() < 1e-7)) {
            commands.entity(entity).insert(fog.clone());
        }
        state.applied = true;
    }
}

fn reset(mut state: ResMut<Atmosphere>) {
    *state = Atmosphere::default();
}

pub fn dump(world: &World) -> Value {
    let a = world.resource::<Atmosphere>();
    json!({
        "fog_strength": a.strength,
        "fog_color": a.color,
        "fog_density": a.density,
        "applied_to_camera": a.applied,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<Atmosphere>()
        .add_systems(OnEnter(AppState::Campaign), reset)
        .add_systems(
            Update,
            update_atmosphere
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: Kind, intensity: f32) -> Entry {
        Entry { kind, intensity }
    }

    #[test]
    fn morning_fog_lifts_but_keeps_its_share() {
        let start = target_strength(entry(Kind::Fog, 1.0), 1.0, 0.35);
        let lifted = target_strength(entry(Kind::Fog, 1.0), 0.0, 0.35);
        assert!(start > lifted && lifted > 0.0);
        assert!((lifted / start - 0.35).abs() < 1e-4);
    }

    #[test]
    fn clear_has_no_haze_and_storm_is_darker() {
        assert_eq!(target_strength(entry(Kind::Clear, 1.0), 1.0, 0.35), 0.0);
        assert!(haze_color(Kind::Storm)[0] < haze_color(Kind::Rain)[0]);
        assert!(
            target_strength(entry(Kind::Storm, 0.5), 0.0, 0.35)
                > target_strength(entry(Kind::Rain, 0.5), 0.0, 0.35)
        );
        assert!(density_for(10.0) > density_for(1000.0));
    }
}
