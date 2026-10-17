//! Deterministic placement of the countryside props by 96 px cell (`CountrysideLayer.cell_instances`
//! and its `_scatter`, `_paddocks`, `_villages`, `_roadside`). Pure: the map is read through the
//! [`Terrain`] trait so that tests run on a synthetic map.
//!
//! The random stream is a splitmix64 seeded by (cell, rule), not Godot's PCG: the placement is
//! deterministic and follows the same rules and densities, but is not identical draw for draw.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use super::config::{Config, Mode, Rule};

/// A village or hamlet (cell index of the Godot `_village_grid`).
#[derive(Debug, Clone)]
pub struct Village {
    pub px: Vec2,
    pub kind: String,
}

/// A road segment (a polyline piece) with its stable id.
#[derive(Debug, Clone, Copy)]
pub struct RoadSeg {
    pub a: Vec2,
    pub b: Vec2,
    pub main: bool,
    pub id: u32,
}

/// What the placement reads of the map. Coordinates are map pixels.
pub trait Terrain {
    fn size(&self) -> Vec2;
    fn is_land(&self, p: Vec2) -> bool;
    /// River bed signed distance in pixels (>= 8 far from rivers, < 0.3 in the bed).
    fn river_sd(&self, p: Vec2) -> f32;
    fn in_lake(&self, p: Vec2) -> bool;
    fn height_m(&self, p: Vec2) -> f32;
    /// Splat weights: r prairie, g crops, b forest, a heath.
    fn splat(&self, p: Vec2) -> [f32; 4];
    /// Wetlands: marsh, ponds.
    fn wet(&self, p: Vec2) -> [f32; 2];
    /// Biome class (0..255), -1 without raster.
    fn biome(&self, p: Vec2) -> i32;
    fn biome_allows(&self, mask: u32, biome: i32) -> bool;
    /// Value of the agrarian landscape mask, -1 without mask.
    fn landscape_row(&self, p: Vec2) -> i32;
    /// Row of a landscape id (`-2` unknown).
    fn landscape_id_row(&self, id: &str) -> i32;
    fn near_town(&self, p: Vec2, clearance: f32) -> bool;
    fn villages_in(&self, cell: IVec2) -> &[Village];
    fn roads_in(&self, cell: IVec2) -> &[RoadSeg];
}

/// One placed prop.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub prop: usize,
    pub rule: usize,
    pub pos: Vec2,
    /// Heading (Godot convention: yaw of the model X axis in the XZ plane, `atan2(-dy, dx)`).
    pub yaw: f32,
    /// Segment length in metres (0 = native size), for stretched fences.
    pub len_m: f32,
    /// Offset to the centre of the group (px), scaled by the spread with the camera distance.
    pub offset: Vec2,
    /// Fixed random number [0, 1): season presence and thinning threshold.
    pub rank: f32,
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / 16_777_216.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

fn seed_of(parts: &[i64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        h ^= *p as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
        h ^= h >> 29;
    }
    h
}

fn text_hash(text: &str) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    (h & 0xffff) as i64
}

pub fn cell_rect(cell: IVec2, side: f32) -> Rect {
    let min = cell.as_vec2() * side;
    Rect::from_corners(min, min + Vec2::splat(side))
}

pub fn cell_of(p: Vec2, side: f32) -> IVec2 {
    (p / side).floor().as_ivec2()
}

fn count_for(expected: f32, rng: &mut Rng) -> usize {
    expected as usize + usize::from(rng.f32() < expected - expected.floor())
}

fn area_km2(rect: Rect, mpp: f32) -> f32 {
    rect.width() * rect.height() * mpp * mpp / 1.0e6
}

fn region_touches(rule: &Rule, rect: Rect) -> bool {
    rule.is_global
        || rule.regions.iter().any(|e| {
            let (lo, hi) = e.bounds();
            lo.x < rect.max.x && hi.x > rect.min.x && lo.y < rect.max.y && hi.y > rect.min.y
        })
}

