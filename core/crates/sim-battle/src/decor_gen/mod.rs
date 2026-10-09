//! Layout of the decor (lot EP6, see [`crate::decor`]): the procedural
//! countryside of a province and the hand placement of the historical maps
//! share the same checks — inside the field, off deep water, bridges, fords
//! and roads, off the woods and the village, clear of what is already laid.

mod camps;
mod hamlets;
mod landmarks;
mod plots;

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
