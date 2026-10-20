//! Groves and inhabited clearings (lot TS-T8, ADR 0710).
//!
//! - **Groves**: compact groups of a few trees on the meadows, a few per grid cell, species drawn
//!   from the local `isolated` role of the biome (one dominant species per grove). They replace the
//!   scattered hedgerow and bocage trees in the medium view (see [`super::look::OpenThinning`]).
//!   The `grove_*` Meshy models of the MX lot are not used: they are single 20 m clump meshes
//!   without an impostor row.
//! - **Clearings**: holes in the heart of the big forests, one per grid cell at most, outline
//!   waved by two harmonics. The sites are a pure function of the grid cell, so tiles agree on
//!   them; they are exposed through [`ForestClearings`], where `clearing_life.rs` (TS-T7c)
//!   places a life object (charcoal kiln, timber yard) in each.
//!
//! Everything is deterministic (hash of the cell) and runs on the scatter worker of the tile.

use bevy::prelude::*;
use vegetation::species::ROLE_ISOLATED;

use super::data::{VegetationWorld, style};
use super::look::{ClearingLook, GroveLook, Role};
use super::tiles::{TILE_PX, TreeInst};

/// Hash in [0, 1) of a cell and a salt.
pub fn hash01(x: i64, y: i64, salt: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn lerp(range: (f32, f32), t: f32) -> f32 {
    range.0 + (range.1 - range.0) * t
}

/// A clearing in a big forest.
#[derive(Clone, Debug, PartialEq)]
pub struct ClearingSite {
    pub x: f32,
    pub z: f32,
    /// Mean radius (map px); the outline waves around it.
    pub radius: f32,
    /// Index into `ClearingLook::kinds`.
    pub kind: usize,
    pub cell: (i64, i64),
    phase: (f32, f32),
    wobble: f32,
}

impl ClearingSite {
    #[cfg(test)]
    pub fn for_test(x: f32, z: f32, kind: usize, cell: (i64, i64)) -> ClearingSite {
        ClearingSite {
            x,
            z,
            radius: 4.0,
            kind,
            cell,
            phase: (0.0, 0.0),
            wobble: 0.25,
        }
    }

    /// Largest distance from the centre of the outline.
    pub fn reach(&self) -> f32 {
        self.radius * (1.0 + self.wobble)
    }

    /// Whether (x, z) is inside the wavy outline.
    pub fn contains(&self, x: f32, z: f32) -> bool {
        let (dx, dz) = (x - self.x, z - self.z);
        let distance = (dx * dx + dz * dz).sqrt();
        if distance >= self.reach() {
            return false;
        }
        let angle = dz.atan2(dx);
        let wave =
            0.6 * (3.0 * angle + self.phase.0).sin() + 0.4 * (5.0 * angle + self.phase.1).sin();
        distance < self.radius * (1.0 + self.wobble * wave)
    }
}

/// Published positions of the clearings of the loaded tiles (key: grid cell).
#[derive(Resource, Default, Debug)]
pub struct ForestClearings {
    pub sites: std::collections::BTreeMap<(i64, i64), ClearingSite>,
}

/// Candidate site of a clearing cell, before the forest check.
fn clearing_candidate(look: &ClearingLook, cx: i64, cy: i64) -> Option<ClearingSite> {
    if hash01(cx, cy, 11) >= look.per_cell {
        return None;
    }
    let cell = look.cell_px.max(1.0);
    Some(ClearingSite {
        x: (cx as f32 + 0.2 + 0.6 * hash01(cx, cy, 12)) * cell,
        z: (cy as f32 + 0.2 + 0.6 * hash01(cx, cy, 13)) * cell,
        radius: lerp(look.radius_px, hash01(cx, cy, 14)),
        kind: (hash01(cx, cy, 15) * look.kinds.len() as f32) as usize % look.kinds.len().max(1),
        cell: (cx, cy),
        phase: (
            hash01(cx, cy, 16) * std::f32::consts::TAU,
            hash01(cx, cy, 17) * std::f32::consts::TAU,
        ),
        wobble: look.wobble,
    })
}

/// Clearings whose outline touches the rectangle `[low, high]` (map px), inside massifs only.
pub fn clearing_sites(
    world: &VegetationWorld,
    low: (f32, f32),
    high: (f32, f32),
) -> Vec<ClearingSite> {
    let look = &world.look.clearings;
    let cell = look.cell_px.max(1.0);
    let margin = look.radius_px.1 * (1.0 + look.wobble);
    let range =
        |a: f32, b: f32| ((a - margin) / cell).floor() as i64..=((b + margin) / cell).ceil() as i64;
    let mut out = Vec::new();
    for cy in range(low.1, high.1) {
        for cx in range(low.0, high.0) {
            let Some(site) = clearing_candidate(look, cx, cy) else {
                continue;
            };
            if clearing_in_forest(world, &site, look.min_forest) {
                out.push(site);
            }
        }
    }
    out
}

/// The centre and a ring of points around the outline are all deep in the forest.
fn clearing_in_forest(world: &VegetationWorld, site: &ClearingSite, min_forest: f32) -> bool {
    let at = |x: f32, z: f32| world.sample(x as f64, z as f64).forest;
    if at(site.x, site.z) < min_forest {
        return false;
    }
    let ring = site.reach() * 1.3;
    (0..6).all(|k| {
        let a = k as f32 * std::f32::consts::TAU / 6.0;
        at(site.x + ring * a.cos(), site.z + ring * a.sin()) >= min_forest * 0.8
    })
}

/// Removes the forest trees standing in a clearing; returns the number removed.
pub fn open_clearings(trees: &mut Vec<TreeInst>, sites: &[ClearingSite]) -> usize {
    if sites.is_empty() {
        return 0;
    }
    let before = trees.len();
    trees.retain(|t| t.role != Role::Forest || !sites.iter().any(|s| s.contains(t.x, t.z)));
    before - trees.len()
}

/// Whether a grove cell holds a grove: its centre (map px) and the dice, before the ground check.
fn grove_candidate(look: &GroveLook, cx: i64, cy: i64) -> Option<(f32, f32)> {
    if hash01(cx, cy, 1) >= look.per_cell {
        return None;
    }
    let cell = look.cell_px.max(1.0);
    Some((
        (cx as f32 + 0.15 + 0.7 * hash01(cx, cy, 2)) * cell,
        (cy as f32 + 0.15 + 0.7 * hash01(cx, cy, 3)) * cell,
    ))
}

/// The groves whose centre lies in the tile at `origin`. `exclusions`: settlement discs
/// `(x, y, r)` near the tile.
pub fn place_groves(
    world: &VegetationWorld,
    origin: (f64, f64),
    exclusions: &[(f64, f64, f64)],
) -> Vec<TreeInst> {
    let look: &GroveLook = &world.look.groves;
    let table = &world.species;
    let cell = look.cell_px.max(1.0) as f64;
    let mut out = Vec::new();
    let mut groves = 0u32;
    let (c0x, c0y) = (
        (origin.0 / cell).floor() as i64,
        (origin.1 / cell).floor() as i64,
    );
    let (c1x, c1y) = (
        ((origin.0 + TILE_PX) / cell).ceil() as i64,
        ((origin.1 + TILE_PX) / cell).ceil() as i64,
    );
    for cy in c0y..=c1y {
        for cx in c0x..=c1x {
            let Some((x, z)) = grove_candidate(look, cx, cy) else {
                continue;
            };
            let (xd, zd) = (x as f64, z as f64);
            if xd < origin.0
                || xd >= origin.0 + TILE_PX
                || zd < origin.1
                || zd >= origin.1 + TILE_PX
            {
                continue;
            }
            if groves >= look.max_per_tile {
                break;
            }
            let sample = world.sample(xd, zd);
            let raw = world.splat_forest.texel(world.size, xd, zd, 0);
            if sample.forest > 0.0
                || raw > world.cover.0
                || sample.crops < look.min_crops
                || exclusions.iter().any(|&(ex, ey, r)| {
                    (ex - xd).powi(2) + (ey - zd).powi(2) < (r + look.radius_px.1 as f64).powi(2)
                })
            {
                continue;
            }
            let biome = world.biome_at(xd, zd);
            let altitude = world.heights.height_m_at(x, z) as f64;
            let radius = lerp(look.radius_px, hash01(cx, cy, 4));
            let count = look.trees.0
                + (hash01(cx, cy, 5) * (look.trees.1 + 1 - look.trees.0) as f32) as u32;
            // One dominant species per grove, a few strangers.
            let grove_roll = hash01(cx, cy, 6) as f64;
            let mut planted = false;
            for k in 0..count as i64 {
                let u = |salt: u64| hash01(cx * 64 + k, cy, 100 + salt);
                let species_roll = if u(1) < 0.75 { grove_roll } else { u(2) as f64 };
                let Some(sp) = table.pick(
                    ROLE_ISOLATED,
                    biome,
                    altitude,
                    1000.0,
                    sample.conifer as f64,
                    species_roll,
                    None,
                ) else {
                    continue;
                };
                // Denser at the middle: radius ~ sqrt(u) squeezed towards the centre.
                let r = radius * u(3).powf(0.7);
                let angle = u(4) * std::f32::consts::TAU;
                let h_raw = lerp((table.height[2 * sp], table.height[2 * sp + 1]), u(5));
                let w_raw = lerp((table.width[2 * sp], table.width[2 * sp + 1]), u(6));
                let scale = style::scale() as f32 * 0.9 * (0.9 + 0.2 * u(7));
                let b = 0.85 + 0.3 * u(8);
                out.push(TreeInst {
                    x: x + r * angle.cos(),
                    z: z + r * angle.sin(),
                    y: 0.0,
                    yaw_deg: u(9) * 360.0,
                    height: h_raw * scale,
                    crown: h_raw * w_raw * scale,
                    row: sp as u16,
                    seed: u(10),
                    tint: [b * (0.93 + 0.12 * u(11)), b, b * (0.92 + 0.14 * u(12))],
                    role: Role::Grove,
                });
                planted = true;
            }
            groves += u32::from(planted);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> ClearingSite {
        clearing_candidate(
            &ClearingLook {
                per_cell: 1.0,
                ..ClearingLook::default()
            },
            3,
            4,
        )
        .expect("site")
    }

    #[test]
    fn clearing_outline_is_a_wavy_disc_around_the_centre() {
        let s = site();
        assert!(s.contains(s.x, s.z));
        assert!(!s.contains(s.x + s.reach() + 0.1, s.z));
        assert!(!s.contains(s.x, s.z - s.reach() - 0.1));
        // Wobble: the radius really changes with the angle.
        let radii: Vec<f32> = (0..16)
            .map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                (0..400)
                    .map(|i| i as f32 * 0.02)
                    .take_while(|r| s.contains(s.x + r * a.cos(), s.z + r * a.sin()))
                    .count() as f32
                    * 0.02
            })
            .collect();
        let (lo, hi) = radii
            .iter()
            .fold((f32::MAX, 0.0f32), |(l, h), r| (l.min(*r), h.max(*r)));
        assert!(hi - lo > 0.2 * s.radius * 0.5, "{radii:?}");
        assert!(lo > 0.3 * s.radius && hi <= s.reach() + 0.05);
    }

    #[test]
    fn clearing_cells_are_deterministic_and_sparse() {
        let look = ClearingLook::default();
        let a: Vec<_> = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter_map(|(x, y)| clearing_candidate(&look, x, y))
            .collect();
        let b: Vec<_> = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter_map(|(x, y)| clearing_candidate(&look, x, y))
            .collect();
        assert_eq!(a, b);
        let share = a.len() as f32 / 1600.0;
        assert!((share - look.per_cell).abs() < 0.06, "{share}");
        assert!(a.iter().all(
            |s| (look.radius_px.0..=look.radius_px.1).contains(&s.radius)
                && s.kind < look.kinds.len()
        ));
    }

    #[test]
    fn open_clearings_removes_forest_trees_only() {
        let s = site();
        let tree = |x: f32, z: f32, role: Role| TreeInst {
            x,
            z,
            y: 0.0,
            yaw_deg: 0.0,
            height: 1.0,
            crown: 1.0,
            row: 0,
            seed: 0.5,
            tint: [1.0; 3],
            role,
        };
        let mut trees = vec![
            tree(s.x, s.z, Role::Forest),
            tree(s.x + 0.1, s.z, Role::Open),
            tree(s.x + 0.2, s.z, Role::Grove),
            tree(s.x + 3.0 * s.reach(), s.z, Role::Forest),
        ];
        assert_eq!(open_clearings(&mut trees, std::slice::from_ref(&s)), 1);
        assert_eq!(trees.len(), 3);
        assert!(
            trees
                .iter()
                .all(|t| t.role != Role::Forest || t.x > s.x + s.reach())
        );
    }

    #[test]
    fn grove_cells_are_a_share_of_the_grid() {
        let look = GroveLook::default();
        let n = (0..50)
            .flat_map(|y| (0..50).map(move |x| (x, y)))
            .filter(|&(x, y)| grove_candidate(&look, x, y).is_some())
            .count();
        let share = n as f32 / 2500.0;
        assert!((share - look.per_cell).abs() < 0.05, "{share}");
        // The centre stays inside its cell.
        let (x, z) = (0..50)
            .find_map(|x| grove_candidate(&look, x, 7))
            .expect("cell");
        assert!((z / look.cell_px).floor() as i64 == 7 && x > 0.0);
    }

    /// Real map (flat relief): groves and clearings land where they should. Skipped when the
    /// rasters are not available.
    #[test]
    fn groves_and_clearings_on_the_real_map() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let Ok(meta) = data_model::MapMeta::load(&root.join("data/map")) else {
            return;
        };
        let (w, h) = (meta.size_px[0] as usize, meta.size_px[1] as usize);
        let heights = std::sync::Arc::new(crate::campaign::terrain::relief_floor::HeightGrid {
            width: w,
            height: h,
            data: vec![9000; w * h],
            h_min_m: 0.0,
            h_range_m: 400.0,
        });
        let Ok(world) = super::super::data::load(&root, heights) else {
            return;
        };
        // Orleans plain and forest, a few tiles around.
        let (mut groves, mut meadow_trees, mut clearing_sites) = (0usize, 0usize, 0usize);
        let tiles = TILE_PX;
        for ty in 0..(h as f64 / tiles).ceil() as i64 {
            for tx in 0..(w as f64 / tiles).ceil() as i64 {
                if (tx + ty) % 7 != 0 {
                    continue;
                }
                let origin = (tx as f64 * tiles, ty as f64 * tiles);
                let trees = place_groves(&world, origin, &[]);
                assert!(
                    trees.len()
                        <= (world.look.groves.max_per_tile * world.look.groves.trees.1) as usize
                );
                groves += trees.len();
                meadow_trees += trees
                    .iter()
                    .filter(|t| {
                        t.role == Role::Grove && world.sample(t.x as f64, t.z as f64).forest <= 0.05
                    })
                    .count();
                clearing_sites += clearing_sites_in(&world, origin);
            }
        }
        eprintln!("groves {groves} meadow {meadow_trees} clearings {clearing_sites}");
        assert!(groves > 0, "no grove placed");
        assert!(
            meadow_trees as f32 > 0.9 * groves as f32,
            "{meadow_trees}/{groves}"
        );
        assert!(clearing_sites > 0, "no clearing");
    }

    fn clearing_sites_in(world: &VegetationWorld, origin: (f64, f64)) -> usize {
        clearing_sites(
            world,
            (origin.0 as f32, origin.1 as f32),
            ((origin.0 + TILE_PX) as f32, (origin.1 + TILE_PX) as f32),
        )
        .len()
    }
}