fn in_regions(rule: &Rule, p: Vec2) -> bool {
    rule.is_global || rule.regions.iter().any(|e| e.contains(p))
}

/// Rectangles to sow: the cell (global rule) or its intersections with each region's bounds.
fn sample_rects(rule: &Rule, rect: Rect) -> Vec<Rect> {
    if rule.is_global {
        return vec![rect];
    }
    rule.regions
        .iter()
        .filter_map(|e| {
            let (lo, hi) = e.bounds();
            let part = Rect::from_corners(lo.max(rect.min), hi.min(rect.max));
            (part.width() > 0.0 && part.height() > 0.0).then_some(part)
        })
        .collect()
}

fn pick_prop(rule: &Rule, rng: &mut Rng) -> usize {
    let mut roll = rng.f32() * rule.total_weight;
    for &(prop, weight) in &rule.props {
        roll -= weight;
        if roll <= 0.0 {
            return prop;
        }
    }
    rule.props[0].0
}

fn landscape_ok(t: &dyn Terrain, p: Vec2, rule: &Rule) -> bool {
    let f = &rule.filters;
    if f.landscapes.is_empty() && f.exclude_landscapes.is_empty() {
        return true;
    }
    let row = t.landscape_row(p);
    if row < 0 {
        return f.landscapes.is_empty();
    }
    if f.exclude_landscapes
        .iter()
        .any(|id| t.landscape_id_row(id) == row)
    {
        return false;
    }
    f.landscapes.is_empty() || f.landscapes.iter().any(|id| t.landscape_id_row(id) == row)
}

/// On land, out of the river bed and lakes (the base test of every mode).
fn plain_ok(t: &dyn Terrain, p: Vec2) -> bool {
    t.is_land(p) && t.river_sd(p) >= 0.3 && !t.in_lake(p)
}

/// Does the site suit the rule? Land, out of river and lake, rule filters.
pub fn site_ok(t: &dyn Terrain, mpp: f32, p: Vec2, rule: &Rule) -> bool {
    let size = t.size();
    if p.x < 1.0 || p.y < 1.0 || p.x >= size.x - 1.0 || p.y >= size.y - 1.0 {
        return false;
    }
    if !plain_ok(t, p) {
        return false;
    }
    let f = &rule.filters;
    let h = t.height_m(p);
    if h < f.alt_min_m || h > f.alt_max_m {
        return false;
    }
    if f.slope_max_deg < 89.0 {
        let gx = t.height_m(p + Vec2::X) - t.height_m(p - Vec2::X);
        let gy = t.height_m(p + Vec2::Y) - t.height_m(p - Vec2::Y);
        if (Vec2::new(gx, gy).length() / (2.0 * mpp))
            .atan()
            .to_degrees()
            > f.slope_max_deg
        {
            return false;
        }
    }
    if !f.wet_ok {
        let wet = t.wet(p);
        if wet[0] > f.marsh_max || wet[1] > 0.4 {
            return false;
        }
    }
    let s = t.splat(p);
    if s[0] < f.prairie_min
        || s[0] + s[1] + s[3] * 0.5 < f.open_min
        || s[2] > f.forest_max
        || s[1] > f.crops_max
    {
        return false;
    }
    if !landscape_ok(t, p, rule) {
        return false;
    }
    if f.biome_mask != 0 && !t.biome_allows(f.biome_mask, t.biome(p)) {
        return false;
    }
    if f.coast_px > 0 {
        let r = f.coast_px as f32;
        let touches_sea = (0..8).any(|k| {
            let a = TAU * k as f32 / 8.0;
            !t.is_land(p + Vec2::new(a.cos(), a.sin()) * r)
        });
        if !touches_sea {
            return false;
        }
    }
    true
}

