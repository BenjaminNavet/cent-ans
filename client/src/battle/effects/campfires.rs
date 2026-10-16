//! Camp fire smoke sources of a battle (port of `BattleDecor.attach_smoke` of
//! game/scripts/battle/battle_decor.gd and of `BattleStaging._place_campfires` of
//! battle_staging.gd). They feed [`SmokeSources`] with `campfire` sources, whose pale smoke rises
//! over the camps and whose night glow is drawn by [`super::glow`].
//!
//! - The decor has camps: at most `camp_fires_per_side[quality]` of each camp's fires smoke,
//!   spread over the camp (`fires[(i + 0.5) * len / n]`), 0.6 m above the ground, intensity
//!   0.6 + 0.4 hash.
//! - No decor camp and not a siege: `count` fires per side `camp_depth_m` behind each battle line,
//!   spread over the middle 40 % of the width (Godot's random jitter is a deterministic hash here).
//! - No fire under the `no_campfire_weather` weathers (rain, snow).

use bevy::prelude::*;
use sim_battle::{BattleSim, DecorPropKind, SideId};

use super::hash01;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::units::smoke::SmokeSources;
use crate::settings::quality::RenderQuality;

/// Settings of `smoke` in `data/fx/battle_staging.json` used to place the fires.
#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSettings {
    /// `camp_fires_per_side` by quality level.
    pub per_side: Vec<(String, usize)>,
    pub camp_depth_m: f32,
    pub no_fire_weather: Vec<String>,
}

impl Default for CampfireSettings {
    fn default() -> Self {
        Self {
            per_side: vec![
                ("low".into(), 1),
                ("medium".into(), 2),
                ("high".into(), 3),
                ("ultra".into(), 4),
            ],
            camp_depth_m: 170.0,
            no_fire_weather: vec!["rain".into(), "snow".into()],
        }
    }
}

impl CampfireSettings {
    pub fn from_json(doc: &serde_json::Value) -> Self {
        let base = Self::default();
        let Some(smoke) = doc.get("smoke") else {
            return base;
        };
        let per_side = smoke
            .get("camp_fires_per_side")
            .and_then(|v| v.as_object())
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_u64()? as usize)))
                    .collect()
            })
            .unwrap_or(base.per_side);
        let camp_depth_m = smoke
            .get("camp_depth_m")
            .and_then(|v| v.as_f64())
            .map_or(base.camp_depth_m, |v| v as f32);
        let no_fire_weather = smoke
            .get("no_campfire_weather")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or(base.no_fire_weather);
        Self {
            per_side,
            camp_depth_m,
            no_fire_weather,
        }
    }

    fn load() -> Self {
        let Ok(root) = crate::paths::find_root() else {
            return Self::default();
        };
        std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/battle_staging.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .map_or_else(Self::default, |doc| Self::from_json(&doc))
    }

    /// Fires per side at `level` (Godot: `per_side.get(level, per_side.get("high", 2))`).
    pub fn count(&self, level: &str) -> usize {
        let get = |k: &str| self.per_side.iter().find(|(l, _)| l == k).map(|(_, n)| *n);
        get(level).or_else(|| get("high")).unwrap_or(2)
    }
}

/// One camp fire smoke source to add: position (m) and intensity.
pub type FireSource = (Vec3, f32);

/// The `count` smoking fires of one decor camp, among its `fires` (ground positions).
pub fn decor_camp_fires(fires: &[Vec3], count: usize) -> Vec<FireSource> {
    let n = count.min(fires.len());
    (0..n)
        .map(|i| {
            let index =
                (((i as f32 + 0.5) * fires.len() as f32 / n as f32) as usize).min(fires.len() - 1);
            let intensity = 0.6 + 0.4 * hash01(i as u64 * 1_000_003 + 73);
            (fires[index] + Vec3::Y * 0.6, intensity)
        })
        .collect()
}

/// Fires behind the battle lines when the decor has no camp (`_place_campfires`): `count` per
/// side, `depth` metres behind each line; `height(x, z)` is the ground.
pub fn line_camp_fires(
    width: f32,
    field_depth: f32,
    lines: (f32, f32),
    depth: f32,
    count: usize,
    seed: u64,
    height: impl Fn(f32, f32) -> f32,
) -> Vec<FireSource> {
    let mut out = Vec::new();
    let sides = [lines.0 - depth, lines.1 + depth];
    for (s, line) in sides.into_iter().enumerate() {
        let z = line.clamp(20.0, field_depth - 20.0);
        for i in 0..count {
            let key = seed.wrapping_mul(31).wrapping_add((s * 64 + i) as u64 * 7);
            let x =
                width * (0.3 + 0.4 * (i as f32 + 0.5) / count as f32) + (hash01(key) * 80.0 - 40.0);
            let pz = z + hash01(key + 1) * 50.0 - 25.0;
            let intensity = 0.6 + 0.4 * hash01(key + 2);
            out.push((Vec3::new(x, height(x, pz), pz), intensity));
        }
    }
    out
}

