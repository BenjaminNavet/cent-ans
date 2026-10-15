//! Season look of the ground (port of `season_visuals.gd` + `season_look.gd`, data
//! `data/ui/campaign_seasons.json`): smoothed season weights, per-season colour gain / saturation,
//! summer gold, cold winter ground, altitude snow lines and the plain-snow fade towards the south.
//! Purely visual. The dev hook `CENT_ANS_SEASON=winter` forces a season (visual checks).

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use serde_json::Value;

use crate::sim::Sim;

/// Seconds of the transition between two seasons (`SeasonVisuals.transition_seconds`).
pub const TRANSITION_SECONDS: f32 = 2.5;
pub const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

/// Uniform block `SeasonParams` of `terrain.wesl`.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SeasonParams {
    /// Weights: spring, summer, autumn, winter (sum 1).
    pub weights: Vec4,
    /// Per season: rgb colour gain, a saturation.
    pub gain: [Vec4; 4],
    /// Snow line (m) per season.
    pub snow_lines: Vec4,
    /// rgb: summer gold multiplier, a: unused.
    pub summer_gold: Vec4,
    /// rgb: winter ground tint, a: winter ground desaturation.
    pub winter_ground: Vec4,
    /// x, y: plain-snow fade towards the south (fractions of the map height), z: plain snow share.
    pub snow_plain: Vec4,
}

impl Default for SeasonParams {
    fn default() -> Self {
        SeasonParams {
            weights: Vec4::new(1.0, 0.0, 0.0, 0.0),
            gain: [Vec4::new(1.0, 1.0, 1.0, 1.0); 4],
            snow_lines: Vec4::splat(-1.0),
            summer_gold: Vec4::ONE,
            winter_ground: Vec4::new(1.0, 1.0, 1.0, 0.0),
            snow_plain: Vec4::new(2.0, 3.0, 0.0, 0.0),
        }
    }
}

fn vec3_of(value: Option<&Value>, fallback: Vec3) -> Vec3 {
    match value.and_then(Value::as_array) {
        Some(a) if a.len() >= 3 => Vec3::new(
            a[0].as_f64().unwrap_or(1.0) as f32,
            a[1].as_f64().unwrap_or(1.0) as f32,
            a[2].as_f64().unwrap_or(1.0) as f32,
        ),
        _ => fallback,
    }
}

impl SeasonParams {
    /// Static part from the text of `campaign_seasons.json`; neutral values on a parse error.
    pub fn from_json(text: &str) -> SeasonParams {
        let mut out = SeasonParams::default();
        let Ok(data) = serde_json::from_str::<Value>(text) else {
            return out;
        };
        for (i, name) in SEASONS.iter().enumerate() {
            let grade = &data["seasons"][name]["grade"];
            let gain = vec3_of(grade.get("gain"), Vec3::ONE);
            let temperature = grade["temperature"].as_f64().unwrap_or(0.0) as f32;
            // Warm / cool shift of the grade, small so that the explicit gain stays the lead.
            let warm = Vec3::new(1.0 + 0.12 * temperature, 1.0, 1.0 - 0.12 * temperature);
            out.gain[i] = (gain * warm).extend(grade["saturation"].as_f64().unwrap_or(1.0) as f32);
        }
        if let Some(lines) = data["snow"]["line_m"].as_array()
            && lines.len() >= 4
        {
            for (i, v) in lines.iter().take(4).enumerate() {
                out.snow_lines[i] = v.as_f64().unwrap_or(-1.0) as f32;
            }
        }
        if let Some(fade) = data["snow"]["south_fade"].as_array()
            && fade.len() >= 2
        {
            out.snow_plain.x = fade[0].as_f64().unwrap_or(2.0) as f32;
            out.snow_plain.y = fade[1].as_f64().unwrap_or(3.0) as f32;
            // Share of a winter plain turned white (the Godot shader gates it by biome and slope).
            out.snow_plain.z = 0.55;
        }
        let ground = &data["ground"];
        out.summer_gold = vec3_of(ground.get("summer_gold"), Vec3::ONE).extend(0.0);
        out.winter_ground = vec3_of(ground.get("winter_ground_tint"), Vec3::ONE)
            .extend(ground["winter_ground_desat"].as_f64().unwrap_or(0.0) as f32);
        out
    }
}

/// Smoothed season weights (`SeasonVisuals`).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SeasonWeights {
    pub weights: Vec4,
    target: Vec4,
    from: Vec4,
    t: f32,
    pub season: &'static str,
}

impl Default for SeasonWeights {
    fn default() -> Self {
        let spring = Vec4::new(1.0, 0.0, 0.0, 0.0);
        SeasonWeights {
            weights: spring,
            target: spring,
            from: spring,
            t: 1.0,
            season: "spring",
        }
    }
}

impl SeasonWeights {
    pub fn weights_of(season: &str) -> Option<Vec4> {
        let index = SEASONS.iter().position(|s| *s == season)?;
        let mut w = Vec4::ZERO;
        w[index] = 1.0;
        Some(w)
    }

    /// Aims at a season; `instant` skips the transition.
    pub fn set_season(&mut self, season: &str, instant: bool) {
        let Some(wanted) = Self::weights_of(season) else {
            return;
        };
        self.season = SEASONS
            .iter()
            .find(|s| **s == season)
            .copied()
            .unwrap_or("spring");
        if instant {
            self.weights = wanted;
            self.target = wanted;
            self.t = 1.0;
        } else if wanted != self.target {
            self.from = self.weights;
            self.target = wanted;
            self.t = 0.0;
        }
    }

    pub fn update(&mut self, delta: f32) {
        if self.t >= 1.0 {
            return;
        }
        self.t = (self.t + delta / TRANSITION_SECONDS.max(0.01)).min(1.0);
        let s = self.t * self.t * (3.0 - 2.0 * self.t);
        self.weights = self.from.lerp(self.target, s);
    }
}

/// Reads the season of the simulation (or the `CENT_ANS_SEASON` hook) and glides the weights.
pub fn update_season(
    sim: Res<Sim>,
    time: Res<Time>,
    mut weights: ResMut<SeasonWeights>,
    mut first: Local<bool>,
) {
    let forced = std::env::var("CENT_ANS_SEASON").ok();
    let name = match (&forced, sim.state.as_ref()) {
        (Some(name), _) => name.to_lowercase(),
        (None, Some(state)) => format!("{:?}", state.season()).to_lowercase(),
        (None, None) => return,
    };
    let instant = !*first;
    *first = true;
    weights.set_season(&name, instant);
    weights.update(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_glide_to_the_target_and_sum_to_one() {
        let mut w = SeasonWeights::default();
        w.set_season("winter", false);
        w.update(1.25);
        assert!(w.weights.x > 0.0 && w.weights.w > 0.0);
        assert!((w.weights.element_sum() - 1.0).abs() < 1e-5);
        w.update(5.0);
        assert_eq!(w.weights, Vec4::new(0.0, 0.0, 0.0, 1.0));
        w.set_season("nonsense", true);
        assert_eq!(w.season, "winter");
    }

    #[test]
    fn parses_the_real_data_file() {
        let text = include_str!("../../../../data/ui/campaign_seasons.json");
        let p = SeasonParams::from_json(text);
        assert_eq!(p.snow_lines, Vec4::new(2000.0, 2900.0, 2300.0, 650.0));
        assert!(p.gain[3].w < 1.0, "winter desaturates");
        assert!(p.winter_ground.w > 0.3);
        assert!((p.snow_plain.x - 0.58).abs() < 1e-5);
    }
}
