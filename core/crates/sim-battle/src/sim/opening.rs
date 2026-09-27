//! CV3-2 (spec campagne vivante § 1.3): the opening of a battle after a
//! campaign stance ([`crate::setup::BattleOpening`], `SideSetup` flags),
//! rules in [`crate::opening::OpeningRules`].
//!
//! - **Ambush**: the victim is laid out in marching column along the longest
//!   road through the field (else the long axis of the map), vanguard →
//!   main body → rearguard, formation `Column`, and gets no deployment
//!   phase. The ambusher's deployment zone is one or two bands parallel to
//!   the column (the flanks with the most woods and hedges); its regiments
//!   start in a line in those bands, facing the column.
//! - **Forced march**: the side starts with `start_fatigue`, placed
//!   automatically, with no deployment phase.
//! - **Entrenched camp**: stakes planted from the start and a low palisade
//!   ([`ObstacleKind::Palisade`]) in front of the line.
//!
//! Everything derives from the setup and the field (no random draw): a
//! replay rebuilds the very same opening.

use data_model::{Ability, UnitCategory};
use serde::{Deserialize, Serialize};

use super::{angle_to, of_faction, BattleSim, DeploymentZone, STAKES_DELAY};
use crate::opening::OpeningRules;
use crate::setup::SideId;
use crate::site::{Obstacle, ObstacleKind};
use crate::unit::{Formation, Unit};

/// Where the ambush was laid out (derived from the setup and the field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbushLayout {
    pub victim: SideId,
    /// `true` when the column follows a road, `false` on the long axis of
    /// the map.
    pub on_road: bool,
    /// Centre line followed by the column, head (vanguard) last.
    pub path: Vec<(f64, f64)>,
    /// The column runs along z (`true`) or along x (`false`).
    pub along_z: bool,
    /// Deployment zones of the ambusher (one or two flanks).
    pub zones: Vec<DeploymentZone>,
}

/// Role of a regiment in the marching order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MarchRole {
    Vanguard,
    Main,
    Rearguard,
}

/// Length of a polyline.
fn polyline_length(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum()
}

/// Point and unit tangent at arc length `s` along `points` (extrapolated
/// past both ends along the end segments).
fn point_at(points: &[(f64, f64)], s: f64) -> ((f64, f64), (f64, f64)) {
    let dir = |a: (f64, f64), b: (f64, f64)| {
        let len = (b.0 - a.0).hypot(b.1 - a.1).max(1e-9);
        ((b.0 - a.0) / len, (b.1 - a.1) / len)
    };
    let mut rest = s;
    for (k, w) in points.windows(2).enumerate() {
        let len = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        let last = k + 2 == points.len();
        if rest <= len || last {
            let d = dir(w[0], w[1]);
            let t = if k == 0 { rest } else { rest.max(0.0) };
            return ((w[0].0 + d.0 * t, w[0].1 + d.1 * t), d);
        }
        rest -= len;
    }
    (points[0], (0.0, 1.0))
}

/// The longest run of consecutive points of `points` inside `[lo, hi]²`
/// (one rectangle per axis).
fn longest_inside(points: &[(f64, f64)], lo: (f64, f64), hi: (f64, f64)) -> Vec<(f64, f64)> {
    let inside = |p: &(f64, f64)| p.0 >= lo.0 && p.0 <= hi.0 && p.1 >= lo.1 && p.1 <= hi.1;
    let mut best: Vec<(f64, f64)> = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    for p in points.iter().chain(std::iter::once(&(f64::NAN, f64::NAN))) {
        if inside(p) {
            run.push(*p);
        } else {
            if polyline_length(&run) > polyline_length(&best) {
                best = std::mem::take(&mut run);
            }
            run.clear();
        }
    }
    best
}

impl BattleSim {
    /// CV3-2: `true` when `side` gets a deployment phase: not the victim of
    /// an ambush (caught in column), not a side in forced march (placed
    /// automatically).
    pub fn can_deploy(&self, side: SideId) -> bool {
        if self.setup.opening.ambush_victim() == Some(side) {
            return false;
        }
        !self.setup.side(side).forced_march
    }

    /// CV3-2: the ambush layout, if the battle opens in ambush.
    pub fn ambush_layout(&self) -> Option<&AmbushLayout> {
        self.ambush.as_ref()
    }