/// The camp fire sources of a battle, per the rules of the module.
pub fn battle_camp_fires(
    sim: &BattleSim,
    settings: &CampfireSettings,
    level: &str,
) -> Vec<FireSource> {
    let weather = sim.weather().key();
    if settings.no_fire_weather.iter().any(|w| w == weather) {
        return Vec::new();
    }
    let count = settings.count(level);
    let field = sim.field();
    if !field.decor.camps.is_empty() {
        let mut out = Vec::new();
        for side in SideId::BOTH {
            let Some(camp) = field.decor.camp(side) else {
                continue;
            };
            let fires: Vec<Vec3> = camp
                .items
                .iter()
                .filter(|p| p.kind == DecorPropKind::Campfire)
                .map(|p| Vec3::new(p.x as f32, field.height(p.x, p.z) as f32, p.z as f32))
                .collect();
            out.extend(decor_camp_fires(&fires, count));
        }
        return out;
    }
    if sim.siege().is_some() {
        return Vec::new();
    }
    line_camp_fires(
        field.size.width as f32,
        field.size.depth as f32,
        (
            field.attacker_line_z() as f32,
            field.defender_line_z() as f32,
        ),
        settings.camp_depth_m,
        count,
        5,
        |x, z| field.height(f64::from(x), f64::from(z)) as f32,
    )
}

#[derive(Resource, Default)]
struct CampfireState {
    placed: bool,
    sources: usize,
}

fn place_campfires(
    mut state: ResMut<CampfireState>,
    mut session: ResMut<BattleSession>,
    quality: Option<Res<RenderQuality>>,
    mut smoke: ResMut<SmokeSources>,
) {
    if state.placed {
        return;
    }
    state.placed = true;
    let settings = CampfireSettings::load();
    let level = quality.map_or_else(|| "medium".to_owned(), |q| q.level.clone());
    let fires = battle_camp_fires(session.sim_mut(), &settings, &level);
    state.sources = fires
        .into_iter()
        .filter(|(at, intensity)| smoke.add(*at, *intensity, "campfire").is_some())
        .count();
    info!("battle: {} camp fire smoke sources", state.sources);
}

fn dump_campfires(world: &World) -> (String, serde_json::Value) {
    let Some(s) = world.get_resource::<CampfireState>() else {
        return ("battle_campfires".into(), serde_json::Value::Null);
    };
    (
        "battle_campfires".into(),
        serde_json::json!({ "placed": s.placed, "sources": s.sources }),
    )
}

pub struct CampfirePlugin;

impl Plugin for CampfirePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CampfireState>()
            .register_dump(dump_campfires)
            .add_systems(
                Update,
                place_campfires
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(OnExit(AppState::Battle), |mut s: ResMut<CampfireState>| {
                *s = CampfireState::default();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_follow_the_staging_data() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/fx/battle_staging.json"
        ))
        .unwrap();
        let s = CampfireSettings::from_json(&serde_json::from_str(&text).unwrap());
        assert_eq!(s.count("medium"), 2);
        assert_eq!(s.count("ultra"), 4);
        assert_eq!(s.count("nonsense"), 3, "falls back on high");
        assert!(s.no_fire_weather.iter().any(|w| w == "rain"));
    }

    #[test]
    fn decor_camp_fires_are_spread_over_the_camp() {
        let fires: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32, 1.0, 0.0)).collect();
        let picked = decor_camp_fires(&fires, 2);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].0, Vec3::new(2.0, 1.6, 0.0));
        assert_eq!(picked[1].0, Vec3::new(7.0, 1.6, 0.0));
        assert!(picked.iter().all(|(_, k)| (0.6..=1.0).contains(k)));
        assert_eq!(decor_camp_fires(&fires[..1], 3).len(), 1);
        assert!(decor_camp_fires(&[], 3).is_empty());
    }

    #[test]
    fn line_fires_stand_behind_each_line() {
        let fires = line_camp_fires(1200.0, 800.0, (250.0, 550.0), 170.0, 2, 5, |_, _| 3.0);
        assert_eq!(fires.len(), 4);
        for (at, _) in &fires[..2] {
            assert!((at.z - 80.0).abs() <= 25.0, "{at}");
            assert!(at.x > 300.0 && at.x < 900.0);
            assert_eq!(at.y, 3.0);
        }
        for (at, _) in &fires[2..] {
            assert!((at.z - 720.0).abs() <= 25.0, "{at}");
        }
    }

    #[test]
    fn camp_fires_feed_the_night_glow() {
        let mut sources = SmokeSources::default();
        for (at, k) in decor_camp_fires(&[Vec3::ZERO, Vec3::X], 2) {
            sources.add(at, k, "campfire");
        }
        assert_eq!(sources.glow_points(8).len(), 2);
    }
}
