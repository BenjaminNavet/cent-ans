//! Village, church and castle silhouettes of the battle horizon (port of `_build_silhouettes`,
//! `_pick_site`, `_box`, `_roof`, `_add_church`, `_add_house` and `_add_castle` of
//! game/scripts/battle/battle_horizon.gd).
//!
//! Godot placed 2-5 villages (a church with a tower and 4-8 houses) and 0-1 castle on the far ring,
//! 2.6-8.5 km from the field (`data/fx/horizon.json` `silhouettes`), as one vertex-coloured mesh
//! without shadow that the distance fog melts away; the smoke columns of `horizon_smoke` rise over
//! the same sites. Sites are drawn by a seeded generator keyed on the province (the same horizon
//! for the same province); candidates are rejected in the sea, off the ring, on slopes steeper
//! than 0.25 or below the tile's sea level. Vertex colours are linear in both engines.
//! Deviations: flat normals (Godot smoothed identical positions; invisible at 3 km), triangle
//! winding reversed (Godot's front faces are clockwise).

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::Battlefield;

use super::features::sea_rect;
use super::horizon::{HorizonStats, RealRelief, ring_height};
use super::look::DecorLook;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::paths;

/// Godot `FAR_RECT`: the far ring reaches 7 km beyond the field on every side.
pub const FAR_MARGIN: f32 = 7000.0;
const SEA_CLASS: u8 = 255;

/// Tiny seeded generator (splitmix64) standing for Godot's `RandomNumberGenerator`.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn seeded(text: &str) -> Rng {
        // FNV-1a of the text.
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in text.bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        Rng(h)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..1.
    pub fn randf(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.randf()
    }

    /// Integer in `a..=b`.
    pub fn int(&mut self, a: u32, b: u32) -> u32 {
        a + (self.next_u64() % u64::from(b.max(a) - a + 1)) as u32
    }
}

/// `horizon.json` `silhouettes`.
#[derive(Debug, Clone, PartialEq)]
pub struct SilhouetteConfig {
    pub min_distance: f32,
    pub max_distance: f32,
    pub scale: f32,
    pub villages: (u32, u32),
    pub castles: (u32, u32),
    pub smokes: (u32, u32),
    pub stone: [f32; 3],
    pub roof: [f32; 3],
    pub smoke: [f32; 3],
}

impl SilhouetteConfig {
    /// `None` when the file has no `silhouettes` block (Godot then built nothing).
    pub fn from_horizon(horizon: &serde_json::Value) -> Option<SilhouetteConfig> {
        let s = horizon.get("silhouettes")?;
        if s.as_object().is_none_or(|o| o.is_empty()) {
            return None;
        }
        let num = |v: &serde_json::Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        let range = |v: &serde_json::Value, d: (u32, u32)| match v.as_array() {
            Some(a) if a.len() >= 2 => (
                a[0].as_u64().unwrap_or(0) as u32,
                a[1].as_u64().unwrap_or(0) as u32,
            ),
            Some(_) => (0, 0),
            None => d,
        };
        let rgb = |v: &serde_json::Value, d: [f32; 3]| match v.as_array() {
            Some(a) if a.len() >= 3 => [num(&a[0], d[0]), num(&a[1], d[1]), num(&a[2], d[2])],
            _ => d,
        };
        Some(SilhouetteConfig {
            min_distance: num(&s["min_distance_m"], 2600.0),
            max_distance: num(&s["max_distance_m"], 8500.0),
            scale: num(&s["scale"], 1.3),
            villages: range(&s["villages"], (2, 5)),
            castles: range(&s["castles"], (0, 1)),
            smokes: range(&s["smokes"], (1, 4)),
            stone: rgb(&s["stone"], [0.36, 0.34, 0.31]),
            roof: rgb(&s["roof"], [0.3, 0.22, 0.18]),
            smoke: rgb(&s["smoke"], [0.62, 0.62, 0.64]),
        })
    }
}

/// Where the sites may stand.
pub struct SiteGround<'a> {
    pub field: &'a Battlefield,
    pub look: &'a DecorLook,
    pub relief: Option<&'a RealRelief>,
}

