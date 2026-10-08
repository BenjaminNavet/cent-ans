//! Decor layout: camps and baggage trains.

use super::*;

impl<'a> Layout<'a> {
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
    pub(super) fn convoy(
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
