//! Smoke of the field: lingering clouds after bombard and handgun shots, and lasting sources
//! (camp fires: pale smoke, burning houses: dark column) that other lots feed through
//! [`SmokeSources`]. Camera-facing soft puffs drifting with the battle wind.
//! Port of game/scripts/battle/battle_smoke.gd (settings `smoke` of `data/fx/battle_staging.json`).
//! Not ported: the flipbook smoke shader (`fire_smoke.gdshader`: soft puffs instead), the night
//! glow of camp fires (`OmniLight3D`).

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::corpses::hash4;
use super::wind::BattleWind;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// Puffs alive at once.
const POOL: usize = 360;
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

struct Puff {
    entity: Entity,
    material: Handle<StandardMaterial>,
    age: f32,
    life: f32,
    start: Vec3,
    velocity: Vec3,
    size: [f32; 2],
    color: [f32; 4],
    alive: bool,
}

#[derive(Resource, Default)]
pub struct Smoke {
    puffs: Vec<Puff>,
    next: usize,
    quad: Option<Handle<Mesh>>,
    texture: Option<Handle<Image>>,
    kinds: HashMap<String, SmokeKind>,
    counter: u64,
    pub emitted: usize,
}

fn puff_texture() -> Image {
    // Soft round puff: alpha falls off with the radius.
    const N: u32 = 64;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let dx = (x as f32 + 0.5) / N as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / N as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt();
            let a = (1.0 - r).clamp(0.0, 1.0);
            let a = a * a * (3.0 - 2.0 * a);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
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

    pub fn alive(&self) -> usize {
        self.puffs.iter().filter(|p| p.alive).count()
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_puff(
    commands: &mut Commands,
    smoke: &mut Smoke,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    velocity: Vec3,
    life: f32,
    size: [f32; 2],
    color: [f32; 4],
) {
    let (Some(quad), Some(texture)) = (smoke.quad.clone(), smoke.texture.clone()) else {
        return;
    };
    smoke.emitted += 1;
    let puff_data = |entity, material| Puff {
        entity,
        material,
        age: 0.0,
        life,
        start: position,
        velocity,
        size,
        color,
        alive: true,
    };
    if smoke.puffs.len() < POOL {
        let material = materials.add(StandardMaterial {
            base_color: Color::NONE,
            base_color_texture: Some(texture),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let entity = commands
            .spawn((
                Name::new("SmokePuff"),
                Mesh3d(quad),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position),
                NoFrustumCulling,
                DespawnOnExit(AppState::Battle),
            ))
            .id();
        smoke.puffs.push(puff_data(entity, material));
    } else {
        // Pool full: the next slot in turn is reused.
        let slot = smoke.next % POOL;
        smoke.next += 1;
        let (entity, material) = (smoke.puffs[slot].entity, smoke.puffs[slot].material.clone());
        smoke.puffs[slot] = puff_data(entity, material);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_smoke(
    mut commands: Commands,
    session: Res<BattleSession>,
    time: Res<Time>,
    wind: Option<Res<BattleWind>>,
    mut smoke: ResMut<Smoke>,
    mut sources: ResMut<SmokeSources>,
    mut requests: ResMut<SmokeRequests>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut transforms: Query<&mut Transform, Without<BattleCamera>>,
) {
    if smoke.quad.is_none() {
        smoke.quad = Some(meshes.add(Rectangle::new(1.0, 1.0)));
        smoke.texture = Some(images.add(puff_texture()));
    }
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
            let velocity = Vec3::new(h[0] - 0.5, 0.3 + h[3] * kind.rise, h[2] - 0.5) * 0.6
                + wind_dir * kind.drift * wind_push * 0.35;
            let life = kind.life * (0.6 + 0.4 * h[3]);
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut materials,
                muzzle + offset,
                velocity,
                life,
                kind.size,
                kind.color,
            );
        }
    }
    for muzzle in std::mem::take(&mut requests.puffs) {
        for k in 0..GUN_PUFFS {
            smoke.counter += 1;
            let h = hash4(smoke.counter * 11 + k as u64);
            let velocity = Vec3::new(h[0] - 0.5, 0.5 + h[1] * 0.5, h[2] - 0.5) * 0.8
                + wind_dir * wind_push * 0.6;
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut materials,
                muzzle + Vec3::new(h[0] - 0.5, h[3] * 0.3, h[2] - 0.5),
                velocity,
                GUN_LIFE_S * (0.7 + 0.3 * h[1]),
                [0.8, 2.6],
                [0.85, 0.84, 0.8, 0.35],
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
            let velocity = Vec3::new(
                (h[0] - 0.5) * 0.5,
                kind.rise * (0.6 + 0.6 * h[1]),
                (h[2] - 0.5) * 0.5,
            ) + wind_dir * kind.drift * wind_push * 0.35;
            spawn_puff(
                &mut commands,
                &mut smoke,
                &mut materials,
                position + Vec3::new((h[3] - 0.5) * 1.2, 0.0, (h[0] - 0.5) * 1.2),
                velocity,
                kind.life,
                kind.size,
                kind.color,
            );
        }
    }
    // Ageing: rise, grow, fade, face the camera.
    let rotation = camera.iter().next().map(|t| t.rotation());
    for puff in &mut smoke.puffs {
        if !puff.alive {
            continue;
        }
        puff.age += dt;
        let t = puff.age / puff.life.max(0.1);
        if t >= 1.0 {
            puff.alive = false;
            if let Ok(mut transform) = transforms.get_mut(puff.entity) {
                transform.scale = Vec3::ZERO;
            }
            continue;
        }
        let alpha = puff.color[3]
            * if t < 0.1 {
                t / 0.1
            } else {
                1.0 - 0.55 * ((t - 0.1) / 0.9)
            }
            * (1.0 - t.powi(4));
        if let Some(mut material) = materials.get_mut(&puff.material) {
            material.base_color = Color::srgba(puff.color[0], puff.color[1], puff.color[2], alpha);
        }
        if let Ok(mut transform) = transforms.get_mut(puff.entity) {
            let size = puff.size[0] + (puff.size[1] - puff.size[0]) * t.sqrt();
            transform.translation = puff.start + puff.velocity * puff.age * (1.0 - 0.3 * t);
            transform.scale = Vec3::splat(size);
            if let Some(rotation) = rotation {
                transform.rotation = rotation;
            }
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
            "alive": smoke.alive(),
            "emitted": smoke.emitted,
            "pool": smoke.puffs.len(),
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
