//! Path preview before the click (lot CB-M2, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Aperçu du trajet »).
//!
//! [`BattleSim::preview_path`] answers « which way would this regiment
//! walk to (x, z)? » with the chain of waypoints of
//! [`BattleSim::plan_route`], the very function whose first waypoint the
//! regiment heads for each tick once the order is given (`route()` in
//! `sim.rs`): the preview and the real order cannot disagree. Read-only
//! (`&self`): only the deterministic obstacle cache of the siege grid may be
//! filled, as by any A* search.

use serde::{Deserialize, Serialize};

use crate::hover::HoverRules;
use crate::sim::BattleSim;
use crate::unit::UnitState;

/// Why no path can be previewed (the cursor turns `forbidden`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewError {
    /// No such regiment.
    UnknownUnit,
    /// Routing, destroyed, off the field or in reserve.
    Unavailable,
    /// The point is off the field.
    OutsideField,
    /// Deployment phase: the point is outside the side's zones.
    OutsideZone,
    /// Deep water without a ford or bridge for horsemen and engines, or no
    /// way through the streets of a besieged town.
    Unreachable,
}

/// One regiment's previewed way (group preview).
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewLeg {
    /// The regiment, or `None` for the single path drawn from the centre of
    /// a large selection.
    pub unit: Option<u32>,
    /// Where the path starts (the regiment, or the centre of the group).
    pub from: (f64, f64),
    /// Waypoints, the destination last; `Err` when unreachable.
    pub path: Result<Vec<(f64, f64)>, PreviewError>,
}

impl BattleSim {
    /// CB-M2: the waypoints regiment `unit` would follow if ordered to
    /// (x, z), the destination last (placed and clamped into the field
    /// exactly as the `Move` order does).
    pub fn preview_path(&self, unit: u32, x: f64, z: f64) -> Result<Vec<(f64, f64)>, PreviewError> {
        self.preview_index(unit)?;
        self.preview_group(&[unit], x, z, None)
            .pop()
            .map_or(Err(PreviewError::Unavailable), |leg| leg.path)
    }

    /// CB-M2: the paths of a group `Move` to (x, z): one per regiment to its
    /// own place in the group (as the order spreads them), or, beyond
    /// `max_individual_paths` regiments, one path from the group's centre
    /// (walked as the first regiment would). Regiments that cannot be
    /// ordered (routing, gone) are left out.
    pub fn preview_group(
        &self,
        units: &[u32],
        x: f64,
        z: f64,
        facing: Option<f64>,
    ) -> Vec<PreviewLeg> {
        let ids: Vec<u32> = units
            .iter()
            .copied()
            .filter(|&id| self.preview_index(id).is_ok())
            .collect();
        let Some(&first) = ids.first() else {
            return Vec::new();
        };
        let inside = x.is_finite() && z.is_finite() && self.field().inside(x, z);
        let limit = HoverRules::bundled().preview.max_individual_paths as usize;
        if ids.len() > limit {
            let n = ids.len() as f64;
            let from = ids.iter().fold((0.0, 0.0), |(sx, sz), &id| {
                let u = &self.units()[id as usize];
                (sx + u.x / n, sz + u.z / n)
            });
            let path = if inside {
                self.preview_to(first as usize, from, (x, z))
            } else {
                Err(PreviewError::OutsideField)
            };
            return vec![PreviewLeg {
                unit: None,
                from,
                path,
            }];
        }
        let destinations = self.group_destinations(&ids, x, z, facing);
        ids.iter()
            .zip(destinations)
            .map(|(&id, to)| {
                let u = &self.units()[id as usize];
                let from = (u.x, u.z);
                let path = if inside {
                    self.preview_to(id as usize, from, to)
                } else {
                    Err(PreviewError::OutsideField)
                };
                PreviewLeg {
                    unit: Some(id),
                    from,
                    path,
                }
            })
            .collect()
    }

    /// Index of a regiment that may be previewed.
    fn preview_index(&self, unit: u32) -> Result<usize, PreviewError> {
        let u = self.unit(unit).ok_or(PreviewError::UnknownUnit)?;
        if !u.present() || u.state == UnitState::Routing {
            return Err(PreviewError::Unavailable);
        }
        Ok(unit as usize)
    }

    /// The path of regiment `index` from `from` to its place `to` in the
    /// group (before the clamp of the order).
    fn preview_to(
        &self,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Result<Vec<(f64, f64)>, PreviewError> {
        let field = self.field();
        // Same clamp as the `Move` order.
        let to = (
            to.0.clamp(5.0, field.width - 5.0),
            to.1.clamp(5.0, field.depth - 5.0),
        );
        if self.is_deploying() {
            // Deployment: the regiment is placed, not walked.
            let side = self.units()[index].side;
            return if self.deployable(side, to.0, to.1) {
                Ok(vec![to])
            } else {
                Err(PreviewError::OutsideZone)
            };
        }
        self.plan_route(index, from, to)
            .ok_or(PreviewError::Unreachable)
    }
}
