//! Sky dome and horizon of a battle (port of game/scripts/battle/battle_horizon.gd, the ring and
//! the skyline of the panorama, and the sky part of battle_atmosphere.gd).
//!
//! - Real relief: the baked tile of a province (`game/assets/horizon/relief/<province>.bin`, format
//!   `HZR1` of `cent-ans geo horizon`: header `HZR1`, `n: u16`, `p: u16`, `step: f32`, `ref_m: f32`,
//!   `raw_len: u32`, then a deflate stream of `n*n` s16 heights (0.25 m), `n*n` classes (0-200
//!   forest share, 255 sea), `p` s16 skyline angles (0.01 deg), `p` u16 distances (100 m), `p` u8
//!   sea shares). The battle carries no province yet, so the tile is chosen among the inland
//!   provinces of the battle's terrain by seed (`CENT_ANS_HORIZON_PROVINCE` forces one, and the
//!   `HorizonProvince` resource is the hook for the flow).
//! - Ring: a mesh round the field (rounded-rectangle offset of its edge, distances of
//!   `battle_decor_look.json`), height blended from the generated relief at the edge to the real
//!   relief beyond 0.7-3 km, vertex colours from the tile classes (fields, forest, rock, snow).
//!   Without a tile a noise relief and noise woods take its place.
//! - Skyline: a cylinder of far ridges following the camera, top edge on the baked skyline.
//! - Sky dome: gradient from the horizon colour (the fog colour, so the ring melts into it) to the
//!   zenith, with a glow towards the sun. Colours per weather in the look file.
//!
//! Not ported: the painted panorama (webp not enabled in the client), far sea mesh, steeple and
//! smoke silhouettes, coast alignment (only inland tiles are chosen), the `battle_horizon_land`
//! shader (per-fragment fields, haze with altitude).

use std::f32::consts::TAU;
use std::io::Read;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::{BattleSeason, Battlefield, Weather};

use super::atmosphere::AtmosphereState;
use super::look::DecorLook;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::scene::terrain::season_ground;
use crate::paths;

const SEA: u8 = 255;
const FOREST_MAX: f32 = 200.0;
/// Where the real relief starts to replace the generated one, and where it fully does (m).
const BLEND: (f32, f32) = (700.0, 3000.0);

/// Province whose tile the horizon must use (set by the flow when a battle is on a known province).
#[derive(Resource, Debug, Clone, Default)]
pub struct HorizonProvince(pub Option<String>);

/// Baked real relief round a province.
#[derive(Debug, Clone)]
pub struct RealRelief {
    pub n: usize,
    pub step: f32,
    pub half: f32,
    pub ref_m: f32,
    pub heights: Vec<f32>,
    pub classes: Vec<u8>,
    /// Skyline angle (deg) per real azimuth.
    pub skyline: Vec<f32>,
    pub skyline_dist: Vec<f32>,
    /// Field centre (battle world, xz) and rotation of the tile.
    pub centre: Vec2,
    pub rotation: f32,
    pub offset_y: f32,
}

impl RealRelief {
    /// Decodes a `HZR1` tile (without placement: `centre`, `rotation`, `offset_y` are set later).
    pub fn parse(bytes: &[u8]) -> Option<RealRelief> {
        if bytes.len() < 20 || &bytes[..4] != b"HZR1" {
            return None;
        }
        let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        let p = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let step = f32::from_le_bytes(bytes[8..12].try_into().ok()?);
        let ref_m = f32::from_le_bytes(bytes[12..16].try_into().ok()?);
        let raw_len = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
        let mut raw = Vec::with_capacity(raw_len);
        // Godot's DEFLATE is a zlib stream; accept a raw one as well.
        if flate2::read::ZlibDecoder::new(&bytes[20..])
            .read_to_end(&mut raw)
            .is_err()
        {
            raw.clear();
            flate2::read::DeflateDecoder::new(&bytes[20..])
                .read_to_end(&mut raw)
                .ok()?;
        }
        if raw.len() != n * n * 3 + p * 5 {
            return None;
        }
        let s16 = |at: usize| i16::from_le_bytes([raw[at], raw[at + 1]]);
        let heights = (0..n * n).map(|i| f32::from(s16(i * 2)) * 0.25).collect();
        let mut at = n * n * 2;
        let classes = raw[at..at + n * n].to_vec();
        at += n * n;
        let skyline = (0..p).map(|k| f32::from(s16(at + k * 2)) * 0.01).collect();
        let skyline_dist = (0..p)
            .map(|k| {
                let o = at + p * 2 + k * 2;
                f32::from(u16::from_le_bytes([raw[o], raw[o + 1]])) * 100.0
            })
            .collect();
        Some(RealRelief {
            n,
            step,
            half: (n - 1) as f32 * step * 0.5,
            ref_m,
            heights,
            classes,
            skyline,
            skyline_dist,
            centre: Vec2::ZERO,
            rotation: 0.0,
            offset_y: 0.0,
        })
    }

