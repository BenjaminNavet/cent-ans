//! 1:1 plan of an emblematic city (v2 format, ADR 0078). Port of `landmark_plan.gd`: pure and
//! deterministic (seed of the city), so it can run on any thread. Output: a [`Plan`] that the
//! town layer builds like an ordinary town (draped streets, strip parcels and houses, towers,
//! gates, trees, ground), plus what only a landmark city has: arbitrary wall rings with explicit
//! normals, real monument templates (meshes prepared here), bridges with their levels, and the
//! houses whose base does not follow the relief (houses on the bridge deck).
//!
//! The streets are the real streets (OSM or hand drawn); strip parcels are drawn along each
//! street inside the district that contains them (density, house mix), never over water, walls,
//! squares, monuments or other parcels. Local frame: metres from the origin, x east, y south.

use std::collections::HashMap;

use bevy::math::{Vec2, Vec3};
use serde_json::Value;

use super::data::PlanParams;
use super::landmark_monuments::{self, Built};
use super::landmark_v2::{local, local_line, num, pair, present, yaw_of, year_of};
use super::plan::{
    self, CELL_M, Ctx, GatePlan, GroundGrid, HOUSE_KINDS, HeightFn, House, Market, Occupancy, Plan,
    Street, Tower, ZONE_FAUBOURG, ZONE_INTRA, add_house, footprint_base,
};
use super::rng::Rng;

const DENSIFY_M: f32 = 10.0;
const GROUND_STEP_M: f32 = 12.0;
const WALL_STEP_M: f32 = 8.0;
const GATE_GAP_M: f32 = 7.0;
const WATER_CELL_M: f32 = 40.0;
const DISTRICT_CELL_M: f32 = 50.0;

/// Wall of any outline (closed or not), with outward normals.
#[derive(Debug, Clone, PartialEq)]
pub struct WallRing {
    pub ring: Vec<Vec2>,
    pub normals: Vec<Vec2>,
    pub gaps: Vec<bool>,
    pub height: f32,
    pub thickness: f32,
}