    /// CV3-2: every deployment zone of `side`: none for a side that cannot
    /// deploy, the flanks of the column for an ambusher, else the usual
    /// rectangle ([`BattleSim::deployment_zone`]).
    pub fn deployment_zones(&self, side: SideId) -> Vec<DeploymentZone> {
        if !self.can_deploy(side) {
            return Vec::new();
        }
        match &self.ambush {
            Some(layout) if layout.victim != side => layout.zones.clone(),
            _ => vec![self.standard_zone(side)],
        }
    }

    /// CV3-2: the opening after the default deployment: start fatigue,
    /// ambush column and flanks, entrenched camp; then its journal lines.
    pub(super) fn apply_opening(&mut self) {
        if self.siege.is_some() {
            return;
        }
        for side in SideId::BOTH {
            let fatigue = self.setup.side(side).start_fatigue.clamp(0.0, 100.0);
            if fatigue > 0.0 {
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    unit.fatigue = fatigue;
                }
            }
        }
        if let Some(victim) = self.setup.opening.ambush_victim() {
            self.lay_ambush(victim);
        }
        for side in SideId::BOTH {
            let column = self.ambush.as_ref().is_some_and(|l| l.victim == side);
            if self.setup.side(side).entrenched && !column {
                self.entrench(side);
            }
        }
    }

    /// CV3-2: journal lines of the opening (after the weather).
    pub(super) fn log_opening(&mut self) {
        if let Some(layout) = &self.ambush {
            let victim = layout.victim;
            let of = of_faction(&self.setup.side(victim).faction_name);
            let road = if layout.on_road {
                "sur la route"
            } else {
                "à travers champs"
            };
            let flanks = if layout.zones.len() > 1 {
                "des deux côtés"
            } else {
                "sur son flanc"
            };
            self.log(
                format!(
                    "Embuscade ! L'ost {of} est surpris en colonne de marche {road} ; \
                     l'ennemi surgit {flanks}."
                ),
                Some(victim),
            );
        }
        for side in SideId::BOTH {
            let setup = self.setup.side(side);
            let of = of_faction(&setup.faction_name);
            let (forced, entrenched) = (setup.forced_march, setup.entrenched);
            if forced {
                let text =
                    format!("Marche forcée : l'ost {of} arrive fourbu et se range sans délai.");
                self.log(text, Some(side));
            }
            if entrenched && self.siege.is_none() {
                let text = format!(
                    "Camp retranché : l'ost {of} attend derrière ses pieux et sa palissade."
                );
                self.log(text, Some(side));
            }
        }
    }

    /// CV3-2: marching order of `ids` (vanguard first): light horse and a
    /// third of the foot ahead; heavy horse, the general, the shooters, the
    /// middle foot and the engines (baggage) in the main body; the last third
    /// of the foot in the rearguard.
    fn march_order(&self, ids: &[usize]) -> Vec<usize> {
        let light = |u: &Unit| u.mounted && !u.has(Ability::ChargeLance);
        let foot: Vec<usize> = ids
            .iter()
            .copied()
            .filter(|&i| {
                let u = &self.units[i];
                !u.mounted && u.category == UnitCategory::Infantry && !u.is_general
            })
            .collect();
        let vanguard = if foot.len() >= 2 {
            foot.len().div_ceil(3)
        } else {
            0
        };
        let rear_from = foot.len() - foot.len() / 3;
        let role = |i: usize| -> (MarchRole, u8) {
            let u = &self.units[i];
            if let Some(k) = foot.iter().position(|&f| f == i) {
                if k < vanguard {
                    return (MarchRole::Vanguard, 1);
                }
                if k >= rear_from {
                    return (MarchRole::Rearguard, 0);
                }
                return (MarchRole::Main, 3);
            }
            if light(u) && !u.is_general {
                (MarchRole::Vanguard, 0)
            } else if u.mounted && !u.is_general {
                (MarchRole::Main, 0)
            } else if u.is_general {
                (MarchRole::Main, 1)
            } else if u.category == UnitCategory::Siege {
                (MarchRole::Main, 5)
            } else if u.category == UnitCategory::Ranged {
                (MarchRole::Main, 4)
            } else {
                (MarchRole::Main, 3)
            }
        };
        let mut order: Vec<usize> = ids.to_vec();
        order.sort_by_key(|&i| (role(i), i));
        order
    }

    /// Centre line of the column: the longest road inside the field, else
    /// the long axis of the map. `(path, on_road)`.
    fn column_path(&self, rules: &OpeningRules) -> (Vec<(f64, f64)>, bool) {
        let m = rules.column.edge_margin_m;
        let (w, d) = (self.field.width, self.field.depth);
        let (lo, hi) = ((m, m), (w - m, d - m));
        let best = self
            .field
            .roads
            .iter()
            .map(|r| longest_inside(&r.points, lo, hi))
            .filter(|p| p.len() >= 2)
            .max_by(|a, b| polyline_length(a).total_cmp(&polyline_length(b)));
        if let Some(path) = best.filter(|p| polyline_length(p) >= rules.column.min_road_length_m) {
            return (path, true);
        }
        let path = if w >= d {
            vec![(m, d * 0.5), (w - m, d * 0.5)]
        } else {
            vec![(w * 0.5, m), (w * 0.5, d - m)]
        };
        (path, false)
    }

    /// Lays the victim out in marching column and the ambusher on the flanks.
    fn lay_ambush(&mut self, victim: SideId) {
        let rules = OpeningRules::bundled();
        let (mut path, on_road) = self.column_path(rules);
        // March away from the victim's own edge: the vanguard (head) at the
        // end of the path nearer the enemy's side.
        let home_z = match victim {
            SideId::Attacker => 0.0,
            SideId::Defender => self.field.depth,
        };
        let (first, last) = (path[0], path[path.len() - 1]);
        let along_z_path = (last.1 - first.1).abs() >= (last.0 - first.0).abs();
        let reverse = if along_z_path {
            (last.1 - home_z).abs() < (first.1 - home_z).abs()
        } else {
            last.0 < first.0
        };
        if reverse {
            path.reverse();
        }
        let ids: Vec<usize> = self.side_units(victim, |u| !u.synthetic);
        let order = self.march_order(&ids);
        for &i in &order {
            let unit = &mut self.units[i];
            unit.formation = Formation::Column;
        }
        let gap = rules.column.gap_m;
        let total: f64 = order
            .iter()
            .map(|&i| self.units[i].extent().1 + gap)
            .sum::<f64>()
            - gap;
        let length = polyline_length(&path);
        // Centred on the path, head towards its end.
        let mut head = (length + total) * 0.5;
        if total > length {
            head = total;
        }
        let (w, d) = (self.field.width, self.field.depth);
        for &i in &order {
            let depth = self.units[i].extent().1;
            let s = head - depth * 0.5;
            head -= depth + gap;
            let ((x, z), dir) = point_at(&path, s);
            let unit = &mut self.units[i];
            unit.x = x.clamp(10.0, w - 10.0);
            unit.z = z.clamp(10.0, d - 10.0);
            unit.facing = angle_to(dir.0, dir.1);
        }
        // Flanks: bands parallel to the column's main axis.
        let xs = order.iter().map(|&i| self.units[i].x);
        let zs = order.iter().map(|&i| self.units[i].z);
        let (x_min, x_max) = xs.fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let (z_min, z_max) = zs.fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let along_z = if order.len() >= 2 {
            (z_max - z_min) >= (x_max - x_min)
        } else {
            along_z_path
        };
        let f = &rules.flank;
        let margin = 20.0;
        let bands: Vec<DeploymentZone> = if along_z {
            let (z0, z1) = (
                (z_min - f.extra_length_m).max(margin),
                (z_max + f.extra_length_m).min(d - margin),
            );
            vec![
                DeploymentZone {
                    x0: (x_min - f.far_m).max(margin),
                    z0,
                    x1: (x_min - f.near_m).max(margin),
                    z1,
                },
                DeploymentZone {
                    x0: (x_max + f.near_m).min(w - margin),
                    z0,
                    x1: (x_max + f.far_m).min(w - margin),
                    z1,
                },
            ]
        } else {
            let (x0, x1) = (
                (x_min - f.extra_length_m).max(margin),
                (x_max + f.extra_length_m).min(w - margin),
            );
            vec![
                DeploymentZone {
                    x0,
                    z0: (z_min - f.far_m).max(margin),
                    x1,
                    z1: (z_min - f.near_m).max(margin),
                },
                DeploymentZone {
                    x0,
                    z0: (z_max + f.near_m).min(d - margin),
                    x1,
                    z1: (z_max + f.far_m).min(d - margin),
                },
            ]
        };
        // Keep the bands deep enough to stand in, best cover first.
        let min_depth = (f.far_m - f.near_m) * 0.25;
        let mut scored: Vec<(f64, DeploymentZone)> = bands
            .into_iter()
            .filter(|z| (z.x1 - z.x0).min(z.z1 - z.z0) >= min_depth)
            .map(|z| (self.cover_score(&z, rules), z))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let zones: Vec<DeploymentZone> = match scored.as_slice() {
            [] => vec![self.standard_zone(victim.other())],
            [(_, only)] => vec![*only],
            [(best, a), (second, b), ..] => {
                if *best <= 0.0 || *second >= best * f.second_flank_ratio {
                    vec![*a, *b]
                } else {
                    vec![*a]
                }
            }
        };
        let column_centre = ((x_min + x_max) * 0.5, (z_min + z_max) * 0.5);
        self.ambush = Some(AmbushLayout {
            victim,
            on_road,
            path,
            along_z,
            zones: zones.clone(),
        });
        self.place_ambushers(victim.other(), &zones, along_z, column_centre, rules);
    }

    /// Cover of a flank band: samples in woods and near hedges or fences.
    fn cover_score(&self, zone: &DeploymentZone, rules: &OpeningRules) -> f64 {
        let f = &rules.flank;
        let step = f.sample_step_m.max(1.0);
        let mut score = 0.0;
        let mut x = zone.x0;
        while x <= zone.x1 {
            let mut z = zone.z0;
            while z <= zone.z1 {
                if self.field.in_forest(x, z) {
                    score += f.forest_weight;
                }
                let near_hedge = self.field.obstacles.iter().any(|o| {
                    matches!(o.kind, ObstacleKind::Hedge | ObstacleKind::Fence)
                        && o.distance(x, z) <= f.hedge_reach_m
                });
                if near_hedge {
                    score += f.hedge_weight;
                }
                z += step;
            }
            x += step;
        }
        score
    }

    /// The ambusher starts in a line along the middle of its band(s),
    /// facing the column (the player may then redeploy inside them).
    fn place_ambushers(
        &mut self,
        side: SideId,
        zones: &[DeploymentZone],
        along_z: bool,
        column_centre: (f64, f64),
        rules: &OpeningRules,
    ) {
        let ids = self.side_units(side, |u| !u.synthetic);
        let mut per_zone: Vec<Vec<usize>> = vec![Vec::new(); zones.len()];
        for (k, &i) in ids.iter().enumerate() {
            per_zone[k % zones.len()].push(i);
        }
        let gap = rules.flank.unit_gap_m;
        for (zone, list) in zones.iter().zip(&per_zone) {
            let (cx, cz) = ((zone.x0 + zone.x1) * 0.5, (zone.z0 + zone.z1) * 0.5);
            // Facing: towards the column, across the band.
            let facing = if along_z {
                angle_to(column_centre.0 - cx, 0.0)
            } else {
                angle_to(0.0, column_centre.1 - cz)
            };
            let (lo, hi) = if along_z {
                (zone.z0, zone.z1)
            } else {
                (zone.x0, zone.x1)
            };
            let widths: Vec<f64> = list.iter().map(|&i| self.units[i].extent().0).collect();
            // Rows along the band, wrapping outwards when one row is full.
            let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
            let mut used = 0.0;
            for (k, _) in list.iter().enumerate() {
                if used + widths[k] > hi - lo && !rows.last().is_some_and(Vec::is_empty) {
                    rows.push(Vec::new());
                    used = 0.0;
                }
                used += widths[k] + gap;
                rows.last_mut().expect("non-empty").push(k);
            }
            let out = if along_z {
                (cx - column_centre.0).signum()
            } else {
                (cz - column_centre.1).signum()
            };
            let across_lo = if along_z { zone.x0 } else { zone.z0 };
            let across_hi = if along_z { zone.x1 } else { zone.z1 };
            let across_mid = (across_lo + across_hi) * 0.5;
            for (r, row) in rows.iter().enumerate() {
                let total: f64 = row.iter().map(|&k| widths[k] + gap).sum::<f64>() - gap;
                let mut cursor = ((lo + hi) * 0.5 - total * 0.5).max(lo);
                let across = (across_mid + out * 30.0 * r as f64).clamp(across_lo, across_hi);
                for &k in row {
                    let along = (cursor + widths[k] * 0.5).min(hi);
                    cursor += widths[k] + gap;
                    let (mut x, mut z) = if along_z {
                        (across, along)
                    } else {
                        (along, across)
                    };
                    // Never in deep water: slide along the band.
                    let mut tries = 0;
                    while self.field.water_at(x, z) == Some(false) && tries < 40 {
                        tries += 1;
                        let shift = 10.0 * f64::from(tries);
                        if along_z {
                            z = (along + shift).clamp(lo, hi);
                            if self.field.water_at(x, z) == Some(false) {
                                z = (along - shift).clamp(lo, hi);
                            }
                        } else {
                            x = (along + shift).clamp(lo, hi);
                            if self.field.water_at(x, z) == Some(false) {
                                x = (along - shift).clamp(lo, hi);
                            }
                        }
                    }
                    let unit = &mut self.units[list[k]];
                    unit.x = x;
                    unit.z = z;
                    unit.facing = facing;
                }
            }
        }
    }

    /// Stakes planted and a low palisade in front of the camp of `side`.
    fn entrench(&mut self, side: SideId) {
        let rules = &OpeningRules::bundled().palisade;
        for unit in self.units.iter_mut().filter(|u| u.side == side) {
            if unit.has(Ability::Stakes) {
                unit.stakes_planted = true;
                unit.still_time = STAKES_DELAY;
            }
        }
        let ids = self.side_units(side, |u| !u.synthetic);
        if ids.is_empty() {
            return;
        }
        // The front faces +z for the attacker, -z for the defender.
        let ahead = match side {
            SideId::Attacker => 1.0,
            SideId::Defender => -1.0,
        };
        let front = ids
            .iter()
            .map(|&i| self.units[i].z * ahead + self.units[i].extent().1 * 0.5)
            .fold(f64::MIN, f64::max);
        let z = ((front + rules.distance_m) * ahead).clamp(10.0, self.field.depth - 10.0);
        let (x0, x1) = ids.iter().fold((f64::MAX, f64::MIN), |(a, b), &i| {
            let u = &self.units[i];
            let half = u.extent().0 * 0.5;
            (a.min(u.x - half), b.max(u.x + half))
        });
        let x0 = (x0 - rules.margin_m).max(10.0);
        let x1 = (x1 + rules.margin_m).min(self.field.width - 10.0);
        let pieces = ((x1 - x0) / rules.segment_m).ceil().max(1.0) as usize;
        let step = (x1 - x0) / pieces as f64;
        for k in 0..pieces {
            let a = (x0 + step * k as f64, z);
            let b = (x0 + step * (k + 1) as f64, z);
            // No palisade across deep water.
            let mid = ((a.0 + b.0) * 0.5, z);
            if self.field.water_at(mid.0, mid.1) == Some(false) {
                continue;
            }
            self.field.obstacles.push(Obstacle {
                a,
                b,
                kind: ObstacleKind::Palisade,
            });
        }
    }

    /// CV3-2: the palisades of the field (entrenched camps).
    pub fn palisades(&self) -> Vec<Obstacle> {
        self.field
            .obstacles
            .iter()
            .filter(|o| o.kind == ObstacleKind::Palisade)
            .copied()
            .collect()
    }

    /// CV3-2: melee divisor of `defender` struck by `attacker` across a
    /// palisade the defender stands close behind (1 without one).
    pub(super) fn palisade_defense(&self, attacker: &Unit, defender: &Unit) -> f64 {
        let rules = &OpeningRules::bundled().palisade;
        let orient = |o: &Obstacle, p: (f64, f64)| {
            (o.b.0 - o.a.0) * (p.1 - o.a.1) - (o.b.1 - o.a.1) * (p.0 - o.a.0)
        };
        let sheltered = self.field.obstacles.iter().any(|o| {
            o.kind == ObstacleKind::Palisade
                && o.distance(defender.x, defender.z) <= rules.reach_m
                && orient(o, (defender.x, defender.z)) * orient(o, (attacker.x, attacker.z)) < 0.0
        });
        if sheltered {
            rules.melee_defense
        } else {
            1.0
        }
    }
}