/// Instances of one cell, all rules, deterministic.
pub fn cell_instances(t: &dyn Terrain, cfg: &Config, cell: IVec2) -> Vec<Instance> {
    let mut out = Vec::new();
    let side = cfg.render.cell_px;
    let rect = cell_rect(cell, side);
    let clearance = cfg.render.town_clearance_px;
    for (index, rule) in cfg.rules.iter().enumerate() {
        if !region_touches(rule, rect) {
            continue;
        }
        let mut rng = Rng::new(seed_of(&[
            cell.x as i64,
            cell.y as i64,
            text_hash(&rule.id),
            index as i64,
        ]));
        match &rule.mode {
            Mode::Scatter => scatter(t, cfg, rule, index, rect, clearance, &mut rng, &mut out),
            Mode::Paddock {
                size_m,
                segment_m,
                gap,
            } => paddocks(
                t, cfg, rule, index, rect, clearance, *size_m, *segment_m, *gap, &mut rng, &mut out,
            ),
            Mode::Village {
                kinds,
                chance,
                offset_m,
            } => villages(
                t, cfg, rule, index, cell, kinds, *chance, *offset_m, &mut rng, &mut out,
            ),
            Mode::Road {
                types: _,
                per_100km,
                offset_m,
                parallel,
            } => roadside(
                t, cfg, rule, index, cell, clearance, *per_100km, *offset_m, *parallel, &mut out,
            ),
        }
    }
    out
}

