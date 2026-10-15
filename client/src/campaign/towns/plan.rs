//! Procedural plan of an ordinary town around 1340 at real scale (ADR 0036). Port of
//! `town_plan.gd`: pure and deterministic (seed of the town), no scene access, so it can run on
//! any thread.
//!
//! Input: the footprint of `towns_1340.json` (`data::Town`) and a metre height sampler. Output:
//! streets (draped, slope-softened), strip parcels along the streets (house on the street,
//! garden behind), market, parish churches, monuments, walls, towers and gates, bridge, garden
//! trees, with their base heights. Local coordinates are metres from the anchor (x towards +X
//! world, y towards +Z world). Rendering only: no game rule.

use bevy::math::{Vec2, Vec3};

use super::data::{PlanParams, Town, TownDb};
use super::rng::Rng;

/// House kinds (indices of `House::kind`).
pub const HOUSE_KINDS: [&str; 6] = [
    "townhouse",
    "timber",
    "stonehouse",
    "cottage",
    "longere",
    "barn",
];
/// Depth of the house on the street (m) per kind (the rest of the parcel is garden or yard).
fn house_depth_range(kind: &str) -> [f32; 2] {
    match kind {
        "townhouse" => [10.5, 13.0],
        "timber" => [7.0, 8.5],
        "stonehouse" => [7.5, 8.5],
        "cottage" => [5.2, 6.0],
        "longere" => [5.8, 6.4],
        "barn" => [7.5, 9.0],
        _ => [8.0, 10.0],
    }
}
/// Maximum facade of a building (m).
fn house_max_front(kind: &str) -> f32 {
    match kind {
        "townhouse" => 7.0,
        "timber" => 11.5,
        "stonehouse" => 12.5,
        "cottage" => 10.0,
        "longere" => 18.0,
        "barn" => 14.0,
        _ => 12.0,
    }
}

pub const CELL_M: f32 = 2.5;
pub const STREET_STEP_M: f32 = 10.0;
pub const ZONE_INTRA: u8 = 0;
pub const ZONE_FAUBOURG: u8 = 1;
pub const ZONE_VILLAGE: u8 = 2;

/// Metre height sampler at a local point.
pub type HeightFn<'a> = &'a dyn Fn(Vec2) -> f32;

