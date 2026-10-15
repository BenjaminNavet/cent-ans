//! Decor of the wetlands: reed clumps and ponds (port of the patch level of
//! `game/scripts/map/freshwater_layer.gd`, `freshwater_reeds.gdshader`, `freshwater_pool.gdshader`;
//! data `data/map/freshwater_px.json` and `data/map/map_freshwater.json`).
//!
//! Blocks of `patch_block_units` map units are seeded around the camera focus (deterministic per
//! block: ellipses of the wetland sites, `cover` x `density`, no reed in the clearings of the
//! places nor on water), seated on the displayed ground and drawn as two instanced batches (reeds,
//! ponds) by `billboard.rs`. A second, near level (blocks of 2 units, a clump every 7 cm, under
//! 4 units of camera distance) fades in/out with the patch level by dithered opacity.
//! Purely visual.

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::Value;

use super::billboard::{Mode, Params, push_record};
use super::data::Plane;
use super::mask;

/// Meters per world unit (`FreshwaterLayer.METERS_PER_UNIT`).
pub const METERS_PER_UNIT: f32 = 719.0;
pub const CARD_TEXTURE: &str = "textures/vegetation/dn_cards/card_reed_phragmites.png";

/// A wetland ellipse.
#[derive(Clone, Debug)]
pub struct Site {
    pub id: String,
    pub kind: String,
    pub density: f32,
    pub lagoon: bool,
    pub center: Vec2,
    pub radii: Vec2,
    pub angle_deg: f32,
}

#[derive(Clone, Debug)]
pub struct Kind {
    pub cover: f32,
    pub pool_share: f32,
    pub tint: [f32; 3],
}

/// The two levels of blocks (`LEVEL_NEAR`, `LEVEL_PATCH`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Near = 0,
    Patch = 1,
}

/// Decor settings and sites.
#[derive(Clone, Debug, Default)]
pub struct Wetlands {
    pub sites: Vec<Site>,
    pub kinds: HashMap<String, Kind>,
    pub block_units: f32,
    pub spacing: f32,
    pub patch_max_distance: f32,
    /// Near level (`near_*`): small blocks seeded every few centimetres under the camera.
    pub near_block_units: f32,
    pub near_spacing: f32,
    pub near_max_distance: f32,
    pub max_instances: usize,
    pub far_scale: f32,
    pub pool_far_scale: f32,
    pub scale_near: f32,
    pub scale_far: f32,
    pub seed: u64,
    pub clump_width_m: f32,
    pub clump_height_m: f32,
    pub pool_diameter_m: f32,
}

fn num(v: &Value, default: f64) -> f32 {
    v.as_f64().unwrap_or(default) as f32
}

impl Wetlands {
    pub fn load(map_dir: &Path) -> Wetlands {
        let read = |name: &str| -> Value {
            std::fs::read_to_string(map_dir.join(name))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or(Value::Null)
        };
        let config = read("map_freshwater.json");
        let sites_json = read("freshwater_px.json");
        let render = &config["render"];
        let reeds = &config["reeds"];
        let mut out = Wetlands {
            block_units: num(&render["patch_block_units"], 16.0),
            spacing: num(&render["patch_spacing_units"], 0.8),
            patch_max_distance: num(&render["patch_max_distance"], 120.0),
            near_block_units: num(&render["near_block_units"], 2.0),
            near_spacing: num(&render["near_spacing_units"], 0.07),
            near_max_distance: num(&render["near_max_distance"], 4.0),
            max_instances: render["max_instances"].as_u64().unwrap_or(60_000) as usize,
            far_scale: num(&render["far_scale"], 36.0),
            pool_far_scale: num(&render["pool_far_scale"], 2.0),
            scale_near: num(&render["scale_near_distance"], 3.0),
            scale_far: num(&render["scale_far_distance"], 60.0),
            seed: render["seed"].as_u64().unwrap_or(1),
            clump_width_m: num(&reeds["clump_width_m"], 7.0),
            clump_height_m: num(&reeds["clump_height_m"], 2.6),
            pool_diameter_m: num(&reeds["pool_diameter_m"], 60.0),
            ..Default::default()
        };
        if let Some(kinds) = reeds["kinds"].as_object() {
            for (name, k) in kinds {
                let tint = k["tint"]
                    .as_array()
                    .filter(|a| a.len() >= 3)
                    .map_or([0.55, 0.55, 0.35], |a| {
                        [num(&a[0], 0.5), num(&a[1], 0.5), num(&a[2], 0.3)]
                    });
                out.kinds.insert(
                    name.clone(),
                    Kind {
                        cover: num(&k["cover"], 0.3),
                        pool_share: num(&k["pool_share"], 0.1),
                        tint,
                    },
                );
            }
        }
        if let Some(sites) = sites_json["wetlands"].as_array() {
            for s in sites {
                let pair = |v: &Value| match v.as_array() {
                    Some(a) if a.len() >= 2 => Vec2::new(num(&a[0], 0.0), num(&a[1], 0.0)),
                    _ => Vec2::ZERO,
                };
                out.sites.push(Site {
                    id: s["id"].as_str().unwrap_or("").to_string(),
                    kind: s["kind"].as_str().unwrap_or("").to_string(),
                    density: num(&s["density"], 0.5),
                    lagoon: s["lagoon"].as_bool().unwrap_or(false),
                    center: pair(&s["center"]),
                    radii: pair(&s["radii_px"]),
                    angle_deg: num(&s["angle_deg"], 0.0),
                });
            }
        }
        out
    }