/// Monument with its prepared mesh (monument frame, see `landmark_monuments`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Monument {
    pub id: String,
    pub model: String,
    pub pos: Vec2,
    pub yaw: f32,
    pub length: f32,
    pub depth: f32,
    pub base: f32,
    pub built: Built,
    /// Not following the relief (bridge chapel): index of its bridge.
    pub bridge: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bridge {
    pub id: String,
    pub pos: Vec2,
    pub yaw: f32,
    pub length: f32,
    pub width: f32,
    pub arches: u32,
    pub pier_m: f32,
    pub starling_m: f32,
    pub a: Vec2,
    pub c: Vec2,
    pub deck_m: f32,
    pub water: f32,
    pub bank_from: f32,
    pub bank_to: f32,
    pub deck: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LmGate {
    pub pos: Vec2,
    pub yaw: f32,
    pub height: f32,
    pub name: String,
    /// Gatehouse standing on a bridge deck.
    pub bridge: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LandmarkPlan {
    pub plan: Plan,
    pub wall_rings: Vec<WallRing>,
    pub monuments: Vec<V2Monument>,
    pub bridges: Vec<Bridge>,
    pub gates: Vec<LmGate>,
    /// House index -> bridge index (houses on the deck).
    pub fixed_bases: HashMap<usize, usize>,
    pub detail_cell_m: f32,
    /// Free garden/cemetery/meadow trees, with their ground height.
    pub open_space_trees: usize,
}

// --- Districts ----------------------------------------------------------------------------

pub fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

struct District {
    poly: Vec<Vec2>,
    lo: Vec2,
    hi: Vec2,
    zone: String,
    density: f32,
    params: PlanParams,
}

#[derive(Default)]
struct Districts {
    list: Vec<District>,
    index: HashMap<(i32, i32), Vec<u32>>,
}

impl Districts {
    fn add(&mut self, poly: Vec<Vec2>, zone: String, density: f32, mix: Vec<(String, f32)>) {
        let mut lo = poly[0];
        let mut hi = poly[0];
        for q in &poly {
            lo = lo.min(*q);
            hi = hi.max(*q);
        }
        let mut params = PlanParams::default();
        params.house_kinds.insert("intra".into(), mix.clone());
        params.house_kinds.insert("faubourg".into(), mix);
        self.list.push(District {
            poly,
            lo,
            hi,
            zone,
            density,
            params,
        });
    }

    fn build_index(&mut self) {
        self.index.clear();
        let cell = |v: f32| (v / DISTRICT_CELL_M).floor() as i32;
        for (i, d) in self.list.iter().enumerate() {
            for cy in cell(d.lo.y)..=cell(d.hi.y) {
                for cx in cell(d.lo.x)..=cell(d.hi.x) {
                    self.index.entry((cx, cy)).or_default().push(i as u32);
                }
            }
        }
    }

    /// Index of the first district that contains `p`.
    fn at(&self, p: Vec2) -> Option<usize> {
        let key = (
            (p.x / DISTRICT_CELL_M).floor() as i32,
            (p.y / DISTRICT_CELL_M).floor() as i32,
        );
        for &i in self.index.get(&key)? {
            let d = &self.list[i as usize];
            if p.cmpge(d.lo).all() && p.cmple(d.hi).all() && point_in_polygon(p, &d.poly) {
                return Some(i as usize);
            }
        }
        None
    }

    fn bounds(&self) -> (Vec2, Vec2) {
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        for d in &self.list {
            lo = lo.min(d.lo);
            hi = hi.max(d.hi);
        }
        (lo, hi)
    }
}

// --- Waters -------------------------------------------------------------------------------

enum Water {
    Rings {
        rings: Vec<Vec<Vec2>>,
        lo: Vec2,
        hi: Vec2,
    },
    Line {
        line: Vec<Vec2>,
        widths: Vec<f32>,
    },
}

fn in_rings(p: Vec2, rings: &[Vec<Vec2>], lo: Vec2, hi: Vec2) -> bool {
    if !(p.cmpge(lo).all() && p.cmple(hi).all()) {
        return false;
    }
    let mut inside = false;
    for ring in rings {
        if point_in_polygon(p, ring) {
            inside = !inside;
        }
    }
    inside
}

fn closest_on_segment(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 < 1e-9 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
}

struct WaterIndex {
    cells: HashMap<(i32, i32), Vec<(usize, i32)>>,
}

impl WaterIndex {
    fn build(waters: &[Water]) -> Self {
        let cell = |v: f32| (v / WATER_CELL_M).floor() as i32;
        let mut cells: HashMap<(i32, i32), Vec<(usize, i32)>> = HashMap::new();
        for (wi, w) in waters.iter().enumerate() {
            match w {
                Water::Rings { rings, lo, hi } => {
                    let mut edge_cells = std::collections::HashSet::new();
                    for ring in rings {
                        for i in 0..ring.len() {
                            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                            let steps =
                                ((a.distance(b) / (WATER_CELL_M * 0.25)).ceil() as i32).max(1);
                            for k in 0..=steps {
                                let q = a.lerp(b, k as f32 / steps as f32);
                                edge_cells.insert((cell(q.x), cell(q.y)));
                            }
                        }
                    }
                    for cy in cell(lo.y)..=cell(hi.y) {
                        for cx in cell(lo.x)..=cell(hi.x) {
                            let code = if edge_cells.contains(&(cx, cy)) {
                                -1
                            } else if in_rings(
                                Vec2::new(cx as f32 + 0.5, cy as f32 + 0.5) * WATER_CELL_M,
                                rings,
                                *lo,
                                *hi,
                            ) {
                                -2
                            } else {
                                continue;
                            };
                            cells.entry((cx, cy)).or_default().push((wi, code));
                        }
                    }
                }
                Water::Line { line, widths } => {
                    for i in 0..line.len().saturating_sub(1) {
                        let g = widths[i] * 0.5 + 1.0;
                        let lo = line[i].min(line[i + 1]) - Vec2::splat(g);
                        let hi = line[i].max(line[i + 1]) + Vec2::splat(g);
                        for cy in cell(lo.y)..=cell(hi.y) {
                            for cx in cell(lo.x)..=cell(hi.x) {
                                cells.entry((cx, cy)).or_default().push((wi, i as i32));
                            }
                        }
                    }
                }
            }
        }
        Self { cells }
    }

    fn contains(&self, waters: &[Water], p: Vec2) -> bool {
        let key = (
            (p.x / WATER_CELL_M).floor() as i32,
            (p.y / WATER_CELL_M).floor() as i32,
        );
        let Some(list) = self.cells.get(&key) else {
            return false;
        };
        for &(wi, code) in list {
            if code == -2 {
                return true;
            }
            match &waters[wi] {
                Water::Rings { rings, lo, hi } => {
                    if in_rings(p, rings, *lo, *hi) {
                        return true;
                    }
                }
                Water::Line { line, widths } => {
                    let i = code as usize;
                    if closest_on_segment(p, line[i], line[i + 1]).distance(p) < widths[i] * 0.5 {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Marks the occupancy cells inside rings (even-odd: holes stay free), by scanlines.
fn fill_rings(occ: &mut Occupancy, rings: &[Vec<Vec2>], lo: Vec2, hi: Vec2) {
    let step = CELL_M;
    let mut y = lo.y + step * 0.5;
    while y < hi.y {
        let mut xs: Vec<f32> = Vec::new();
        for ring in rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if (a.y <= y) != (b.y <= y) {
                    xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks_exact(2) {
            let mut x = pair[0] + step * 0.5;
            while x < pair[1] {
                occ.mark(Vec2::new(x, y));
                x += step;
            }
        }
        y += step;
    }
}

fn densify(line: &[Vec2], step: f32) -> Vec<Vec2> {
    let mut out = Vec::new();
    for (i, q) in line.iter().enumerate() {
        if i == 0 {
            out.push(*q);
            continue;
        }
        let a = line[i - 1];
        let n = ((a.distance(*q) / step).ceil() as usize).max(1);
        for k in 1..=n {
            out.push(a.lerp(*q, k as f32 / n as f32));
        }
    }
    out
}

fn mix_of(houses: &Value) -> Vec<(String, f32)> {
    let mut mix: Vec<(String, f32)> = houses
        .as_object()
        .map(|o| o.iter().map(|(k, w)| (k.clone(), num(w, 0.0))).collect())
        .unwrap_or_default();
    mix.sort_by(|a, b| a.0.cmp(&b.0));
    mix
}

fn rank_order(rank: &str) -> u8 {
    match rank {
        "main" => 0,
        "secondary" => 1,
        "lane" => 2,
        _ => 3,
    }
}

fn empty_plan(city: &Value, extent: f32) -> Plan {
    Plan {
        id: city["settlement"].as_str().unwrap_or("").to_string(),
        kind: "landmark".into(),
        walls: "stone".into(),
        radii: vec![extent; 4],
        extent_m: extent,
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
            step: GROUND_STEP_M,
            n: 0,
            inside: Vec::new(),
            heights: Vec::new(),
        },
        budget_left: (0, 0),
    }
}

struct Builder<'a> {
    h: HeightFn<'a>,
    occ: Occupancy,
    districts: Districts,
    rng: Rng,
    out: LandmarkPlan,
    year: i32,
    widths: HashMap<String, f32>,
    /// `frontage_m`, `depth_m`, `faubourg_frontage_m`, `faubourg_depth_m` of the `plan` section.
    parcel: [[f32; 2]; 4],
}

/// Full plan of `city` for its year. `h`: metre height sampler at local points.
pub fn generate(city: &Value, h: HeightFn) -> LandmarkPlan {
    let year = year_of(city);
    let extent = num(&city["extent_m"], 1500.0);
    let seed = city["seed"].as_u64().unwrap_or(1) as u32;
    let plan = &city["plan"];
    let mut widths: HashMap<String, f32> = [
        ("main", 8.0),
        ("secondary", 5.0),
        ("lane", 3.0),
        ("quay", 14.0),
        ("square", 6.0),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    if let Some(o) = plan["street_width_m"].as_object() {
        for (k, v) in o {
            widths.insert(k.clone(), num(v, 5.0));
        }
    }
    let mut b = Builder {
        h,
        occ: Occupancy::new(extent + 60.0),
        districts: Districts::default(),
        rng: Rng::new(seed),
        out: LandmarkPlan {
            plan: empty_plan(city, extent),
            wall_rings: Vec::new(),
            monuments: Vec::new(),
            bridges: Vec::new(),
            gates: Vec::new(),
            fixed_bases: HashMap::new(),
            detail_cell_m: num(&plan["detail_cell_m"], 250.0),
            open_space_trees: 0,
        },
        year,
        widths,
        parcel: [
            pair(&plan["frontage_m"], [5.0, 8.0]),
            pair(&plan["depth_m"], [20.0, 40.0]),
            pair(&plan["faubourg_frontage_m"], [8.0, 16.0]),
            pair(&plan["faubourg_depth_m"], [25.0, 50.0]),
        ],
    };
    if let Some(ds) = city["districts"].as_array() {
        for d in ds {
            if present(d, year) {
                let poly = local_line(&d["polygon"]);
                if poly.len() >= 3 {
                    b.districts.add(
                        poly,
                        d["zone"].as_str().unwrap_or("intra").to_string(),
                        num(&d["density"], 0.9),
                        mix_of(&d["houses"]),
                    );
                }
            }
        }
    }
    b.districts.build_index();
    let waters = b.waters(city);
    if let Some(ms) = city["monuments"].as_array() {
        for m in ms {
            if present(m, year) {
                b.add_monument(m);
            }
        }
    }
    if let Some(ws) = city["walls"].as_array() {
        for w in ws {
            if present(w, year) {
                b.add_wall(w);
            }
        }
    }
    if let Some(bs) = city["bridges"].as_array() {
        for br in bs {
            if present(br, year) {
                b.add_bridge(br);
            }
        }
    }
    if let Some(qs) = city["quays"].as_array() {
        for q in qs {
            let line = densify(&local_line(&q["points"]), DENSIFY_M);
            let qw = num(&q["width_m"], b.widths["quay"]);
            for w in line.windows(2) {
                b.occ.segment(w[0], w[1], qw * 0.5);
            }
            b.push_street(line, qw);
        }
    }
    if let Some(ss) = city["open_spaces"].as_array() {
        for s in ss {
            if present(s, year) {
                b.add_open_space(s);
            }
        }
    }
    // Real streets.
    let mut ranked: Vec<(Vec<Vec2>, f32, bool, u8)> = Vec::new();
    if let Some(ss) = city["streets"].as_array() {
        for s in ss {
            if !present(s, year) {
                continue;
            }
            let rank = s["rank"].as_str().unwrap_or("secondary");
            let width = num(&s["width_m"], b.widths.get(rank).copied().unwrap_or(5.0));
            let pts = densify(&local_line(&s["points"]), DENSIFY_M);
            if pts.len() < 2 {
                continue;
            }
            for w in pts.windows(2) {
                b.occ.segment(w[0], w[1], width * 0.5 + 0.5);
            }
            b.push_street(pts.clone(), width);
            ranked.push((pts, width, rank == "main", rank_order(rank)));
        }
    }
    ranked.sort_by_key(|r| r.3);
    if let Some(items) = city["parcels"].as_array()
        && !items.is_empty()
    {
        b.imported_parcels(items);
    }
    for (pts, width, main, _) in &ranked {
        b.line_parcels(pts, *width, *main);
    }
    b.out.plan.ground = b.ground_grid(&waters);
    b.out.plan.trees.shrink_to_fit();
    b.out
}

impl Builder<'_> {
    fn push_street(&mut self, points: Vec<Vec2>, width: f32) {
        let (bases_l, bases_r) = plan::street_bases(&points, width, self.h);
        self.out.plan.streets.push(Street {
            points,
            width,
            bases_l,
            bases_r,
            market: false,
        });
    }

    /// Forbidden water corridors and polygon beds; drawn streams become ribbons.
    fn waters(&mut self, city: &Value) -> Vec<Water> {
        let mut waters = Vec::new();
        let Some(ws) = city["waters"].as_array() else {
            return waters;
        };
        for w in ws {
            if w["polygon"].is_array() {
                let mut rings = vec![local_line(&w["polygon"])];
                if let Some(holes) = w["holes"].as_array() {
                    for hole in holes {
                        rings.push(local_line(hole));
                    }
                }
                if rings[0].len() < 3 {
                    continue;
                }
                let mut lo = rings[0][0];
                let mut hi = rings[0][0];
                for q in &rings[0] {
                    lo = lo.min(*q);
                    hi = hi.max(*q);
                }
                fill_rings(&mut self.occ, &rings, lo, hi);
                waters.push(Water::Rings { rings, lo, hi });
                continue;
            }
            let mut line = Vec::new();
            let mut widths = Vec::new();
            for q in w["points"].as_array().into_iter().flatten() {
                line.push(Vec2::new(num(&q[0], 0.0), -num(&q[1], 0.0)));
                widths.push(num(&q[2], 10.0));
            }
            if line.len() < 2 {
                continue;
            }
            for i in 0..line.len() - 1 {
                self.occ.segment(
                    line[i],
                    line[i + 1],
                    (widths[i] + widths[i + 1]) * 0.25 + 4.0,
                );
            }
            if w["draw"].as_bool().unwrap_or(false) {
                let dense = densify(&line, DENSIFY_M);
                self.push_street(dense, widths[0]);
            }
            waters.push(Water::Line { line, widths });
        }
        waters
    }

    fn add_monument(&mut self, m: &Value) {
        let at = local(&m["at"]);
        let yaw = yaw_of(num(&m["angle_deg"], 0.0));
        let fp = landmark_monuments::footprint(m);
        let clear = num(&m["clear_m"], 6.0);
        let u = Vec2::new(yaw.cos(), yaw.sin());
        self.occ
            .rect(at, u, fp.x * 0.5 + clear, fp.y * 0.5 + clear, false);
        let built = landmark_monuments::build(m);
        self.out.monuments.push(V2Monument {
            id: m["id"].as_str().unwrap_or("").to_string(),
            model: m["model"].as_str().unwrap_or("").to_string(),
            pos: at,
            yaw,
            length: fp.x,
            depth: fp.y,
            base: footprint_base(self.h, at, yaw, fp.x, fp.y),
            built,
            bridge: None,
        });
    }

    fn add_wall(&mut self, wall: &Value) {
        let mut pts = local_line(&wall["points"]);
        if pts.len() < 2 {
            return;
        }
        let closed = wall["closed"].as_bool().unwrap_or(false);
        let outline = pts.clone();
        if closed {
            pts.push(pts[0]);
        }
        let ring = densify(&pts, WALL_STEP_M);
        let mut gates: Vec<(Vec2, &Value)> = Vec::new();
        for g in wall["gates"].as_array().into_iter().flatten() {
            if present(g, self.year) {
                gates.push((local(&g["at"]), g));
            }
        }
        let n = ring.len();
        let mut normals = Vec::with_capacity(n);
        let mut gaps = Vec::with_capacity(n);
        for i in 0..n {
            let t = (ring[(i + 1).min(n - 1)] - ring[i.saturating_sub(1)]).normalize_or_zero();
            let mut nrm = Vec2::new(-t.y, t.x);
            if closed && point_in_polygon(ring[i] + nrm * 6.0, &outline) {
                nrm = -nrm;
            }
            normals.push(nrm);
            gaps.push(gates.iter().any(|(g, _)| g.distance(ring[i]) < GATE_GAP_M));
        }
        let thickness = num(&wall["thickness_m"], 2.4);
        let height = num(&wall["height_m"], 8.0);
        let ditch = num(&wall["ditch_m"], 0.0);
        for i in 0..n - 1 {
            self.occ
                .segment(ring[i], ring[i + 1], thickness * 0.5 + 4.0);
            if ditch > 0.0 {
                let off = normals[i] * (thickness * 0.5 + ditch * 0.5 + 1.0);
                self.occ
                    .segment(ring[i] + off, ring[i + 1] + off, ditch * 0.5);
            }
        }
        // Regular towers (away from the gates).
        let spacing = num(&wall["tower_spacing_m"], 0.0);
        let tower_r = num(&wall["tower_radius_m"], 0.0);
        let tower_h = num(&wall["tower_height_m"], height + 4.0);
        if spacing > 0.0 && tower_r > 0.0 {
            let mut along = spacing * 0.5;
            for i in 1..n {
                along += ring[i].distance(ring[i - 1]);
                if along < spacing || gaps[i] {
                    continue;
                }
                if gates.iter().any(|(g, _)| g.distance(ring[i]) < 18.0) {
                    continue;
                }
                along = 0.0;
                let p = ring[i] + normals[i] * (thickness * 0.5 + tower_r * 0.35);
                self.out.plan.towers.push(Tower {
                    pos: p,
                    radius: tower_r,
                    height: tower_h,
                    base: footprint_base(self.h, p, 0.0, tower_r * 2.0, tower_r * 2.0),
                });
            }
        }
        // Gates: gatehouse crossed by the street (axis = wall normal).
        for (g, meta) in &gates {
            let best = (0..n)
                .min_by(|&a, &b| {
                    ring[a]
                        .distance_squared(*g)
                        .total_cmp(&ring[b].distance_squared(*g))
                })
                .unwrap_or(0);
            let nrm = normals[best];
            let yaw = nrm.y.atan2(nrm.x);
            let gh = num(&meta["height_m"], tower_h + 2.0);
            self.out.gates.push(LmGate {
                pos: *g,
                yaw,
                height: gh,
                name: meta["name"].as_str().unwrap_or("").to_string(),
                bridge: None,
            });
            self.out.plan.gates.push(GatePlan {
                pos: *g,
                yaw,
                base: footprint_base(self.h, *g, yaw, 12.0, 11.0),
                height: gh,
                palisade: false,
            });
            self.occ
                .rect(*g, Vec2::new(yaw.cos(), yaw.sin()), 10.0, 9.0, false);
        }
        self.out.wall_rings.push(WallRing {
            ring,
            normals,
            gaps,
            height,
            thickness,
        });
    }

    fn bridge_levels(&self, br: &mut Bridge) {
        let (a, c) = (br.a, br.c);
        let mid = (a + c) * 0.5;
        br.bank_from = (self.h)(a);
        br.bank_to = (self.h)(c);
        let mut water = (self.h)(mid);
        for k in 0..5 {
            water = water.min((self.h)(a.lerp(c, 0.3 + 0.1 * k as f32)));
        }
        br.water = water;
        br.deck = br.bank_from.min(br.bank_to).max(water + br.deck_m);
    }

    fn add_bridge(&mut self, b: &Value) {
        let a = local(&b["from"]);
        let c = local(&b["to"]);
        let d = (c - a).normalize_or_zero();
        let length = a.distance(c);
        let width = num(&b["width_m"], 8.0);
        let mid = (a + c) * 0.5;
        let index = self.out.bridges.len();
        let mut br = Bridge {
            id: b["id"].as_str().unwrap_or("").to_string(),
            pos: mid,
            yaw: d.y.atan2(d.x),
            length: length + 8.0,
            width,
            arches: b["arches"]
                .as_u64()
                .map_or(((length / 18.0) as u32).max(1), |x| x as u32),
            pier_m: num(&b["pier_m"], 3.5),
            starling_m: num(&b["starling_m"], 0.0),
            a,
            c,
            deck_m: num(&b["deck_m"], 7.0),
            water: 0.0,
            bank_from: 0.0,
            bank_to: 0.0,
            deck: 0.0,
        };
        self.bridge_levels(&mut br);
        let (deck, water) = (br.deck, br.water);
        if self.out.plan.bridge.is_none() {
            self.out.plan.bridge = Some(plan::BridgePlan {
                pos: mid,
                yaw: br.yaw,
                length: br.length,
                width,
            });
        }
        self.out.bridges.push(br);
        self.occ
            .segment(a - d * 10.0, c + d * 10.0, width * 0.5 + 1.0);
        let n = Vec2::new(-d.y, d.x);
        // Spans without houses: drawbridge (both sides), chapel (its side): [from, to, side].
        let mut gaps: Vec<[f32; 3]> = Vec::new();
        if let Some(t) = b["drawbridge_at"].as_f64() {
            let t = t as f32 * length;
            gaps.push([t - 9.0, t + 9.0, 0.0]);
        }
        let chapel_side = if b["chapel_side"].as_str() == Some("left") {
            -1.0
        } else {
            1.0
        };
        if let Some(t) = b["chapel_at"].as_f64() {
            let t = t as f32 * length;
            gaps.push([t - 12.0, t + 12.0, chapel_side]);
            self.add_bridge_chapel(b, a + d * t, n * chapel_side, width, deck, water, index);
        }
        if b["houses"].as_bool().unwrap_or(false) {
            let span = pair(&b["house_span"], [0.15, 0.85]);
            let mut s = span[0] * length;
            while s < span[1] * length {
                let front = self.rng.range(5.0, 7.0);
                for side in [-1.0_f32, 1.0] {
                    let blocked = gaps
                        .iter()
                        .any(|g| s + front > g[0] && s < g[1] && (g[2] == 0.0 || g[2] == side));
                    if blocked {
                        continue;
                    }
                    let nn = n * side;
                    let hd = self.rng.range(6.0, 8.0);
                    let center = a + d * (s + front * 0.5) + nn * (width * 0.5 + hd * 0.5);
                    let idx = self.out.plan.houses.len();
                    self.out.fixed_bases.insert(idx, index);
                    let tint = self.rng.randf();
                    self.out.plan.houses.push(House {
                        kind: HOUSE_KINDS.iter().position(|k| *k == "timber").unwrap_or(1),
                        pos: center,
                        yaw: nn.x.atan2(-nn.y),
                        front: front - 0.3,
                        depth: hd,
                        base: deck - 0.2,
                        tint,
                        zone: ZONE_INTRA,
                    });
                }
                s += front;
            }
        }
        for f in b["gatehouses_at"].as_array().into_iter().flatten() {
            let p = a + d * num(f, 0.5) * length;
            let yaw = d.y.atan2(d.x);
            self.out.gates.push(LmGate {
                pos: p,
                yaw,
                height: 13.0,
                name: String::new(),
                bridge: Some(index),
            });
            self.out.plan.gates.push(GatePlan {
                pos: p,
                yaw,
                base: deck - 0.5,
                height: 13.0,
                palisade: false,
            });
        }
    }

    /// Bridge chapel (St Thomas on London Bridge): on a pier, side `side`, choir outwards.
    #[allow(clippy::too_many_arguments)]
    fn add_bridge_chapel(
        &mut self,
        b: &Value,
        at_deck: Vec2,
        side: Vec2,
        width: f32,
        deck: f32,
        water: f32,
        bridge: usize,
    ) {
        let length = num(&b["chapel_length_m"], 18.0);
        let chapel_w = num(&b["chapel_width_m"], 9.0);
        let center = at_deck + side * (width * 0.5 + length * 0.5 - 1.0);
        let yaw = side.y.atan2(side.x);
        let base = water + 0.5;
        let id = format!("{}_chapel", b["id"].as_str().unwrap_or("bridge"));
        let m = serde_json::json!({
            "id": id, "model": "church",
            "params": {"length_m": length, "width_m": chapel_w, "height_m": deck - base + 8.0, "apse": "flat"}
        });
        let built = landmark_monuments::build(&m);
        self.out.monuments.push(V2Monument {
            id,
            model: "church".into(),
            pos: center,
            yaw,
            length,
            depth: chapel_w,
            base,
            built,
            bridge: Some(bridge),
        });
    }

    fn add_open_space(&mut self, s: &Value) {
        let poly = local_line(&s["polygon"]);
        if poly.len() < 3 {
            return;
        }
        let mut lo = poly[0];
        let mut hi = poly[0];
        for q in &poly {
            lo = lo.min(*q);
            hi = hi.max(*q);
        }
        let kind = s["kind"].as_str().unwrap_or("square");
        let paved = matches!(kind, "square" | "market" | "parvis" | "strand");
        let step = CELL_M;
        let mut y = lo.y + step * 0.5;
        while y < hi.y {
            let mut x = lo.x + step * 0.5;
            let mut run_start: Option<f32> = None;
            while x < hi.x + step {
                let p = Vec2::new(x, y);
                let inside = x < hi.x && point_in_polygon(p, &poly);
                if inside {
                    self.occ.mark(p);
                    run_start.get_or_insert(x);
                } else if let Some(r0) = run_start.take()
                    && paved
                {
                    let strip = vec![Vec2::new(r0 - step * 0.5, y), Vec2::new(x - step * 0.5, y)];
                    self.push_street(strip, step + 0.4);
                }
                x += step;
            }
            y += step;
        }
        let every = match kind {
            "garden" => 90.0,
            "cemetery" => 160.0,
            "meadow" => 400.0,
            "cloister" => 250.0,
            _ => 0.0,
        };
        if every > 0.0 {
            let count = ((hi.x - lo.x) * (hi.y - lo.y) / every) as i32;
            for _ in 0..count {
                let p = Vec2::new(self.rng.range(lo.x, hi.x), self.rng.range(lo.y, hi.y));
                if point_in_polygon(p, &poly) {
                    self.out.plan.trees.push(Vec3::new(p.x, p.y, (self.h)(p)));
                    self.out.open_space_trees += 1;
                }
            }
        }
    }

    fn house_ctx_add(
        &mut self,
        di: usize,
        center_along: Vec2,
        n: Vec2,
        width: f32,
        front: f32,
        depth: f32,
        zone: u8,
    ) {
        let params = &self.districts.list[di].params;
        let ctx = Ctx {
            radii: &[],
            h: self.h,
            params,
            village: false,
            budget_core: 0,
            budget_out: 0,
        };
        add_house(
            &mut self.out.plan,
            &ctx,
            &mut self.rng,
            center_along,
            n,
            width,
            front,
            depth,
            zone,
        );
    }

    /// Parcels on both sides of a street: frontage and depth by the district zone, house on the
    /// street, garden or yard behind.
    fn line_parcels(&mut self, pts: &[Vec2], width: f32, main: bool) {
        for side in [1.0_f32, -1.0] {
            let mut carry = 0.0_f32;
            for i in 1..pts.len() {
                let (a, bpt) = (pts[i - 1], pts[i]);
                let seg = bpt - a;
                let seg_len = seg.length();
                if seg_len < 0.5 {
                    continue;
                }
                let u = seg / seg_len;
                let n = Vec2::new(-u.y, u.x) * side;
                let mut s = carry;
                while s < seg_len {
                    let mid = a + u * s;
                    let probe = mid + n * (width * 0.5 + 6.0);
                    let Some(di) = self.districts.at(probe) else {
                        s += 4.0;
                        continue;
                    };
                    let (faubourg, density) = {
                        let d = &self.districts.list[di];
                        (d.zone == "faubourg", d.density)
                    };
                    let fr = self.parcel[if faubourg { 2 } else { 0 }];
                    let dp = self.parcel[if faubourg { 3 } else { 1 }];
                    let front = self.rng.range(fr[0], fr[1]);
                    if self.rng.randf() > density {
                        s += front;
                        continue;
                    }
                    let mut depth = self.rng.range(dp[0], dp[1]);
                    if !main && !faubourg {
                        depth *= 0.85;
                    }
                    let center_along = mid + u * (front * 0.5);
                    let mut placed = false;
                    for factor in [1.0_f32, 0.7, 0.5, 0.35] {
                        let d = (depth * factor).max(9.0);
                        let parcel_center = center_along + n * (width * 0.5 + 0.8 + d * 0.5);
                        if self.districts.at(parcel_center) != Some(di) {
                            continue;
                        }
                        // Test without the 2.5 m strip on the street: the street cells (rounded
                        // to 2.5 m) overflow onto the parcel front.
                        let test_center =
                            center_along + n * (width * 0.5 + 0.8 + 2.5 + (d - 2.5) * 0.5);
                        if self.occ.rect(
                            test_center,
                            u,
                            (front * 0.5 - 1.5).max(0.5),
                            ((d - 2.5) * 0.5 - 1.5).max(0.5),
                            true,
                        ) {
                            self.occ.rect(
                                parcel_center,
                                u,
                                (front * 0.5 - 1.3).max(0.5),
                                d * 0.5,
                                false,
                            );
                            let zone = if faubourg { ZONE_FAUBOURG } else { ZONE_INTRA };
                            self.house_ctx_add(di, center_along, n, width, front, d, zone);
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

    /// Imported parcels (`[dE, dN, angle, front, depth]`): middle of the street front and
    /// normal towards the inside; a long front is shared between several houses (<= 11 m).
    fn imported_parcels(&mut self, items: &[Value]) {
        for item in items {
            let p = local(&Value::Array(vec![item[0].clone(), item[1].clone()]));
            let yaw = yaw_of(num(&item[2], 0.0));
            let n = Vec2::new(yaw.cos(), yaw.sin());
            let t = Vec2::new(-n.y, n.x);
            let Some(di) = self.districts.at(p + n * 3.0) else {
                continue;
            };
            let (faubourg, density) = {
                let d = &self.districts.list[di];
                (d.zone == "faubourg", d.density)
            };
            let zone = if faubourg { ZONE_FAUBOURG } else { ZONE_INTRA };
            let front_total = num(&item[3], 8.0);
            let depth = num(&item[4], 20.0);
            let k = ((front_total / 11.0).ceil() as i32).max(1);
            let front = front_total / k as f32;
            for j in 0..k {
                if self.rng.randf() > density {
                    continue;
                }
                let center_along = p + t * (-front_total * 0.5 + front * (j as f32 + 0.5));
                for factor in [1.0_f32, 0.6, 0.4] {
                    let d = (depth * factor).max(6.0);
                    let test_center = center_along + n * (1.5 + (d - 1.5) * 0.5);
                    if self.occ.rect(
                        test_center,
                        t,
                        (front * 0.5 - 0.8).max(0.5),
                        ((d - 1.5) * 0.5 - 0.8).max(0.5),
                        true,
                    ) {
                        self.occ.rect(
                            center_along + n * (d * 0.5),
                            t,
                            (front * 0.5 - 0.3).max(0.5),
                            d * 0.5,
                            false,
                        );
                        self.house_ctx_add(di, center_along, n, 0.0, front, d, zone);
                        break;
                    }
                    if d <= 6.0 {
                        break;
                    }
                }
            }
        }
    }

    /// Ground of the districts (yards, gardens, beaten earth): square draped grid, vertices in a
    /// district and out of the water; faubourg ground only around houses (30 m cells).
    fn ground_grid(&self, waters: &[Water]) -> GroundGrid {
        let index = WaterIndex::build(waters);
        let mut near = std::collections::HashSet::new();
        for hs in &self.out.plan.houses {
            near.insert((
                (hs.pos.x / 30.0).floor() as i32,
                (hs.pos.y / 30.0).floor() as i32,
            ));
        }
        let (lo, hi) = self.districts.bounds();
        if !lo.is_finite() {
            return self.out.plan.ground.clone();
        }
        let (lo, hi) = (
            lo - Vec2::splat(GROUND_STEP_M),
            hi + Vec2::splat(GROUND_STEP_M),
        );
        let side = (hi.x - lo.x).max(hi.y - lo.y);
        let step = GROUND_STEP_M;
        let n = (side / step).ceil() as usize + 1;
        let mut inside = vec![false; n * n];
        let mut heights = vec![0.0; n * n];
        for j in 0..n {
            for i in 0..n {
                let p = lo + Vec2::new(i as f32, j as f32) * step;
                let Some(di) = self.districts.at(p) else {
                    continue;
                };
                if index.contains(waters, p) {
                    continue;
                }
                if self.districts.list[di].zone == "faubourg"
                    && !near.contains(&((p.x / 30.0).floor() as i32, (p.y / 30.0).floor() as i32))
                {
                    continue;
                }
                inside[j * n + i] = true;
                heights[j * n + i] = (self.h)(p);
            }
        }
        GroundGrid {
            origin: lo,
            step,
            n,
            inside,
            heights,
        }
    }
}

/// Summary used by the dump and the tests.
impl LandmarkPlan {
    pub fn tower_count(&self) -> usize {
        self.plan.towers.len()
    }

    pub fn wall_length_m(&self) -> f32 {
        self.wall_rings
            .iter()
            .map(|w| w.ring.windows(2).map(|p| p[0].distance(p[1])).sum::<f32>())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn flat(_: Vec2) -> f32 {
        10.0
    }

    #[test]
    fn polygon_membership() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(5.0, 5.0), &sq));
        assert!(!point_in_polygon(Vec2::new(15.0, 5.0), &sq));
    }

    #[test]
    fn small_city_builds_streets_houses_walls_and_a_bridge() {
        let city = json!({
            "settlement": "set_test", "year": 1340, "seed": 7, "extent_m": 400,
            "districts": [{"polygon": [[-300,-300],[300,-300],[300,300],[-300,300]], "zone": "intra", "density": 1.0, "houses": {"timber": 1.0}}],
            "streets": [{"points": [[-250, 0], [250, 0]], "rank": "main"}],
            "walls": [{"points": [[-280,-280],[280,-280],[280,280],[-280,280]], "closed": true, "height_m": 9.0, "tower_spacing_m": 60.0, "tower_radius_m": 4.0,
                       "gates": [{"name": "Porte", "at": [280, 0]}]}],
            "bridges": [{"id": "b", "from": [0, 100], "to": [0, 200], "width_m": 8.0, "houses": true}],
            "monuments": [{"id": "m", "model": "belfry", "at": [0, -100], "params": {"size": 10, "height": 30}}],
            "open_spaces": [{"kind": "garden", "polygon": [[100,100],[200,100],[200,200],[100,200]]}]
        });
        let a = generate(&city, &flat);
        assert!(a.plan.houses.len() > 20, "{}", a.plan.houses.len());
        assert_eq!(a.wall_rings.len(), 1);
        assert!(a.wall_rings[0].gaps.iter().any(|g| *g));
        assert!(!a.plan.towers.is_empty());
        assert_eq!(a.gates.len(), 1);
        assert_eq!(a.bridges.len(), 1);
        assert!(!a.fixed_bases.is_empty());
        assert_eq!(a.monuments.len(), 1);
        assert!(a.plan.ground.n > 0);
        assert!(a.open_space_trees > 0);
        // Deterministic.
        let b = generate(&city, &flat);
        assert_eq!(a, b);
    }

    #[test]
    fn dated_walls_disappear() {
        let city = json!({
            "settlement": "set_test", "year": 1429, "seed": 1, "extent_m": 300,
            "districts": [{"polygon": [[-100,-100],[100,-100],[100,100],[-100,100]], "zone": "intra"}],
            "walls": [{"points": [[-90,-90],[90,-90],[90,90]], "closed": false, "height_m": 9.0, "until_year": 1344}]
        });
        assert!(generate(&city, &flat).wall_rings.is_empty());
    }

    #[test]
    fn real_cities_build() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let lib = super::super::landmark_v2::LandmarkLibrary::load(&crate::paths::data_dir(&root));
        if lib.cities.is_empty() {
            return;
        }
        assert_eq!(lib.cities.len(), 8);
        for (id, city) in &lib.cities {
            let started = std::time::Instant::now();
            let p = generate(city, &flat);
            let expected = city["monuments"].as_array().map_or(0, |a| {
                a.iter().filter(|m| present(m, year_of(city))).count()
            });
            assert!(p.monuments.len() >= expected, "{id}");
            assert!(p.plan.houses.len() > 300, "{id}: {}", p.plan.houses.len());
            assert!(!p.plan.streets.is_empty(), "{id}");
            assert!(p.plan.ground.n > 10, "{id}");
            eprintln!(
                "{id}: {} houses, {} streets, {} monuments, {} towers, {} bridges, {} wall rings, {:?}",
                p.plan.houses.len(),
                p.plan.streets.len(),
                p.monuments.len(),
                p.tower_count(),
                p.bridges.len(),
                p.wall_rings.len(),
                started.elapsed()
            );
        }
    }
}
