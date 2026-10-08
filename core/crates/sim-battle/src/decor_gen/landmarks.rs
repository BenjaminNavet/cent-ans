//! Decor layout: single landmarks: windmill, watermill, manor and church.

use super::*;

impl<'a> Layout<'a> {
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
}