    /// Block side of a level (`block_units`).
    pub fn block_of(&self, level: Level) -> f32 {
        match level {
            Level::Near => self.near_block_units,
            Level::Patch => self.block_units,
        }
    }

    /// Seeding step of a level (`_spacing`).
    pub fn spacing_of(&self, level: Level) -> f32 {
        match level {
            Level::Near => self.near_spacing,
            Level::Patch => self.spacing,
        }
    }

    /// Opacity of a level at camera distance `d` (`FreshwaterLayer.update_view`): the near level
    /// fades out as the camera rises, the patch level fades in, then out towards its maximum.
    pub fn alpha_of(&self, level: Level, d: f32) -> f32 {
        let near_max = self.near_max_distance;
        match level {
            Level::Near => 1.0 - mask::smoothstep(near_max, near_max * 1.7, d),
            Level::Patch => {
                mask::smoothstep(near_max * 0.8, near_max * 1.4, d)
                    * (1.0
                        - mask::smoothstep(
                            self.patch_max_distance * 0.75,
                            self.patch_max_distance,
                            d,
                        ))
            }
        }
    }

    /// Radius (map units) of the blocks seeded around the focus for a level.
    pub fn radius_of(&self, level: Level, d: f32) -> f32 {
        match level {
            Level::Near => (d * 1.6).clamp(2.5, 5.0),
            Level::Patch => (d * 1.8).clamp(20.0, 140.0),
        }
    }

    /// Magnification of the clumps for the camera at distance `d` (`scale_for`).
    pub fn scale_for(&self, d: f32) -> f32 {
        let near = self.scale_near;
        let far = self.scale_far.max(near * 1.01);
        let t = ((d.max(1e-3) / near).ln() / (far / near).ln()).clamp(0.0, 1.0);
        1.0 + (self.far_scale - 1.0) * t * t * (3.0 - 2.0 * t)
    }
}

/// Normalised distance (1 on the edge) of `p` from the ellipse of `site` (`ellipse_radius`).
pub fn ellipse_radius(site: &Site, p: Vec2) -> f32 {
    let angle = site.angle_deg.to_radians();
    let d = p - site.center;
    let north = -d.y;
    let u = d.x * angle.cos() + north * angle.sin();
    let v = -d.x * angle.sin() + north * angle.cos();
    ((u / site.radii.x).powi(2) + (v / site.radii.y).powi(2)).sqrt()
}

/// Small deterministic generator (splitmix64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next_f32()
    }
}

/// A reed clump (flat ground position, ground height set at seating).
#[derive(Clone, Copy, Debug)]
pub struct Reed {
    pub x: f32,
    pub z: f32,
    pub size: f32,
    pub tint: [f32; 3],
    pub phase: f32,
}

/// A pond.
#[derive(Clone, Copy, Debug)]
pub struct Pond {
    pub x: f32,
    pub z: f32,
    /// Diameter in world units.
    pub diameter: f32,
    pub water: [f32; 3],
    pub seed: f32,
}

#[derive(Default, Clone, Debug)]
pub struct Seeded {
    pub reeds: Vec<Reed>,
    pub ponds: Vec<Pond>,
}

