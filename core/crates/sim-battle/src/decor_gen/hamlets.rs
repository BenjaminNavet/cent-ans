//! Decor layout: hamlets, farmsteads and the tracks that serve them.

use super::*;

impl<'a> Layout<'a> {
    // ----- hamlets -------------------------------------------------------------------------

    /// A point along a road, clear of bridges, fords and the field edges:
    /// (point, heading, road index).
    pub(super) fn road_spot(&mut self, main_only: bool) -> Option<RoadSpot> {
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
    pub(super) fn settlement_clear(&self, p: (f64, f64), gap: f64) -> bool {
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
    pub(super) fn props_round_houses(&mut self, index: usize) {
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
    pub(super) fn road_gap(&self, p: (f64, f64)) -> f64 {
        self.roads()
            .map(|r| hydro::polyline_distance(&r.points, p.0, p.1))
            .fold(f64::INFINITY, f64::min)
    }

    pub(super) fn track_to_road(&mut self, from: (f64, f64), min: f64) {
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
}
