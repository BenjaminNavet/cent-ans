//! War and plague scars on the campaign map (port of `war_scars.gd`, lot TB4): everything comes
//! from the simulation state, thresholds and durations from `data/ui/war_scars.json`. No rule.
//!
//! - **Battlefield**: mound, debris and crows where a land battle was fought, for
//!   `battlefield.turns` turns (crows then debris leave earlier). Place and turn come from the
//!   battle history of the core (`CampaignState::battle_history`), so the marks survive a reload.
//!   Size follows the army figure scale law.
//! - **Plague**: mass graves grouped at the edge of the plague-struck settlements (scenes
//!   `plague` of `map_scenes`) and the cart of the dead, enlarged with the camera distance to stay
//!   readable at play distances (`plague.scale_per_distance`, below `plague.max_distance`).
//!
//! Not ported: crosses on the doors of the 1:1 houses (`_try_doors`, needs the town plans of the
//! towns lot), siege engines near the besiegers' camp (`_refresh_sieges`, needs the army markers
//! and `get_assault_odds`), hidden provinces of the army markers.
//! Devastation scenes are staged by the folk layer of the life lot, not here.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::scar_meshes::{self, hash01};
use crate::app::AppState;
use crate::campaign::armies::scale::{DEFAULT_SCALE_EXPONENT, scale_for_distance};
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::surface::GroundHeight;

/// Gap between the army and its mound (world units).
pub const FIELD_OFFSET: f32 = 0.7;
/// Height of a man in the army figure frame (`FIGURE_UNIT`).
pub const FIGURE_UNIT: f32 = 4.3;

#[derive(Debug, Clone, Default)]
pub struct BattlefieldCfg {
    pub turns: u32,
    pub crow_turns: u32,
    pub debris_turns: u32,
    pub crows: usize,
    pub debris: usize,
    pub radius: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PlagueCfg {
    pub max_distance: f32,
    pub min_intensity: f32,
    pub pits: [f32; 2],
    pub cart: bool,
    pub scale_per_distance: f32,
    pub max_scale: f32,
    pub cart_scale: f32,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ScarCfg {
    pub battlefield: BattlefieldCfg,
    pub plague: PlagueCfg,
    pub loaded: bool,
}

fn f(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key)
        .and_then(Value::as_f64)
        .map_or(default, |x| x as f32)
}

pub fn parse_cfg(doc: &Value) -> ScarCfg {
    let b = &doc["battlefield"];
    let p = &doc["plague"];
    let pits = p["pits"]
        .as_array()
        .filter(|a| a.len() >= 2)
        .map_or([0.0, 0.0], |a| {
            [
                a[0].as_f64().unwrap_or(0.0) as f32,
                a[1].as_f64().unwrap_or(0.0) as f32,
            ]
        });
    ScarCfg {
        battlefield: BattlefieldCfg {
            turns: f(b, "turns", 0.0) as u32,
            crow_turns: f(b, "crow_turns", 0.0) as u32,
            debris_turns: f(b, "debris_turns", 0.0) as u32,
            crows: f(b, "crows", 0.0) as usize,
            debris: f(b, "debris", 0.0) as usize,
            radius: f(b, "radius", 2.0),
            max_distance: f(b, "max_distance", 0.0),
        },
        plague: PlagueCfg {
            max_distance: f(p, "max_distance", 0.0),
            min_intensity: f(p, "min_intensity", 0.0),
            pits,
            cart: p["cart"].as_bool().unwrap_or(false),
            scale_per_distance: f(p, "scale_per_distance", 0.0),
            max_scale: f(p, "max_scale", 1.0),
            cart_scale: f(p, "cart_scale", 1.0),
        },
        loaded: true,
    }
}

/// A battle mark to stage.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPlan {
    pub province: String,
    pub turn: u32,
    pub source: Vec2,
    pub at: Vec2,
    pub seed: u64,
    pub crows: bool,
    pub debris: bool,
}