    /// Tile coordinates (cells) of a battle world point.
    fn sample_coords(&self, x: f32, z: f32) -> Vec2 {
        let e = x - self.centre.x;
        let nth = self.centre.y - z;
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        let real = Vec2::new(e * c - nth * s, nth * c + e * s);
        Vec2::new(
            (real.x + self.half) / self.step,
            (self.half - real.y) / self.step,
        )
    }

    /// Real altitude in battle world metres (bilinear, clamped to the tile).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let c = self.sample_coords(x, z);
        let fx = c.x.clamp(0.0, (self.n - 1) as f32);
        let fz = c.y.clamp(0.0, (self.n - 1) as f32);
        let ix = (fx as usize).min(self.n - 2);
        let iz = (fz as usize).min(self.n - 2);
        let (tx, tz) = (fx - ix as f32, fz - iz as f32);
        let i = iz * self.n + ix;
        let top = self.heights[i] + (self.heights[i + 1] - self.heights[i]) * tx;
        let bottom = self.heights[i + self.n]
            + (self.heights[i + self.n + 1] - self.heights[i + self.n]) * tx;
        top + (bottom - top) * tz + self.offset_y
    }

    /// Class at the nearest cell: 0-200 forest share, 255 sea.
    pub fn class_at(&self, x: f32, z: f32) -> u8 {
        let c = self.sample_coords(x, z);
        let ix = (c.x.round().max(0.0) as usize).min(self.n - 1);
        let iz = (c.y.round().max(0.0) as usize).min(self.n - 1);
        self.classes[iz * self.n + ix]
    }

    /// Skyline angle (deg) and distance (m) towards a world bearing (turns, 0 = north).
    pub fn skyline_at(&self, bearing_turns: f32) -> (f32, f32) {
        let p = self.skyline.len() as f32;
        let u = (bearing_turns - self.rotation / TAU).rem_euclid(1.0) * p;
        let (i, t) = (u.floor() as usize % self.skyline.len(), u.fract());
        let j = (i + 1) % self.skyline.len();
        (
            self.skyline[i] + (self.skyline[j] - self.skyline[i]) * t,
            self.skyline_dist[i] + (self.skyline_dist[j] - self.skyline_dist[i]) * t,
        )
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn value_noise(x: f32, z: f32) -> f32 {
    let (ix, iz) = (x.floor(), z.floor());
    let (fx, fz) = (x - ix, z - iz);
    let (ux, uz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let h = |a: f32, b: f32| super::hash01(a, b, 9.0);
    let top = h(ix, iz) + (h(ix + 1.0, iz) - h(ix, iz)) * ux;
    let bottom = h(ix, iz + 1.0) + (h(ix + 1.0, iz + 1.0) - h(ix, iz + 1.0)) * ux;
    top + (bottom - top) * uz
}

/// Colours of the real relief from `data/fx/horizon.json` (`relief.colors`, `snow_line_m`).
#[derive(Debug, Clone)]
pub struct HorizonColors {
    pub field: [f32; 3],
    pub meadow: [f32; 3],
    pub forest: [f32; 3],
    pub rock: [f32; 3],
    pub snow: [f32; 3],
    pub snow_line: f32,
}

impl HorizonColors {
    pub fn load(data_dir: &std::path::Path, season: BattleSeason) -> HorizonColors {
        let json: serde_json::Value = std::fs::read(data_dir.join("fx/horizon.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let colors = &json["relief"]["colors"];
        let rgb = |key: &str, default: [f32; 3]| {
            let v = &colors[key];
            if v.is_array() {
                [0, 1, 2].map(|i| v[i].as_f64().map_or(default[i], |x| x as f32))
            } else {
                default
            }
        };
        let mut field = rgb("field", [0.34, 0.37, 0.21]);
        let mut forest = rgb("forest", [0.1, 0.16, 0.08]);
        let season_key = match season {
            BattleSeason::Spring => "spring",
            BattleSeason::Summer => "summer",
            BattleSeason::Autumn => "autumn",
            BattleSeason::Winter => "winter",
        };
        match season {
            BattleSeason::Winter => field = rgb("winter_field", field),
            BattleSeason::Autumn => forest = lerp3(forest, rgb("autumn_forest", forest), 0.5),
            _ => {}
        }
        HorizonColors {
            meadow: rgb("meadow", field),
            rock: rgb("rock", [0.42, 0.4, 0.37]),
            snow: rgb("snow", [0.92, 0.94, 0.97]),
            snow_line: json["relief"]["snow_line_m"][season_key]
                .as_f64()
                .unwrap_or(2500.0) as f32,
            field,
            forest,
        }
    }
}

/// Buffers of the ring mesh.
#[derive(Debug, Default)]
pub struct RingMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub rings: usize,
    pub sectors: usize,
}

/// Generated height of the field extended flat beyond its edge.
fn edge_height(field: &Battlefield, x: f32, z: f32) -> f32 {
    field.height(
        f64::from(x.clamp(0.0, field.width as f32)),
        f64::from(z.clamp(0.0, field.depth as f32)),
    ) as f32
}

fn field_distance(field: &Battlefield, x: f32, z: f32) -> f32 {
    let dx = (-x).max(x - field.width as f32).max(0.0);
    let dz = (-z).max(z - field.depth as f32).max(0.0);
    dx.hypot(dz)
}

/// Ring of the horizon round the field. `relief`: real relief (else a noise relief when the look
/// allows it); `snowy`: snow on the ground.
pub fn build_ring(
    field: &Battlefield,
    look: &DecorLook,
    relief: Option<&RealRelief>,
    colors: &HorizonColors,
    ground: [f32; 3],
    snowy: bool,
) -> RingMesh {
    let h = &look.horizon;
    let sectors = h.sectors.max(16);
    // Inner ring (hidden under the field edge) then the distances of the look.
    let mut distances = vec![-6.0f32];
    distances.extend(
        h.distances_m
            .iter()
            .copied()
            .filter(|d| *d <= h.ring_radius_m),
    );
    let rings = distances.len();
    let (w, d) = (field.width as f32, field.depth as f32);
    let (hw, hd) = (w * 0.5, d * 0.5);
    let corner = hw.min(hd) * 0.5;
    let mut out = RingMesh {
        rings,
        sectors,
        ..default()
    };
    let mut heights = vec![0.0f32; rings * sectors];
    let mut points = vec![Vec2::ZERO; rings * sectors];
    for k in 0..sectors {
        let theta = TAU * k as f32 / sectors as f32;
        let dir = Vec2::new(theta.cos(), theta.sin());
        let t = (hw / dir.x.abs().max(1e-6)).min(hd / dir.y.abs().max(1e-6));
        let b = dir * t;
        let inner = Vec2::new(
            b.x.clamp(-(hw - corner), hw - corner),
            b.y.clamp(-(hd - corner), hd - corner),
        );
        let normal = (b - inner).normalize_or_zero();
        for (i, dist) in distances.iter().enumerate() {
            let p = Vec2::new(hw, hd) + b + normal * *dist;
            let generated = edge_height(field, p.x, p.y);
            let weight = smoothstep(BLEND.0, BLEND.1, field_distance(field, p.x, p.y));
            let mut height = match relief {
                Some(r) => generated + (r.height(p.x, p.y) - generated) * weight,
                None if h.fallback_ring.enabled => {
                    let s = h.fallback_ring.noise_scale_m;
                    let n = value_noise(p.x / s, p.y / s) * 0.65
                        + value_noise(p.x / (s * 0.31), p.y / (s * 0.31)) * 0.35;
                    generated + (n - 0.35) * h.fallback_ring.relief_m * 2.0 * weight
                }
                None => generated,
            };
            if i == 0 {
                height -= 0.8;
            }
            heights[i * sectors + k] = height;
            points[i * sectors + k] = p;
        }
    }
    for i in 0..rings {
        for k in 0..sectors {
            let at = i * sectors + k;
            let p = points[at];
            out.positions.push([p.x, heights[at], p.y]);
        }
    }
    // Normals from the neighbours of the grid.
    for i in 0..rings {
        for k in 0..sectors {
            let pos = |i: usize, k: usize| {
                let at = i * sectors + k;
                Vec3::new(points[at].x, heights[at], points[at].y)
            };
            let along = pos(i, (k + 1) % sectors) - pos(i, (k + sectors - 1) % sectors);
            let across = pos((i + 1).min(rings - 1), k) - pos(i.saturating_sub(1), k);
            let mut n = along.cross(across).normalize_or_zero();
            if n.y < 0.0 {
                n = -n;
            }
            if n == Vec3::ZERO {
                n = Vec3::Y;
            }
            out.normals.push(n.to_array());
        }
    }
    // Colours.
    for (i, ring_distance) in distances.iter().enumerate() {
        for k in 0..sectors {
            let at = i * sectors + k;
            let (p, y) = (points[at], heights[at]);
            let slope = 1.0 - out.normals[at][1].clamp(0.0, 1.0);
            let patch = value_noise(p.x / 900.0, p.y / 900.0);
            let mut col = lerp3(colors.field, colors.meadow, patch);
            let (forest, sea) = match relief {
                Some(r) => {
                    let class = r.class_at(p.x, p.y);
                    if class == SEA {
                        (0.0, true)
                    } else {
                        (f32::from(class) / FOREST_MAX, false)
                    }
                }
                None => (
                    smoothstep(
                        1.0 - h.fallback_ring.forest_share,
                        1.0,
                        value_noise(p.x / 1400.0 + 11.0, p.y / 1400.0),
                    ),
                    false,
                ),
            };
            col = lerp3(col, colors.forest, forest.clamp(0.0, 1.0));
            col = lerp3(col, colors.rock, smoothstep(0.18, 0.4, slope));
            let snow = smoothstep(colors.snow_line, colors.snow_line + 250.0, y);
            col = lerp3(col, colors.snow, snow.max(if snowy { 0.8 } else { 0.0 }));
            if sea {
                col = h.sea_color;
            }
            let near = smoothstep(0.0, h.colors_mix_with_field_m, ring_distance.max(0.0));
            col = lerp3(
                [ground[0] * 0.9, ground[1] * 0.9, ground[2] * 0.9],
                col,
                near,
            );
            let lin = Color::srgb(col[0], col[1], col[2]).to_linear();
            out.colors.push([lin.red, lin.green, lin.blue, 1.0]);
        }
    }
    for i in 0..rings - 1 {
        for k in 0..sectors {
            let k1 = (k + 1) % sectors;
            let (a, b) = ((i * sectors + k) as u32, (i * sectors + k1) as u32);
            let (c, dd) = (
                ((i + 1) * sectors + k) as u32,
                ((i + 1) * sectors + k1) as u32,
            );
            out.indices.extend_from_slice(&[a, b, c, b, dd, c]);
        }
    }
    out
}

impl RingMesh {
    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Inland province whose tile fits the battle terrain, drawn by `seed`.
pub fn choose_province(index: &serde_json::Value, terrain_key: &str, seed: u64) -> Option<String> {
    let provinces = index["provinces"].as_object()?;
    let mut keys: Vec<&String> = provinces
        .iter()
        .filter(|(_, v)| v["coastal"].as_bool() == Some(false) && v["terrain"] == terrain_key)
        .map(|(k, _)| k)
        .collect();
    if keys.is_empty() {
        keys = provinces
            .iter()
            .filter(|(_, v)| v["coastal"].as_bool() == Some(false) && v["terrain"] == "plains")
            .map(|(k, _)| k)
            .collect();
    }
    keys.sort();
    if keys.is_empty() {
        return None;
    }
    Some(keys[(seed as usize) % keys.len()].clone())
}

/// What the horizon is (read by the dump and by the fog).
#[derive(Resource, Debug, Clone, Default)]
pub struct HorizonStats {
    pub active: bool,
    pub province: String,
    pub offset_y: f32,
    pub rotation_deg: f32,
    pub ring_vertices: usize,
    pub ring_triangles: usize,
    pub ring_max_height: f32,
    pub skyline_segments: usize,
    pub sky_vertices: usize,
    pub real_relief: bool,
}

#[derive(Component)]
pub struct SkyDome;
#[derive(Component)]
pub struct Skyline;
#[derive(Resource)]
struct SkyMesh(Handle<Mesh>);

pub struct HorizonPlugin;

impl Plugin for HorizonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HorizonProvince>()
            .add_systems(
                Update,
                build_horizon.run_if(resource_added::<BattleSession>),
            )
            .add_systems(
                Update,
                (spawn_sky, update_sky, follow_camera)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<HorizonStats>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<HorizonStats>();
                commands.remove_resource::<SkyMesh>();
            });
    }
}

fn load_relief(
    root: &std::path::Path,
    forced: Option<String>,
    terrain_key: &str,
    seed: u64,
) -> Option<(String, RealRelief)> {
    let dir = paths::asset_root(root).join("horizon/relief");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("index.json")).ok()?).ok()?;
    let province = forced
        .or_else(|| std::env::var("CENT_ANS_HORIZON_PROVINCE").ok())
        .or_else(|| choose_province(&index, terrain_key, seed))?;
    let file = index["provinces"][&province]["file"].as_str()?;
    let relief = RealRelief::parse(&std::fs::read(dir.join(file)).ok()?)?;
    Some((province, relief))
}

