//! TW2 T4 (ADR 0104): the step of the capture points of a siege battle
//! (rules and types in [`crate::capture`]) and the last stand of the
//! garrison on the square.

use super::{BattleSim, DT};
use crate::capture::{CapturePoint, CapturePointKind, CaptureRules};
use crate::setup::SideId;
use crate::siege::SiegeWorks;
use crate::unit::Unit;

/// The market square and the gate (inside it) of `works`.
pub(super) fn capture_points(works: &SiegeWorks, rules: &CaptureRules) -> Vec<CapturePoint> {
    let square = CapturePoint::new(
        CapturePointKind::Square,
        works.center.0,
        works.center.1,
        &rules.square,
    );
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    let offset = rules.gate.inside_offset_m.unwrap_or(20.0);
    let gate = CapturePoint::new(
        CapturePointKind::Gate,
        mx - nx * offset,
        mz - nz * offset,
        &rules.gate,
    );
    vec![square, gate]
}

/// Men of a regiment that count on a point: able, on the ground (not on the
/// wall walk nor on a ladder), not a machine.
fn counts(unit: &Unit) -> bool {
    unit.able()
        && !unit.synthetic
        && !unit.on_wall
        && unit.climbing.is_none()
        && !unit.ram
        && !unit.siege_tower()
}

impl BattleSim {
    /// One step of the capture points: men on each point, progress, the
    /// gate opening when taken, the "square threatened" alert. The square's
    /// progress is mirrored in [`SiegeWorks::hold_time`] (HUD, victory).
    pub(super) fn resolve_capture_points(&mut self) {
        let rules = CaptureRules::bundled();
        let Some(works) = self.siege.as_mut() else {
            return;
        };
        if works.points.is_empty() {
            works.points = capture_points(works, rules);
        }
        let mut logs: Vec<(String, SideId)> = Vec::new();
        let mut gate_taken = false;
        for point in works.points.iter_mut() {
            let (mut attackers, mut defenders) = (0.0, 0.0);
            for unit in self.units.iter().filter(|u| counts(u)) {
                if point.contains(unit.x, unit.z) {
                    match unit.side {
                        SideId::Attacker => attackers += unit.hp,
                        SideId::Defender => defenders += unit.hp,
                    }
                }
            }
            let was_capturing = point.progress > 0.0;
            let point_rules = rules.point(point.kind);
            let taken = point.advance(attackers, defenders, point_rules, rules.superiority, DT);
            match point.kind {
                CapturePointKind::Square => {
                    if !was_capturing && point.progress > 0.0 && !self.square_announced {
                        self.square_announced = true;
                        logs.push((
                            "Les assaillants s'emparent de la place centrale !".to_owned(),
                            SideId::Attacker,
                        ));
                    }
                }
                CapturePointKind::Gate => {
                    if taken {
                        gate_taken = true;
                        logs.push((
                            "Les assaillants tiennent la porte : elle s'ouvre, les tours voisines se taisent !"
                                .to_owned(),
                            SideId::Attacker,
                        ));
                    }
                }
            }
        }
        if gate_taken {
            let gate = works.gate;
            works.pieces[gate].hp = 0.0;
        }
        let square = works
            .points
            .iter()
            .find(|p| p.kind == CapturePointKind::Square)
            .map(|p| (p.progress, p.share(), p.x, p.z));
        if let Some((progress, share, x, z)) = square {
            works.hold_time = progress;
            if share >= rules.threatened_share && !self.square_threatened {
                self.square_threatened = true;
                self.alert(
                    crate::alerts::AlertKind::SquareThreatened,
                    x,
                    z,
                    Some(SideId::Defender),
                    None,
                );
            } else if progress <= 0.0 {
                self.square_threatened = false;
            }
        }
        for (text, side) in logs {
            self.log(text, Some(side));
        }
    }

    /// The square is held long enough: the town falls.
    pub(super) fn square_taken(&self) -> bool {
        self.siege.as_ref().is_some_and(|w| {
            w.points
                .iter()
                .any(|p| p.kind == CapturePointKind::Square && p.taken())
        })
    }

    /// The towers this close to a gate taken by the attacker are silent.
    pub(super) fn tower_silenced(&self, x: f64, z: f64) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        let reach = CaptureRules::bundled()
            .gate
            .silences_towers_m
            .unwrap_or(0.0);
        let (gx, gz) = works.pieces[works.gate].midpoint();
        works
            .points
            .iter()
            .any(|p| p.kind == CapturePointKind::Gate && p.taken())
            && (x - gx).powi(2) + (z - gz).powi(2) <= reach * reach
    }

    /// Last stand (ADR 0104): a defender regiment on or around the square
    /// once the town is open (a breach or the gate down).
    pub(super) fn in_last_stand(&self, unit: &Unit) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        let stand = &CaptureRules::bundled().last_stand;
        unit.side == SideId::Defender
            && stand.radius_m > 0.0
            && !unit.on_wall
            && (unit.x - works.center.0).powi(2) + (unit.z - works.center.1).powi(2)
                <= stand.radius_m * stand.radius_m
            && works.pieces.iter().any(|p| !p.intact())
    }
}