impl FieldPlan {
    pub fn signature(&self) -> String {
        format!(
            "{}|{:.1},{:.1}|{}{}",
            self.turn, self.source.x, self.source.y, self.crows as u8, self.debris as u8
        )
    }
}

fn text_seed(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// One record of the battle history, reduced to what the scars need.
#[derive(Debug, Clone)]
pub struct BattleMark {
    pub province: String,
    pub turn: u32,
    pub position: Vec2,
    pub losses: u32,
}

/// Latest battle of each province still within `battlefield.turns` turns, with at least one death
/// (a refused battle marks nothing).
pub fn plan_fields(turn: u32, records: &[BattleMark], cfg: &BattlefieldCfg) -> Vec<FieldPlan> {
    if cfg.turns == 0 {
        return Vec::new();
    }
    let mut latest: HashMap<&str, &BattleMark> = HashMap::new();
    for r in records {
        let age = turn.saturating_sub(r.turn);
        if r.turn > turn || age >= cfg.turns || r.losses == 0 {
            continue;
        }
        match latest.get(r.province.as_str()) {
            Some(prev) if prev.turn > r.turn => {}
            _ => {
                latest.insert(&r.province, r);
            }
        }
    }
    let mut out: Vec<FieldPlan> = latest
        .values()
        .map(|r| {
            let age = turn - r.turn;
            let angle =
                hash01(text_seed(&format!("{}{}", r.province, r.turn)), 7) * std::f32::consts::TAU;
            FieldPlan {
                province: r.province.clone(),
                turn: r.turn,
                source: r.position,
                at: r.position + Vec2::new(angle.cos(), angle.sin()) * FIELD_OFFSET,
                seed: text_seed(&r.province) & 0x7fff_ffff,
                crows: age < cfg.crow_turns,
                debris: age < cfg.debris_turns,
            }
        })
        .collect();
    out.sort_by(|a, b| a.province.cmp(&b.province));
    out
}

/// A plague-struck settlement.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaguePlan {
    pub settlement: String,
    pub center: Vec2,
    pub out: Vec2,
    pub footprint: f32,
    pub intensity: f32,
    pub seed: u64,
}

impl PlaguePlan {
    pub fn signature(&self) -> String {
        format!(
            "{:.2}|{:.1},{:.1}",
            self.intensity, self.center.x, self.center.y
        )
    }

    pub fn pit_count(&self, cfg: &PlagueCfg) -> usize {
        let t = self.intensity.clamp(0.0, 1.0);
        (cfg.pits[0] + (cfg.pits[1] - cfg.pits[0]) * t)
            .round()
            .max(0.0) as usize
    }
}

/// Plague scenes of the state at the settlements' maquettes.
pub fn plan_plague(scenes: &[(String, f32)], places: &Places, cfg: &PlagueCfg) -> Vec<PlaguePlan> {
    let mut out = Vec::new();
    for (settlement, intensity) in scenes {
        let intensity = intensity.clamp(0.0, 1.0);
        if intensity < cfg.min_intensity {
            continue;
        }
        let Some(place) = places.get(settlement) else {
            continue;
        };
        let seed = text_seed(settlement) & 0x7fff_ffff;
        let angle = hash01(seed, 1) * std::f32::consts::TAU;
        out.push(PlaguePlan {
            settlement: settlement.clone(),
            center: place.px,
            out: Vec2::new(angle.cos(), angle.sin()),
            footprint: place.radius,
            intensity,
            seed,
        });
    }
    out.sort_by(|a, b| a.settlement.cmp(&b.settlement));
    out
}

/// Enlargement of the pits and the cart with the camera distance (`plague_factor`).
pub fn plague_factor(cfg: &PlagueCfg, camera_distance: f32) -> f32 {
    (cfg.scale_per_distance * camera_distance).clamp(1.0, cfg.max_scale.max(1.0))
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Mound { yaw: f32 },
    Debris { local: Vec3, yaw: f32 },
    Crow { orbit: Vec4 },
    Pit { offset: Vec2, turn: f32 },
    Cart { pits: f32 },
}