fn build_horizon(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    look: Res<DecorLook>,
    forced: Res<HorizonProvince>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let seed = session.seed;
    let sim = session.sim_mut();
    let field = sim.field();
    if !look.horizon.enabled {
        return;
    }
    let root = paths::find_root().ok();
    let data_dir = root.as_deref().map(paths::data_dir).unwrap_or_default();
    let colors = HorizonColors::load(&data_dir, field.season);
    let mean = field.heights.iter().sum::<f64>() as f32 / field.heights.len().max(1) as f32;
    let mut loaded = root
        .as_deref()
        .and_then(|r| load_relief(r, forced.0.clone(), field.terrain.key(), seed));
    let rotation = (super::hash01(seed as f32, 3.0, 5.0)) * TAU;
    if let Some((_, relief)) = loaded.as_mut() {
        relief.centre = Vec2::new(field.width as f32 * 0.5, field.depth as f32 * 0.5);
        relief.rotation = rotation;
        relief.offset_y = mean - relief.ref_m;
    }
    let weather = sim.weather();
    let snowy =
        matches!(weather, Weather::Snow) || matches!(field.ground, sim_battle::Ground::Snowy);
    let ring = build_ring(
        field,
        &look,
        loaded.as_ref().map(|(_, r)| r),
        &colors,
        season_ground(field.season),
        snowy,
    );
    let mut stats = HorizonStats {
        active: true,
        province: loaded.as_ref().map(|(p, _)| p.clone()).unwrap_or_default(),
        offset_y: loaded.as_ref().map_or(0.0, |(_, r)| r.offset_y),
        rotation_deg: rotation.to_degrees(),
        ring_vertices: ring.positions.len(),
        ring_triangles: ring.indices.len() / 3,
        ring_max_height: ring.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max),
        real_relief: loaded.is_some(),
        ..default()
    };
    let scope = DespawnOnExit(AppState::Battle);
    commands.spawn((
        Name::new("HorizonRing"),
        Mesh3d(meshes.add(ring.into_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            reflectance: 0.04,
            ..default()
        })),
        NotShadowCaster,
        scope.clone(),
    ));
    if let Some((_, relief)) = &loaded {
        let (skyline, segments) = build_skyline(relief, &look, weather);
        stats.skyline_segments = segments;
        commands.spawn((
            Name::new("Skyline"),
            Skyline,
            Mesh3d(meshes.add(skyline)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                fog_enabled: false,
                cull_mode: None,
                double_sided: true,
                ..default()
            })),
            Transform::from_xyz(field.width as f32 * 0.5, 0.0, field.depth as f32 * 0.5),
            NotShadowCaster,
            scope,
        ));
    }
    commands.insert_resource(stats);
}