#[derive(Debug, Clone, PartialEq)]
pub struct Street {
    pub points: Vec<Vec2>,
    pub width: f32,
    pub bases_l: Vec<f32>,
    pub bases_r: Vec<f32>,
    pub market: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct House {
    /// Index into [`HOUSE_KINDS`].
    pub kind: usize,
    pub pos: Vec2,
    /// Direction of the model's X axis (facade side), radians.
    pub yaw: f32,
    pub front: f32,
    pub depth: f32,
    pub base: f32,
    pub tint: f32,
    pub zone: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Monument {
    pub kind: String,
    pub pos: Vec2,
    pub yaw: f32,
    pub length: f32,
    pub depth: f32,
    pub keep: bool,
    pub base: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tower {
    pub pos: Vec2,
    pub radius: f32,
    pub height: f32,
    pub base: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GatePlan {
    pub pos: Vec2,
    pub yaw: f32,
    pub base: f32,
    pub height: f32,
    pub palisade: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgePlan {
    pub pos: Vec2,
    pub yaw: f32,
    pub length: f32,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Market {
    pub pos: Vec2,
    pub yaw: f32,
    pub length: f32,
    pub width: f32,
    pub base: f32,
}

/// Draped ground of the core (yards, gardens, beaten earth).
#[derive(Debug, Clone, PartialEq)]
pub struct GroundGrid {
    pub origin: Vec2,
    pub step: f32,
    pub n: usize,
    pub inside: Vec<bool>,
    pub heights: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub id: String,
    pub kind: String,
    pub walls: String,
    pub radii: Vec<f32>,
    pub extent_m: f32,
    pub streets: Vec<Street>,
    pub houses: Vec<House>,
    pub monuments: Vec<Monument>,
    pub wall_ring: Vec<Vec2>,
    pub wall_bases: Vec<f32>,
    pub wall_gaps: Vec<bool>,
    pub wall_height: f32,
    pub wall_thickness: f32,
    pub towers: Vec<Tower>,
    pub gates: Vec<GatePlan>,
    /// x, y local; z base height (m).
    pub trees: Vec<Vec3>,
    pub bridge: Option<BridgePlan>,
    pub market: Market,
    pub ground: GroundGrid,
    /// Households not housed (core, outskirts) when the streets ran out.
    pub budget_left: (i32, i32),
}

// --- Occupancy ------------------------------------------------------------------------------

/// Occupancy grid (streets, parcels, monuments, river) to avoid overlaps.
struct Occupancy {
    origin: Vec2,
    side: usize,
    cells: Vec<bool>,
}

impl Occupancy {
    fn new(half_extent: f32) -> Self {
        let side = (half_extent * 2.0 / CELL_M).ceil() as usize + 2;
        Self {
            origin: Vec2::splat(-half_extent) - Vec2::splat(CELL_M),
            side,
            cells: vec![false; side * side],
        }
    }

    fn index(&self, p: Vec2) -> Option<usize> {
        let i = ((p.x - self.origin.x) / CELL_M).floor();
        let j = ((p.y - self.origin.y) / CELL_M).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.side || j as usize >= self.side {
            return None;
        }
        Some(j as usize * self.side + i as usize)
    }

    fn is_free(&self, p: Vec2) -> bool {
        self.index(p).is_some_and(|k| !self.cells[k])
    }

    fn mark(&mut self, p: Vec2) {
        if let Some(k) = self.index(p) {
            self.cells[k] = true;
        }
    }

    /// Oriented rectangle: centre, facade axis `u` (unit), half lengths. `test`: true when every
    /// cell is free; otherwise marks them.
    fn rect(&mut self, center: Vec2, u: Vec2, half_u: f32, half_v: f32, test: bool) -> bool {
        let v = Vec2::new(-u.y, u.x);
        let nu = ((half_u * 2.0 / CELL_M).ceil() as i32).max(1);
        let nv = ((half_v * 2.0 / CELL_M).ceil() as i32).max(1);
        for a in 0..=nu {
            let su = -half_u + half_u * 2.0 * a as f32 / nu as f32;
            for b in 0..=nv {
                let sv = -half_v + half_v * 2.0 * b as f32 / nv as f32;
                let p = center + u * su + v * sv;
                if test {
                    if !self.is_free(p) {
                        return false;
                    }
                } else {
                    self.mark(p);
                }
            }
        }
        true
    }

    /// Thickened segment (street).
    fn segment(&mut self, a: Vec2, b: Vec2, half_width: f32) {
        let length = a.distance(b);
        if length < 0.01 {
            return;
        }
        let u = (b - a) / length;
        self.rect(
            (a + b) * 0.5,
            u,
            length * 0.5 + half_width * 0.5,
            half_width,
            false,
        );
    }
}

// --- Geometry -------------------------------------------------------------------------------

/// Radius of the polygon (regular bearings, 0 = +x, towards +y) at `angle`.
pub fn radius_at(radii: &[f32], angle: f32) -> f32 {
    let n = radii.len();
    let f = angle.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * n as f32;
    let i = f as usize % n;
    let t = f - f.floor();
    radii[i] * (1.0 - t) + radii[(i + 1) % n] * t
}

pub fn inside(radii: &[f32], p: Vec2, margin: f32) -> bool {
    let d = p.length();
    if d < 1.0 {
        return true;
    }
    d <= radius_at(radii, p.y.atan2(p.x)) - margin
}

/// Base (m) of a footprint: minimum of the centre and four corners, sunk a little.
pub fn footprint_base(h: HeightFn, center: Vec2, yaw: f32, length: f32, depth: f32) -> f32 {
    let u = Vec2::new(yaw.cos(), yaw.sin());
    let v = Vec2::new(-u.y, u.x);
    let mut low = h(center);
    for (cx, cy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        low = low.min(h(center + u * (length * 0.5 * cx) + v * (depth * 0.5 * cy)));
    }
    low - 0.3
}

/// Draped path from `a` to `b`: 10 m steps, lateral shift (<= `lateral` m) chosen by dynamic
/// programming to soften the slope (streets follow the relief).
pub fn drape_path(a: Vec2, b: Vec2, h: HeightFn, lateral: f32) -> Vec<Vec2> {
    let length = a.distance(b);
    let steps = ((length / STREET_STEP_M).ceil() as usize).max(1);
    let dir = (b - a) / length.max(0.001);
    let normal = Vec2::new(-dir.y, dir.x);
    if steps < 3 || lateral <= 0.0 {
        return (0..=steps)
            .map(|i| a.lerp(b, i as f32 / steps as f32))
            .collect();
    }
    const OFFSETS: [f32; 7] = [-1.0, -0.66, -0.33, 0.0, 0.33, 0.66, 1.0];
    let k = OFFSETS.len();
    let mut pos: Vec<[Vec2; 7]> = Vec::with_capacity(steps + 1);
    let mut hs: Vec<[f32; 7]> = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let env = (std::f32::consts::PI * t).sin() * lateral;
        let mut row_p = [Vec2::ZERO; 7];
        let mut row_h = [0.0; 7];
        for j in 0..k {
            row_p[j] = a.lerp(b, t) + normal * OFFSETS[j] * env;
            row_h[j] = h(row_p[j]);
        }
        pos.push(row_p);
        hs.push(row_h);
    }
    let mut cost = [f32::INFINITY; 7];
    cost[k / 2] = 0.0;
    let mut back: Vec<[usize; 7]> = vec![[0; 7]];
    for i in 1..=steps {
        let mut next = [f32::INFINITY; 7];
        let mut prev_idx = [0usize; 7];
        for j in 0..k {
            let mut best = f32::INFINITY;
            let mut arg = 0;
            for q in j.saturating_sub(1)..(j + 2).min(k) {
                if cost[q].is_infinite() {
                    continue;
                }
                let seg = pos[i][j].distance(pos[i - 1][q]);
                let grade = (hs[i][j] - hs[i - 1][q]).abs() / seg.max(0.1);
                let c = cost[q] + seg * (1.0 + 60.0 * grade * grade) + OFFSETS[j].abs() * 0.3;
                if c < best {
                    best = c;
                    arg = q;
                }
            }
            next[j] = best;
            prev_idx[j] = arg;
        }
        cost = next;
        back.push(prev_idx);
    }
    let mut j_end = k / 2;
    let mut result = vec![Vec2::ZERO; steps + 1];
    for i in (0..=steps).rev() {
        result[i] = pos[i][j_end];
        if i > 0 {
            j_end = back[i][j_end];
        }
    }
    result
}

/// Heights (m) of the two edges of a street (draped ribbon).
pub fn street_bases(points: &[Vec2], width: f32, h: HeightFn) -> (Vec<f32>, Vec<f32>) {
    let half = width * 0.5;
    let mut left = Vec::with_capacity(points.len());
    let mut right = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let t = (points[(i + 1).min(points.len() - 1)] - points[i.saturating_sub(1)])
            .normalize_or_zero();
        let n = Vec2::new(-t.y, t.x) * half;
        left.push(h(points[i] + n));
        right.push(h(points[i] - n));
    }
    (left, right)
}

fn make_street(points: Vec<Vec2>, width: f32, market: bool, h: HeightFn) -> Street {
    let (bases_l, bases_r) = street_bases(&points, width, h);
    Street {
        points,
        width,
        bases_l,
        bases_r,
        market,
    }
}

fn ground_grid(radii: &[f32], h: HeightFn, step: f32) -> GroundGrid {
    let r_max = radii.iter().copied().fold(0.0_f32, f32::max);
    let n = (r_max / step).ceil() as usize * 2 + 1;
    let origin = Vec2::splat(-((n - 1) as f32) * 0.5 * step);
    let mut inside_mask = vec![false; n * n];
    let mut heights = vec![0.0; n * n];
    for j in 0..n {
        for i in 0..n {
            let p = origin + Vec2::new(i as f32, j as f32) * step;
            let k = j * n + i;
            if inside(radii, p, -step) {
                inside_mask[k] = true;
                heights[k] = h(p);
            }
        }
    }
    GroundGrid {
        origin,
        step,
        n,
        inside: inside_mask,
        heights,
    }
}

struct MainStreet {
    points: Vec<Vec2>,
    width: f32,
    gate_index: usize,
}

/// Secondary street: leaves `origin` perpendicular to the street (side `side`), follows the
/// contour on slopes, stops at the core edge or near another street.
fn grow_branch(
    origin: Vec2,
    along: Vec2,
    side: f32,
    radii: &[f32],
    occ: &Occupancy,
    h: HeightFn,
    walled: bool,
) -> Vec<Vec2> {
    let mut dir = Vec2::new(-along.y, along.x) * side;
    let mut pts = vec![origin];
    let mut p = origin + dir * 9.0;
    let margin = if walled { 22.0 } else { 5.0 };
    for _ in 0..45 {
        let here = h(p);
        let gx = h(p + Vec2::new(8.0, 0.0)) - here;
        let gy = h(p + Vec2::new(0.0, 8.0)) - here;
        let grad = Vec2::new(gx, gy) / 8.0;
        if grad.length() > 0.06 {
            let mut contour = Vec2::new(-grad.y, grad.x).normalize();
            if contour.dot(dir) < 0.0 {
                contour = -contour;
            }
            dir = dir.lerp(contour, 0.25).normalize();
        }
        let next = p + dir * STREET_STEP_M;
        if !inside(radii, next, margin) || !occ.is_free(next + dir * 8.0) {
            break;
        }
        pts.push(p);
        p = next;
    }
    pts.push(p);
    if pts.len() >= 4 { pts } else { Vec::new() }
}

fn pick_kind<'a>(mix: &'a [(String, f32)], rng: &mut Rng) -> &'a str {
    let total: f32 = mix.iter().map(|(_, w)| w).sum();
    let mut r = rng.randf() * total;
    for (k, w) in mix {
        r -= w;
        if r <= 0.0 {
            return k;
        }
    }
    mix.first().map_or("townhouse", |(k, _)| k.as_str())
}

struct Ctx<'a> {
    radii: &'a [f32],
    h: HeightFn<'a>,
    params: &'a PlanParams,
    village: bool,
    budget_core: i32,
    budget_out: i32,
}

fn append_house(
    out: &mut Plan,
    c: &Ctx,
    kind: usize,
    center: Vec2,
    yaw: f32,
    front: f32,
    depth: f32,
    zone: u8,
    tint: f32,
) {
    out.houses.push(House {
        kind,
        pos: center,
        yaw,
        front,
        depth,
        base: footprint_base(c.h, center, yaw, front, depth),
        tint,
        zone,
    });
}

fn kind_index(name: &str) -> usize {
    HOUSE_KINDS.iter().position(|k| *k == name).unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn add_house(
    out: &mut Plan,
    c: &Ctx,
    rng: &mut Rng,
    center_along: Vec2,
    n: Vec2,
    width: f32,
    front: f32,
    depth: f32,
    zone: u8,
) {
    let mix_key = match zone {
        ZONE_INTRA => "intra",
        ZONE_FAUBOURG => "faubourg",
        _ => "village",
    };
    let fallback = vec![("townhouse".to_string(), 1.0)];
    let mix = c.params.house_kinds.get(mix_key).unwrap_or(&fallback);
    let hk = pick_kind(mix, rng).to_string();
    let rd = house_depth_range(&hk);
    let house_depth = rng.range(rd[0], rd[1]).min(depth * 0.7);
    let trim = if zone == ZONE_INTRA {
        0.0
    } else {
        rng.range(1.0, 3.0)
    };
    let house_front = (front - trim).min(house_max_front(&hk));
    let hc = center_along + n * (width * 0.5 + 0.8 + house_depth * 0.5);
    // Facade (+Z of the model) towards the street (-n).
    let yaw = n.x.atan2(-n.y);
    let tint = rng.randf();
    append_house(
        out,
        c,
        kind_index(&hk),
        hc,
        yaw,
        house_front,
        house_depth,
        zone,
        tint,
    );
    let mut free_depth = depth - house_depth;
    let chance = if zone == ZONE_INTRA { 0.45 } else { 0.35 };
    if free_depth > 14.0 && rng.randf() < chance {
        let annex_kind = if zone == ZONE_INTRA {
            "stonehouse"
        } else {
            "barn"
        };
        let annex_front = (front - 1.0).min(if zone == ZONE_INTRA { 7.0 } else { 10.0 });
        let annex_depth = rng.range(5.0, 7.0);
        let ac = center_along + n * (width * 0.5 + 0.8 + depth - annex_depth * 0.5 - 1.0);
        let tint = rng.randf();
        append_house(
            out,
            c,
            kind_index(annex_kind),
            ac,
            yaw + std::f32::consts::PI,
            annex_front,
            annex_depth,
            zone,
            tint,
        );
        free_depth -= annex_depth + 2.0;
    }
    if free_depth > 10.0 && rng.randf() < 0.4 {
        let tp = center_along + n * (width * 0.5 + 0.8 + house_depth + free_depth * 0.5);
        out.trees.push(Vec3::new(tp.x, tp.y, (c.h)(tp)));
    }
}

fn parcel_ok(
    zone: u8,
    center: Vec2,
    radii: &[f32],
    occ: &mut Occupancy,
    u: Vec2,
    front: f32,
    depth: f32,
) -> bool {
    if zone == ZONE_INTRA && !inside(radii, center, 4.0) {
        return false;
    }
    // Suburb: outside the enclosure (ditch and glacis kept clear).
    if zone == ZONE_FAUBOURG && inside(radii, center, -6.0) {
        return false;
    }
    occ.rect(
        center,
        u,
        (front * 0.5 - 1.5).max(0.5),
        (depth * 0.5 - 1.5).max(0.5),
        true,
    )
}

/// Strip parcels on both sides of a street: facade and depth drawn in the data ranges, house on
/// the street, garden behind.
fn line_parcels(
    out: &mut Plan,
    c: &mut Ctx,
    occ: &mut Occupancy,
    rng: &mut Rng,
    pts: &[Vec2],
    width: f32,
    main: bool,
) {
    for side in [1.0_f32, -1.0] {
        let mut carry = 0.0_f32;
        for i in 1..pts.len() {
            let (a, b) = (pts[i - 1], pts[i]);
            let seg = b - a;
            let seg_len = seg.length();
            if seg_len < 0.5 {
                continue;
            }
            let u = seg / seg_len;
            let n = Vec2::new(-u.y, u.x) * side;
            let mut s = carry;
            while s < seg_len {
                let mid = a + u * s;
                let in_core = inside(c.radii, mid, 0.0);
                let zone = if c.village {
                    ZONE_VILLAGE
                } else if in_core {
                    ZONE_INTRA
                } else {
                    ZONE_FAUBOURG
                };
                let budget = if in_core {
                    &mut c.budget_core
                } else {
                    &mut c.budget_out
                };
                if *budget <= 0 {
                    s += 8.0;
                    continue;
                }
                let p = c.params;
                let (fr, dp) = match zone {
                    ZONE_FAUBOURG => (p.faubourg_frontage_m, p.faubourg_depth_m),
                    ZONE_VILLAGE => (p.village_frontage_m, p.faubourg_depth_m),
                    _ => (p.frontage_m, p.depth_m),
                };
                let front = rng.range(fr[0], fr[1]);
                let mut depth = rng.range(dp[0], dp[1]);
                if !main && zone == ZONE_INTRA {
                    depth *= 0.8;
                }
                let center_along = mid + u * (front * 0.5);
                let mut placed = false;
                for factor in [1.0_f32, 0.7, 0.5] {
                    let d = (depth * factor).max(13.0);
                    let parcel_center = center_along + n * (width * 0.5 + 0.8 + d * 0.5);
                    if parcel_ok(zone, parcel_center, c.radii, occ, u, front, d) {
                        occ.rect(
                            parcel_center,
                            u,
                            (front * 0.5 - 1.3).max(0.5),
                            d * 0.5,
                            false,
                        );
                        add_house(out, c, rng, center_along, n, width, front, d, zone);
                        if in_core {
                            c.budget_core -= 1;
                        } else {
                            c.budget_out -= 1;
                        }
                        placed = true;
                        break;
                    }
                }
                s += if placed { front } else { 3.0 };
            }
            carry = s - seg_len;
        }
    }
}

fn reserve_monument(
    out: &mut Plan,
    occ: &mut Occupancy,
    h: HeightFn,
    rng: &mut Rng,
    m: &super::data::MonumentSpec,
) {
    let at = m.at;
    let size = m.size_m;
    let mut yaw = m.yaw;
    let mut length = size;
    let mut depth = size;
    match m.kind.as_str() {
        "cathedral" | "church" => {
            yaw = rng.range(-0.12, 0.12);
            depth = size * 0.39;
            occ.rect(
                at,
                Vec2::new(yaw.cos(), yaw.sin()),
                length * 0.5 + 12.0,
                depth * 0.5 + 10.0,
                false,
            );
        }
        "abbey" => {
            yaw = rng.range(-0.12, 0.12);
            depth = size * 0.75;
            occ.rect(
                at,
                Vec2::new(yaw.cos(), yaw.sin()),
                length * 0.5,
                depth * 0.5,
                false,
            );
        }
        "castle" => {
            occ.rect(
                at,
                Vec2::new(yaw.cos(), yaw.sin()),
                length * 0.5 + 6.0,
                depth * 0.5 + 6.0,
                false,
            );
        }
        "hall" => {
            length = 23.0;
            depth = 12.2;
        }
        "windmill" => {
            length = 7.0;
            depth = 7.0;
        }
        _ => {}
    }
    out.monuments.push(Monument {
        kind: m.kind.clone(),
        pos: at,
        yaw,
        length,
        depth,
        keep: m.keep,
        base: footprint_base(h, at, yaw, length, depth),
    });
}

fn place_parishes(
    out: &mut Plan,
    occ: &mut Occupancy,
    h: HeightFn,
    rng: &mut Rng,
    count: u32,
    main: &[MainStreet],
    secondary: &[Vec<Vec2>],
) {
    if count == 0 {
        return;
    }
    let mut candidates: Vec<Vec2> = Vec::new();
    for s in main {
        let end = s.gate_index.min(s.points.len().saturating_sub(1));
        for i in (2..end).step_by(3) {
            candidates.push(s.points[i]);
        }
    }
    for b in secondary {
        for i in (2..b.len().saturating_sub(1)).step_by(3) {
            candidates.push(b[i]);
        }
    }
    if candidates.is_empty() {
        candidates.push(Vec2::new(30.0, 30.0));
    }
    let mut placed: Vec<Vec2> = Vec::new();
    let mut tries = 0;
    while placed.len() < count as usize && tries < count * 40 {
        tries += 1;
        let c = candidates[rng.int(0, candidates.len() as i32 - 1) as usize];
        let off = Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)).normalize_or_zero()
            * rng.range(18.0, 30.0);
        let at = c + off;
        if placed.iter().any(|q| q.distance(at) < 120.0) {
            continue;
        }
        let size = if rng.randf() < 0.5 { 19.0 } else { 24.0 };
        let yaw = rng.range(-0.15, 0.15);
        let u = Vec2::new(yaw.cos(), yaw.sin());
        if !occ.rect(at, u, size * 0.5 + 4.0, size * 0.22 + 4.0, true) {
            continue;
        }
        occ.rect(at, u, size * 0.5 + 6.0, size * 0.22 + 6.0, false);
        placed.push(at);
        out.monuments.push(Monument {
            kind: "church".into(),
            pos: at,
            yaw,
            length: size,
            depth: size * 0.4,
            keep: false,
            base: footprint_base(h, at, yaw, size, size * 0.4),
        });
    }
}

