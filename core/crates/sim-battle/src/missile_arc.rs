//! Direct and indirect shooting over the relief (lot R4, ADR 0046).
//!
//! A shooter sees its target: it shoots **direct** (flat trajectory, full
//! accuracy). The ground masks the target: only regiments shooting volleys
//! of a missile listed in `indirect_missiles` (the bows) may still reach it,
//! **indirectly**, lobbing their arrows over the crest, and only
//! - when a trajectory launched at most `max_launch_angle_deg` above the
//!   horizontal clears the ground between them ([`arc_clears`]: a crest too
//!   close to the shooter, or too close in front of the target, stops it),
//! - and when a friendly regiment sees the target (it directs the shooting)
//!   or the target was seen within `memory_s` seconds;
//!
//! with a strong accuracy penalty (the arrows scatter over ground nobody
//! aims at). The rules live in `data/rules/missile_arc.json` (schema
//! `data/schemas/missile_arc_rules.schema.json`); the checks themselves run
//! in the battle tick (`sim/indirect.rs`).

use serde::{Deserialize, Serialize};

use crate::field::Battlefield;
use crate::shot::MissileKind;

/// Contents of `data/rules/missile_arc.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissileArcRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Missile kinds (keys of [`MissileKind`]) that may be lobbed over a crest.
    pub indirect_missiles: Vec<String>,
    /// Steepest launch of a lobbed volley (degrees above the horizontal).
    pub max_launch_angle_deg: f64,
    /// Height the trajectory keeps above the ground (metres).
    pub clearance_m: f64,
    /// Accuracy factor of an indirect volley at a target a friend sees.
    pub indirect_accuracy: f64,
    /// Accuracy factor of an indirect volley at a target only remembered.
    pub remembered_accuracy: f64,
    /// A friendly regiment this close to the target directs the shooting.
    pub spotter_range_m: f64,
    /// A target seen this many seconds ago may still be shot at indirectly.
    pub memory_s: f64,
}

data_model::bundled_rules!(MissileArcRules, "rules/missile_arc.json");

impl MissileArcRules {
    /// May this missile be lobbed over a crest?
    pub fn lobs(&self, kind: MissileKind) -> bool {
        self.indirect_missiles.iter().any(|k| k == kind.key())
    }
}

/// How a volley reaches its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FireMode {
    /// The shooters see the target.
    Direct,
    /// Lobbed over the ground that masks the target, directed by a friend
    /// who sees it.
    Spotted,
    /// Lobbed at a target seen a moment ago that nobody sees any more.
    Remembered,
}

impl FireMode {
    pub fn indirect(self) -> bool {
        self != FireMode::Direct
    }

    /// Accuracy factor of the volley.
    pub fn accuracy(self, rules: &MissileArcRules) -> f64 {
        match self {
            FireMode::Direct => 1.0,
            FireMode::Spotted => rules.indirect_accuracy,
            FireMode::Remembered => rules.remembered_accuracy,
        }
    }
}

/// Height of the shooters' hands and of the men aimed at above the ground.
const BODY: f64 = 1.5;
/// Samples along the trajectory.
const SAMPLES: u32 = 24;

/// Does a trajectory from `from` to `to`, launched at most
/// `max_launch_angle_deg` above the horizontal, clear the ground between them
/// by `clearance_m`?
///
/// The trajectory is a parabola through both ends (the arrows' hands and the
/// men aimed at, [`BODY`] above the ground); the steepest launch allowed
/// gives the highest one. A crest near the shooter stops the rising arrows,
/// a crest right in front of the target the falling ones (both descend and
/// rise at comparable angles): the target is then in dead ground.
pub fn arc_clears(
    field: &Battlefield,
    from: (f64, f64),
    to: (f64, f64),
    max_launch_angle_deg: f64,
    clearance_m: f64,
) -> bool {
    let (dx, dz) = (to.0 - from.0, to.1 - from.1);
    let d = dx.hypot(dz);
    if d < 1.0 {
        return true;
    }
    let h0 = field.height(from.0, from.1) + BODY;
    let h1 = field.height(to.0, to.1) + BODY;
    let chord = (h1 - h0) / d;
    let launch = max_launch_angle_deg.to_radians().tan();
    // y(s) = h0 + chord s + bow s (d - s): launch slope chord + bow d.
    let bow = (launch - chord) / d;
    if bow <= 0.0 {
        // The target stands above the steepest launch: no lob reaches it.
        return false;
    }
    (1..SAMPLES).all(|k| {
        let t = f64::from(k) / f64::from(SAMPLES);
        let s = d * t;
        let ground = field.height(from.0 + dx * t, from.1 + dz * t);
        ground + clearance_m <= h0 + chord * s + bow * s * (d - s)
    })
}