/// Cylinder of far ridges: top edge on the baked skyline (exaggerated and bounded), hazed.
fn build_skyline(relief: &RealRelief, look: &DecorLook, weather: Weather) -> (Mesh, usize) {
    let h = &look.horizon;
    let segments = 192usize;
    let radius = h.skyline_radius_m;
    let ref_y = relief.offset_y + relief.ref_m.max(0.0) + 25.0;
    let haze = DecorLook::weather_value(&h.skyline_haze, weather);
    let fog = look.fog_of(weather).color;
    let ridge = lerp3(h.skyline_color, fog, haze);
    let low = lerp3(ridge, fog, 0.55);
    let to_linear = |c: [f32; 3]| {
        let l = Color::srgb(c[0], c[1], c[2]).to_linear();
        [l.red, l.green, l.blue, 1.0]
    };
    let (ridge, low) = (to_linear(ridge), to_linear(low));
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for s in 0..=segments {
        let bearing = s as f32 / segments as f32;
        let a = bearing * TAU;
        let (angle, _) = relief.skyline_at(bearing);
        let crest = (angle * h.skyline_exaggeration).clamp(h.skyline_min_deg, h.skyline_max_deg);
        let top = ref_y + radius * crest.to_radians().tan();
        let (x, z) = (a.sin() * radius, -a.cos() * radius);
        positions.push([x, ref_y - radius * 0.2, z]);
        positions.push([x, top, z]);
        positions.push([x, top + 1.0, z]);
        colors.push(low);
        colors.push(ridge);
        colors.push(ridge);
        if s < segments {
            let i = (s * 3) as u32;
            indices.extend_from_slice(&[i, i + 3, i + 1, i + 1, i + 3, i + 4]);
        }
    }
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    (mesh, segments)
}

