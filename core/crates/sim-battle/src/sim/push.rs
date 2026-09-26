//! EP11 (ADR 0070): continuous push of the lines in melee, resolved once per
//! tick just before the melee blows (rules and geometry in
//! [`crate::push`]).
//!
//! Deterministic: pairs in index order, displacements summed then applied in
//! index order, no random draw. Cost: one pass over the contact pairs, plus,
//! for each regiment giving ground, one pass over its side to find a friend
//! in the way.

use super::{BattleSim, DT};
use crate::push::{opponent_span, to_local, PushRules, PushShape};
use crate::unit::{Unit, UnitState};
use data_model::UnitCategory;

/// Can this regiment push or be pushed?
fn engaged(unit: &Unit) -> bool {
    unit.present()
        && unit.state != UnitState::Routing
        && !unit.withdrawing
        && !unit.disengaging
        && !unit.on_wall
        && unit.climbing.is_none()
        && !unit.synthetic
        && unit.category != UnitCategory::Siege
}

/// Gap between the rectangles of `a` moved by `delta` and `b` (same test as
/// the contacts and the friendly separation).
fn gap_after(a: &Unit, delta: (f64, f64), b: &Unit) -> f64 {
    let (ax, az) = (a.x + delta.0, a.z + delta.1);
    let (dx, dz) = (b.x - ax, b.z - az);
    let dist = (dx * dx + dz * dz).sqrt();
    let dir = if dist > 1e-6 {
        (dx / dist, dz / dist)
    } else {
        (0.0, 1.0)
    };
    let gap_ab = a.distance_to_rect(b.x - delta.0, b.z - delta.1) - b.support(dir);
    let gap_ba = b.distance_to_rect(ax, az) - a.support(dir);
    gap_ab.max(gap_ba)
}

fn normalized(v: (f64, f64)) -> Option<(f64, f64)> {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    (len > 1e-9).then(|| (v.0 / len, v.1 / len))
}

impl BattleSim {
    /// EP11: the rules of the push (`data/rules/battle_push.json`).
    pub fn push_rules(&self) -> &PushRules {
        &self.push_rules
    }

    /// EP11: replaces the rules of the push (tests and A/B probes).
    pub fn set_push_rules(&mut self, rules: PushRules) {
        self.push_rules = rules;
    }

    /// EP11: multiplier of the melee blows of `attacker` on `defender`
    /// (compression of both, wrapping files).
    pub(super) fn push_melee_factor(&self, attacker: &Unit, defender: &Unit) -> f64 {
        if self.siege.is_some() {
            return 1.0;
        }
        self.push_rules.melee_factor(attacker, defender)
    }

    /// Direction in which `winner` drives `loser` back: its own front when
    /// the loser stands before it, else straight away from it.
    fn push_direction(&self, winner: usize, loser: usize) -> Option<(f64, f64)> {
        let (w, l) = (&self.units[winner], &self.units[loser]);
        let to = normalized((l.x - w.x, l.z - w.z))?;
        let forward = w.forward();
        if forward.0 * to.0 + forward.1 * to.1 > 0.5 {
            Some(forward)
        } else {
            Some(to)
        }
    }

    /// Can regiment `i` move by `delta` this tick (field edge, walls,
    /// houses, deep water, a friend in the way)?
    fn push_free(&self, i: usize, delta: (f64, f64)) -> bool {
        let unit = &self.units[i];
        let from = (unit.x, unit.z);
        let to = (from.0 + delta.0, from.1 + delta.1);
        let (width, depth) = (self.field.width, self.field.depth);
        if to.0 < 1.0 || to.0 > width - 1.0 || to.1 < 1.0 || to.1 > depth - 1.0 {
            return false;
        }
        if self.wall_block(i, from, to).is_some()
            || self.house_block(i, from, to)
            || self.water_blocks(i, from, to)
        {
            return false;
        }
        let gap = self.push_rules.compression.friend_gap_m;
        !self.units.iter().enumerate().any(|(j, f)| {
            if j == i || f.side != unit.side || !f.present() || f.state == UnitState::Routing {
                return false;
            }
            let (dx, dz) = (f.x - unit.x, f.z - unit.z);
            if dx * dx + dz * dz > 250.0 * 250.0 {
                return false;
            }
            let after = gap_after(unit, delta, f);
            after < gap && after < gap_after(unit, (0.0, 0.0), f)
        })
    }

