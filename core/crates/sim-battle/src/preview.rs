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
    /// CB1: frontage and depth (metres) of the regiment on arrival (its
    /// Line for a dragged width, else its present formation); `None` for
    /// the single path of a large selection.
    pub extent: Option<(f64, f64)>,
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
        self.preview_group_with(units, x, z, facing, None, false)
    }

    /// CB1: [`Self::preview_group`] (or, `queued`,
    /// [`Self::preview_group_queued`]) of a dragged `Move` `width` metres
    /// wide: the places of the regiments in their new frontages, each leg
    /// with the frontage and depth taken on arrival.
    pub fn preview_group_width(
        &self,
        units: &[u32],
        x: f64,
        z: f64,
        facing: Option<f64>,
        width: Option<f64>,
        queued: bool,
    ) -> Vec<PreviewLeg> {
        self.preview_group_with(units, x, z, facing, width, queued)
    }

    /// CB-M3: the way regiment `unit` would walk to (x, z) starting from
    /// `from` (the last point of its queue) instead of where it stands;
    /// `None` is [`Self::preview_path`]. The same `plan_route` as the order
    /// once the queue reaches it (clamped into the field alike).
    pub fn preview_path_from(
        &self,
        unit: u32,
        from: Option<(f64, f64)>,
        x: f64,
        z: f64,
    ) -> Result<Vec<(f64, f64)>, PreviewError> {
        let Some(from) = from else {
            return self.preview_path(unit, x, z);
        };
        let index = self.preview_index(unit)?;
        if !(x.is_finite() && z.is_finite() && self.field().inside(x, z)) {
            return Err(PreviewError::OutsideField);
        }
        self.preview_to(index, from, (x, z))
    }

    /// CB-M3: [`Self::preview_group`] for a queued `Move` (Shift held): each
    /// path starts where the regiment's queue leaves it
    /// ([`BattleSim::queue_anchor`]), and the group is spread around those
    /// points, as the queued order does.
    pub fn preview_group_queued(
        &self,
        units: &[u32],
        x: f64,
        z: f64,
        facing: Option<f64>,
    ) -> Vec<PreviewLeg> {
        self.preview_group_with(units, x, z, facing, None, true)
    }

    fn preview_group_with(
        &self,
        units: &[u32],
        x: f64,
        z: f64,
        facing: Option<f64>,
        width: Option<f64>,
        queued: bool,
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
        let anchors: Vec<(f64, f64)> = ids
            .iter()
            .map(|&id| {
                if queued {
                    self.queue_anchor(id as usize)
                } else {
                    let u = &self.units()[id as usize];
                    (u.x, u.z)
                }
            })
            .collect();
        let limit = HoverRules::bundled().preview.max_individual_paths as usize;
        if ids.len() > limit {
            let n = ids.len() as f64;
            let from = anchors
                .iter()
                .fold((0.0, 0.0), |(sx, sz), &(ax, az)| (sx + ax / n, sz + az / n));
            let path = if inside {
                self.preview_to(first as usize, from, (x, z))
            } else {
                Err(PreviewError::OutsideField)
            };
            return vec![PreviewLeg {
                unit: None,
                from,
                path,
                extent: None,
            }];
        }
        let widths = self.move_widths(&ids, width);
        let frontages = self.move_frontages(&ids, widths.as_deref());
        let destinations =
            self.group_destinations_with(&ids, &anchors, x, z, facing, frontages.as_deref());
        ids.iter()
            .enumerate()
            .zip(anchors.iter().zip(destinations))
            .map(|((k, &id), (&from, to))| {
                let path = if inside {
                    self.preview_to(id as usize, from, to)
                } else {
                    Err(PreviewError::OutsideField)
                };
                let share = widths.as_ref().map(|w| w[k]);
                PreviewLeg {
                    unit: Some(id),
                    from,
                    path,
                    extent: Some(self.units()[id as usize].extent_for_width(share)),
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