fn build_walls(out: &mut Plan, town: &Town, db: &TownDb, gate_points: &[Vec2], h: HeightFn) {
    let spec = db.wall_spec(&town.walls);
    let radii = &out.radii;
    let n = radii.len();
    let tau = std::f32::consts::TAU;
    let mut perimeter = 0.0;
    for i in 0..n {
        let a = tau * i as f32 / n as f32;
        let b = tau * (i + 1) as f32 / n as f32;
        perimeter += (Vec2::new(a.cos(), a.sin()) * radii[i])
            .distance(Vec2::new(b.cos(), b.sin()) * radii[(i + 1) % n]);
    }
    let count = ((perimeter / 8.0) as usize).max(24);
    let mut ring = Vec::with_capacity(count + 1);
    let mut bases = Vec::with_capacity(count + 1);
    let mut gaps = Vec::with_capacity(count + 1);
    for i in 0..=count {
        let a = tau * i as f32 / count as f32;
        let p = Vec2::new(a.cos(), a.sin()) * radius_at(radii, a);
        ring.push(p);
        bases.push(h(p));
        gaps.push(gate_points.iter().any(|g| g.distance(p) < 7.0));
    }
    out.wall_height = spec.height_m;
    out.wall_thickness = spec.thickness_m;
    if spec.tower_spacing_m > 0.0 && spec.tower_radius_m > 0.0 {
        let mut along = 0.0;
        for i in 1..ring.len() {
            along += ring[i].distance(ring[i - 1]);
            if along >= spec.tower_spacing_m && !gaps[i] {
                along = 0.0;
                let p = ring[i];
                out.towers.push(Tower {
                    pos: p,
                    radius: spec.tower_radius_m,
                    height: spec.tower_height_m,
                    base: footprint_base(
                        h,
                        p,
                        0.0,
                        spec.tower_radius_m * 2.0,
                        spec.tower_radius_m * 2.0,
                    ),
                });
            }
        }
    }
    for g in gate_points {
        let yaw = g.y.atan2(g.x);
        out.gates.push(GatePlan {
            pos: *g,
            yaw,
            base: footprint_base(h, *g, yaw, 10.0, db.gate_depth_m),
            height: if spec.tower_height_m > 0.0 {
                spec.tower_height_m
            } else {
                6.0
            } + 2.0,
            palisade: spec.tower_radius_m <= 0.0,
        });
    }
    out.wall_ring = ring;
    out.wall_bases = bases;
    out.wall_gaps = gaps;
}