impl SiteGround<'_> {
    fn centre(&self) -> Vec2 {
        Vec2::new(self.field.width as f32 * 0.5, self.field.depth as f32 * 0.5)
    }

    fn height(&self, x: f32, z: f32) -> f32 {
        ring_height(self.field, self.look, self.relief, x, z)
    }

    fn offset_y(&self) -> f32 {
        self.relief.map_or(0.0, |r| r.offset_y)
    }

    fn in_far_rect(&self, p: Vec2) -> bool {
        p.x >= -FAR_MARGIN
            && p.y >= -FAR_MARGIN
            && p.x < self.field.width as f32 + FAR_MARGIN
            && p.y < self.field.depth as f32 + FAR_MARGIN
    }

    fn is_sea(&self, p: Vec2) -> bool {
        if self
            .relief
            .is_some_and(|r| r.class_at(p.x, p.y) == SEA_CLASS)
        {
            return true;
        }
        sea_rect(self.field)
            .is_some_and(|r| p.x >= r[0] && p.x <= r[1] && p.y >= r[2] && p.y <= r[3])
    }

    /// Ground site between `dmin` and `dmax` of the field; `high`: the highest of several tries.
    fn pick_site(&self, rng: &mut Rng, (dmin, dmax): (f32, f32), high: bool) -> Option<Vec3> {
        let half_diagonal =
            Vec2::new(self.field.width as f32, self.field.depth as f32).length() * 0.5;
        let mut best: Option<Vec3> = None;
        for _ in 0..(if high { 40 } else { 12 }) {
            let a = rng.randf() * TAU;
            let d = rng.range(dmin, dmax);
            let p = self.centre() + Vec2::new(a.sin(), -a.cos()) * (d + half_diagonal);
            if !self.in_far_rect(p) || self.is_sea(p) {
                continue;
            }
            let h = self.height(p.x, p.y);
            let slope = (self.height(p.x + 30.0, p.y) - self.height(p.x - 30.0, p.y)).abs() / 60.0;
            if slope > 0.25 || h < self.offset_y() + 1.0 {
                continue;
            }
            if best.is_none() || (high && h > best.map_or(f32::MIN, |b| b.y)) {
                best = Some(Vec3::new(p.x, h, p.y));
                if !high {
                    break;
                }
            }
        }
        best
    }
}

/// A smoke column: base position and the shader seed (0..10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmokeSite {
    pub position: Vec3,
    pub seed: f32,
}

/// Triangle soup with a colour per triangle.
#[derive(Debug, Default)]
pub struct Soup {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
}

impl Soup {
    /// Adds a triangle given in Godot's (clockwise front) order.
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Vec3) {
        for p in [a, c, b] {
            self.positions.push(p.to_array());
            self.colors.push([color.x, color.y, color.z, 1.0]);
        }
    }

    pub fn into_mesh(self) -> Mesh {
        let mut normals = Vec::with_capacity(self.positions.len());
        for t in self.positions.chunks(3) {
            let (a, b, c) = (
                Vec3::from_array(t[0]),
                Vec3::from_array(t[1]),
                Vec3::from_array(t[2]),
            );
            let n = (b - a).cross(c - a).normalize_or_zero();
            normals.extend([n.to_array(); 3]);
        }
        let count = self.positions.len() as u32;
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32((0..count).collect()))
    }
}

fn darkened(c: Vec3, a: f32) -> Vec3 {
    c * (1.0 - a)
}

fn lightened(c: Vec3, a: f32) -> Vec3 {
    c + (Vec3::ONE - c) * a
}

fn yaw(angle: f32) -> Quat {
    Quat::from_rotation_y(angle)
}

