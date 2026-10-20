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
    /// rgb: colour of the sunlit snow of the ground, a: unused.
    pub snow_color: Vec4,
    /// rgb: colour of the snow in the shade (cold), a: snow share kept under the forests.
    pub snow_shade: Vec4,
    /// x: patch noise frequency (1 / map px), y: altitude jitter of the snow line (m),
    /// z, w: displayed-normal `y` where the snow starts / is full (steeper stays bare).
    pub snow_mask: Vec4,
    /// rgb: mud of the fringe between snow and grass and of the tracks under the snow, a: fringe width.
    pub mud: Vec4,
    /// rgb: dark rock of the heights and steep slopes, a: strength.
    pub rock: Vec4,
    /// x, y: displayed-normal `y` of full / no rock, z, w: altitude (m) where the rock starts / is full.
    pub rock_band: Vec4,
    /// x: sunlit / shaded slope contrast on the rock, y: lighter crests, z: strata strength,
    /// w: crest detection gain (curvature of the displayed normal per map px).
    pub rock_light: Vec4,
    /// rgb: ground tint under the forests (green-black), a: strength.
    pub forest_floor: Vec4,
    /// x: frequency (1 / map px) of the soil patches, y: strength, z: sharpening, w: unused.
    pub patches: Vec4,
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
            snow_color: Vec4::new(0.86, 0.88, 0.92, 0.0),
            snow_shade: Vec4::new(0.86, 0.88, 0.92, 1.0),
            snow_mask: Vec4::new(0.0, 0.0, 0.55, 0.85),
            mud: Vec4::new(0.2, 0.15, 0.1, 0.0),
            rock: Vec4::new(0.19, 0.17, 0.15, 0.0),
            rock_band: Vec4::new(0.4, 0.7, 1000.0, 2000.0),
            rock_light: Vec4::ZERO,
            forest_floor: Vec4::new(1.0, 1.0, 1.0, 0.0),
            patches: Vec4::ZERO,
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
        }
        // Share of a winter plain turned white (the Godot shader gates it by biome and slope).
        out.snow_plain.z = data["snow"]["plain_share"].as_f64().unwrap_or(0.55) as f32;
        out.snow_color =
            vec3_of(data["snow"].get("plain_color"), out.snow_color.truncate()).extend(0.0);
        let snow = &data["snow"];
        let num = |v: &Value, fallback: f32| v.as_f64().map_or(fallback, |x| x as f32);
        let pair = |v: &Value, fallback: Vec2| match v.as_array() {
            Some(a) if a.len() >= 2 => Vec2::new(num(&a[0], fallback.x), num(&a[1], fallback.y)),
            _ => fallback,
        };
        out.snow_shade = vec3_of(snow.get("shade_color"), out.snow_color.truncate())
            .extend(num(&snow["forest_share"], out.snow_shade.w));
        let flat = pair(
            &snow["flat_normal"],
            Vec2::new(out.snow_mask.z, out.snow_mask.w),
        );
        out.snow_mask = Vec4::new(
            num(&snow["patch_frequency"], out.snow_mask.x),
            num(&snow["line_jitter_m"], out.snow_mask.y),
            flat.x,
            flat.y,
        );
        out.mud = vec3_of(snow.get("mud_color"), out.mud.truncate())
            .extend(num(&snow["mud_fringe"], out.mud.w));
        let look = &data["ground_look"];
        out.rock = vec3_of(look.get("rock_color"), out.rock.truncate())
            .extend(num(&look["rock_strength"], out.rock.w));
        let steep = pair(
            &look["rock_normal"],
            Vec2::new(out.rock_band.x, out.rock_band.y),
        );
        let alt = pair(
            &look["rock_altitude_m"],
            Vec2::new(out.rock_band.z, out.rock_band.w),
        );
        out.rock_band = Vec4::new(steep.x, steep.y, alt.x, alt.y);
        out.rock_light = Vec4::new(
            num(&look["rock_slope_contrast"], 0.0),
            num(&look["rock_crest_light"], 0.0),
            num(&look["rock_strata"], 0.0),
            num(&look["rock_crest_gain"], 0.0),
        );
        out.forest_floor = vec3_of(look.get("forest_floor_tint"), Vec3::ONE)
            .extend(num(&look["forest_floor_strength"], 0.0));
        out.patches = Vec4::new(
            num(&look["patch_frequency"], 0.0),
            num(&look["patch_strength"], 0.0),
            num(&look["patch_sharpen"], 1.0),
            0.0,
        );
        let ground = &data["ground"];
        out.summer_gold = vec3_of(ground.get("summer_gold"), Vec3::ONE).extend(0.0);
        out.winter_ground = vec3_of(ground.get("winter_ground_tint"), Vec3::ONE)
            .extend(ground["winter_ground_desat"].as_f64().unwrap_or(0.0) as f32);
        out
    }
}

/// TS-T1: winter colours of a dirt road over the snow (dark mud): `(dirt, rut, edge)` from the
/// summer ones, the mud of the snow fringe and the winter weight.
pub fn road_winter_colors(base: [Vec4; 3], mud: Vec3, winter: f32) -> [Vec4; 3] {
    let k = (winter * 0.85).clamp(0.0, 1.0);
    let target = [mud * 0.75, mud * 0.5, mud];
    let mut out = base;
    for (o, t) in out.iter_mut().zip(target) {
        *o = o.truncate().lerp(t, k).extend(o.w);
    }
    out
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
    fn roads_turn_to_mud_in_winter_only() {
        let base = [Vec4::new(0.2, 0.15, 0.1, 1.0); 3];
        let mud = Vec3::new(0.1, 0.06, 0.04);
        assert_eq!(road_winter_colors(base, mud, 0.0), base);
        let w = road_winter_colors(base, mud, 1.0);
        assert!(w[0].x < base[0].x && w[1].x < w[0].x && w[2].w == 1.0);
    }

    #[test]
    fn parses_the_real_data_file() {
        let text = include_str!("../../../../data/ui/campaign_seasons.json");
        let p = SeasonParams::from_json(text);
        assert_eq!(p.snow_lines, Vec4::new(2000.0, 2900.0, 2300.0, 650.0));
        assert!(p.gain[3].w < 1.0, "winter desaturates");
        assert!(p.winter_ground.w > 0.3);
        assert!((p.snow_plain.x - 0.58).abs() < 1e-5);
        assert!((p.snow_plain.z - 0.55).abs() < 1e-5);
        let l = |c: Vec4| 0.299 * c.x + 0.587 * c.y + 0.114 * c.z;
        assert!(l(p.snow_color) < 0.8, "sunlit snow is never pure white");
        assert!(l(p.snow_shade) < l(p.snow_color), "shaded snow is darker");
        assert!(
            p.snow_shade.z > p.snow_shade.x,
            "shaded snow is cold (blue)"
        );
        assert!(p.snow_mask.z < p.snow_mask.w && p.snow_mask.x > 0.0);
        assert!(p.mud.w > 0.0 && p.rock.w > 0.0 && p.forest_floor.w > 0.0);
        assert!(p.rock_band.x < p.rock_band.y && p.rock_band.z < p.rock_band.w);
        assert!(l(p.rock) < 0.2, "the rock of the heights is dark");
        assert!(p.patches.x > 0.0 && p.patches.z >= 1.0);
    }
}
