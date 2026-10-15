//! Visible growth of the 1:1 towns (ADR 0162 point 4): the 1340 plan stays frozen (ADR 0138)
//! and this layer adds on top of it
//! - **suburb quarters** along the gate roads, beyond the 1340 suburbs, when the simulated
//!   population of the province exceeds its 1337 population (`growth.suburbs`);
//! - an **enclosure** (palisade, then stone walls, towers, gates) when a fortification building
//!   is built during the game while the town had neither that enclosure in its plan nor that
//!   building in 1337 (`growth.enclosure`).
//!
//! Pure functions returning instances in local metres from the anchor. Port of `town_growth.gd`.
//! Rendering only: no game rule.

use bevy::math::{Vec2, Vec3};

use super::data::{GrowthConfig, Town, WallSpec};
use super::kit::Kit;
use super::plan::radius_at;
use super::rng::Rng;

pub const KINDS: [&str; 3] = ["none", "palisade", "stone"];

/// What an instance is drawn with.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// Kit glTF by manifest name.
    Kit(String),
    /// Unit box scaled to `scale`, masonry or planks.
    Box { masonry: bool },
    /// Unit cylinder tower (radius x height scale).
    Tower,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub piece: Piece,
    /// Local metres from the anchor.
    pub pos: Vec2,
    /// Rotation about +Y (radians, Bevy convention).
    pub rot_y: f32,
    pub scale: Vec3,
    /// Vertical lift (m) of the base (walls are sunk into the ground).
    pub lift: f32,
    /// Growth family: `suburb` or `enclosure`.
    pub family: &'static str,
    pub tint: f32,
}

/// Number of suburb quarters added for a population / 1337 population ratio.
pub fn quarter_count(cfg: &GrowthConfig, ratio: f32) -> u32 {
    let step = cfg.population_step.max(1e-3);
    (((ratio - 1.0) / step + 1e-6).floor().max(0.0) as u32).min(cfg.max_quarters)
}

/// Enclosure rank (0 none, 1 palisade, 2 stone) given by built buildings.
pub fn enclosure_rank(cfg: &GrowthConfig, buildings: &[String]) -> usize {
    let mut best = 0;
    for (kind, ids) in &cfg.enclosure_kinds {
        if ids.iter().any(|id| buildings.contains(id)) {
            best = best.max(KINDS.iter().position(|k| k == kind).unwrap_or(0));
        }
    }
    best
}

/// Enclosure to add (`None`: nothing): the one of the built buildings when it exceeds both the
/// 1340 plan's enclosure and the starting buildings' (fortification went up).
pub fn enclosure_to_add(
    cfg: &GrowthConfig,
    buildings: &[String],
    initial_buildings: &[String],
    plan_walls: &str,
) -> Option<&'static str> {
    let wanted = enclosure_rank(cfg, buildings);
    let plan_rank = KINDS.iter().position(|k| *k == plan_walls).unwrap_or(0);
    let had = enclosure_rank(cfg, initial_buildings).max(plan_rank);
    (wanted > had).then_some(KINDS[wanted])
}

fn hash3(a: u32, b: u32, c: u32) -> u32 {
    let mut r = Rng::new(
        a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35),
    );
    (r.next_u64() >> 16) as u32
}

fn frac(h: u32, shift: u32) -> f32 {
    ((h >> shift) % 100) as f32 / 100.0
}

/// Kit houses of the added quarters, in two rows along the road of each gate.
pub fn suburb_instances(
    cfg: &GrowthConfig,
    town: &Town,
    kit: &Kit,
    quarters: u32,
) -> Vec<Instance> {
    let mut out = Vec::new();
    if quarters == 0 || cfg.house_models.is_empty() || town.radii.is_empty() {
        return out;
    }
    let seed = town
        .id
        .bytes()
        .fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b)));
    let mut bearings: Vec<f32> = town.gates.iter().map(|g| g.bearing).collect();
    if bearings.is_empty() {
        bearings = (0..3)
            .map(|k| ((seed % 360) as f32 + 120.0 * k as f32).rem_euclid(360.0))
            .collect();
    }
    for q in 0..quarters {
        let bearing = bearings[q as usize % bearings.len()];
        let ring = q as usize / bearings.len();
        let angle = bearing.to_radians();
        let dir = Vec2::new(angle.cos(), angle.sin());
        let side = Vec2::new(-dir.y, dir.x);
        let mut start = radius_at(&town.radii, angle);
        for fb in &town.faubourgs {
            if ((fb.bearing - bearing + 180.0).rem_euclid(360.0) - 180.0).abs() < 20.0 {
                start = start.max(fb.start_m + fb.length_m);
            }
        }
        let length = (cfg.houses_per_quarter as f32 / 2.0).ceil() * cfg.frontage_m;
        start += cfg.gap_m + ring as f32 * (length + cfg.gap_m);
        for k in 0..cfg.houses_per_quarter {
            let h = hash3(seed, q, k);
            let model = &cfg.house_models[h as usize % cfg.house_models.len()];
            let row = if k % 2 == 0 { 1.0 } else { -1.0 };
            let depth = kit.get(model).map_or(7.0, |e| e.depth);
            let along = start + ((k / 2) as f32 + 0.5) * cfg.frontage_m + frac(h, 8) * 3.0;
            let local =
                dir * along + side * row * (cfg.setback_m + depth * 0.5 + frac(h, 16) * 4.0);
            let facing = -side * row;
            out.push(Instance {
                piece: Piece::Kit(model.clone()),
                pos: local,
                rot_y: facing.x.atan2(facing.y) + (frac(h, 4) - 0.5) * 0.25,
                scale: Vec3::ONE,
                lift: 0.0,
                family: "suburb",
                tint: 0.35 + frac(h, 12) * 0.3,
            });
        }
    }
    out
}