#[derive(Component)]
pub struct ScarPart {
    pub field: bool,
    pub at: Vec2,
    pub out: Vec2,
    pub footprint: f32,
    pub role: Role,
}

struct Live {
    signature: String,
    entities: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct Scars {
    live: HashMap<String, Live>,
    assets: Option<ScarAssets>,
    pub fields: usize,
    pub plague_sites: usize,
    pub pits: usize,
    pub crows: usize,
    pub debris: usize,
    pub visible_parts: usize,
    last_turn: Option<u32>,
    last_player_hash: u64,
}

struct ScarAssets {
    material: Handle<StandardMaterial>,
    pit: Handle<Mesh>,
    cart: Handle<Mesh>,
    mound: Handle<Mesh>,
    debris: Vec<Handle<Mesh>>,
    crow: Handle<Mesh>,
}

pub fn load_cfg(mut cfg: ResMut<ScarCfg>) {
    if cfg.loaded {
        return;
    }
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let path = root.join("data/ui/war_scars.json");
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        Some(doc) => *cfg = parse_cfg(&doc),
        None => {
            warn!("scars: {} missing or invalid", path.display());
            cfg.loaded = true;
        }
    }
}

/// Re-reads the simulation state and rebuilds the scars that changed.
pub fn sync_scars(
    mut commands: Commands,
    sim: Res<crate::sim::Sim>,
    places: Res<Places>,
    cfg: Res<ScarCfg>,
    mut scars: ResMut<Scars>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !cfg.loaded || places.places.is_empty() {
        return;
    }
    *timer = 0.5;
    let (Some(state), Some(data)) = (sim.state.as_ref(), sim.data.as_ref()) else {
        return;
    };
    let turn = state.turn();
    let marks: Vec<BattleMark> = state
        .battle_history
        .records()
        .iter()
        .map(|r| BattleMark {
            province: r.province.as_str().to_string(),
            turn: r.turn,
            position: Vec2::new(r.position[0], r.position[1]),
            losses: r.attacker.losses + r.defender.losses,
        })
        .collect();
    let fields = plan_fields(turn, &marks, &cfg.battlefield);
    let scenes: Vec<(String, f32)> = sim_campaign::map_scenes::map_scenes(state, data)
        .into_iter()
        .filter(|s| s.kind == data_model::SceneKind::Plague)
        .filter_map(|s| {
            s.settlement
                .map(|id| (id.as_str().to_string(), s.intensity))
        })
        .collect();
    let plagues = plan_plague(&scenes, &places, &cfg.plague);
    let assets = scars.assets.get_or_insert_with(|| {
        let radius = cfg.battlefield.radius;
        ScarAssets {
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 1.0,
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            pit: meshes.add(scar_meshes::plague_pit()),
            cart: meshes.add(scar_meshes::dead_cart()),
            mound: meshes.add(scar_meshes::mound(radius * 0.4, radius * 0.17)),
            debris: (0..5)
                .map(|v| meshes.add(scar_meshes::debris(v, FIGURE_UNIT)))
                .collect(),
            crow: meshes.add(scar_meshes::crow(FIGURE_UNIT * 0.55)),
        }
    });
    let (material, pit, cart, mound, debris_meshes, crow) = (
        assets.material.clone(),
        assets.pit.clone(),
        assets.cart.clone(),
        assets.mound.clone(),
        assets.debris.clone(),
        assets.crow.clone(),
    );
    let mut wanted: HashMap<String, String> = HashMap::new();
    for p in &fields {
        wanted.insert(format!("field:{}", p.province), p.signature());
    }
    for p in &plagues {
        wanted.insert(format!("plague:{}", p.settlement), p.signature());
    }
    let stale: Vec<String> = scars
        .live
        .iter()
        .filter(|(k, l)| wanted.get(*k) != Some(&l.signature))
        .map(|(k, _)| k.clone())
        .collect();
    for key in stale {
        if let Some(live) = scars.live.remove(&key) {
            for e in live.entities {
                commands.entity(e).despawn();
            }
        }
    }
    let spawn_part = |commands: &mut Commands, mesh: &Handle<Mesh>, part: ScarPart| -> Entity {
        commands
            .spawn((
                Name::new("scar"),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_scale(Vec3::splat(1e-3)),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
                part,
                DespawnOnExit(AppState::Campaign),
            ))
            .id()
    };
    for p in &fields {
        let key = format!("field:{}", p.province);
        if scars.live.contains_key(&key) {
            continue;
        }
        let b = &cfg.battlefield;
        let mut entities = Vec::new();
        let base = |role| ScarPart {
            field: true,
            at: p.at,
            out: Vec2::X,
            footprint: 0.0,
            role,
        };
        entities.push(spawn_part(
            &mut commands,
            &mound,
            base(Role::Mound {
                yaw: hash01(p.seed, 1) * std::f32::consts::TAU,
            }),
        ));
        if p.debris {
            for k in 0..b.debris as u64 {
                let angle = hash01(p.seed, 10 + k) * std::f32::consts::TAU;
                let reach = b.radius * (0.5 + 0.5 * hash01(p.seed, 60 + k).sqrt());
                let variant = (hash01(p.seed, 110 + k) * 5.0) as usize;
                entities.push(spawn_part(
                    &mut commands,
                    &debris_meshes[variant % 5],
                    base(Role::Debris {
                        local: Vec3::new(angle.cos() * reach, 0.0, angle.sin() * reach),
                        yaw: hash01(p.seed, 160 + k) * std::f32::consts::TAU,
                    }),
                ));
            }
        }
        if p.crows {
            for k in 0..b.crows as u64 {
                let sign = if hash01(p.seed, 360 + k) < 0.5 {
                    1.0
                } else {
                    -1.0
                };
                let orbit = Vec4::new(
                    b.radius * (0.35 + 0.5 * hash01(p.seed, 210 + k)),
                    b.radius * (0.55 + 0.5 * hash01(p.seed, 260 + k)),
                    (0.35 + 0.3 * hash01(p.seed, 310 + k)) * sign,
                    hash01(p.seed, 410 + k) * std::f32::consts::TAU,
                );
                entities.push(spawn_part(&mut commands, &crow, base(Role::Crow { orbit })));
            }
        }
        scars.live.insert(
            key,
            Live {
                signature: p.signature(),
                entities,
            },
        );
    }
    for p in &plagues {
        let key = format!("plague:{}", p.settlement);
        if scars.live.contains_key(&key) {
            continue;
        }
        let pits = p.pit_count(&cfg.plague);
        let mut entities = Vec::new();
        let base = |role| ScarPart {
            field: false,
            at: p.center,
            out: p.out,
            footprint: p.footprint,
            role,
        };
        for k in 0..pits {
            let offset = Vec2::new(
                10.0 + 7.5 * (k % 2) as f32,
                (k as f32 - (pits as f32 - 1.0) * 0.5) * 8.5,
            );
            let turn = (hash01(p.seed, 40 + k as u64) - 0.5) * 0.5;
            entities.push(spawn_part(
                &mut commands,
                &pit,
                base(Role::Pit { offset, turn }),
            ));
        }
        if cfg.plague.cart && pits > 0 {
            entities.push(spawn_part(
                &mut commands,
                &cart,
                base(Role::Cart { pits: pits as f32 }),
            ));
        }
        scars.live.insert(
            key,
            Live {
                signature: p.signature(),
                entities,
            },
        );
    }
    scars.fields = fields.len();
    scars.plague_sites = plagues.len();
    scars.pits = plagues.iter().map(|p| p.pit_count(&cfg.plague)).sum();
    scars.crows = fields.iter().filter(|p| p.crows).count() * cfg.battlefield.crows;
    scars.debris = fields.iter().filter(|p| p.debris).count() * cfg.battlefield.debris;
    scars.last_turn = Some(turn);
    scars.last_player_hash = 0;
}

