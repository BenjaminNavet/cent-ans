//! Smoke of the field: lingering clouds after bombard and handgun shots, and lasting sources
//! (camp fires: pale smoke, burning houses: dark column) that other lots feed through
//! [`SmokeSources`]. Camera-facing soft puffs drifting with the battle wind.
//! Port of game/scripts/battle/battle_smoke.gd (settings `smoke` of `data/fx/battle_staging.json`).
//! The puffs are the player's smoke flipbook sprites (`crate::fx::flipbook`, `fire_smoke.gdshader`),
//! one tinted material per kind. Not ported: the night glow of camp fires (`OmniLight3D`), the
//! soft contact fade and the near fade of the shader.

use std::collections::HashMap;

use bevy::prelude::*;

use super::corpses::hash4;
use super::wind::BattleWind;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::fx::flipbook::{FlipbookLooks, FlipbookMaterial, SmokeSprite, spawn_smoke_sprite};

/// Short cloud of a handgun shot: puffs, life (s), size (m) range.
const GUN_PUFFS: usize = 3;
const GUN_LIFE_S: f32 = 3.5;
/// Sources kept alive at once (`max_sources`, medium preset).
const MAX_SOURCES: usize = 8;

/// One kind of smoke (`campfire`, `column`, `bombard_linger` of battle_staging.json).
#[derive(Clone, Debug)]
pub struct SmokeKind {
    pub amount: usize,
    pub life: f32,
    pub size: [f32; 2],
    pub rise: f32,
    pub drift: f32,
    pub color: [f32; 4],
}

impl SmokeKind {
    fn fallback(name: &str) -> Self {
        match name {
            "column" => Self {
                amount: 22,
                life: 16.0,
                size: [8.0, 26.0],
                rise: 3.2,
                drift: 2.6,
                color: [0.16, 0.15, 0.14, 0.62],
            },
            "bombard_linger" => Self {
                amount: 10,
                life: 14.0,
                size: [6.0, 16.0],
                rise: 0.5,
                drift: 1.2,
                color: [0.84, 0.83, 0.8, 0.4],
            },
            _ => Self {
                amount: 14,
                life: 9.0,
                size: [3.0, 9.0],
                rise: 1.6,
                drift: 1.4,
                color: [0.58, 0.57, 0.55, 0.3],
            },
        }
    }