/// Walls (sunk 3 m into the ground) and flat top of a box.
fn add_box(soup: &mut Soup, base: Vec3, angle: f32, size: Vec3, color: Vec3) {
    let b = yaw(angle);
    let (hx, hz) = (size.x * 0.5, size.z * 0.5);
    let c = [
        Vec3::new(-hx, 0.0, -hz),
        Vec3::new(hx, 0.0, -hz),
        Vec3::new(hx, 0.0, hz),
        Vec3::new(-hx, 0.0, hz),
    ];
    let sink = Vec3::new(0.0, -3.0, 0.0);
    let up = Vec3::new(0.0, size.y, 0.0);
    for k in 0..4 {
        let a = base + b * c[k] + sink;
        let d = base + b * c[(k + 1) % 4] + sink;
        let a2 = base + b * c[k] + up;
        let d2 = base + b * c[(k + 1) % 4] + up;
        soup.tri(a, a2, d, color);
        soup.tri(d, a2, d2, color);
    }
    let top: Vec<Vec3> = c.iter().map(|v| base + b * *v + up).collect();
    // Godot listed the top the other way round (facing down, so culled from above).
    soup.tri(top[0], top[1], top[2], color);
    soup.tri(top[0], top[2], top[3], color);
}

/// Gable roof (prism) or spire (pyramid when `ridge` is 0).
fn add_roof(soup: &mut Soup, base: Vec3, angle: f32, size: Vec3, ridge: f32, color: Vec3) {
    let b = yaw(angle);
    let (hx, hz) = (size.x * 0.5, size.z * 0.5);
    let r = ridge * 0.5;
    let p0 = base + b * Vec3::new(-hx, 0.0, -hz);
    let p1 = base + b * Vec3::new(hx, 0.0, -hz);
    let p2 = base + b * Vec3::new(hx, 0.0, hz);
    let p3 = base + b * Vec3::new(-hx, 0.0, hz);
    let t0 = base + b * Vec3::new(-r, size.y, 0.0);
    let t1 = base + b * Vec3::new(r, size.y, 0.0);
    for [a, m, c] in [
        [p0, t0, p1],
        [p1, t0, t1],
        [p2, t1, p3],
        [p3, t1, t0],
        [p1, t1, p2],
        [p3, t0, p0],
    ] {
        soup.tri(a, m, c, color);
    }
}

fn add_church(soup: &mut Soup, p: Vec3, angle: f32, s: f32, stone: Vec3, roof: Vec3) {
    add_box(soup, p, angle, Vec3::new(24.0, 11.0, 10.0) * s, stone);
    add_roof(
        soup,
        p + Vec3::new(0.0, 11.0 * s, 0.0),
        angle,
        Vec3::new(24.0, 7.0, 10.0) * s,
        24.0 * s,
        roof,
    );
    let tower = p + yaw(angle) * Vec3::new(-14.0 * s, 0.0, 0.0);
    add_box(soup, tower, angle, Vec3::new(6.0, 24.0, 6.0) * s, stone);
    add_roof(
        soup,
        tower + Vec3::new(0.0, 24.0 * s, 0.0),
        angle,
        Vec3::new(6.0, 16.0, 6.0) * s,
        0.0,
        darkened(roof, 0.2),
    );
}

fn add_house(soup: &mut Soup, p: Vec3, angle: f32, s: f32, wall: Vec3, roof: Vec3) {
    add_box(soup, p, angle, Vec3::new(10.0, 5.0, 6.0) * s, wall);
    add_roof(
        soup,
        p + Vec3::new(0.0, 5.0 * s, 0.0),
        angle,
        Vec3::new(10.0, 4.5, 6.0) * s,
        10.0 * s,
        roof,
    );
}

fn add_castle(soup: &mut Soup, p: Vec3, angle: f32, s: f32, stone: Vec3) {
    let b = yaw(angle);
    add_box(
        soup,
        p,
        angle,
        Vec3::new(14.0, 32.0, 14.0) * s,
        darkened(stone, 0.08),
    );
    for k in 0..4 {
        let corner = p + b
            * (Vec3::new(
                if k % 2 == 0 { 1.0 } else { -1.0 },
                0.0,
                if k < 2 { 1.0 } else { -1.0 },
            ) * 34.0
                * s);
        add_box(soup, corner, angle, Vec3::new(9.0, 20.0, 9.0) * s, stone);
    }
    for k in 0..4 {
        let turn = k as f32 * PI * 0.5;
        let side = b * (Quat::from_rotation_y(turn) * Vec3::new(0.0, 0.0, 34.0 * s));
        add_box(
            soup,
            p + side,
            angle + turn,
            Vec3::new(66.0, 12.0, 3.0) * s,
            stone,
        );
    }
}