/// Places the parts at the current scale, grounds them, flies the crows.
pub fn animate_scars(
    time: Res<Time>,
    cfg: Res<ScarCfg>,
    rig: Option<Res<CampaignRig>>,
    zoom: Option<Res<ZoomView>>,
    ground: GroundHeight,
    mut scars: ResMut<Scars>,
    mut parts: Query<(&ScarPart, &mut Transform, &mut Visibility)>,
    loaded: Option<Res<super::LoadedCountryside>>,
) {
    let Some(rig) = rig else {
        return;
    };
    let d = rig.distance;
    let normal = 1.0 - zoom.map_or(0.0, |z| z.strategic);
    let mpp = loaded.map_or(719.0, |l| l.config.meters_per_px);
    let field_on = normal > 0.02 && d <= cfg.battlefield.max_distance;
    let plague_on = normal > 0.02 && d <= cfg.plague.max_distance;
    let field_scale = scale_for_distance(d, 0.0, DEFAULT_SCALE_EXPONENT) * normal.max(0.02);
    let plague_scale = plague_factor(&cfg.plague, d);
    let t = time.elapsed_secs();
    let mut visible = 0;
    for (part, mut tr, mut vis) in &mut parts {
        let on = if part.field { field_on } else { plague_on } && ground.is_ready();
        let wanted = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != wanted {
            *vis = wanted;
        }
        if !on {
            continue;
        }
        visible += 1;
        let g = |p: Vec2| ground.at(p.x, p.y).unwrap_or(0.0);
        match part.role {
            Role::Mound { yaw } => {
                let s = field_scale;
                tr.translation = Vec3::new(part.at.x, g(part.at), part.at.y);
                tr.rotation = Quat::from_rotation_y(yaw);
                tr.scale = Vec3::splat(s);
            }
            Role::Debris { local, yaw } => {
                let s = field_scale;
                let xz = part.at + Vec2::new(local.x, local.z) * s;
                tr.translation = Vec3::new(xz.x, g(xz) + local.y * s, xz.y);
                tr.rotation = Quat::from_rotation_y(yaw);
                tr.scale = Vec3::splat(s);
            }
            Role::Crow { orbit } => {
                let s = field_scale;
                let angle = orbit.w + t * orbit.z;
                let bob = (t * 1.7 + orbit.w * 3.0).sin() * 0.08;
                let local = Vec3::new(
                    angle.cos() * orbit.x,
                    orbit.y * (1.0 + bob),
                    angle.sin() * orbit.x,
                );
                let base = Vec3::new(part.at.x, g(part.at), part.at.y);
                tr.translation = base + local * s;
                let back = if orbit.z > 0.0 {
                    0.0
                } else {
                    std::f32::consts::PI
                };
                tr.rotation = Quat::from_rotation_y(-angle + back)
                    * Quat::from_rotation_z(0.3 * orbit.z.signum());
                tr.scale = Vec3::splat(s);
            }
            Role::Pit { offset, turn } => {
                let unit = plague_scale / mpp;
                let along = Vec2::new(-part.out.y, part.out.x);
                let edge = part.at + part.out * part.footprint;
                let at = edge + (part.out * offset.x + along * offset.y) * unit;
                let dir = Vec2::from_angle(turn).rotate(along);
                tr.translation = Vec3::new(at.x, g(at), at.y);
                tr.rotation = Quat::from_rotation_y((-dir.y).atan2(dir.x));
                tr.scale = Vec3::splat(unit);
            }
            Role::Cart { pits } => {
                let unit = plague_scale / mpp;
                let cart_unit = unit * cfg.plague.cart_scale.max(1.0);
                let along = Vec2::new(-part.out.y, part.out.x);
                let edge = part.at + part.out * part.footprint;
                let at = edge + (part.out * 2.0 - along * (pits * 4.3 + 6.0)) * unit;
                tr.translation = Vec3::new(at.x, g(at), at.y);
                tr.rotation = Quat::from_rotation_y(along.x.atan2(along.y));
                tr.scale = Vec3::splat(cart_unit);
            }
        }
    }
    scars.visible_parts = visible;
}

