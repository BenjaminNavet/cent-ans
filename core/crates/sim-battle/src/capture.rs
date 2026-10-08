//! Capture points of a siege battle (TW2 T4, ADR 0108), Total War style.
//!
//! Two points: the **market square** (victory point: held long enough, the
//! town falls) and the **gate** (just inside it: taken, the gate opens and
//! the towers around it fall silent). A point progresses while the attacker
//! has more able men on it than the defender (× [`CaptureRules::superiority`]);
//! otherwise its progress falls back, faster when the defender dominates it.
//!
//! Defenders on and around the square hold a **last stand**
//! ([`LastStandRules`]): they lose less morale to their losses and to the
//! routs around them, and recover some even in the melee.
//!
//! All numbers come from `data/rules/siege_capture.json`
//! (`data/schemas/siege_capture_rules.schema.json`). The step itself is in
//! `sim/capture.rs`.

use serde::{Deserialize, Serialize};

/// Rules of one capture point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointRules {
    /// Radius of the point (m): the men inside it count.
    pub radius_m: f64,
    /// Seconds of attacker superiority to take the point.
    pub hold_s: f64,
    /// Progress lost per second when nobody dominates the point.
    pub decay_per_s: f64,
    /// Progress lost per second when the defender dominates it.
    pub retake_per_s: f64,
    /// Gate only: distance from the middle of the gate, towards the town.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inside_offset_m: Option<f64>,
    /// Gate only: once taken, the towers this close to the gate stop shooting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silences_towers_m: Option<f64>,
}

/// The defenders' last stand on the square.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastStandRules {
    /// Radius around the centre of the square (0: no last stand).
    pub radius_m: f64,
    /// Multiplier of the morale lost to the regiment's losses.
    pub loss_morale_factor: f64,
    /// Morale recovered per second, even in the melee, up to the cap.
    pub morale_per_s: f64,
    /// Multiplier of the morale lost to the routs nearby.
    pub contagion_factor: f64,
}

/// How the AI garrison falls back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FallBackRules {
    /// Fall back to the square at the first breach, not only when the gate
    /// falls.
    pub on_breach: bool,
    /// Regiments kept to block the broken gate; the rest regroups on the square.
    pub gate_blockers: usize,
    /// Regiments kept to block each wall breach.
    pub breach_blockers: usize,
    /// Cap on the regiments blocking the openings.
    pub max_blockers: usize,
    /// Fallen back, the garrison only charges attackers this close to the
    /// centre of the square.
    pub engage_radius_m: f64,
}

/// Contents of `data/rules/siege_capture.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub square: PointRules,
    pub gate: PointRules,
    /// The attacker progresses while its men exceed the defender's × this.
    pub superiority: f64,
    /// Share of the square's progress from which "La place est menacée".
    pub threatened_share: f64,
    pub last_stand: LastStandRules,
    pub fall_back: FallBackRules,
    pub assault: AssaultRules,
}

/// How the AI attacker storms the square.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssaultRules {
    /// March on the square once this share of the melee regiments is inside.
    pub gather_share: f64,
    /// A melee regiment this close to the centre of the square: the assault
    /// is under way, nobody waits any more.
    pub committed_radius_m: f64,
}

data_model::bundled_rules!(CaptureRules, "rules/siege_capture.json");

impl CaptureRules {

    pub fn point(&self, kind: CapturePointKind) -> &PointRules {
        match kind {
            CapturePointKind::Square => &self.square,
            CapturePointKind::Gate => &self.gate,
        }
    }
}

/// Which point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePointKind {
    Square,
    Gate,
}

impl CapturePointKind {
    /// Stable key for the Godot bridge.
    pub fn key(self) -> &'static str {
        match self {
            CapturePointKind::Square => "square",
            CapturePointKind::Gate => "gate",
        }
    }
}

/// Who has the upper hand on a point this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointStatus {
    /// Nobody from the attacker on it: the defender holds it.
    #[default]
    Held,
    /// Both sides on it, the attacker not superior enough.
    Contested,
    /// The attacker is superior: the progress grows.
    Capturing,
    /// Taken by the attacker (the gate for good; the square: the town falls).
    Taken,
}

