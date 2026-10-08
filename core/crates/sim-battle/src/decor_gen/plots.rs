//! Decor layout: plots of land (orchards, vineyards, fields), hedges and carts.

use super::*;

impl<'a> Layout<'a> {
    // ----- plots ------------------------------------------------------------------------------

    pub(super) fn plot_size(&self, kind: AreaKind) -> Span {
        let s = &self.rules.placement.plot_size_m;
        match kind {
            AreaKind::Vineyard => s.vineyard,
            AreaKind::Orchard => s.orchard,
            AreaKind::Meadow => s.meadow,
            _ => s.ploughland,
        }
    }

    pub(super) fn field_state(&mut self) -> FieldState {
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

    pub(super) fn slope_at(&self, x: f64, z: f64) -> f64 {
        let d = 20.0;
        let f = self.field;
        let gx = f.height(x + d, z) - f.height(x - d, z);
        let gz = f.height(x, z + d) - f.height(x, z - d);
        gx.hypot(gz) / (2.0 * d)
    }

    /// Hedges round a plot, with a gate gap on each side (bocage).
    pub(super) fn hedge_round(&mut self, fp: &Footprint) {
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
    pub(super) fn carts(&mut self, count: u32) {
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
}