pub fn reset(mut scars: ResMut<Scars>) {
    scars.live.clear();
    scars.assets = None;
    scars.fields = 0;
    scars.plague_sites = 0;
}

pub fn dump(world: &World) -> (String, Value) {
    let s = world.resource::<Scars>();
    let cfg = world.resource::<ScarCfg>();
    (
        "countryside_scars".into(),
        json!({
            "cfg_loaded": cfg.loaded,
            "battlefield": {
                "turns": cfg.battlefield.turns,
                "marked": s.fields,
                "crows": s.crows,
                "debris": s.debris,
            },
            "plague": { "sites": s.plague_sites, "pits": s.pits },
            "live_groups": s.live.len(),
            "parts": s.live.values().map(|l| l.entities.len()).sum::<usize>(),
            "visible_parts": s.visible_parts,
            "last_turn": s.last_turn,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> BattlefieldCfg {
        BattlefieldCfg {
            turns: 4,
            crow_turns: 2,
            debris_turns: 3,
            crows: 5,
            debris: 9,
            radius: 8.0,
            max_distance: 420.0,
        }
    }

    fn mark(province: &str, turn: u32, losses: u32) -> BattleMark {
        BattleMark {
            province: province.into(),
            turn,
            position: Vec2::new(100.0, 200.0),
            losses,
        }
    }

    #[test]
    fn marks_age_out_and_lose_their_crows_then_debris() {
        let c = cfg();
        let recs = [mark("a", 10, 5)];
        let at = |turn| plan_fields(turn, &recs, &c);
        let fresh = at(10);
        assert_eq!(fresh.len(), 1);
        assert!(fresh[0].crows && fresh[0].debris);
        let t12 = at(12);
        assert!(!t12[0].crows && t12[0].debris);
        let t13 = at(13);
        assert!(!t13[0].crows && !t13[0].debris);
        assert!(at(14).is_empty());
        assert_ne!(fresh[0].signature(), t12[0].signature());
    }

    #[test]
    fn latest_battle_wins_and_bloodless_ones_mark_nothing() {
        let c = cfg();
        let recs = [mark("a", 8, 5), mark("a", 10, 3), mark("b", 10, 0)];
        let plans = plan_fields(10, &recs, &c);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].turn, 10);
        // The mound sits next to the battle, not on it.
        let d = plans[0].at.distance(plans[0].source);
        assert!((d - FIELD_OFFSET).abs() < 1e-4);
        assert_eq!(plan_fields(10, &recs, &BattlefieldCfg::default()).len(), 0);
    }

    #[test]
    fn plague_pits_follow_intensity_and_distance_enlarges() {
        let cfg = PlagueCfg {
            pits: [2.0, 5.0],
            scale_per_distance: 4.2,
            max_scale: 260.0,
            ..Default::default()
        };
        let plan = |i: f32| PlaguePlan {
            settlement: "s".into(),
            center: Vec2::ZERO,
            out: Vec2::X,
            footprint: 1.0,
            intensity: i,
            seed: 1,
        };
        assert_eq!(plan(0.0).pit_count(&cfg), 2);
        assert_eq!(plan(1.0).pit_count(&cfg), 5);
        assert_eq!(plague_factor(&cfg, 0.1), 1.0);
        assert!((plague_factor(&cfg, 10.0) - 42.0).abs() < 1e-3);
        assert_eq!(plague_factor(&cfg, 1000.0), 260.0);
    }

    #[test]
    fn repository_document_parses() {
        let root = crate::paths::find_root().expect("root");
        let doc: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("data/ui/war_scars.json")).unwrap(),
        )
        .unwrap();
        let c = parse_cfg(&doc);
        assert_eq!(c.battlefield.turns, 4);
        assert_eq!(c.battlefield.crows, 5);
        assert_eq!(c.plague.pits, [2.0, 5.0]);
        assert!(c.plague.cart);
    }
}