/// The silhouettes of a horizon.
#[derive(Resource, Debug, Default)]
pub struct HorizonSilhouettes {
    pub villages: usize,
    pub castles: usize,
    pub triangles: usize,
    pub sites: Vec<Vec3>,
    pub smokes: Vec<SmokeSite>,
    pub smoke_color: [f32; 3],
    /// Field centre the distances are measured from.
    pub centre: Vec2,
}

/// Draws the sites and builds the mesh buffers (`snowy`: snow-white roofs).
pub fn plan(
    cfg: &SilhouetteConfig,
    province: &str,
    ground: &SiteGround,
    snowy: bool,
) -> (HorizonSilhouettes, Soup) {
    let mut rng = Rng::seeded(&format!("{province}silhouettes"));
    let stone = Vec3::from_array(cfg.stone);
    let roof = if snowy {
        Vec3::new(0.85, 0.87, 0.9)
    } else {
        Vec3::from_array(cfg.roof)
    };
    let mut soup = Soup::default();
    let mut out = HorizonSilhouettes {
        smoke_color: cfg.smoke,
        centre: ground.centre(),
        ..default()
    };
    let span = (cfg.min_distance, cfg.max_distance);
    for _ in 0..rng.int(cfg.villages.0, cfg.villages.1) {
        let Some(p) = ground.pick_site(&mut rng, span, false) else {
            continue;
        };
        out.sites.push(p);
        add_church(&mut soup, p, rng.randf() * TAU, cfg.scale, stone, roof);
        for _ in 0..rng.int(4, 8) {
            let off = Vec2::from_angle(rng.randf() * TAU) * rng.range(25.0, 90.0);
            let q = Vec3::new(
                p.x + off.x,
                ground.height(p.x + off.x, p.z + off.y),
                p.z + off.y,
            );
            let angle = rng.randf() * TAU;
            let s = cfg.scale * rng.range(0.8, 1.2);
            add_house(&mut soup, q, angle, s, lightened(stone, 0.15), roof);
        }
        out.villages += 1;
    }
    for _ in 0..rng.int(cfg.castles.0, cfg.castles.1) {
        let Some(p) = ground.pick_site(&mut rng, span, true) else {
            continue;
        };
        add_castle(&mut soup, p, rng.randf() * TAU, cfg.scale, stone);
        out.sites.push(p);
        out.castles += 1;
    }
    if !out.sites.is_empty() {
        for _ in 0..rng.int(cfg.smokes.0, cfg.smokes.1) {
            let p = out.sites[(rng.next_u64() % out.sites.len() as u64) as usize];
            let jitter = Vec3::new(rng.range(-40.0, 40.0), 5.0, rng.range(-40.0, 40.0));
            out.smokes.push(SmokeSite {
                position: p + jitter,
                seed: rng.randf() * 10.0,
            });
        }
    }
    out.triangles = soup.positions.len() / 3;
    (out, soup)
}

pub struct HorizonSilhouettesPlugin;

impl Plugin for HorizonSilhouettesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            build_silhouettes
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<HorizonStats>)
                .run_if(not(resource_exists::<HorizonSilhouettes>)),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<HorizonSilhouettes>();
        });
    }
}

