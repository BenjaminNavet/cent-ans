//! Layout of the decor (lot EP6, see [`crate::decor`]): the procedural
//! countryside of a province and the hand placement of the historical maps
//! share the same checks — inside the field, off deep water, bridges, fords
//! and roads, off the woods and the village, clear of what is already laid.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use data_model::Terrain;

use crate::decor::{
    Area, AreaKind, Camp, Decor, DecorProp, DecorPropKind, DecorRules, FieldState, Hamlet,
    HamletLayout, LandscapeProfile, Moat, Mound,
};
use crate::field::Battlefield;
use crate::hydro::{self, CountSpan, Road, RoadKind, Span};
use crate::rng::BattleRng;
use crate::setup::SideId;
use crate::site::{House, HouseKind, Obstacle, ObstacleKind};
use crate::town::Footprint;

/// A point on a road, the unit heading of the road there and the road index.
type RoadSpot = ((f64, f64), (f64, f64), usize);

/// What a footprint must keep clear of.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Keep {
    /// Margin from the edge of the roads (`None`: may cross them).
    pub road: Option<f64>,
    /// Margin from the water's edge (`None`: may touch the water).
    pub water: Option<f64>,
    /// Stays off the centre of the deployment lines.
    pub lines: bool,
    /// Gap to what is already laid (`None`: may overlap it).
    pub gap: Option<f64>,
}

impl Keep {
    pub(crate) fn building(rules: &DecorRules) -> Keep {
        let p = &rules.placement;
        Keep {
            road: Some(p.road_margin_m),
            water: Some(p.water_margin_m),
            lines: true,
            gap: Some(p.house_gap_m),
        }
    }
}

/// The decor being laid on a field.
pub(crate) struct Layout<'a> {
    pub field: &'a Battlefield,
    pub rules: &'static DecorRules,
    pub decor: Decor,
    /// Farm tracks laid with the decor.
    pub tracks: Vec<Road>,
    pub hedges: Vec<Obstacle>,
    /// Footprints already laid (buildings, areas, props, camps).
    taken: Vec<Footprint>,
    pub stream: BattleRng,
}

/// A point and the unit direction of a polyline at arc length `s`.
fn point_at(points: &[(f64, f64)], mut s: f64) -> ((f64, f64), (f64, f64)) {
    for w in points.windows(2) {
        let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let len = dx.hypot(dz);
        if len < 1e-9 {
            continue;
        }
        if s <= len {
            let t = s / len;
            return ((w[0].0 + dx * t, w[0].1 + dz * t), (dx / len, dz / len));
        }
        s -= len;
    }
    let n = points.len();
    let (a, b) = (points[n.saturating_sub(2)], points[n - 1]);
    let len = (b.0 - a.0).hypot(b.1 - a.1).max(1e-9);
    (b, ((b.0 - a.0) / len, (b.1 - a.1) / len))
}