/// Seeds the block `key` (`seed_block`): `land` and `exclusions` are those of the world.
pub fn seed_block(
    wet: &Wetlands,
    land: &Plane,
    map_size: (f64, f64),
    exclusions: &[(f64, f64, f64)],
    level: Level,
    key: (i32, i32),
) -> Seeded {
    let size = wet.block_of(level);
    let spacing = wet.spacing_of(level);
    let low = Vec2::new(key.0 as f32, key.1 as f32) * size;
    let high = low + Vec2::splat(size);
    let local: Vec<&Site> = wet
        .sites
        .iter()
        .filter(|s| {
            let r = s.radii.x.max(s.radii.y);
            s.center.x >= low.x - r
                && s.center.x <= high.x + r
                && s.center.y >= low.y - r
                && s.center.y <= high.y + r
        })
        .collect();
    let mut out = Seeded::default();
    if local.is_empty() {
        return out;
    }
    let circles: Vec<(f32, f32, f32)> = exclusions
        .iter()
        .filter(|&&(x, y, r)| {
            x as f32 >= low.x - r as f32
                && x as f32 <= high.x + r as f32
                && y as f32 >= low.y - r as f32
                && y as f32 <= high.y + r as f32
        })
        .map(|&(x, y, r)| (x as f32, y as f32, r as f32))
        .collect();
    let mut rng = Rng::new(
        wet.seed
            ^ (level as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ ((key.0 as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            ^ ((key.1 as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)),
    );
    let cells = (size / spacing).ceil() as i32;
    for ix in 0..cells {
        for iz in 0..cells {
            let p = low
                + Vec2::new(
                    (ix as f32 + rng.next_f32()) * spacing,
                    (iz as f32 + rng.next_f32()) * spacing,
                );
            if p.x >= high.x || p.y >= high.y {
                continue;
            }
            let mut best = 0.0f32;
            let mut pick: Option<(&Site, f32)> = None;
            for site in &local {
                let r = ellipse_radius(site, p);
                if r >= 1.0 {
                    continue;
                }
                let cover = wet.kinds.get(&site.kind).map_or(0.3, |k| k.cover);
                let t = ((r - 1.0) / (0.6 - 1.0)).clamp(0.0, 1.0);
                let weight = cover * site.density * t * t * (3.0 - 2.0 * t);
                if weight > best {
                    best = weight;
                    pick = Some((site, r));
                }
            }
            let roll = rng.next_f32();
            let Some((site, pick_r)) = pick.filter(|_| roll <= best) else {
                continue;
            };
            if land.texel(map_size, p.x as f64, p.y as f64, 0) <= 0.5
                || circles
                    .iter()
                    .any(|&(cx, cy, cr)| (p - Vec2::new(cx, cy)).length_squared() < cr * cr)
            {
                continue;
            }
            let kind = wet.kinds.get(&site.kind);
            let tint = kind.map_or([0.55, 0.55, 0.35], |k| k.tint);
            let mut pool_share = kind.map_or(0.1, |k| k.pool_share);
            if site.lagoon {
                pool_share = if pick_r < 0.62 { 0.4 } else { 0.0 };
            }
            if rng.next_f32() < pool_share {
                let mut diameter = wet.pool_diameter_m * rng.range(0.6, 1.8);
                if site.lagoon {
                    diameter *= 3.0;
                }
                let water = if site.lagoon {
                    [0.30, 0.46, 0.46]
                } else {
                    [0.10, 0.17, 0.17]
                };
                out.ponds.push(Pond {
                    x: p.x,
                    z: p.y,
                    diameter: diameter / METERS_PER_UNIT,
                    water,
                    seed: rng.next_f32(),
                });
            } else if !site.lagoon || pick_r > 0.5 {
                let jitter = rng.range(0.85, 1.15);
                out.reeds.push(Reed {
                    x: p.x,
                    z: p.y,
                    size: rng.range(0.7, 1.4),
                    tint: [tint[0] * jitter, tint[1] * jitter, tint[2] * jitter],
                    phase: rng.next_f32(),
                });
            }
        }
    }
    out
}

/// Records of a seeded block on the ground (`height(x, z)`), reeds then ponds.
pub fn block_records(
    seeded: &Seeded,
    reeds: &mut Vec<u8>,
    ponds: &mut Vec<u8>,
    height: impl Fn(f32, f32) -> f32,
) {
    for r in &seeded.reeds {
        push_record(
            reeds,
            Vec3::new(r.x, height(r.x, r.z), r.z),
            r.phase * 360.0,
            r.size,
            r.phase,
            r.tint,
        );
    }
    for p in &seeded.ponds {
        push_record(
            ponds,
            Vec3::new(p.x, height(p.x, p.z) + 0.002, p.z),
            p.seed * 360.0,
            p.diameter,
            p.seed,
            p.water,
        );
    }
}

/// Two crossed 1 x 1 cards, foot at y = 0, UV.y = 1 at the foot.
pub fn reed_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for angle in [0.0f32, std::f32::consts::FRAC_PI_2] {
        let dir = Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.5;
        let base = positions.len() as u32;
        for (p, uv) in [
            (-dir, [0.0, 1.0]),
            (dir, [1.0, 1.0]),
            (dir + Vec3::Y, [1.0, 0.0]),
            (-dir + Vec3::Y, [0.0, 0.0]),
        ] {
            positions.push(p.to_array());
            uvs.push(uv);
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; positions.len()],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Flat disc of 16 sectors, diameter 1, UV centred.
pub fn pond_mesh() -> Mesh {
    let mut positions = vec![[0.0, 0.0, 0.0]];
    let mut uvs = vec![[0.5, 0.5]];
    let mut indices: Vec<u32> = Vec::new();
    for i in 0..16 {
        let a = i as f32 / 16.0 * std::f32::consts::TAU;
        positions.push([a.cos() * 0.5, 0.0, a.sin() * 0.5]);
        uvs.push([0.5 + a.cos() * 0.5, 0.5 + a.sin() * 0.5]);
        let next = (i + 1) % 16;
        indices.extend([0, 1 + i as u32, 1 + next as u32]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; positions.len()],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Parameter block of the reed / pond batches.
pub fn params(wet: &Wetlands, mode: Mode, level: Level) -> Params {
    let mut p = Params::new(mode);
    p.frame.z = 0.5;
    // The near level is not magnified (`fw_scale` 1); `range.w` is the dithered opacity.
    let (far_scale, pool_far) = match level {
        Level::Near => (1.0, 1.0),
        Level::Patch => (wet.far_scale, wet.pool_far_scale),
    };
    p.extra = Vec4::new(
        wet.clump_width_m / METERS_PER_UNIT,
        wet.clump_height_m / METERS_PER_UNIT,
        0.25,
        far_scale,
    );
    p.range = Vec4::new(wet.scale_near, wet.scale_far, pool_far, 1.0);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_crossfade_with_the_camera_distance() {
        let wet = Wetlands {
            near_max_distance: 4.0,
            patch_max_distance: 120.0,
            near_block_units: 2.0,
            near_spacing: 0.07,
            block_units: 16.0,
            spacing: 0.8,
            ..Default::default()
        };
        assert_eq!(wet.alpha_of(Level::Near, 1.0), 1.0);
        assert_eq!(wet.alpha_of(Level::Patch, 1.0), 0.0);
        assert!(wet.alpha_of(Level::Near, 5.0) > 0.0 && wet.alpha_of(Level::Near, 5.0) < 1.0);
        assert_eq!(wet.alpha_of(Level::Near, 7.0), 0.0);
        assert_eq!(wet.alpha_of(Level::Patch, 20.0), 1.0);
        assert_eq!(wet.alpha_of(Level::Patch, 120.0), 0.0);
        assert_eq!(wet.radius_of(Level::Near, 1.0), 2.5);
        assert_eq!(wet.radius_of(Level::Patch, 500.0), 140.0);
        assert_eq!(wet.block_of(Level::Near), 2.0);
        assert_eq!(wet.spacing_of(Level::Near), 0.07);
    }

    #[test]
    fn near_level_is_denser_than_patch() {
        let wet = world();
        let mut wet = wet;
        wet.near_block_units = 2.0;
        wet.near_spacing = 0.07;
        let near = seed_block(&wet, &land(), (100.0, 100.0), &[], Level::Near, (4, 4));
        // Block (4, 4) of 2 units is inside the site ellipse around (8, 8).
        assert!(near.reeds.len() + near.ponds.len() > 50);
        let p = params(&wet, Mode::Reeds, Level::Near);
        assert_eq!((p.extra.w, p.range.w), (1.0, 1.0));
    }

    fn world() -> Wetlands {
        let mut wet = Wetlands {
            block_units: 16.0,
            spacing: 0.8,
            seed: 7,
            pool_diameter_m: 60.0,
            ..Default::default()
        };
        wet.kinds.insert(
            "marsh".into(),
            Kind {
                cover: 0.8,
                pool_share: 0.12,
                tint: [0.5, 0.4, 0.2],
            },
        );
        wet.sites.push(Site {
            id: "m".into(),
            kind: "marsh".into(),
            density: 1.0,
            lagoon: false,
            center: Vec2::new(8.0, 8.0),
            radii: Vec2::new(6.0, 4.0),
            angle_deg: 0.0,
        });
        wet
    }

    fn land() -> Plane {
        Plane {
            w: 1,
            h: 1,
            ch: 1,
            data: vec![255],
        }
    }

    #[test]
    fn ellipse_radius_is_one_on_the_edge() {
        let wet = world();
        let site = &wet.sites[0];
        assert!((ellipse_radius(site, Vec2::new(14.0, 8.0)) - 1.0).abs() < 1e-5);
        assert!(ellipse_radius(site, Vec2::new(8.0, 8.0)) < 1e-5);
    }

    #[test]
    fn block_is_deterministic_and_stays_in_the_ellipse() {
        let wet = world();
        let a = seed_block(&wet, &land(), (100.0, 100.0), &[], Level::Patch, (0, 0));
        let b = seed_block(&wet, &land(), (100.0, 100.0), &[], Level::Patch, (0, 0));
        assert!(!a.reeds.is_empty());
        assert_eq!(a.reeds.len(), b.reeds.len());
        assert_eq!(a.ponds.len(), b.ponds.len());
        for r in &a.reeds {
            assert!(ellipse_radius(&wet.sites[0], Vec2::new(r.x, r.z)) < 1.0);
        }
        // A block far from every site is empty.
        assert!(
            seed_block(&wet, &land(), (1e4, 1e4), &[], Level::Patch, (50, 50))
                .reeds
                .is_empty()
        );
    }

    #[test]
    fn clearings_and_water_hold_reeds_back() {
        let wet = world();
        let free = seed_block(&wet, &land(), (100.0, 100.0), &[], Level::Patch, (0, 0));
        let cleared = seed_block(
            &wet,
            &land(),
            (100.0, 100.0),
            &[(8.0, 8.0, 100.0)],
            Level::Patch,
            (0, 0),
        );
        assert!(cleared.reeds.is_empty() && cleared.ponds.is_empty());
        let sea = Plane {
            w: 1,
            h: 1,
            ch: 1,
            data: vec![0],
        };
        assert!(
            seed_block(&wet, &sea, (100.0, 100.0), &[], Level::Patch, (0, 0))
                .reeds
                .is_empty()
        );
        assert!(free.reeds.len() > 20);
    }

    #[test]
    fn records_have_the_declared_size() {
        let seeded = Seeded {
            reeds: vec![Reed {
                x: 1.0,
                z: 2.0,
                size: 1.0,
                tint: [1.0; 3],
                phase: 0.5,
            }],
            ponds: vec![],
        };
        let (mut reeds, mut ponds) = (Vec::new(), Vec::new());
        block_records(&seeded, &mut reeds, &mut ponds, |_, _| 3.0);
        assert_eq!(reeds.len(), super::super::billboard::RECORD_BYTES);
        assert!(ponds.is_empty());
    }

    #[test]
    fn scale_grows_with_distance() {
        let mut wet = world();
        wet.scale_near = 3.0;
        wet.scale_far = 60.0;
        wet.far_scale = 36.0;
        assert_eq!(wet.scale_for(1.0), 1.0);
        assert!((wet.scale_for(1000.0) - 36.0).abs() < 1e-4);
        assert!(wet.scale_for(20.0) > 1.0);
    }

    #[test]
    fn real_data_loads() {
        let root = crate::paths::find_root().expect("repository root");
        let wet = Wetlands::load(&root.join("data/map"));
        assert!(wet.sites.len() > 50);
        assert!(wet.kinds.contains_key("marsh"));
        assert_eq!(wet.block_units, 16.0);
    }
}