fn build_silhouettes(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    look: Res<DecorLook>,
    horizon: Res<HorizonStats>,
    relief: Option<Res<RealRelief>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let json: serde_json::Value = paths::find_root()
        .ok()
        .and_then(|root| std::fs::read(paths::data_dir(&root).join("fx/horizon.json")).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let Some(cfg) = SilhouetteConfig::from_horizon(&json) else {
        commands.insert_resource(HorizonSilhouettes::default());
        return;
    };
    let sim = session.sim_mut();
    let snowy = matches!(sim.weather(), sim_battle::Weather::Snow)
        || matches!(sim.field().ground, sim_battle::Ground::Snowy);
    let ground = SiteGround {
        field: sim.field(),
        look: &look,
        relief: relief.as_deref(),
    };
    let (silhouettes, soup) = plan(&cfg, &horizon.province, &ground, snowy);
    if silhouettes.triangles > 0 {
        commands.spawn((
            Name::new("Silhouettes"),
            Mesh3d(meshes.add(soup.into_mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.95,
                ..default()
            })),
            NotShadowCaster,
            DespawnOnExit(AppState::Battle),
        ));
    }
    commands.insert_resource(silhouettes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SilhouetteConfig {
        let root = paths::find_root().unwrap();
        let bytes = std::fs::read(paths::data_dir(&root).join("fx/horizon.json")).unwrap();
        SilhouetteConfig::from_horizon(&serde_json::from_slice(&bytes).unwrap()).unwrap()
    }

    #[test]
    fn config_reads_the_data_file() {
        let c = config();
        assert_eq!(c.villages, (2, 5));
        assert_eq!(c.castles, (0, 1));
        assert_eq!(c.smokes, (1, 4));
        assert!((c.scale - 1.3).abs() < 1e-6);
        assert!(SilhouetteConfig::from_horizon(&serde_json::json!({})).is_none());
    }

    #[test]
    fn rng_is_deterministic_and_bounded() {
        let mut a = Rng::seeded("normandysilhouettes");
        let mut b = Rng::seeded("normandysilhouettes");
        for _ in 0..100 {
            let (x, y) = (a.randf(), b.randf());
            assert_eq!(x, y);
            assert!((0.0..1.0).contains(&x));
        }
        let mut r = Rng::seeded("x");
        assert!((0..500).all(|_| (2..=5).contains(&r.int(2, 5))));
    }

    #[test]
    fn box_faces_look_outwards() {
        let mut soup = Soup::default();
        add_box(
            &mut soup,
            Vec3::ZERO,
            0.0,
            Vec3::new(10.0, 5.0, 6.0),
            Vec3::ONE,
        );
        let mesh = soup.into_mesh();
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|a| a.as_float3())
            .unwrap()
            .to_vec();
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap()
            .to_vec();
        // Every triangle's normal points away from the box axis (walls) or up (top).
        for (t, n) in positions.chunks(3).zip(normals.chunks(3)) {
            let centre =
                (Vec3::from_array(t[0]) + Vec3::from_array(t[1]) + Vec3::from_array(t[2])) / 3.0;
            let out = Vec3::new(centre.x, 2.5, centre.z);
            let n = Vec3::from_array(n[0]);
            assert!(n.dot(centre - out) > -1e-4, "{centre:?} {n:?}");
        }
        assert!(normals.iter().any(|n| n[1] > 0.99), "a top face looks up");
    }

    #[test]
    fn a_village_has_a_church_and_houses() {
        let mut soup = Soup::default();
        add_church(
            &mut soup,
            Vec3::ZERO,
            0.3,
            1.3,
            Vec3::splat(0.4),
            Vec3::splat(0.3),
        );
        // church: 2 boxes (10 tris each) + gable (6) + spire (6)
        assert_eq!(soup.positions.len() / 3, 10 + 6 + 10 + 6);
        add_house(
            &mut soup,
            Vec3::ZERO,
            0.0,
            1.0,
            Vec3::splat(0.5),
            Vec3::splat(0.3),
        );
        assert_eq!(soup.positions.len() / 3, 32 + 10 + 6);
    }

    #[test]
    fn castle_has_keep_towers_and_curtain_walls() {
        let mut soup = Soup::default();
        add_castle(&mut soup, Vec3::ZERO, 0.0, 1.3, Vec3::splat(0.4));
        assert_eq!(soup.positions.len() / 3, 9 * 10);
    }
}