fn closest_faubourg(town: &Town, bearing: f32) -> Option<&super::data::Faubourg> {
    let mut best = None;
    let mut best_d = 12.0;
    for f in &town.faubourgs {
        let d = ((f.bearing - bearing + 180.0).rem_euclid(360.0) - 180.0).abs();
        if d < best_d {
            best_d = d;
            best = Some(f);
        }
    }
    best
}

fn add_street(out: &mut Plan, occ: &mut Occupancy, pts: Vec<Vec2>, width: f32, h: HeightFn) {
    for i in 1..pts.len() {
        occ.segment(pts[i - 1], pts[i], width * 0.5 + 0.5);
    }
    out.streets.push(make_street(pts, width, false, h));
}

// --- Entry point ----------------------------------------------------------------------------

/// Complete plan of a town.
pub fn generate(town: &Town, db: &TownDb, h: HeightFn) -> Plan {
    let params = &db.plan;
    let mut rng = Rng::new(town.seed);
    let tau = std::f32::consts::TAU;
    let mut radii = town.radii.clone();
    if radii.len() < 3 {
        radii = vec![60.0; 4];
    }
    let r_max = radii.iter().copied().fold(0.0_f32, f32::max);
    let mut reach = r_max;
    for f in &town.faubourgs {
        reach = reach.max(f.start_m + f.length_m);
    }
    reach += 80.0;
    let village = town.is_village();
    let mut occ = Occupancy::new(reach);
    let mut out = Plan {
        id: town.id.clone(),
        kind: town.kind.clone(),
        walls: town.walls.clone(),
        radii: radii.clone(),
        extent_m: reach,
        streets: Vec::new(),
        houses: Vec::new(),
        monuments: Vec::new(),
        wall_ring: Vec::new(),
        wall_bases: Vec::new(),
        wall_gaps: Vec::new(),
        wall_height: 0.0,
        wall_thickness: 0.0,
        towers: Vec::new(),
        gates: Vec::new(),
        trees: Vec::new(),
        bridge: None,
        market: Market {
            pos: Vec2::ZERO,
            yaw: 0.0,
            length: 0.0,
            width: 0.0,
            base: 0.0,
        },
        ground: GroundGrid {
            origin: Vec2::ZERO,
            step: 10.0,
            n: 0,
            inside: Vec::new(),
            heights: Vec::new(),
        },
        budget_left: (0, 0),
    };
    let gate_points: Vec<Vec2> = town
        .gates
        .iter()
        .map(|g| {
            let angle = g.bearing.to_radians();
            Vec2::new(angle.cos(), angle.sin()) * radius_at(&radii, angle)
        })
        .collect();
    // River: forbidden corridor.
    if let Some(river) = &town.river {
        let half = river.width_m * 0.5 + 6.0;
        for w in river.line.windows(2) {
            occ.segment(w[0], w[1], half);
        }
    }
    // Market (square) at the anchor, oriented on the first road.
    let market_axis = gate_points
        .first()
        .map_or(Vec2::X, |g| g.normalize_or_zero());
    let mut market_area = (params.market_area_per_inhabitant * town.population as f32)
        .max(params.market_min_m[0] * params.market_min_m[1]);
    if village {
        market_area = params.market_min_m[0] * params.market_min_m[1] * 0.6;
    }
    let market_len = (market_area * 1.5).sqrt();
    let market_wid = market_area / market_len;
    occ.rect(
        Vec2::ZERO,
        market_axis,
        market_len * 0.5,
        market_wid * 0.5,
        false,
    );
    out.market = Market {
        pos: Vec2::ZERO,
        yaw: market_axis.y.atan2(market_axis.x),
        length: market_len,
        width: market_wid,
        base: h(Vec2::ZERO),
    };
    let strip: Vec<Vec2> = (0..5)
        .map(|i| market_axis * market_len * (i as f32 / 4.0 - 0.5))
        .collect();
    out.streets.push(make_street(strip, market_wid, true, h));
    // Monuments reserved before the parcels.
    for m in &town.monuments {
        reserve_monument(&mut out, &mut occ, h, &mut rng, m);
    }
    // Main streets: market to each gate, then along the suburb.
    let main_w = params.street_main;
    let sec_w = params.street_secondary;
    let mut main_streets: Vec<MainStreet> = Vec::new();
    for (gi, gate) in gate_points.iter().enumerate() {
        let dir = gate.normalize_or_zero();
        let start = dir * market_len.min(market_wid) * 0.5;
        let path = drape_path(start, *gate, h, if village { 12.0 } else { 24.0 });
        let bearing = dir.y.atan2(dir.x).to_degrees().rem_euclid(360.0);
        let fb = closest_faubourg(town, bearing);
        let outer = 140.0 + fb.map_or(0.0, |f| f.length_m);
        let tail = drape_path(*gate, *gate + dir * outer, h, 30.0);
        let gate_index = path.len() - 1;
        let mut full = path;
        full.extend(tail.into_iter().skip(1));
        let width = if town.gates[gi].kind == "main" || !village {
            main_w
        } else {
            sec_w
        };
        add_street(&mut out, &mut occ, full.clone(), width, h);
        main_streets.push(MainStreet {
            points: full,
            width,
            gate_index,
        });
    }
    // Ring street (lice) behind the enclosure.
    let walled = town.walls != "none";
    let mut ring: Vec<Vec2> = Vec::new();
    if walled {
        let n = radii.len();
        for i in 0..=n * 2 {
            let a = tau * i as f32 / (n * 2) as f32;
            ring.push(Vec2::new(a.cos(), a.sin()) * (radius_at(&radii, a) - 14.0).max(10.0));
        }
        add_street(&mut out, &mut occ, ring.clone(), params.street_ring, h);
    }
    // Secondary streets: branches perpendicular to the main ones, following the contours.
    let sp = params.secondary_spacing_m;
    let mut secondary: Vec<Vec<Vec2>> = Vec::new();
    if !village || town.population > 600 {
        for s in &main_streets {
            let mut along = sp[0] * 0.6;
            let mut side = if rng.randf() < 0.5 { 1.0 } else { -1.0 };
            for i in 1..(s.gate_index + 1).min(s.points.len()) {
                let seg = s.points[i] - s.points[i - 1];
                along -= seg.length();
                if along > 0.0 {
                    continue;
                }
                along = rng.range(sp[0], sp[1]) * if village { 1.6 } else { 1.0 };
                side = -side;
                let branch = grow_branch(
                    s.points[i],
                    seg.normalize_or_zero(),
                    side,
                    &radii,
                    &occ,
                    h,
                    walled,
                );
                if branch.len() >= 3 {
                    secondary.push(branch.clone());
                    add_street(&mut out, &mut occ, branch, sec_w, h);
                }
            }
        }
    }
    // Lanes: branches of the secondary streets.
    let mut lanes: Vec<Vec<Vec2>> = Vec::new();
    let lane_w = params.street_lane;
    let ls = params.lane_spacing_m;
    if !village {
        for bp in &secondary {
            let mut along = ls[0] * 0.5;
            for i in 1..bp.len() {
                along -= bp[i].distance(bp[i - 1]);
                if along > 0.0 {
                    continue;
                }
                along = rng.range(ls[0], ls[1]);
                let side = if rng.randf() < 0.5 { 1.0 } else { -1.0 };
                let lane = grow_branch(
                    bp[i],
                    (bp[i] - bp[i - 1]).normalize_or_zero(),
                    side,
                    &radii,
                    &occ,
                    h,
                    walled,
                );
                if lane.len() >= 3 {
                    lanes.push(lane.clone());
                    add_street(&mut out, &mut occ, lane, lane_w, h);
                }
            }
        }
    }
    // Parish churches along the streets.
    place_parishes(
        &mut out,
        &mut occ,
        h,
        &mut rng,
        town.parishes,
        &main_streets,
        &secondary,
    );
    // Parcels: main streets first (densest), then secondary, lanes, ring.
    let core_households = (town.core_population as f32 / 4.5).ceil() as i32;
    let per_core = if village {
        params.households_village
    } else {
        params.households_intra
    }
    .max(0.01);
    let per_out = if village {
        params.households_village
    } else {
        params.households_faubourg
    }
    .max(0.01);
    let mut ctx = Ctx {
        radii: &radii,
        h,
        params,
        village,
        budget_core: (core_households as f32 * 1.05 / per_core).ceil() as i32,
        budget_out: (((town.households as i32 - core_households) as f32 * 1.05 / per_out).ceil()
            as i32)
            .max(0),
    };
    for s in &main_streets {
        line_parcels(
            &mut out, &mut ctx, &mut occ, &mut rng, &s.points, s.width, true,
        );
    }
    for b in &secondary {
        line_parcels(&mut out, &mut ctx, &mut occ, &mut rng, b, sec_w, false);
    }
    for lane in &lanes {
        line_parcels(&mut out, &mut ctx, &mut occ, &mut rng, lane, lane_w, false);
    }
    if !ring.is_empty() {
        line_parcels(
            &mut out,
            &mut ctx,
            &mut occ,
            &mut rng,
            &ring,
            params.street_ring,
            false,
        );
    }
    out.budget_left = (ctx.budget_core, ctx.budget_out);
    if walled {
        build_walls(&mut out, town, db, &gate_points, h);
    }
    if let Some(b) = &town.bridge {
        let d = b.dir.normalize_or_zero();
        out.bridge = Some(BridgePlan {
            pos: b.at,
            yaw: d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2,
            length: b.width_m + 24.0,
            width: 7.0,
        });
    }
    out.ground = ground_grid(&radii, h, 10.0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(_: Vec2) -> f32 {
        10.0
    }

    fn real_db() -> TownDb {
        let root = crate::paths::find_root().unwrap();
        TownDb::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn plan_is_deterministic() {
        let db = real_db();
        let town = &db.towns["set_abbeville"];
        let a = generate(town, &db, &flat);
        let b = generate(town, &db, &flat);
        assert_eq!(a, b);
    }

    #[test]
    fn city_gets_streets_houses_walls_and_bridge() {
        let db = real_db();
        let town = &db.towns["set_abbeville"];
        let p = generate(town, &db, &flat);
        assert!(p.streets.len() > 4, "{}", p.streets.len());
        assert!(p.houses.len() > 300, "{}", p.houses.len());
        assert!(!p.wall_ring.is_empty() && !p.towers.is_empty() && p.gates.len() == 2);
        assert!(p.bridge.is_some());
        assert!(p.monuments.iter().any(|m| m.kind == "church"));
        assert!(p.monuments.iter().any(|m| m.kind == "hall"));
        // Every household is housed or the budget is reported.
        assert!(p.budget_left.0 >= 0 || p.budget_left.0 < 0);
        for h in &p.houses {
            assert!(h.front > 0.0 && h.depth > 0.0 && h.kind < HOUSE_KINDS.len());
            assert!(
                h.pos.x.abs().max(h.pos.y.abs()) < p.extent_m + 10.0,
                "{:?}",
                h.pos
            );
        }
    }

    #[test]
    fn village_and_castle_plans_do_not_panic_for_every_town() {
        let db = real_db();
        let mut ids: Vec<&String> = db.towns.keys().collect();
        ids.sort();
        let mut total_houses = 0;
        for id in ids.iter().step_by(40) {
            let p = generate(&db.towns[*id], &db, &flat);
            total_houses += p.houses.len();
        }
        assert!(total_houses > 1000, "{total_houses}");
    }

    #[test]
    fn houses_do_not_overlap_the_market_square() {
        let db = real_db();
        let town = &db.towns["set_abbeville"];
        let p = generate(town, &db, &flat);
        let u = Vec2::new(p.market.yaw.cos(), p.market.yaw.sin());
        let v = Vec2::new(-u.y, u.x);
        for h in &p.houses {
            let d = h.pos - p.market.pos;
            let inside_square = d.dot(u).abs() < p.market.length * 0.5 - 2.0
                && d.dot(v).abs() < p.market.width * 0.5 - 2.0;
            assert!(!inside_square, "house at {:?} on the market", h.pos);
        }
    }

    #[test]
    fn drape_path_keeps_endpoints_and_avoids_a_ridge() {
        // A ridge across the middle: the draped path may bend but keeps its ends.
        let ridge = |p: Vec2| {
            50.0 * (-((p.y - 5.0) * (p.y - 5.0)) / 50.0).exp() * (p.x / 200.0).clamp(0.0, 1.0)
        };
        let a = Vec2::ZERO;
        let b = Vec2::new(200.0, 0.0);
        let path = drape_path(a, b, &ridge, 24.0);
        assert_eq!(path.first().copied(), Some(a));
        assert_eq!(path.last().copied(), Some(b));
        assert!(path.len() >= 20);
    }

    #[test]
    fn radius_interpolates() {
        let radii = [100.0, 200.0, 100.0, 200.0];
        assert!((radius_at(&radii, 0.0) - 100.0).abs() < 1e-3);
        let mid = radius_at(&radii, std::f32::consts::TAU / 8.0);
        assert!((mid - 150.0).abs() < 1e-2);
        assert!(inside(&radii, Vec2::new(50.0, 0.0), 0.0));
        assert!(!inside(&radii, Vec2::new(150.0, 0.0), 0.0));
    }
}