/// Instances of enclosure `kind` (`palisade` or `stone`): one section per bearing of the radii,
/// `margin_m` outside the built area, towers at the corners (stone) and gates on the roads.
pub fn enclosure_instances(
    cfg: &GrowthConfig,
    walls: &WallSpec,
    gate_depth: f32,
    town: &Town,
    kind: &str,
) -> Vec<Instance> {
    let mut out = Vec::new();
    let n = town.radii.len();
    if n == 0 {
        return out;
    }
    let stone = kind == "stone";
    let margin = cfg.enclosure_margin_m;
    let sink = cfg.enclosure_sink_m;
    let thick = walls.thickness_m;
    let wall_h = walls.height_m;
    let tau = std::f32::consts::TAU;
    let points: Vec<Vec2> = (0..n)
        .map(|k| {
            let a = tau * k as f32 / n as f32;
            Vec2::new(a.cos(), a.sin()) * (town.radii[k] + margin)
        })
        .collect();
    let mut perimeter = 0.0;
    for k in 0..n {
        let (a, b) = (points[k], points[(k + 1) % n]);
        perimeter += a.distance(b);
        let d = b - a;
        out.push(Instance {
            piece: Piece::Box { masonry: stone },
            pos: (a + b) * 0.5,
            rot_y: (-d.y).atan2(d.x),
            scale: Vec3::new(d.length() + thick, wall_h + sink, thick),
            lift: -sink,
            family: "enclosure",
            tint: 0.5,
        });
    }
    if stone && walls.tower_spacing_m > 0.0 {
        let every =
            ((walls.tower_spacing_m / (perimeter / n as f32).max(1.0)).round() as usize).max(1);
        for k in (0..n).step_by(every) {
            out.push(Instance {
                piece: Piece::Tower,
                pos: points[k],
                rot_y: 0.0,
                scale: Vec3::new(
                    walls.tower_radius_m,
                    walls.tower_height_m + sink,
                    walls.tower_radius_m,
                ),
                lift: -sink,
                family: "enclosure",
                tint: 0.5,
            });
        }
    }
    for g in &town.gates {
        let angle = g.bearing.to_radians();
        let dir = Vec2::new(angle.cos(), angle.sin());
        let gate_h = if stone { cfg.gate_height_m } else { 7.0 };
        out.push(Instance {
            piece: Piece::Box { masonry: stone },
            pos: dir * (radius_at(&town.radii, angle) + margin),
            rot_y: (-dir.y).atan2(dir.x),
            scale: Vec3::new(gate_depth, gate_h + sink, 11.0),
            lift: -sink,
            family: "enclosure",
            tint: 0.5,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::towns::data::TownDb;

    fn db() -> TownDb {
        let root = crate::paths::find_root().unwrap();
        TownDb::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn quarter_count_follows_ratio() {
        let cfg = GrowthConfig::default();
        assert_eq!(quarter_count(&cfg, 1.0), 0);
        assert_eq!(quarter_count(&cfg, 0.8), 0);
        assert_eq!(quarter_count(&cfg, 1.08), 1);
        assert_eq!(quarter_count(&cfg, 1.5), 6);
        assert_eq!(quarter_count(&cfg, 9.0), 6);
    }

    #[test]
    fn enclosure_rises_only_with_new_fortification() {
        let db = db();
        let cfg = &db.growth;
        let none: Vec<String> = vec![];
        let pal = vec!["bld_palisade".to_string()];
        let stone = vec!["bld_stone_walls".to_string()];
        assert_eq!(enclosure_to_add(cfg, &none, &none, "none"), None);
        assert_eq!(enclosure_to_add(cfg, &pal, &none, "none"), Some("palisade"));
        assert_eq!(enclosure_to_add(cfg, &stone, &pal, "none"), Some("stone"));
        assert_eq!(
            enclosure_to_add(cfg, &stone, &none, "stone"),
            None,
            "already in the plan"
        );
        assert_eq!(
            enclosure_to_add(cfg, &stone, &stone, "none"),
            None,
            "built in 1337"
        );
    }

    #[test]
    fn suburbs_and_enclosure_have_instances() {
        let db = db();
        let kit = Kit::load(&crate::paths::asset_root(
            &crate::paths::find_root().unwrap(),
        ));
        let town = &db.towns["set_abbeville"];
        let sub = suburb_instances(&db.growth, town, &kit, 3);
        assert_eq!(sub.len(), 3 * db.growth.houses_per_quarter as usize);
        assert!(
            sub.iter()
                .all(|i| matches!(&i.piece, Piece::Kit(m) if kit.get(m).is_some()))
        );
        let enc = enclosure_instances(
            &db.growth,
            &db.wall_spec("stone"),
            db.gate_depth_m,
            town,
            "stone",
        );
        assert!(enc.iter().any(|i| i.piece == Piece::Tower));
        assert!(enc.len() > town.radii.len());
        let pal = enclosure_instances(
            &db.growth,
            &db.wall_spec("palisade"),
            db.gate_depth_m,
            town,
            "palisade",
        );
        assert!(!pal.iter().any(|i| i.piece == Piece::Tower));
    }
}