    /// One step of the push (called just before the melee blows).
    pub(super) fn resolve_push(&mut self, contacts: &[Vec<usize>]) {
        if self.siege.is_some() {
            return;
        }
        let n = self.units.len();
        let pressure = &self.push_rules.pressure;
        // Phase 1: pressures of every pair in contact; recoil of the weaker.
        let mut recoil = vec![(0.0_f64, 0.0_f64); n];
        let mut pairs: Vec<(usize, usize, (f64, f64))> = Vec::new();
        for i in 0..n {
            if !engaged(&self.units[i]) {
                continue;
            }
            for &j in &contacts[i] {
                if j <= i || !engaged(&self.units[j]) {
                    continue;
                }
                let (a, b) = (&self.units[i], &self.units[j]);
                // The defensive factors (square, stakes) are at least 1: at
                // most one of the two drives the other back.
                let a_drives =
                    pressure.recoil_speed(pressure.drive(a, b), pressure.resistance(b, a));
                let b_drives =
                    pressure.recoil_speed(pressure.drive(b, a), pressure.resistance(a, b));
                let (winner, loser, speed) = if a_drives >= b_drives {
                    (i, j, a_drives)
                } else {
                    (j, i, b_drives)
                };
                if speed <= 0.0 {
                    continue;
                }
                let Some(dir) = self.push_direction(winner, loser) else {
                    continue;
                };
                recoil[loser].0 += dir.0 * speed;
                recoil[loser].1 += dir.1 * speed;
                pairs.push((winner, loser, dir));
            }
        }
        let max_speed = pressure.max_speed_mps;
        // Phase 2: the losers give ground, or compress against what stops them.
        let mut moved = vec![(0.0_f64, 0.0_f64); n];
        let mut pushed_speed = vec![0.0_f64; n];
        for i in 0..n {
            let (vx, vz) = recoil[i];
            let speed = (vx * vx + vz * vz).sqrt();
            if speed < 1e-9 {
                continue;
            }
            let k = speed.min(max_speed) / speed;
            let delta = (vx * k * DT, vz * k * DT);
            pushed_speed[i] = speed.min(max_speed);
            if self.push_free(i, delta) {
                let unit = &mut self.units[i];
                unit.x += delta.0;
                unit.z += delta.1;
                unit.push.ground_lost += (delta.0 * delta.0 + delta.1 * delta.1).sqrt();
                moved[i] = delta;
            }
        }
        // Phase 3: the winners follow the ground they gained (the largest
        // advance among their pushed opponents), unless pushed themselves.
        let follow_share = self.push_rules.pressure.follow_share;
        let mut follow = vec![(0.0_f64, 0.0_f64, 0.0_f64); n];
        for &(winner, loser, dir) in &pairs {
            // Archers keep to their stakes rather than follow; a regiment
            // also engaged by another enemy is pinned where it stands.
            if pushed_speed[winner] > 0.0
                || self.units[winner].stakes_planted
                || contacts[winner].len() > 1
            {
                continue;
            }
            let along = (moved[loser].0 * dir.0 + moved[loser].1 * dir.1) * follow_share;
            if along > follow[winner].2 {
                follow[winner] = (dir.0 * along, dir.1 * along, along);
            }
        }
        for (i, &(fx, fz, along)) in follow.iter().enumerate() {
            if along <= 0.0 || !self.push_free(i, (fx, fz)) {
                continue;
            }
            let unit = &mut self.units[i];
            unit.x += fx;
            unit.z += fz;
        }
        // Phase 4: compression, morale, shape of the front.
        for i in 0..n {
            let blocked = pushed_speed[i] > 0.0 && moved[i] == (0.0, 0.0);
            let speed = if pushed_speed[i] > 0.0 {
                -pushed_speed[i]
            } else {
                follow[i].2 / DT
            };
            let in_melee = engaged(&self.units[i]) && !contacts[i].is_empty();
            let primary = if in_melee {
                self.primary_opponent(i, &contacts[i])
            } else {
                None
            };
            let shape = self.push_shape(i, primary, speed, blocked, pushed_speed[i]);
            // Giving ground shakes a regiment; being crushed against an
            // obstacle more so.
            let morale_loss = shape.compression * self.push_rules.compression.morale_per_second
                + pushed_speed[i] / max_speed.max(1e-9)
                    * self.push_rules.pressure.recoil_morale_per_second;
            let unit = &mut self.units[i];
            unit.push = shape;
            if morale_loss > 0.0 {
                unit.morale -= morale_loss * DT;
            }
        }
    }

    /// Next shape of regiment `i` (see [`PushShape`]).
    fn push_shape(
        &self,
        i: usize,
        primary: Option<usize>,
        speed: f64,
        blocked: bool,
        pushed_speed: f64,
    ) -> PushShape {
        let rules = &self.push_rules;
        let unit = &self.units[i];
        let mut shape = unit.push;
        shape.speed = speed;
        // Compression.
        let c = &rules.compression;
        if blocked {
            let share = pushed_speed / rules.pressure.max_speed_mps.max(1e-9);
            shape.compression = (shape.compression + c.gain_per_second * share * DT).min(1.0);
        } else {
            shape.compression = (shape.compression - c.decay_per_second * DT).max(0.0);
        }
        // Bulge of the front around the point of contact.
        let b = &rules.bulge;
        let target = match primary {
            Some(_) => (b.melee_surge_m + b.per_mps * speed).clamp(-b.max_m, b.max_m),
            None => 0.0,
        };
        let step = b.rate_mps * DT;
        shape.bulge += (target - shape.bulge).clamp(-step, step);
        let (w, _) = unit.extent();
        if let Some(p) = primary {
            let (lx, _) = to_local(unit, self.units[p].x, self.units[p].z);
            shape.bulge_at = lx.clamp(-w * 0.5, w * 0.5);
        }
        // Wrap of the files overhanging a narrower opponent.
        let wr = &rules.wrap;
        let span = primary.and_then(|p| opponent_span(unit, &self.units[p]));
        if let Some(p) = primary {
            let id = self.units[p].id;
            if shape.wrap_of != Some(id) {
                // A new opponent: the files straighten before wrapping again.
                shape.wrap = [0.0; 2];
                shape.wrap_of = Some(id);
            }
        }
        let half = w * 0.5;
        for s in 0..2 {
            let overhang = match span {
                Some([lo, hi]) => {
                    if s == 0 {
                        lo + half
                    } else {
                        half - hi
                    }
                }
                None => 0.0,
            };
            if overhang >= wr.min_overhang_m {
                shape.wrap[s] = (shape.wrap[s] + wr.grow_per_second * DT).min(1.0);
                if let Some(edges) = span {
                    shape.wrap_edge[s] = edges[s];
                }
            } else {
                shape.wrap[s] = (shape.wrap[s] - wr.decay_per_second * DT).max(0.0);
            }
        }
        if shape.wrap == [0.0; 2] && primary.is_none() {
            shape.wrap_of = None;
        }
        shape
    }
}