    pub fn load(data_dir: &std::path::Path, name: &str) -> Self {
        let value = std::fs::read_to_string(data_dir.join("fx/battle_staging.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        let entry = value.as_ref().and_then(|v| v.get("smoke")?.get(name));
        let base = Self::fallback(name);
        let Some(e) = entry else {
            return base;
        };
        let num = |k: &str, d: f32| e.get(k).and_then(|v| v.as_f64()).map_or(d, |v| v as f32);
        let pair = |k: &str, d: [f32; 2]| {
            e.get(k)
                .and_then(|v| v.as_array())
                .filter(|a| a.len() >= 2)
                .map_or(d, |a| {
                    [
                        a[0].as_f64().map_or(d[0], |v| v as f32),
                        a[1].as_f64().map_or(d[1], |v| v as f32),
                    ]
                })
        };
        let color = e
            .get("color")
            .and_then(|v| v.as_array())
            .filter(|a| a.len() >= 4)
            .map_or(base.color, |a| {
                let c = |i: usize| a[i].as_f64().unwrap_or(0.5) as f32;
                [c(0), c(1), c(2), c(3)]
            });
        Self {
            amount: num("amount", base.amount as f32) as usize,
            life: num("lifetime_s", base.life),
            size: pair("size_m", base.size),
            rise: num("rise_m_s", base.rise),
            drift: num("drift_m_s", base.drift),
            color,
        }
    }
}

struct Source {
    position: Vec3,
    intensity: f32,
    kind: String,
    /// Time left before the next puff (s).
    emit_in: f32,
}

/// Lasting smoke sources: the public interface of other lots (camp fires, burning houses).
#[derive(Resource, Default)]
pub struct SmokeSources {
    sources: HashMap<u32, Source>,
    next: u32,
}

impl SmokeSources {
    /// Adds a source (`campfire` or `column`); `None` when the budget is full and every source
    /// is stronger.
    pub fn add(&mut self, position: Vec3, intensity: f32, kind: &str) -> Option<u32> {
        let intensity = intensity.clamp(0.05, 2.0);
        if self.sources.len() >= MAX_SOURCES {
            let weakest = self
                .sources
                .iter()
                .min_by(|a, b| a.1.intensity.total_cmp(&b.1.intensity))
                .map(|(id, s)| (*id, s.intensity))?;
            if weakest.1 >= intensity {
                return None;
            }
            self.sources.remove(&weakest.0);
        }
        self.next += 1;
        self.sources.insert(
            self.next,
            Source {
                position,
                intensity,
                kind: kind.to_owned(),
                emit_in: 0.0,
            },
        );
        Some(self.next)
    }

    pub fn remove(&mut self, id: u32) {
        self.sources.remove(&id);
    }

    /// Changes the intensity of a source; 0 puts it out.
    pub fn set_intensity(&mut self, id: u32, intensity: f32) {
        if intensity <= 0.02 {
            self.sources.remove(&id);
        } else if let Some(source) = self.sources.get_mut(&id) {
            source.intensity = intensity.clamp(0.05, 2.0);
        }
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

/// Smoke asked by the volleys.
#[derive(Resource, Default)]
pub struct SmokeRequests {
    /// Muzzles of bombards that fired: a lingering low cloud.
    pub bombard: Vec<Vec3>,
    /// Muzzles of handguns that fired: a short puff.
    pub puffs: Vec<Vec3>,
}

#[derive(Resource, Default)]
pub struct Smoke {
    kinds: HashMap<String, SmokeKind>,
    counter: u64,
    pub emitted: usize,
}

impl Smoke {
    fn kind(&mut self, name: &str) -> SmokeKind {
        if let Some(k) = self.kinds.get(name) {
            return k.clone();
        }
        let kind = crate::paths::find_root()
            .map(|root| SmokeKind::load(&crate::paths::data_dir(&root), name))
            .unwrap_or_else(|_| SmokeKind::fallback(name));
        self.kinds.insert(name.to_owned(), kind.clone());
        kind
    }
}

/// Wind push applied to the sprites as an acceleration (`gravity` of `battle_smoke.gd`).
struct Puff {
    position: Vec3,
    velocity: Vec3,
    accel: Vec3,
    damping: f32,
    life: f32,
    size: [f32; 2],
    color: [f32; 4],
}

/// Spawns a puff as a flipbook smoke sprite. Godot scales its particles 0.35 -> `size[1]` / 1.2
/// of `size[0]`; the sprite grows 0.5 -> 1.25 of its size, so that it ends at `size[1]`.
fn spawn_puff(
    commands: &mut Commands,
    smoke: &mut Smoke,
    looks: &mut FlipbookLooks,
    materials: &mut Assets<FlipbookMaterial>,
    puff: Puff,
    random: [f32; 4],
) {
    let size = puff.size[1] / 1.25 * (0.8 + 0.2 * random[0]);
    let sprite = SmokeSprite {
        position: puff.position,
        velocity: puff.velocity,
        accel: puff.accel,
        damping: puff.damping,
        life: puff.life,
        size,
        spin: (random[1] - 0.5) * std::f32::consts::TAU,
        spin_rate: (random[2] - 0.5) * 24f32.to_radians(),
        tint: Some(puff.color),
    };
    if let Some(entity) = spawn_smoke_sprite(commands, looks, materials, &sprite) {
        commands
            .entity(entity)
            .insert(DespawnOnExit(AppState::Battle));
        smoke.emitted += 1;
    }
}

fn update_smoke(
    mut commands: Commands,
    session: Res<BattleSession>,
    time: Res<Time>,
    wind: Option<Res<BattleWind>>,
    mut smoke: ResMut<Smoke>,
    mut sources: ResMut<SmokeSources>,
    mut requests: ResMut<SmokeRequests>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
) {
    let Some(mut looks) = looks else {
        // No flipbook plugin: nothing is drawn, the requests are dropped.
        requests.bombard.clear();
        requests.puffs.clear();
        return;
    };
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs() * session.speed as f32
    };
    let wind = wind.map_or(BattleWind::default(), |w| *w);
    let wind_dir = Vec3::new(wind.dir.x, 0.0, wind.dir.y);
    let wind_push = wind.strength.max(0.2);
    // Lingering bombard clouds and handgun puffs.
    for muzzle in std::mem::take(&mut requests.bombard) {
        let kind = smoke.kind("bombard_linger");
        for k in 0..kind.amount {
            smoke.counter += 1;
            let h = hash4(smoke.counter * 7 + k as u64);
            let offset = Vec3::new(h[0] - 0.5, h[1] * 0.4, h[2] - 0.5) * 3.0;
            let velocity = Vec3::new(h[0] - 0.5, 0.3 + h[3] * kind.rise, h[2] - 0.5) * 0.6;
            let puff = Puff {
                position: muzzle + offset,
                velocity,
                accel: wind_dir * kind.drift * wind_push * 0.35 + Vec3::Y * kind.rise * 0.05,
                damping: 0.1 + 1.1 * h[1],
                life: kind.life * (0.6 + 0.4 * h[3]),
                size: kind.size,
                color: kind.color,
            };
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut looks,
                &mut materials,
                puff,
                h,
            );
        }
    }
    for muzzle in std::mem::take(&mut requests.puffs) {
        for k in 0..GUN_PUFFS {
            smoke.counter += 1;
            let h = hash4(smoke.counter * 11 + k as u64);
            let puff = Puff {
                position: muzzle + Vec3::new(h[0] - 0.5, h[3] * 0.3, h[2] - 0.5),
                velocity: Vec3::new(h[0] - 0.5, 0.5 + h[1] * 0.5, h[2] - 0.5) * 0.8
                    + wind_dir * wind_push * 0.6,
                accel: Vec3::ZERO,
                damping: 0.3,
                life: GUN_LIFE_S * (0.7 + 0.3 * h[1]),
                size: [0.8, 2.6],
                color: [0.85, 0.84, 0.8, 0.35],
            };
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut looks,
                &mut materials,
                puff,
                h,
            );
        }
    }
    // Lasting sources: one puff every life / amount seconds, fewer when weak.
    let ids: Vec<u32> = sources.sources.keys().copied().collect();
    for id in ids {
        let (position, intensity, kind_name) = {
            let s = &sources.sources[&id];
            (s.position, s.intensity, s.kind.clone())
        };
        let kind = smoke.kind(&kind_name);
        let period = kind.life / (kind.amount.max(1) as f32 * intensity.clamp(0.05, 1.0));
        let mut emit = false;
        if let Some(source) = sources.sources.get_mut(&id) {
            source.emit_in -= dt;
            if source.emit_in <= 0.0 {
                source.emit_in += period;
                emit = true;
            }
        }
        if emit && dt > 0.0 {
            smoke.counter += 1;
            let h = hash4(smoke.counter * 5 + u64::from(id));
            let puff = Puff {
                position: position + Vec3::new((h[3] - 0.5) * 2.4, 0.0, (h[0] - 0.5) * 2.4),
                velocity: Vec3::new(
                    (h[0] - 0.5) * 0.5,
                    kind.rise * (0.6 + 0.6 * h[1]),
                    (h[2] - 0.5) * 0.5,
                ),
                accel: wind_dir * kind.drift * wind_push * 0.35 + Vec3::Y * kind.rise * 0.05,
                damping: 0.1 + 0.3 * h[2],
                life: kind.life,
                size: kind.size,
                color: kind.color,
            };
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut looks,
                &mut materials,
                puff,
                h,
            );
        }
    }
}