/// Vertex colours of the sky dome for a sun direction (towards the sun).
fn sky_vertex_colour(look: &DecorLook, weather: Weather, dir: Vec3, to_sun: Vec3) -> [f32; 4] {
    let sky = look.sky_of(weather);
    let elevation = dir.y.clamp(-1.0, 1.0).asin().to_degrees();
    let t = (elevation / 70.0).clamp(0.0, 1.0).powf(0.55);
    let mut col = lerp3(sky.horizon, sky.zenith, t * t * (3.0 - 2.0 * t));
    let angle = dir.dot(to_sun).clamp(-1.0, 1.0).acos().to_degrees();
    let glow = (-(angle / look.sky.glow_width_deg).powi(2)).exp()
        * look.sky.glow_strength
        * (1.0 - (elevation / look.sky.horizon_band_deg.max(1.0) * 0.25).clamp(0.0, 0.7));
    col = lerp3(col, sky.glow, glow.clamp(0.0, 1.0));
    let l = Color::srgb(col[0], col[1], col[2]).to_linear();
    [l.red, l.green, l.blue, 1.0]
}

fn dome_mesh(look: &DecorLook, weather: Weather, to_sun: Vec3) -> Mesh {
    let radius = look.sky.dome_radius_m;
    let (rows, columns) = (24usize, 48usize);
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for r in 0..=rows {
        // Elevation from -20 to +90 degrees.
        let e = (-20.0 + 110.0 * r as f32 / rows as f32).to_radians();
        for c in 0..=columns {
            let a = TAU * c as f32 / columns as f32;
            let dir = Vec3::new(e.cos() * a.sin(), e.sin(), -e.cos() * a.cos());
            positions.push((dir * radius).to_array());
            colors.push(sky_vertex_colour(look, weather, dir, to_sun));
        }
    }
    for r in 0..rows {
        for c in 0..columns {
            let i = (r * (columns + 1) + c) as u32;
            let n = (columns + 1) as u32;
            indices.extend_from_slice(&[i, i + 1, i + n, i + 1, i + n + 1, i + n]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn weather_of(state: &AtmosphereState) -> Weather {
    match state.weather.as_str() {
        "rain" => Weather::Rain,
        "fog" => Weather::Fog,
        "snow" => Weather::Snow,
        _ => Weather::Clear,
    }
}

fn spawn_sky(
    mut commands: Commands,
    state: Option<Res<AtmosphereState>>,
    look: Res<DecorLook>,
    existing: Option<Res<SkyMesh>>,
    mut stats: ResMut<HorizonStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(state) = state else { return };
    if existing.is_some() {
        return;
    }
    let to_sun = -Vec3::from_array(state.sun_direction);
    let mesh = dome_mesh(&look, weather_of(&state), to_sun);
    stats.sky_vertices = mesh.count_vertices();
    let handle = meshes.add(mesh);
    commands.insert_resource(SkyMesh(handle.clone()));
    commands.spawn((
        Name::new("SkyDome"),
        SkyDome,
        Mesh3d(handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            fog_enabled: false,
            cull_mode: None,
            double_sided: true,
            ..default()
        })),
        Transform::default(),
        NotShadowCaster,
        DespawnOnExit(AppState::Battle),
    ));
}

/// The glow follows the sun when the phase of the day changes.
fn update_sky(
    state: Option<Res<AtmosphereState>>,
    look: Res<DecorLook>,
    sky: Option<Res<SkyMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut last_key: Local<String>,
) {
    let (Some(state), Some(sky)) = (state, sky) else {
        return;
    };
    if state.time_key == *last_key {
        return;
    }
    let first = last_key.is_empty();
    *last_key = state.time_key.clone();
    if first || state.sun_direction == [0.0; 3] {
        return;
    }
    if let Some(mut mesh) = meshes.get_mut(&sky.0) {
        *mesh = dome_mesh(
            &look,
            weather_of(&state),
            -Vec3::from_array(state.sun_direction),
        );
    }
}

fn follow_camera(
    cameras: Query<&GlobalTransform, With<BattleCamera>>,
    mut domes: Query<&mut Transform, (With<SkyDome>, Without<Skyline>)>,
    mut skylines: Query<&mut Transform, (With<Skyline>, Without<SkyDome>)>,
) {
    let Ok(camera) = cameras.single() else { return };
    let p = camera.translation();
    for mut t in &mut domes {
        t.translation = p;
    }
    for mut t in &mut skylines {
        t.translation = Vec3::new(p.x, 0.0, p.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::scene::setup;
    use data_model::GameData;

    fn field() -> sim_battle::BattleSim {
        let root = paths::find_root().unwrap();
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        setup::build_sim(&game, &dir, &custom, 7).unwrap()
    }

    #[test]
    fn a_baked_tile_decodes_and_is_sampled() {
        let root = paths::find_root().unwrap();
        let dir = paths::asset_root(&root).join("horizon/relief");
        let index: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("index.json")).unwrap()).unwrap();
        let province = choose_province(&index, "plains", 3).unwrap();
        let file = index["provinces"][&province]["file"].as_str().unwrap();
        let mut relief = RealRelief::parse(&std::fs::read(dir.join(file)).unwrap()).unwrap();
        assert_eq!(relief.heights.len(), relief.n * relief.n);
        relief.centre = Vec2::new(600.0, 400.0);
        let h = relief.height(600.0, 400.0);
        assert!((h - relief.ref_m).abs() < 2000.0);
        assert!(relief.skyline_at(0.3).0.is_finite());
    }

    #[test]
    fn province_choice_is_deterministic_and_inland() {
        let root = paths::find_root().unwrap();
        let index: serde_json::Value = serde_json::from_slice(
            &std::fs::read(paths::asset_root(&root).join("horizon/relief/index.json")).unwrap(),
        )
        .unwrap();
        let a = choose_province(&index, "hills", 5);
        assert_eq!(a, choose_province(&index, "hills", 5));
        assert_eq!(index["provinces"][a.unwrap()]["coastal"], false);
    }

    #[test]
    fn ring_surrounds_the_field_up_to_the_radius() {
        let sim = field();
        let f = sim.field();
        let look = DecorLook::load();
        let colors = HorizonColors::load(&paths::data_dir(&paths::find_root().unwrap()), f.season);
        let ring = build_ring(f, &look, None, &colors, [0.4, 0.45, 0.2], false);
        assert_eq!(ring.positions.len(), ring.rings * ring.sectors);
        assert_eq!(ring.positions.len(), ring.colors.len());
        let far = ring
            .positions
            .iter()
            .map(|p| (p[0] - f.width as f32 * 0.5).hypot(p[2] - f.depth as f32 * 0.5))
            .fold(0.0, f32::max);
        assert!(far > look.horizon.ring_radius_m * 0.9, "ring reaches {far}");
        assert!(
            ring.indices
                .iter()
                .all(|i| (*i as usize) < ring.positions.len())
        );
        assert!(ring.normals.iter().all(|n| n[1] > 0.0));
    }

    #[test]
    fn sky_is_bluer_overhead_than_at_the_horizon() {
        let look = DecorLook::load();
        let to_sun = Vec3::new(0.0, 0.6, 0.8).normalize();
        let up = sky_vertex_colour(&look, Weather::Clear, Vec3::Y, to_sun);
        let low = sky_vertex_colour(
            &look,
            Weather::Clear,
            Vec3::new(1.0, 0.0, -0.2).normalize(),
            to_sun,
        );
        assert!(up[2] > up[0], "zenith is blue");
        assert!(low[0] + low[1] > up[0] + up[1], "horizon is paler");
    }
}