fn scatter(
    t: &dyn Terrain,
    cfg: &Config,
    rule: &Rule,
    index: usize,
    rect: Rect,
    clearance: f32,
    rng: &mut Rng,
    out: &mut Vec<Instance>,
) {
    for part in sample_rects(rule, rect) {
        let expected = rule.density_per_1000km2 * area_km2(part, cfg.meters_per_px) / 1000.0;
        for _ in 0..count_for(expected, rng) {
            let p = part.min + Vec2::new(rng.f32(), rng.f32()) * part.size();
            let prop = pick_prop(rule, rng);
            let yaw = rng.f32() * TAU;
            let rank = rng.f32();
            if !in_regions(rule, p)
                || t.near_town(p, clearance)
                || !site_ok(t, cfg.meters_per_px, p, rule)
            {
                continue;
            }
            out.push(Instance {
                prop,
                rule: index,
                pos: p,
                yaw,
                len_m: 0.0,
                offset: Vec2::ZERO,
                rank,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paddocks(
    t: &dyn Terrain,
    cfg: &Config,
    rule: &Rule,
    index: usize,
    rect: Rect,
    clearance: f32,
    size_m: [f32; 2],
    segment_m: f32,
    gap: f32,
    rng: &mut Rng,
    out: &mut Vec<Instance>,
) {
    let mpp = cfg.meters_per_px;
    let mut centres = Vec::new();
    for part in sample_rects(rule, rect) {
        let expected = rule.density_per_1000km2 * area_km2(part, mpp) / 1000.0;
        for _ in 0..count_for(expected, rng) {
            centres.push(part.min + Vec2::new(rng.f32(), rng.f32()) * part.size());
        }
    }
    for centre in centres {
        let prop = pick_prop(rule, rng);
        let width = rng.range(size_m[0], size_m[1]) / mpp;
        let height = rng.range(size_m[0], size_m[1]) / mpp;
        let angle = rng.f32() * PI;
        let rank = rng.f32();
        let gate_side = (rng.next_u64() % 4) as usize;
        if !in_regions(rule, centre)
            || t.near_town(centre, clearance)
            || !site_ok(t, mpp, centre, rule)
        {
            continue;
        }
        let u = Vec2::new(angle.cos(), angle.sin());
        let v = Vec2::new(-u.y, u.x);
        let corners = [
            centre - u * width * 0.5 - v * height * 0.5,
            centre + u * width * 0.5 - v * height * 0.5,
            centre + u * width * 0.5 + v * height * 0.5,
            centre - u * width * 0.5 + v * height * 0.5,
        ];
        for side in 0..4 {
            let from = corners[side];
            let to = corners[(side + 1) % 4];
            let length_px = from.distance(to);
            let segments = ((length_px * mpp / segment_m).round() as i32).max(1);
            let dir = (to - from).normalize_or_zero();
            let yaw = (-dir.y).atan2(dir.x);
            for s in 0..segments {
                if side == gate_side
                    && ((s as f32 + 0.5) - segments as f32 * 0.5).abs()
                        < (segments as f32 * gap).max(1.0)
                {
                    continue;
                }
                let at = from + dir * length_px * (s as f32 + 0.5) / segments as f32;
                if !plain_ok(t, at) {
                    continue;
                }
                out.push(Instance {
                    prop,
                    rule: index,
                    pos: at,
                    yaw,
                    len_m: length_px * mpp / segments as f32,
                    offset: at - centre,
                    rank,
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn villages(
    t: &dyn Terrain,
    cfg: &Config,
    rule: &Rule,
    index: usize,
    cell: IVec2,
    kinds: &[String],
    chance: f32,
    offset_m: [f32; 2],
    rng: &mut Rng,
    out: &mut Vec<Instance>,
) {
    for village in t.villages_in(cell) {
        if !kinds.contains(&village.kind) {
            continue;
        }
        let roll = rng.f32();
        let prop = pick_prop(rule, rng);
        let angle = rng.f32() * TAU;
        let distance = rng.range(offset_m[0], offset_m[1]) / cfg.meters_per_px;
        let yaw = rng.f32() * TAU;
        let rank = rng.f32();
        if roll > chance || !in_regions(rule, village.px) {
            continue;
        }
        let at = village.px + Vec2::new(angle.cos(), angle.sin()) * distance;
        if !plain_ok(t, at) || !landscape_ok(t, at, rule) {
            continue;
        }
        out.push(Instance {
            prop,
            rule: index,
            pos: at,
            yaw,
            len_m: 0.0,
            offset: at - village.px,
            rank,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn roadside(
    t: &dyn Terrain,
    cfg: &Config,
    rule: &Rule,
    index: usize,
    cell: IVec2,
    clearance: f32,
    per_100km: f32,
    offset_m: [f32; 2],
    parallel: bool,
    out: &mut Vec<Instance>,
) {
    let mpp = cfg.meters_per_px;
    for seg in t.roads_in(cell) {
        let mut rng = Rng::new(seed_of(&[seg.id as i64, text_hash(&rule.id), 7]));
        let expected = seg.a.distance(seg.b) * mpp / 100_000.0 * per_100km;
        let dir = (seg.b - seg.a).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        for _ in 0..count_for(expected, &mut rng) {
            let along = rng.f32();
            let side = if rng.f32() < 0.5 { 1.0 } else { -1.0 };
            let lateral = rng.range(offset_m[0], offset_m[1]) / mpp;
            let prop = pick_prop(rule, &mut rng);
            let rank = rng.f32();
            let yaw_free = rng.f32() * TAU;
            let at = seg.a.lerp(seg.b, along) + normal * lateral * side;
            if !in_regions(rule, at)
                || t.near_town(at, clearance * 0.6)
                || !site_ok(t, mpp, at, rule)
            {
                continue;
            }
            let yaw = if parallel {
                (-dir.y).atan2(dir.x)
            } else {
                yaw_free
            };
            out.push(Instance {
                prop,
                rule: index,
                pos: at,
                yaw,
                len_m: 0.0,
                offset: Vec2::ZERO,
                rank,
            });
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::campaign::countryside::config::{Filters, Prop, Render};

    /// Flat island: land everywhere, optional blockers.
    pub struct Flat {
        pub villages: Vec<Village>,
        pub roads: Vec<RoadSeg>,
        pub lake: Option<(Vec2, f32)>,
        pub town: Option<Vec2>,
    }

    impl Terrain for Flat {
        fn size(&self) -> Vec2 {
            Vec2::splat(2000.0)
        }
        fn is_land(&self, _: Vec2) -> bool {
            true
        }
        fn river_sd(&self, _: Vec2) -> f32 {
            8.0
        }
        fn in_lake(&self, p: Vec2) -> bool {
            self.lake.is_some_and(|(c, r)| c.distance(p) < r)
        }
        fn height_m(&self, _: Vec2) -> f32 {
            100.0
        }
        fn splat(&self, _: Vec2) -> [f32; 4] {
            [0.6, 0.1, 0.1, 0.0]
        }
        fn wet(&self, _: Vec2) -> [f32; 2] {
            [0.0, 0.0]
        }
        fn biome(&self, _: Vec2) -> i32 {
            1
        }
        fn biome_allows(&self, mask: u32, biome: i32) -> bool {
            mask & (1 << biome) != 0
        }
        fn landscape_row(&self, _: Vec2) -> i32 {
            -1
        }
        fn landscape_id_row(&self, _: &str) -> i32 {
            -2
        }
        fn near_town(&self, p: Vec2, clearance: f32) -> bool {
            self.town.is_some_and(|t| t.distance(p) < clearance)
        }
        fn villages_in(&self, _: IVec2) -> &[Village] {
            &self.villages
        }
        fn roads_in(&self, _: IVec2) -> &[RoadSeg] {
            &self.roads
        }
    }

    pub fn filters() -> Filters {
        Filters {
            alt_min_m: -50.0,
            alt_max_m: 4000.0,
            slope_max_deg: 90.0,
            marsh_max: 0.5,
            prairie_min: 0.0,
            open_min: 0.0,
            forest_max: 1.0,
            crops_max: 1.0,
            wet_ok: false,
            biome_mask: 0,
            coast_px: 0,
            landscapes: vec![],
            exclude_landscapes: vec![],
        }
    }

    pub fn config(mode: Mode, density: f32) -> Config {
        Config {
            render: Render {
                cell_px: 96.0,
                load_factor: 2.4,
                spawn_seconds: 0.0,
                load_hysteresis: 0.0,
                lod_hysteresis: 0.0,
                min_radius_px: 24.0,
                max_radius_px: 200.0,
                max_cached_cells: 64,
                cells_per_frame: 1,
                size_k: 0.0045,
                size_exponent: 0.85,
                spread_exponent: 0.5,
                lod_distances: [25.0, 60.0],
                town_clearance_px: 0.8,
                max_instances_per_prop: 700,
                max_distance: 110.0,
                fade_from: 70.0,
            },
            props: vec![Prop {
                id: "p".into(),
                model: "props/p".into(),
                max_mult: 30.0,
                size_scale: 1.0,
                fade_from: 40.0,
                max_distance: 80.0,
                stretched: false,
                fixed_size: false,
                max_instances: 100,
                albedo_gain: 1.0,
            }],
            rules: vec![Rule {
                id: "r".into(),
                mode,
                props: vec![(0, 1.0)],
                total_weight: 1.0,
                regions: vec![],
                is_global: true,
                density_per_1000km2: density,
                filters: filters(),
                seasons: [1.0; 4],
            }],
            meters_per_px: 719.0,
            maquette_size: 1.0,
            maquette_range: 1.0,
            fixed_mult: 0.0,
        }
    }

    fn flat() -> Flat {
        Flat {
            villages: vec![],
            roads: vec![],
            lake: None,
            town: None,
        }
    }

    #[test]
    fn scatter_is_deterministic_and_dense_enough() {
        let cfg = config(Mode::Scatter, 2000.0);
        let t = flat();
        let a = cell_instances(&t, &cfg, IVec2::new(3, 4));
        let b = cell_instances(&t, &cfg, IVec2::new(3, 4));
        assert_eq!(a, b);
        // 96 px cell = 4770 km2: density 2000 / 1000 km2 gives ~9500 expected.
        assert!(a.len() > 5000, "{}", a.len());
        let rect = cell_rect(IVec2::new(3, 4), 96.0);
        assert!(
            a.iter()
                .all(|i| i.pos.x >= rect.min.x && i.pos.x <= rect.max.x)
        );
        assert_ne!(a, cell_instances(&t, &cfg, IVec2::new(4, 4)));
    }

    #[test]
    fn lakes_and_towns_are_avoided() {
        let cfg = config(Mode::Scatter, 200.0);
        let mut t = flat();
        t.lake = Some((Vec2::new(340.0, 420.0), 20.0));
        t.town = Some(Vec2::new(330.0, 400.0));
        let cell = cell_of(Vec2::new(340.0, 420.0), 96.0);
        for i in cell_instances(&t, &cfg, cell) {
            assert!(i.pos.distance(Vec2::new(340.0, 420.0)) >= 20.0);
            assert!(i.pos.distance(Vec2::new(330.0, 400.0)) >= 0.8);
        }
    }

    #[test]
    fn filters_reject_forest_and_wrong_biome() {
        let mut cfg = config(Mode::Scatter, 200.0);
        cfg.rules[0].filters.forest_max = 0.05;
        assert!(cell_instances(&flat(), &cfg, IVec2::new(1, 1)).is_empty());
        let mut cfg = config(Mode::Scatter, 200.0);
        cfg.rules[0].filters.biome_mask = 1 << 3;
        assert!(cell_instances(&flat(), &cfg, IVec2::new(1, 1)).is_empty());
        cfg.rules[0].filters.biome_mask = 1 << 1;
        assert!(!cell_instances(&flat(), &cfg, IVec2::new(1, 1)).is_empty());
    }

    #[test]
    fn paddock_makes_closed_fence_with_a_gate() {
        let cfg = config(
            Mode::Paddock {
                size_m: [150.0, 150.0],
                segment_m: 15.0,
                gap: 0.1,
            },
            0.1,
        );
        let t = flat();
        let mut found = false;
        for x in 0..20 {
            let out = cell_instances(&t, &cfg, IVec2::new(x, 2));
            if !out.is_empty() {
                // 4 sides x 10 segments, minus the gate.
                assert!((30..40).contains(&out.len()), "{}", out.len());
                assert!(out.iter().all(|i| i.len_m > 14.0 && i.len_m < 16.0));
                found = true;
            }
        }
        assert!(found);
    }

    #[test]
    fn village_and_road_modes_follow_their_anchors() {
        let mut t = flat();
        t.villages = (0..40)
            .map(|n| Village {
                px: Vec2::new(10.0 + n as f32, 20.0),
                kind: if n % 2 == 0 { "village" } else { "castle" }.into(),
            })
            .collect();
        let cfg = config(
            Mode::Village {
                kinds: vec!["village".into()],
                chance: 1.0,
                offset_m: [200.0, 380.0],
            },
            0.0,
        );
        let out = cell_instances(&t, &cfg, IVec2::ZERO);
        assert_eq!(out.len(), 20);
        assert!(out.iter().all(|i| i.offset.length() * 719.0 <= 381.0));
        t.roads = vec![RoadSeg {
            a: Vec2::new(10.0, 100.0),
            b: Vec2::new(90.0, 100.0),
            main: true,
            id: 5,
        }];
        let cfg = config(
            Mode::Road {
                types: vec![],
                per_100km: 100.0,
                offset_m: [5.0, 11.0],
                parallel: true,
            },
            0.0,
        );
        let out = cell_instances(&t, &cfg, IVec2::ZERO);
        // 80 px = 57.5 km at 100 per 100 km.
        assert!((40..=75).contains(&out.len()), "{}", out.len());
        assert!(
            out.iter()
                .all(|i| (i.pos.y - 100.0).abs() < 0.02 && i.yaw.abs() < 1e-4)
        );
    }
}