fn clear_smoke(
    mut smoke: ResMut<Smoke>,
    mut sources: ResMut<SmokeSources>,
    mut requests: ResMut<SmokeRequests>,
) {
    *smoke = Smoke::default();
    *sources = SmokeSources::default();
    *requests = SmokeRequests::default();
}

fn dump_smoke(world: &World) -> (String, serde_json::Value) {
    let Some(smoke) = world.get_resource::<Smoke>() else {
        return ("battle_smoke".into(), serde_json::Value::Null);
    };
    (
        "battle_smoke".into(),
        serde_json::json!({
            "emitted": smoke.emitted,
            "flipbook_tints": world.get_resource::<FlipbookLooks>().map_or(0, FlipbookLooks::tint_count),
            "sources": world.get_resource::<SmokeSources>().map_or(0, SmokeSources::len),
        }),
    )
}

pub struct SmokePlugin;

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Smoke>()
            .init_resource::<SmokeSources>()
            .init_resource::<SmokeRequests>()
            .register_dump(dump_smoke)
            .add_systems(
                Update,
                update_smoke
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(super::volleys::update_volleys),
            )
            .add_systems(OnExit(AppState::Battle), clear_smoke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_budget_replaces_the_weakest() {
        let mut sources = SmokeSources::default();
        let ids: Vec<u32> = (0..MAX_SOURCES)
            .map(|i| {
                sources
                    .add(Vec3::ZERO, 0.5 + i as f32 * 0.1, "campfire")
                    .unwrap()
            })
            .collect();
        assert_eq!(sources.len(), MAX_SOURCES);
        assert!(
            sources.add(Vec3::ZERO, 0.2, "campfire").is_none(),
            "weaker than all"
        );
        assert!(sources.add(Vec3::ZERO, 1.9, "column").is_some());
        assert!(!sources.sources.contains_key(&ids[0]), "weakest evicted");
        sources.set_intensity(ids[1], 0.0);
        assert_eq!(sources.len(), MAX_SOURCES - 1);
    }

    #[test]
    fn repository_kinds_parse() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
        let kind = SmokeKind::load(&dir, "column");
        assert_eq!(kind.amount, 22);
        assert!((kind.size[1] - 26.0).abs() < 1e-4);
    }
}