/// Arc length along `points` of the point of the polyline nearest `p`.
fn arc_at(points: &[(f64, f64)], p: (f64, f64)) -> f64 {
    let mut acc = 0.0;
    let mut best = (0.0, f64::INFINITY);
    for w in points.windows(2) {
        let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let len2 = dx * dx + dz * dz;
        let len = len2.sqrt();
        let t = if len2 > 1e-12 {
            (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = (w[0].0 + dx * t - p.0).hypot(w[0].1 + dz * t - p.1);
        if d < best.1 {
            best = (acc + len * t, d);
        }
        acc += len;
    }
    best.0
}

fn arc_length(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum()
}

/// `[min, max]` drawn uniformly (inclusive).
fn draw_count(span: CountSpan, stream: &mut BattleRng) -> u32 {
    span[0] + stream.below(span[1].saturating_sub(span[0]) + 1)
}

fn draw(span: Span, stream: &mut BattleRng) -> f64 {
    stream.range(span[0], span[1])
}

impl<'a> Layout<'a> {
    pub(crate) fn new(field: &'a Battlefield, stream: BattleRng) -> Self {
        let decor = field.decor.clone();
        let mut taken = Vec::new();
        for b in &decor.buildings {
            taken.push(Footprint::new(b.x, b.z, b.length, b.width, b.yaw));
        }
        for a in &decor.areas {
            taken.push(a.footprint());
        }
        for p in &decor.props {
            taken.push(p.footprint());
        }
        for c in &decor.camps {
            taken.push(c.area.footprint());
            taken.extend(c.convoy.iter().map(DecorProp::footprint));
        }
        Layout {
            field,
            rules: DecorRules::bundled(),
            decor,
            tracks: Vec::new(),
            hedges: Vec::new(),
            taken,
            stream,
        }
    }

    /// Writes the decor, tracks, hedges and mounds back into `field`.
    pub(crate) fn finish(self) -> (Decor, Vec<Road>, Vec<Obstacle>) {
        (self.decor, self.tracks, self.hedges)
    }

    fn roads(&self) -> impl Iterator<Item = &Road> {
        self.field.roads.iter().chain(&self.tracks)
    }

    /// Count drawn from `span` for the standard field, scaled by the area of
    /// this one (the fraction left is a chance of one more).
    fn scaled_count(&mut self, span: CountSpan) -> usize {
        let n = f64::from(draw_count(span, &mut self.stream)) * self.field.size.area_ratio();
        let base = n.floor();
        base as usize + usize::from(self.stream.unit() < n - base)
    }

    /// `chance` of one, on the standard field; more on a larger one.
    fn scaled_chance(&mut self, chance: f64) -> usize {
        let n = chance * self.field.size.area_ratio().sqrt();
        let base = n.floor();
        base as usize + usize::from(self.stream.unit() < n - base)
    }

    // ----- checks -------------------------------------------------------------------

    fn inside(&self, fp: &Footprint, margin: f64) -> bool {
        fp.corners().iter().all(|&(x, z)| {
            (margin..=self.field.width - margin).contains(&x)
                && (margin..=self.field.depth - margin).contains(&z)
        })
    }

    fn road_clear(&self, fp: &Footprint, margin: f64) -> bool {
        let r = fp.bounding_radius();
        self.roads().all(|road| {
            let reach = r + road.width * 0.5 + margin;
            road.points.windows(2).all(|w| {
                // Quick rejection by the segment's distance to the centre.
                let d = hydro::polyline_distance(w, fp.x, fp.z);
                d > reach || fp.distance_to_segment(w[0], w[1]) > road.width * 0.5 + margin
            })
        })
    }

    /// Samples the rectangle every few metres: no water within `margin`.
    fn water_clear(&self, fp: &Footprint, margin: f64) -> bool {
        let f = self.field;
        if let Some(coast) = &f.coast {
            if fp
                .corners()
                .iter()
                .any(|&(x, _)| coast.from_edge(x) < coast.beach + margin)
            {
                return false;
            }
        }
        let step = 6.0;
        let nu = ((fp.half_length * 2.0 / step).ceil() as usize).max(1);
        let nv = ((fp.half_depth * 2.0 / step).ceil() as usize).max(1);
        for i in 0..=nu {
            for j in 0..=nv {
                let u = -fp.half_length + fp.half_length * 2.0 * i as f64 / nu as f64;
                let v = -fp.half_depth + fp.half_depth * 2.0 * j as f64 / nv as f64;
                let (x, z) = fp.world(u, v);
                if f.water_gap(x, z) < margin || f.water_kind(x, z).is_some() {
                    return false;
                }
                if f.pools
                    .iter()
                    .any(|p| (x - p.x).hypot(z - p.z) < p.radius + margin)
                {
                    return false;
                }
            }
        }
        true
    }

    /// Off the bridges and fords (and their approaches).
    fn crossings_clear(&self, fp: &Footprint) -> bool {
        let margin = self.rules.placement.bridge_margin_m;
        let f = self.field;
        let bridges = f
            .bridges
            .iter()
            .all(|b| fp.signed_distance(b.x, b.z) > b.length * 0.5 + margin);
        let fords = f.river.as_ref().is_none_or(|r| {
            r.fords.iter().all(|ford| {
                fp.signed_distance(ford.x, r.center_z(ford.x)) > ford.half_width + margin
            })
        });
        bridges && fords
    }

    fn woods_clear(&self, fp: &Footprint) -> bool {
        let f = self.field;
        f.forests
            .iter()
            .chain(&f.forest_parts)
            .all(|z| fp.signed_distance(z.x, z.z) > z.radius + 3.0)
            && f.village
                .as_ref()
                .is_none_or(|v| fp.signed_distance(v.zone.x, v.zone.z) > v.zone.radius + 15.0)
    }

    fn free(&self, fp: &Footprint, gap: f64) -> bool {
        let r = fp.bounding_radius();
        self.taken.iter().all(|t| {
            (t.x - fp.x).hypot(t.z - fp.z) > r + t.bounding_radius() + gap
                || t.distance_to(fp) > gap
        })
    }

    pub(crate) fn clear(&self, fp: &Footprint, keep: Keep) -> bool {
        self.inside(fp, 4.0)
            && (!keep.lines
                || !crate::site::on_line(&self.field.size, fp.x, fp.z, fp.bounding_radius()))
            && keep.gap.is_none_or(|gap| self.free(fp, gap))
            && self.woods_clear(fp)
            && self.crossings_clear(fp)
            && keep.road.is_none_or(|m| self.road_clear(fp, m))
            && keep.water.is_none_or(|m| self.water_clear(fp, m))
    }

    fn take(&mut self, fp: Footprint) {
        self.taken.push(fp);
    }

    // ----- buildings -----------------------------------------------------------------

    /// Kind and size of a dwelling of the landscape.
    fn dwelling(&mut self, profile: &LandscapeProfile, barn_share: f64) -> (HouseKind, f64, f64) {
        let s = &mut self.stream;
        if s.unit() < barn_share {
            return (HouseKind::Barn, s.range(12.0, 18.0), s.range(6.5, 8.5));
        }
        let kind = if s.unit() < profile.stone_share {
            HouseKind::Stone
        } else if s.unit()
            < match self.field.terrain {
                Terrain::Bocage | Terrain::Plains | Terrain::Forest => 0.5,
                _ => 0.3,
            }
        {
            HouseKind::Timbered
        } else {
            HouseKind::Cottage
        };
        (kind, s.range(8.0, 13.0), s.range(5.0, 6.5))
    }

    /// Adds a building if its footprint is clear; its index.
    fn try_building(&mut self, house: House, keep: Keep) -> Option<usize> {
        let fp = Footprint::new(house.x, house.z, house.length, house.width, house.yaw);
        if !self.clear(&fp, keep) {
            return None;
        }
        self.take(fp);
        self.decor.buildings.push(house);
        Some(self.decor.buildings.len() - 1)
    }

    /// Forgets the buildings laid from `first` on (a hamlet that failed).
    fn rollback(&mut self, first_building: usize, taken: usize) {
        self.decor.buildings.truncate(first_building);
        self.taken.truncate(taken);
    }

    /// Rectangle round `points` in the frame of `yaw`, grown by `grow`.
    fn enclosing(points: &[(f64, f64)], yaw: f64, grow: f64) -> (f64, f64, f64, f64) {
        let (c, s) = (yaw.cos(), yaw.sin());
        let (mut u0, mut u1, mut v0, mut v1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(x, z) in points {
            let (u, v) = (x * c + z * s, -x * s + z * c);
            u0 = u0.min(u);
            u1 = u1.max(u);
            v0 = v0.min(v);
            v1 = v1.max(v);
        }
        let (u, v) = ((u0 + u1) * 0.5, (v0 + v1) * 0.5);
        (
            u * c - v * s,
            u * s + v * c,
            u1 - u0 + 2.0 * grow,
            v1 - v0 + 2.0 * grow,
        )
    }

    fn corners_of(&self, indices: &[usize]) -> Vec<(f64, f64)> {
        indices
            .iter()
            .flat_map(|&i| {
                let b = &self.decor.buildings[i];
                Footprint::new(b.x, b.z, b.length, b.width, b.yaw).corners()
            })
            .collect()
    }

    /// Churchyard round the church `index`: area, graves.
    fn churchyard(&mut self, index: usize) {
        let b = self.decor.buildings[index];
        let m = self.rules.placement.churchyard_margin_m;
        let area = Area {
            kind: AreaKind::Church,
            x: b.x,
            z: b.z,
            length: b.length + 2.0 * m,
            width: b.width + 2.0 * m,
            yaw: b.yaw,
            state: None,
        };
        self.decor.areas.push(area);
        let size = self.rules.placement.prop_size_m.graves;
        let fp = b_footprint(&b);
        for side in [-1.0, 1.0] {
            let v = side * (b.width * 0.5 + m * 0.5);
            let u = self.stream.range(-b.length * 0.25, b.length * 0.25);
            let (x, z) = fp.world(u, v);
            self.decor.props.push(DecorProp {
                kind: DecorPropKind::Graves,
                x,
                z,
                yaw: b.yaw,
                length: size[0].min(b.length * 0.6),
                depth: size[1].min(m * 0.9),
                count: 0,
            });
        }
    }

    // ----- hamlets -------------------------------------------------------------------------

    /// A point along a road, clear of bridges, fords and the field edges:
    /// (point, heading, road index).
    fn road_spot(&mut self, main_only: bool) -> Option<RoadSpot> {
        let roads: Vec<usize> = self
            .field
            .roads
            .iter()
            .enumerate()
            .filter(|(_, r)| !main_only || r.kind == RoadKind::Main)
            .map(|(i, _)| i)
            .collect();
        if roads.is_empty() {
            return None;
        }
        let i = roads[self.stream.below(roads.len() as u32) as usize];
        let points = &self.field.roads[i].points;
        let total = arc_length(points);
        if total < 200.0 {
            return None;
        }
        let s = self.stream.range(total * 0.08, total * 0.92);
        let (p, dir) = point_at(points, s);
        Some((p, dir, i))
    }

    /// The point of the roads nearest `p`: (point, heading, road index).
    pub(crate) fn road_near(&self, p: (f64, f64)) -> Option<RoadSpot> {
        self.field
            .roads
            .iter()
            .enumerate()
            .map(|(i, r)| (hydro::nearest_on(&r.points, p), i))
            .min_by(|a, b| {
                (a.0 .0 - p.0)
                    .hypot(a.0 .1 - p.1)
                    .total_cmp(&(b.0 .0 - p.0).hypot(b.0 .1 - p.1))
            })
            .and_then(|(q, i)| {
                let dir = road_dir_near(&self.field.roads[i].points, q)?;
                Some((q, dir, i))
            })
    }

    /// Settlements already laid keep this far apart (m).
    fn settlement_clear(&self, p: (f64, f64), gap: f64) -> bool {
        self.decor
            .hamlets
            .iter()
            .all(|h| (h.x - p.0).hypot(h.z - p.1) > gap)
            && self
                .decor
                .camps
                .iter()
                .all(|c| c.area.footprint().signed_distance(p.0, p.1) > gap * 0.4)
    }

    /// Street hamlet along a road (Midi bastides, villages-rues).
    pub(crate) fn street_hamlet(
        &mut self,
        profile: &LandscapeProfile,
        at: Option<RoadSpot>,
        houses: u32,
    ) -> bool {
        let keep = Keep::building(self.rules);
        let Some((p, _, road)) = at.or_else(|| self.road_spot(false)) else {
            return false;
        };
        if at.is_none()
            && (!self.settlement_clear(p, 260.0)
                || crate::site::on_line(&self.field.size, p.0, p.1, 70.0))
        {
            return false;
        }
        let points = self.field.roads[road].points.clone();
        let road_width = self.field.roads[road].width;
        let s0 = arc_at(&points, p);
        let first = self.decor.buildings.len();
        let taken = self.taken.len();
        let mut built: Vec<usize> = Vec::new();
        let spacing = self.rules.placement.street_spacing_m;
        let margin = self.rules.placement.road_margin_m;
        let total = arc_length(&points);
        for direction in [1.0, -1.0] {
            let mut s = s0
                + if direction > 0.0 {
                    0.0
                } else {
                    -draw(spacing, &mut self.stream)
                };
            for _ in 0..(houses / 2 + 2) {
                if built.len() as u32 >= houses || s < 20.0 || s > total - 20.0 {
                    break;
                }
                let (q, d) = point_at(&points, s);
                let n = (-d.1, d.0);
                let heading = d.1.atan2(d.0);
                for side in [1.0, -1.0] {
                    if built.len() as u32 >= houses || self.stream.unit() < 0.12 {
                        continue;
                    }
                    let (kind, length, width) = self.dwelling(profile, 0.12);
                    let off = road_width * 0.5 + margin + width * 0.5 + self.stream.range(0.3, 2.5);
                    let house = House {
                        x: q.0 + n.0 * off * side,
                        z: q.1 + n.1 * off * side,
                        length,
                        width,
                        // The facade (front) faces the street.
                        yaw: heading + if side > 0.0 { PI } else { 0.0 },
                        kind,
                    };
                    if let Some(i) = self.try_building(house, keep) {
                        built.push(i);
                    }
                }
                s += direction * draw(spacing, &mut self.stream);
            }
        }
        if built.len() < 4 {
            self.rollback(first, taken);
            return false;
        }
        // The church at one end of the street, set back behind its yard.
        if self.stream.unit() < 0.6 {
            let end = self.stream.range(-1.0, 1.0).signum();
            let (q, d) = point_at(&points, (s0 + end * 70.0).clamp(20.0, total - 20.0));
            let n = (-d.1, d.0);
            let side = if self.stream.unit() < 0.5 { 1.0 } else { -1.0 };
            let (length, width) = (self.stream.range(16.0, 22.0), self.stream.range(7.0, 9.0));
            let m = self.rules.placement.churchyard_margin_m;
            let off = road_width * 0.5 + margin + width * 0.5 + m + 1.0;
            let church = House {
                x: q.0 + n.0 * off * side,
                z: q.1 + n.1 * off * side,
                length,
                width,
                yaw: d.1.atan2(d.0),
                kind: HouseKind::Church,
            };
            let yard = Footprint::new(
                church.x,
                church.z,
                length + 2.0 * m,
                width + 2.0 * m,
                church.yaw,
            );
            if self.clear(&yard, keep) {
                if let Some(i) = self.try_building(church, Keep { gap: None, ..keep }) {
                    self.take(yard);
                    built.push(i);
                    self.churchyard(i);
                }
            }
        }
        let (_, d) = point_at(&points, s0);
        let heading = d.1.atan2(d.0);
        let corners = self.corners_of(&built);
        let (x, z, length, width) = Self::enclosing(&corners, heading, 10.0);
        self.decor.areas.push(Area {
            kind: AreaKind::Hamlet,
            x,
            z,
            length,
            width,
            yaw: heading,
            state: None,
        });
        self.decor.hamlets.push(Hamlet {
            layout: HamletLayout::Street,
            x,
            z,
            yaw: heading,
            buildings: built,
        });
        self.props_round_houses(self.decor.hamlets.len() - 1);
        true
    }

    /// Hamlet grouped round its green and church; the road runs by or
    /// through the green.
    pub(crate) fn green_hamlet(
        &mut self,
        profile: &LandscapeProfile,
        at: Option<(f64, f64, f64)>,
        houses: u32,
    ) -> bool {
        let keep = Keep::building(self.rules);
        let (cx, cz, yaw) = match at {
            Some(a) => a,
            None => {
                let spot = self.road_spot(false);
                let (p, d) = match spot {
                    Some((p, d, _)) => (p, d),
                    None => (
                        (
                            self.stream.range(150.0, self.field.width - 150.0),
                            self.stream.range(120.0, self.field.depth - 120.0),
                        ),
                        (1.0, 0.0),
                    ),
                };
                let n = (-d.1, d.0);
                let off = self.stream.range(-15.0, 30.0);
                let c = (p.0 + n.0 * off, p.1 + n.1 * off);
                if !self.settlement_clear(c, 260.0)
                    || crate::site::on_line(&self.field.size, c.0, c.1, 80.0)
                {
                    return false;
                }
                (c.0, c.1, d.1.atan2(d.0))
            }
        };
        let radius = self.stream.range(45.0, 70.0);
        let (areas0, props0) = (self.decor.areas.len(), self.decor.props.len());
        let first = self.decor.buildings.len();
        let taken = self.taken.len();
        let mut built = Vec::new();
        // The church near the middle of the green, nave east-west.
        let m = self.rules.placement.churchyard_margin_m;
        for attempt in 0..8 {
            let a = self.stream.range(0.0, TAU);
            let r = radius * 0.15 * f64::from(attempt).min(3.0);
            let (length, width) = (self.stream.range(16.0, 22.0), self.stream.range(7.0, 9.0));
            let church = House {
                x: cx + a.cos() * r,
                z: cz + a.sin() * r,
                length,
                width,
                yaw: self.stream.range(-0.15, 0.15),
                kind: HouseKind::Church,
            };
            let yard = Footprint::new(
                church.x,
                church.z,
                length + 2.0 * m,
                width + 2.0 * m,
                church.yaw,
            );
            if self.clear(&yard, keep) {
                if let Some(i) = self.try_building(church, Keep { gap: None, ..keep }) {
                    self.take(yard);
                    built.push(i);
                    self.churchyard(i);
                }
                break;
            }
        }
        let mut tries = 0;
        while (built.len() as u32) < houses + 1 && tries < 160 {
            tries += 1;
            let angle = self.stream.range(0.0, TAU);
            let dist = radius * self.stream.range(0.5, 0.95);
            let (kind, length, width) = self.dwelling(profile, 0.2);
            let house = House {
                x: cx + angle.cos() * dist,
                z: cz + angle.sin() * dist,
                length,
                width,
                // Facade towards the green.
                yaw: angle + FRAC_PI_2 + self.stream.range(-0.25, 0.25),
                kind,
            };
            if let Some(i) = self.try_building(house, keep) {
                built.push(i);
            }
        }
        if built.len() < 4 {
            self.decor.areas.truncate(areas0);
            self.decor.props.truncate(props0);
            self.rollback(first, taken);
            return false;
        }
        let corners = self.corners_of(&built);
        let (x, z, length, width) = Self::enclosing(&corners, yaw, 10.0);
        self.decor.areas.push(Area {
            kind: AreaKind::Hamlet,
            x,
            z,
            length,
            width,
            yaw,
            state: None,
        });
        self.decor.hamlets.push(Hamlet {
            layout: HamletLayout::Green,
            x: cx,
            z: cz,
            yaw,
            buildings: built,
        });
        self.props_round_houses(self.decor.hamlets.len() - 1);
        true
    }

    /// A lone farm: farmhouse, barn and byre round a yard, a track to the
    /// nearest road.
    pub(crate) fn farmstead(
        &mut self,
        profile: &LandscapeProfile,
        at: Option<(f64, f64, f64)>,
        buildings: u32,
    ) -> bool {
        let keep = Keep::building(self.rules);
        let (cx, cz, yaw) = match at {
            Some(a) => a,
            None => {
                let c = (
                    self.stream.range(60.0, self.field.width - 60.0),
                    self.stream.range(60.0, self.field.depth - 60.0),
                );
                if !self.settlement_clear(c, 150.0)
                    || crate::site::on_line(&self.field.size, c.0, c.1, 45.0)
                {
                    return false;
                }
                (c.0, c.1, self.stream.range(0.0, TAU))
            }
        };
        let yard = Footprint::new(cx, cz, 44.0, 40.0, yaw);
        if !self.clear(
            &yard,
            Keep {
                gap: Some(6.0),
                ..keep
            },
        ) {
            return false;
        }
        let first = self.decor.buildings.len();
        let taken = self.taken.len();
        let fp = yard;
        let (farm_kind, _, _) = self.dwelling(profile, 0.0);
        let slots = [
            (0.0, -11.0, 0.0, farm_kind, (12.0, 15.0), 6.5),
            (-12.0, 3.0, FRAC_PI_2, HouseKind::Barn, (14.0, 18.0), 8.0),
            (12.0, 3.0, FRAC_PI_2, HouseKind::Barn, (9.0, 12.0), 5.5),
            (0.0, 16.0, 0.0, HouseKind::Cottage, (7.0, 7.0), 4.5),
        ];
        let mut built = Vec::new();
        for (k, &(u, v, turn, kind, (l0, l1), width)) in slots.iter().enumerate() {
            if k as u32 >= buildings.max(2) {
                break;
            }
            let (x, z) = fp.world(u, v);
            let house = House {
                x,
                z,
                length: self.stream.range(l0, l1),
                width,
                yaw: yaw + turn,
                kind,
            };
            if let Some(i) = self.try_building(
                house,
                Keep {
                    gap: Some(1.0),
                    ..keep
                },
            ) {
                built.push(i);
            }
        }
        if built.len() < 2 {
            self.rollback(first, taken);
            return false;
        }
        self.take(yard);
        self.decor.areas.push(Area {
            kind: AreaKind::Farmstead,
            x: cx,
            z: cz,
            length: 44.0,
            width: 40.0,
            yaw,
            state: None,
        });
        self.decor.hamlets.push(Hamlet {
            layout: HamletLayout::Farmstead,
            x: cx,
            z: cz,
            yaw,
            buildings: built,
        });
        // The track leaves from the side of the yard nearest the road.
        let gate = [(25.0, 0.0), (-25.0, 0.0), (0.0, 23.0), (0.0, -23.0)]
            .iter()
            .map(|&(u, v)| fp.world(u, v))
            .min_by(|a, b| self.road_gap(*a).total_cmp(&self.road_gap(*b)))
            .unwrap_or((cx, cz));
        self.track_to_road(gate, 22.0);
        self.props_round_houses(self.decor.hamlets.len() - 1);
        true
    }

    /// Woodpiles, carts and a well by the houses of hamlet `index`.
    fn props_round_houses(&mut self, index: usize) {
        let buildings = self.decor.hamlets[index].buildings.clone();
        let sizes = self.rules.placement.prop_size_m.clone();
        let keep = Keep {
            road: Some(1.0),
            water: Some(3.0),
            lines: false,
            gap: Some(1.0),
        };
        for (k, &i) in buildings.iter().enumerate() {
            let b = self.decor.buildings[i];
            if b.kind == HouseKind::Church || self.stream.unit() < 0.5 {
                continue;
            }
            let (kind, size) = match self.stream.below(4) {
                0 | 1 => (DecorPropKind::Woodpile, sizes.woodpile),
                2 => (DecorPropKind::Cart, sizes.cart),
                _ => (DecorPropKind::Haystack, sizes.haystack),
            };
            // Behind the house (the croft), away from the street.
            let fp = b_footprint(&b);
            let (x, z) = fp.world(
                self.stream.range(-b.length * 0.3, b.length * 0.3),
                -(b.width * 0.5 + size[1] * 0.5 + 1.2),
            );
            let prop = DecorProp {
                kind,
                x,
                z,
                yaw: b.yaw
                    + if kind == DecorPropKind::Cart {
                        self.stream.range(-0.4, 0.4)
                    } else {
                        0.0
                    },
                length: size[0],
                depth: size[1],
                count: 0,
            };
            if self.clear(&prop.footprint(), keep) {
                self.take(prop.footprint());
                self.decor.props.push(prop);
            }
            if k == 0 && self.stream.unit() < 0.6 {
                let (x, z) = fp.world(b.length * 0.5 + 4.0, b.width * 0.5 + 3.0);
                let well = DecorProp {
                    kind: DecorPropKind::Well,
                    x,
                    z,
                    yaw: 0.0,
                    length: sizes.well[0],
                    depth: sizes.well[1],
                    count: 0,
                };
                if self.clear(&well.footprint(), keep) {
                    self.take(well.footprint());
                    self.decor.props.push(well);
                }
            }
        }
    }

    /// A farm track from `from` to the nearest road when it is farther than
    /// `min` metres and the way is dry.
    /// Distance from `p` to the nearest road (infinite without roads).
    fn road_gap(&self, p: (f64, f64)) -> f64 {
        self.roads()
            .map(|r| hydro::polyline_distance(&r.points, p.0, p.1))
            .fold(f64::INFINITY, f64::min)
    }

    fn track_to_road(&mut self, from: (f64, f64), min: f64) {
        let nearest = self
            .roads()
            .map(|r| hydro::nearest_on(&r.points, from))
            .min_by(|a, b| {
                (a.0 - from.0)
                    .hypot(a.1 - from.1)
                    .total_cmp(&(b.0 - from.0).hypot(b.1 - from.1))
            });
        let Some(to) = nearest else {
            return;
        };
        let d = (to.0 - from.0).hypot(to.1 - from.1);
        if d < min || d > 450.0 {
            return;
        }
        let mid = (
            (from.0 + to.0) * 0.5 + self.stream.range(-0.08, 0.08) * d,
            (from.1 + to.1) * 0.5 + self.stream.range(-0.08, 0.08) * d,
        );
        let points = vec![from, mid, to];
        let dry = points.windows(2).all(|w| {
            let n = (((w[1].0 - w[0].0).hypot(w[1].1 - w[0].1) / 5.0).ceil() as usize).max(1);
            (0..=n).all(|k| {
                let t = k as f64 / n as f64;
                let (x, z) = (
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                );
                self.field.water_kind(x, z).is_none()
                    && self.field.bridge_at(x, z).is_none()
                    && !self.field.in_forest(x, z)
            })
        });
        let free = points.windows(2).all(|w| {
            self.taken
                .iter()
                .all(|t| t.distance_to_segment(w[0], w[1]) > 2.5)
        });
        if dry && free {
            self.tracks.push(Road {
                kind: RoadKind::Track,
                points,
                width: hydro::WaterRules::bundled().roads.track_width_m,
            });
        }
    }

    // ----- landmarks ------------------------------------------------------------------------

    /// Post mill on a hilltop, or on a mound raised for it.
    pub(crate) fn windmill(&mut self, at: Option<(f64, f64, f64)>, mound: Option<bool>) -> bool {
        let keep = Keep::building(self.rules);
        let (x, z, yaw) = match at {
            Some(a) => a,
            None => {
                let mut best: Option<((f64, f64), f64)> = None;
                for _ in 0..16 {
                    let p = (
                        self.stream.range(60.0, self.field.width - 60.0),
                        self.stream.range(60.0, self.field.depth - 60.0),
                    );
                    let fp = Footprint::new(p.0, p.1, 24.0, 24.0, 0.0);
                    if !self.clear(&fp, keep) || !self.settlement_clear(p, 80.0) {
                        continue;
                    }
                    let h = self.field.height(p.0, p.1);
                    if best.is_none_or(|(_, bh)| h > bh) {
                        best = Some((p, h));
                    }
                }
                let Some((p, _)) = best else {
                    return false;
                };
                (p.0, p.1, self.stream.range(0.0, TAU))
            }
        };
        let mill = House {
            x,
            z,
            length: 7.0,
            width: 7.0,
            yaw,
            kind: HouseKind::Windmill,
        };
        let placed = if at.is_some() {
            self.take(b_footprint(&mill));
            self.decor.buildings.push(mill);
            true
        } else {
            self.try_building(mill, keep).is_some()
        };
        if !placed {
            return false;
        }
        // A mound unless the mill already stands above its surroundings.
        let raise = mound.unwrap_or_else(|| {
            let h = self.field.height(x, z);
            let ring = (0..8)
                .map(|k| {
                    let a = f64::from(k) * TAU / 8.0;
                    self.field.height(x + a.cos() * 60.0, z + a.sin() * 60.0)
                })
                .sum::<f64>()
                / 8.0;
            h - ring < 2.0
        });
        if raise {
            let p = &self.rules.placement;
            self.decor.mounds.push(Mound {
                x,
                z,
                radius: p.mound_radius_m,
                height: p.mound_height_m,
            });
        }
        true
    }

    /// Water mill on the bank of the river or a brook, wheel on the water.
    pub(crate) fn watermill(&mut self, at: Option<(f64, f64, f64)>) -> bool {
        let (x, z, yaw) = match at {
            Some(a) => a,
            None => {
                let near = self
                    .decor
                    .hamlets
                    .first()
                    .map_or((self.field.width * 0.5, self.field.depth * 0.5), |h| {
                        (h.x, h.z)
                    });
                let near = (
                    near.0 + self.stream.range(-200.0, 200.0),
                    near.1 + self.stream.range(-150.0, 150.0),
                );
                let Some(spot) = self.field.waterside_spot(near, 5.0) else {
                    return false;
                };
                // Front (-sin yaw, cos yaw) towards the water.
                let (tx, tz) = spot.towards_water;
                (spot.x, spot.z, (-tx).atan2(tz))
            }
        };
        let mill = House {
            x,
            z,
            length: 11.0,
            width: 7.0,
            yaw,
            kind: HouseKind::Watermill,
        };
        let keep = Keep {
            water: None,
            ..Keep::building(self.rules)
        };
        let fp = b_footprint(&mill);
        let centre_dry = self.field.water_kind(x, z).is_none();
        if at.is_none() && (!centre_dry || !self.clear(&fp, keep)) {
            return false;
        }
        self.take(fp);
        self.decor.buildings.push(mill);
        // The track reaches the back of the mill, away from the water.
        let back = fp.world(0.0, -(fp.half_depth + 3.0));
        self.track_to_road(back, 15.0);
        true
    }

    /// Manor (tower house) with its yard, a barn, a well, and a moat.
    pub(crate) fn manor(&mut self, at: Option<(f64, f64, f64)>, moat: bool) -> bool {
        let p = self.rules.placement.clone();
        let (length, width) = (20.5, 8.0);
        let (inner_l, inner_w) = (length + 26.0, width + 30.0);
        let ring = if moat { p.moat_width_m } else { 0.0 };
        let (outer_l, outer_w) = (inner_l + 2.0 * ring, inner_w + 2.0 * ring);
        let (x, z, yaw) = match at {
            Some(a) => a,
            None => {
                let mut found = None;
                for _ in 0..24 {
                    let anchor = if self.decor.hamlets.is_empty() || self.stream.unit() < 0.3 {
                        (
                            self.stream.range(80.0, self.field.width - 80.0),
                            self.stream.range(80.0, self.field.depth - 80.0),
                        )
                    } else {
                        let h = &self.decor.hamlets
                            [self.stream.below(self.decor.hamlets.len() as u32) as usize];
                        let a = self.stream.range(0.0, TAU);
                        let d = self.stream.range(120.0, 260.0);
                        (h.x + a.cos() * d, h.z + a.sin() * d)
                    };
                    let yaw = self.stream.range(0.0, TAU);
                    let fp = Footprint::new(
                        anchor.0,
                        anchor.1,
                        outer_l + 2.0 * p.moat_margin_m,
                        outer_w + 2.0 * p.moat_margin_m,
                        yaw,
                    );
                    if self.clear(&fp, Keep::building(self.rules))
                        && self.settlement_clear(anchor, 120.0)
                    {
                        found = Some((anchor.0, anchor.1, yaw));
                        break;
                    }
                }
                let Some(f) = found else {
                    return false;
                };
                f
            }
        };
        let outer = Footprint::new(x, z, outer_l, outer_w, yaw);
        self.take(outer);
        let (hx, hz) = outer.world(0.0, -inner_w * 0.5 + width * 0.5 + 4.0);
        self.decor.buildings.push(House {
            x: hx,
            z: hz,
            length,
            width,
            yaw,
            kind: HouseKind::Manor,
        });
        let (bx, bz) = outer.world(inner_l * 0.5 - 8.0, 6.0);
        self.decor.buildings.push(House {
            x: bx,
            z: bz,
            length: 14.0,
            width: 7.0,
            yaw: yaw + FRAC_PI_2,
            kind: HouseKind::Barn,
        });
        let (wx, wz) = outer.world(-6.0, 7.0);
        let well = p.prop_size_m.well;
        self.decor.props.push(DecorProp {
            kind: DecorPropKind::Well,
            x: wx,
            z: wz,
            yaw: 0.0,
            length: well[0],
            depth: well[1],
            count: 0,
        });
        self.decor.areas.push(Area {
            kind: AreaKind::Manor,
            x,
            z,
            length: outer_l,
            width: outer_w,
            yaw,
            state: None,
        });
        if moat {
            self.decor.moats.push(Moat {
                x,
                z,
                length: outer_l,
                width: outer_w,
                yaw,
                ring,
            });
        }
        let gate = outer.world(0.0, outer_w * 0.5 + 3.5);
        self.track_to_road(gate, 20.0);
        true
    }

    /// Parish church standing alone in its churchyard.
    pub(crate) fn church(&mut self, x: f64, z: f64, yaw: f64) -> bool {
        let church = House {
            x,
            z,
            length: 19.0,
            width: 8.0,
            yaw,
            kind: HouseKind::Church,
        };
        let m = self.rules.placement.churchyard_margin_m;
        self.take(Footprint::new(x, z, 19.0 + 2.0 * m, 8.0 + 2.0 * m, yaw));
        self.decor.buildings.push(church);
        let i = self.decor.buildings.len() - 1;
        self.churchyard(i);
        true
    }

    // ----- plots ------------------------------------------------------------------------------

    fn plot_size(&self, kind: AreaKind) -> Span {
        let s = &self.rules.placement.plot_size_m;
        match kind {
            AreaKind::Vineyard => s.vineyard,
            AreaKind::Orchard => s.orchard,
            AreaKind::Meadow => s.meadow,
            _ => s.ploughland,
        }
    }

    fn field_state(&mut self) -> FieldState {
        let w = &self.rules.seasons.of(self.field.season).field_states;
        let total = w.ploughed + w.sown + w.crop + w.stubble;
        let mut roll = self.stream.unit() * total.max(1e-9);
        for (state, weight) in [
            (FieldState::Ploughed, w.ploughed),
            (FieldState::Sown, w.sown),
            (FieldState::Crop, w.crop),
            (FieldState::Stubble, w.stubble),
        ] {
            if roll < weight {
                return state;
            }
            roll -= weight;
        }
        FieldState::Ploughed
    }

    /// A plot of `kind` somewhere suitable (vineyards on slopes, orchards
    /// near the houses, meadows low by the water); its area index.
    pub(crate) fn plot(&mut self, kind: AreaKind) -> Option<usize> {
        let size = self.plot_size(kind);
        let keep = Keep {
            road: Some(self.rules.placement.road_margin_m),
            water: Some(self.rules.placement.water_margin_m),
            lines: !matches!(kind, AreaKind::Ploughland | AreaKind::Meadow),
            gap: Some(4.0),
        };
        let mut best: Option<(Footprint, f64)> = None;
        let tries = 10;
        for _ in 0..tries {
            let length = draw(size, &mut self.stream);
            let width = length * self.stream.range(0.5, 0.85);
            let (x, z) = if kind == AreaKind::Orchard && !self.decor.hamlets.is_empty() {
                let h = &self.decor.hamlets
                    [self.stream.below(self.decor.hamlets.len() as u32) as usize];
                let a = self.stream.range(0.0, TAU);
                let d = self.stream.range(50.0, 130.0);
                (h.x + a.cos() * d, h.z + a.sin() * d)
            } else {
                (
                    self.stream
                        .range(length * 0.5, self.field.width - length * 0.5),
                    self.stream
                        .range(length * 0.5, self.field.depth - length * 0.5),
                )
            };
            // Plots follow the nearest road, else the lie of the land.
            let road_dir = self
                .roads()
                .map(|r| {
                    let q = hydro::nearest_on(&r.points, (x, z));
                    ((q.0 - x).hypot(q.1 - z), road_dir_near(&r.points, q))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .filter(|(d, _)| *d < 250.0)
                .and_then(|(_, dir)| dir);
            let yaw = match road_dir {
                Some(dir) => dir.1.atan2(dir.0) + self.stream.range(-0.08, 0.08),
                None => self.stream.range(-0.4, 0.4),
            };
            let fp = Footprint::new(x, z, length, width, yaw);
            if !self.clear(&fp, keep) {
                continue;
            }
            let score = match kind {
                AreaKind::Vineyard => self.slope_at(x, z),
                AreaKind::Meadow => -self.field.height(x, z),
                _ => 0.0,
            };
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((fp, score));
            }
            if !matches!(kind, AreaKind::Vineyard | AreaKind::Meadow) {
                break;
            }
        }
        let (fp, _) = best?;
        Some(self.lay_plot(kind, fp, None))
    }

    /// Lays a plot on `fp` (ploughland as parallel strips of various
    /// states); index of its (first) area.
    pub(crate) fn lay_plot(
        &mut self,
        kind: AreaKind,
        fp: Footprint,
        state: Option<FieldState>,
    ) -> usize {
        self.take(fp);
        let first = self.decor.areas.len();
        if kind == AreaKind::Ploughland && state.is_none() {
            let strips = 2 + self.stream.below(3) as usize;
            let w = fp.half_depth * 2.0 / strips as f64;
            for k in 0..strips {
                let v = -fp.half_depth + w * (k as f64 + 0.5);
                let (x, z) = fp.world(0.0, v);
                let state = Some(self.field_state());
                self.decor.areas.push(Area {
                    kind,
                    x,
                    z,
                    length: fp.half_length * 2.0,
                    width: w,
                    yaw: fp.yaw,
                    state,
                });
            }
        } else {
            self.decor.areas.push(Area {
                kind,
                x: fp.x,
                z: fp.z,
                length: fp.half_length * 2.0,
                width: fp.half_depth * 2.0,
                yaw: fp.yaw,
                state: if kind == AreaKind::Ploughland {
                    state
                } else {
                    None
                },
            });
        }
        if kind == AreaKind::Meadow {
            let season = self.rules.seasons.of(self.field.season);
            let n = draw_count(season.haystacks_per_meadow, &mut self.stream);
            let size = self.rules.placement.prop_size_m.haystack;
            for _ in 0..n {
                let (x, z) = fp.world(
                    self.stream
                        .range(-fp.half_length * 0.7, fp.half_length * 0.7),
                    self.stream.range(-fp.half_depth * 0.7, fp.half_depth * 0.7),
                );
                let stack = DecorProp {
                    kind: DecorPropKind::Haystack,
                    x,
                    z,
                    yaw: self.stream.range(0.0, TAU),
                    length: size[0],
                    depth: size[1],
                    count: 0,
                };
                if self
                    .decor
                    .props
                    .iter()
                    .all(|p| p.footprint().distance_to(&stack.footprint()) > 3.0)
                {
                    self.decor.props.push(stack);
                }
            }
        }
        first
    }

    fn slope_at(&self, x: f64, z: f64) -> f64 {
        let d = 20.0;
        let f = self.field;
        let gx = f.height(x + d, z) - f.height(x - d, z);
        let gz = f.height(x, z + d) - f.height(x, z - d);
        gx.hypot(gz) / (2.0 * d)
    }

    /// Hedges round a plot, with a gate gap on each side (bocage).
    fn hedge_round(&mut self, fp: &Footprint) {
        let c = fp.corners();
        for k in 0..4 {
            if self.stream.unit() < 0.2 {
                continue;
            }
            let (a, b) = (c[k], c[(k + 1) % 4]);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if len < 30.0 {
                continue;
            }
            let gate = self.stream.range(0.2, 0.8);
            let half = 5.0 / len;
            for (t0, t1) in [(0.0, gate - half), (gate + half, 1.0)] {
                let p = (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0);
                let q = (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1);
                let o = Obstacle {
                    a: p,
                    b: q,
                    kind: ObstacleKind::Hedge,
                };
                if o.length() > 12.0 {
                    self.hedges.push(o);
                }
            }
        }
    }

    /// Carts left by the roads and in the fields.
    fn carts(&mut self, count: u32) {
        let size = self.rules.placement.prop_size_m.cart;
        let keep = Keep {
            road: Some(1.0),
            water: Some(3.0),
            lines: true,
            gap: Some(1.5),
        };
        for _ in 0..count {
            for _ in 0..6 {
                let Some((p, d, road)) = self.road_spot(false) else {
                    return;
                };
                let n = (-d.1, d.0);
                let side = if self.stream.unit() < 0.5 { 1.0 } else { -1.0 };
                let off = self.field.roads[road].width * 0.5 + size[1] * 0.5 + 1.8;
                let cart = DecorProp {
                    kind: DecorPropKind::Cart,
                    x: p.0 + n.0 * off * side,
                    z: p.1 + n.1 * off * side,
                    yaw: d.1.atan2(d.0) + self.stream.range(-0.3, 0.3),
                    length: size[0],
                    depth: size[1],
                    count: 0,
                };
                if self.clear(&cart.footprint(), keep) {
                    self.take(cart.footprint());
                    self.decor.props.push(cart);
                    break;
                }
            }
        }
    }

    // ----- camps -------------------------------------------------------------------------------

    /// The camp of `side`: behind its deployment zone (or at `at`), off the
    /// roads and the water; its tents, pavilions, wagons, fires and horse
    /// lines, and the baggage train parked along the nearest road.
    pub(crate) fn camp(&mut self, side: SideId, at: Option<(f64, f64, f64)>) -> bool {
        let cr = self.rules.camp.clone();
        let sx = self.field.size.sx();
        let (w, d) = (cr.width_m * sx, cr.depth_m);
        let back = if side == SideId::Attacker {
            0.0
        } else {
            self.field.depth
        };
        let toward_enemy = if side == SideId::Attacker { 1.0 } else { -1.0 };
        // Front (-sin yaw, cos yaw) faces the enemy.
        let default_yaw = if side == SideId::Attacker { 0.0 } else { PI };
        let keep = Keep {
            road: Some(4.0),
            water: Some(self.rules.placement.water_margin_m),
            lines: false,
            gap: Some(6.0),
        };
        let fp = match at {
            Some((x, z, yaw)) => Footprint::new(x, z, w, d, yaw),
            None => {
                let mut found = None;
                'search: for (scale, forward) in [(1.0, 0.0), (0.75, 25.0), (0.55, 60.0)] {
                    let z = back + toward_enemy * (cr.setback_m + forward);
                    for k in 0..13 {
                        let off = if k == 0 {
                            0.0
                        } else {
                            let step = ((k + 1) / 2) as f64 * 0.06;
                            if k % 2 == 1 {
                                step
                            } else {
                                -step
                            }
                        };
                        let x = self.field.width * (0.5 + off);
                        let fp = Footprint::new(x, z, w * scale, d * scale.max(0.8), default_yaw);
                        if self.clear(&fp, keep) {
                            found = Some(fp);
                            break 'search;
                        }
                    }
                }
                let Some(fp) = found else {
                    return false;
                };
                fp
            }
        };
        self.take(fp);
        let mut items = Vec::new();
        let sizes = self.rules.placement.prop_size_m.clone();
        let item_keep = Keep {
            road: Some(1.5),
            water: Some(4.0),
            lines: false,
            gap: None,
        };
        let (hl, hd) = (fp.half_length, fp.half_depth);
        let scale_count = |n: u32| ((f64::from(n) * sx).round() as u32).max(n.min(1));
        let spot = CampSpot {
            camp: fp,
            keep: item_keep,
        };
        // Pavilions of the lords at the back, in the middle.
        let n = scale_count(draw_count(cr.pavilions, &mut self.stream));
        for k in 0..n {
            let u = (f64::from(k) - f64::from(n.saturating_sub(1)) * 0.5) * 9.0
                + self.stream.range(-1.5, 1.5);
            let item = spot.item(
                DecorPropKind::Pavilion,
                (u, -hd * 0.3),
                0.0,
                sizes.pavilion,
                0,
            );
            self.camp_item(&spot, item, &mut items);
        }
        // Rows of tents, fires between them.
        let n = scale_count(draw_count(cr.tents, &mut self.stream));
        let fires = scale_count(draw_count(cr.fires, &mut self.stream));
        for k in 0..n {
            let row = (k % 3) as f64;
            let u = self.stream.range(-hl * 0.85, hl * 0.85);
            let v = hd * (0.25 - row * 0.28) + self.stream.range(-2.0, 2.0);
            let turn = if self.stream.unit() < 0.3 {
                FRAC_PI_2
            } else {
                0.0
            };
            let yaw = self.stream.range(-0.2, 0.2) + turn;
            let item = spot.item(DecorPropKind::Tent, (u, v), yaw, sizes.tent, 0);
            self.camp_item(&spot, item, &mut items);
        }
        for _ in 0..fires {
            let u = self.stream.range(-hl * 0.8, hl * 0.8);
            let v = self.stream.range(-hd * 0.5, hd * 0.5);
            let yaw = self.stream.range(0.0, TAU);
            let item = spot.item(DecorPropKind::Campfire, (u, v), yaw, sizes.campfire, 0);
            self.camp_item(&spot, item, &mut items);
        }
        // The wagon laager along the back edge.
        let n = scale_count(draw_count(cr.wagons, &mut self.stream));
        for k in 0..n {
            let u = -hl * 0.85 + (hl * 1.7) * (f64::from(k) + 0.5) / f64::from(n.max(1));
            let yaw = self.stream.range(-0.15, 0.15);
            let item = spot.item(DecorPropKind::Wagon, (u, -hd + 3.0), yaw, sizes.wagon, 0);
            self.camp_item(&spot, item, &mut items);
        }
        // Horse lines on the flanks, along the depth.
        let n = scale_count(draw_count(cr.horse_lines, &mut self.stream));
        for k in 0..n {
            let flank = if k % 2 == 0 { 1.0 } else { -1.0 };
            let u = flank * (hl - 6.0 - (k / 2) as f64 * 7.0);
            let horses = draw_count(cr.horses_per_line, &mut self.stream);
            let length = f64::from(horses) * 1.6 + 3.0;
            let item = spot.item(
                DecorPropKind::HorseLine,
                (u, 0.0),
                FRAC_PI_2,
                [length, 3.0],
                horses,
            );
            self.camp_item(&spot, item, &mut items);
        }
        let wagons = scale_count(draw_count(cr.convoy_wagons, &mut self.stream));
        let convoy = self.convoy(&fp, toward_enemy, wagons, sizes.wagon);
        for p in &convoy {
            self.take(p.footprint());
        }
        self.decor.camps.retain(|c| c.side != side);
        self.decor.camps.push(Camp {
            side,
            area: Area {
                kind: AreaKind::Camp,
                x: fp.x,
                z: fp.z,
                length: fp.half_length * 2.0,
                width: fp.half_depth * 2.0,
                yaw: fp.yaw,
                state: None,
            },
            items,
            convoy,
        });
        true
    }

    /// Adds a camp item when it keeps clear of the roads, the water and the
    /// other items.
    fn camp_item(&self, spot: &CampSpot, item: DecorProp, items: &mut Vec<DecorProp>) {
        let pf = item.footprint();
        if self.clear(&pf, spot.keep) && items.iter().all(|o| o.footprint().distance_to(&pf) > 1.2)
        {
            items.push(item);
        }
    }

    /// The baggage train: wagons parked beside the road nearest the camp,
    /// behind it (else in a column on its flank).
    fn convoy(
        &mut self,
        camp: &Footprint,
        toward_enemy: f64,
        count: u32,
        size: Span,
    ) -> Vec<DecorProp> {
        let keep = Keep {
            road: Some(0.8),
            water: Some(4.0),
            lines: false,
            gap: Some(1.0),
        };
        let mut out: Vec<DecorProp> = Vec::new();
        let centre = (camp.x, camp.z);
        let road = self
            .roads()
            .filter(|r| r.kind == RoadKind::Main)
            .map(|r| (hydro::nearest_on(&r.points, centre), r))
            .min_by(|a, b| {
                (a.0 .0 - centre.0)
                    .hypot(a.0 .1 - centre.1)
                    .total_cmp(&(b.0 .0 - centre.0).hypot(b.0 .1 - centre.1))
            })
            .filter(|(q, _)| (q.0 - centre.0).hypot(q.1 - centre.1) < 260.0)
            .map(|(_, r)| r.clone());
        if let Some(road) = road {
            // Walk the road from its point nearest the camp, away from the enemy.
            let total = arc_length(&road.points);
            let q = hydro::nearest_on(&road.points, centre);
            let s0 = arc_at(&road.points, q);
            let (_, dir) = point_at(&road.points, s0);
            let sign = if dir.1 * toward_enemy > 0.0 {
                -1.0
            } else {
                1.0
            };
            let side = {
                let n = (-dir.1, dir.0);
                if (camp.x - q.0) * n.0 + (camp.z - q.1) * n.1 >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            };
            let mut s = s0;
            let mut steps = 0;
            while (out.len() as u32) < count && steps < count * 3 {
                steps += 1;
                s += sign * 8.5;
                if s < 5.0 || s > total - 5.0 {
                    break;
                }
                let (p, d) = point_at(&road.points, s);
                let n = (-d.1, d.0);
                let off = road.width * 0.5 + size[1] * 0.5 + 1.2;
                let wagon = DecorProp {
                    kind: DecorPropKind::Wagon,
                    x: p.0 + n.0 * off * side,
                    z: p.1 + n.1 * off * side,
                    yaw: d.1.atan2(d.0),
                    length: size[0],
                    depth: size[1],
                    count: 0,
                };
                let fp = wagon.footprint();
                if self.clear(&fp, keep) && out.iter().all(|o| o.footprint().distance_to(&fp) > 1.0)
                {
                    out.push(wagon);
                }
            }
        }
        if out.is_empty() {
            // A column on the flank of the camp.
            for k in 0..count {
                let (x, z) = camp.world(
                    camp.half_length + 6.0,
                    camp.half_depth - 4.0 - f64::from(k) * 7.0,
                );
                let wagon = DecorProp {
                    kind: DecorPropKind::Wagon,
                    x,
                    z,
                    yaw: camp.yaw,
                    length: size[0],
                    depth: size[1],
                    count: 0,
                };
                if self.clear(&wagon.footprint(), keep) {
                    out.push(wagon);
                }
            }
        }
        out
    }

    // ----- the whole countryside ----------------------------------------------------------------

    /// Lays the procedural decor of a landscape.
    pub(crate) fn lay(&mut self, profile_key: &str) {
        let rules = self.rules;
        let Some(profile) = rules.profiles.get(profile_key).cloned() else {
            return;
        };
        let season = rules.seasons.of(self.field.season).clone();
        self.decor.profile = profile_key.to_owned();
        self.decor.vines_leafy = season.vines_leafy;
        self.decor.orchard_blossom = season.orchard_blossom;
        for side in SideId::BOTH {
            self.camp(side, None);
        }
        let hamlets = self.scaled_count(profile.hamlets);
        for _ in 0..hamlets {
            let houses = draw_count(rules.placement.hamlet_houses, &mut self.stream);
            let street = !self.field.roads.is_empty() && self.stream.unit() < profile.street_share;
            for _ in 0..8 {
                let done = if street {
                    self.street_hamlet(&profile, None, houses)
                } else {
                    self.green_hamlet(&profile, None, houses)
                };
                if done {
                    break;
                }
            }
        }
        let farms = self.scaled_count(profile.farmsteads);
        for _ in 0..farms {
            let buildings = draw_count(rules.placement.farmstead_houses, &mut self.stream);
            for _ in 0..10 {
                if self.farmstead(&profile, None, buildings) {
                    break;
                }
            }
        }
        for _ in 0..self.scaled_chance(profile.manor_chance) {
            let moat = self.stream.unit() < profile.moat_chance;
            self.manor(None, moat);
        }
        for _ in 0..self.scaled_chance(profile.windmill_chance) {
            self.windmill(None, None);
        }
        if (self.field.river.is_some() || !self.field.streams.is_empty())
            && self.stream.unit() < profile.watermill_chance
        {
            for _ in 0..4 {
                if self.watermill(None) {
                    break;
                }
            }
        }
        for (kind, span) in [
            (AreaKind::Vineyard, profile.vineyards),
            (AreaKind::Orchard, profile.orchards),
            (AreaKind::Meadow, profile.meadows),
            (AreaKind::Ploughland, profile.ploughland),
        ] {
            let n = self.scaled_count(span);
            for _ in 0..n {
                let Some(first) = self.plot(kind) else {
                    continue;
                };
                if profile.hedgerows
                    && self.field.terrain != Terrain::Bocage
                    && matches!(kind, AreaKind::Orchard | AreaKind::Meadow)
                {
                    let a = self.decor.areas[first];
                    self.hedge_round(&a.footprint());
                }
            }
        }
        let carts = draw_count(season.carts, &mut self.stream);
        let carts = (f64::from(carts) * self.field.size.area_ratio()).round() as u32;
        self.carts(carts);
    }
}

/// Where camp items go: the camp's rectangle and what they keep clear of.
struct CampSpot {
    camp: Footprint,
    keep: Keep,
}

impl CampSpot {
    /// An item at (u, v) in the camp's frame, turned by `yaw`.
    fn item(
        &self,
        kind: DecorPropKind,
        (u, v): (f64, f64),
        yaw: f64,
        size: Span,
        count: u32,
    ) -> DecorProp {
        let (x, z) = self.camp.world(u, v);
        DecorProp {
            kind,
            x,
            z,
            yaw: self.camp.yaw + yaw,
            length: size[0],
            depth: size[1],
            count,
        }
    }
}

fn b_footprint(b: &House) -> Footprint {
    Footprint::new(b.x, b.z, b.length, b.width, b.yaw)
}

/// Unit direction of the polyline at its point nearest `q`.
fn road_dir_near(points: &[(f64, f64)], q: (f64, f64)) -> Option<(f64, f64)> {
    points
        .windows(2)
        .min_by(|a, b| {
            hydro::polyline_distance(a, q.0, q.1).total_cmp(&hydro::polyline_distance(b, q.0, q.1))
        })
        .and_then(|w| {
            let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len = dx.hypot(dz);
            (len > 1e-9).then(|| (dx / len, dz / len))
        })
}

/// Raises the mounds of the windmills into the height grid.
pub(crate) fn raise_mounds(field: &mut Battlefield, mounds: &[Mound]) {
    for m in mounds {
        let r = m.radius;
        let (ix0, ix1) = (
            ((m.x - r) / field.resolution).floor().max(0.0) as usize,
            (((m.x + r) / field.resolution).ceil() as usize).min(field.nx - 1),
        );
        let (iz0, iz1) = (
            ((m.z - r) / field.resolution).floor().max(0.0) as usize,
            (((m.z + r) / field.resolution).ceil() as usize).min(field.nz - 1),
        );
        for iz in iz0..=iz1 {
            for ix in ix0..=ix1 {
                let (x, z) = (ix as f64 * field.resolution, iz as f64 * field.resolution);
                let d = (x - m.x).hypot(z - m.z) / r;
                if d >= 1.0 {
                    continue;
                }
                // Flat top, smooth flanks.
                let t = ((d - 0.3) / 0.7).clamp(0.0, 1.0);
                let bump = 0.5 * (1.0 + (t * PI).cos());
                field.heights[iz * field.nx + ix] += m.height * bump;
            }
        }
    }
}