impl PointStatus {
    pub fn key(self) -> &'static str {
        match self {
            PointStatus::Held => "held",
            PointStatus::Contested => "contested",
            PointStatus::Capturing => "capturing",
            PointStatus::Taken => "taken",
        }
    }
}

/// One capture point during the battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturePoint {
    pub kind: CapturePointKind,
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    /// Seconds of superiority accumulated (0 ..= `hold_s`).
    pub progress: f64,
    pub hold_s: f64,
    pub status: PointStatus,
    /// Able men of each side on the point at the last step.
    pub attackers: f64,
    pub defenders: f64,
}

impl CapturePoint {
    pub fn new(kind: CapturePointKind, x: f64, z: f64, rules: &PointRules) -> Self {
        Self {
            kind,
            x,
            z,
            radius: rules.radius_m,
            progress: 0.0,
            hold_s: rules.hold_s,
            status: PointStatus::Held,
            attackers: 0.0,
            defenders: 0.0,
        }
    }

    pub fn contains(&self, x: f64, z: f64) -> bool {
        (x - self.x).powi(2) + (z - self.z).powi(2) <= self.radius * self.radius
    }

    /// Progress share (0-1).
    pub fn share(&self) -> f64 {
        if self.hold_s > 0.0 {
            (self.progress / self.hold_s).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn taken(&self) -> bool {
        self.status == PointStatus::Taken
    }

    /// One step of `dt` seconds with `attackers` and `defenders` able men
    /// on the point. Returns `true` on the step the point is taken.
    pub fn advance(
        &mut self,
        attackers: f64,
        defenders: f64,
        rules: &PointRules,
        superiority: f64,
        dt: f64,
    ) -> bool {
        self.attackers = attackers;
        self.defenders = defenders;
        if self.taken() {
            return false;
        }
        if attackers > 0.0 && attackers > defenders * superiority {
            self.status = PointStatus::Capturing;
            self.progress = (self.progress + dt).min(self.hold_s);
            if self.progress >= self.hold_s {
                self.status = PointStatus::Taken;
                return true;
            }
        } else {
            let rate = if defenders > 0.0 && defenders * superiority >= attackers {
                rules.retake_per_s
            } else {
                rules.decay_per_s
            };
            self.status = if attackers > 0.0 {
                PointStatus::Contested
            } else {
                PointStatus::Held
            };
            self.progress = (self.progress - rate * dt).max(0.0);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> &'static CaptureRules {
        CaptureRules::bundled()
    }

    #[test]
    fn bundled_rules_load() {
        let r = rules();
        assert!(r.square.hold_s >= r.gate.hold_s);
        assert!(r.gate.inside_offset_m.is_some());
        assert!(r.gate.silences_towers_m.is_some());
    }

    #[test]
    fn superiority_takes_the_point_in_hold_s() {
        let r = rules();
        let mut p = CapturePoint::new(CapturePointKind::Square, 0.0, 0.0, &r.square);
        let steps = (r.square.hold_s / 0.1).round() as usize;
        let mut taken_at = None;
        for k in 0..steps + 5 {
            if p.advance(200.0, 50.0, &r.square, r.superiority, 0.1) {
                taken_at = Some(k + 1);
            }
        }
        assert!(p.taken());
        assert!(taken_at.unwrap().abs_diff(steps) <= 1);
    }

    #[test]
    fn a_stronger_garrison_pushes_the_progress_back() {
        let r = rules();
        let mut p = CapturePoint::new(CapturePointKind::Square, 0.0, 0.0, &r.square);
        for _ in 0..100 {
            p.advance(100.0, 0.0, &r.square, r.superiority, 0.1);
        }
        let before = p.progress;
        assert!(before > 9.0);
        p.advance(50.0, 200.0, &r.square, r.superiority, 0.1);
        assert_eq!(p.status, PointStatus::Contested);
        assert!(p.progress < before - 0.1 * r.square.retake_per_s + 1e-9);
        p.advance(0.0, 0.0, &r.square, r.superiority, 0.1);
        assert_eq!(p.status, PointStatus::Held);
    }
}
