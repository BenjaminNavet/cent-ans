//! Landmark cities (`data/landmarks/<id>.json`, lots L1-L3), siege subset.
//!
//! The file describes the whole historical plan for the Blender generator
//! (`tools/blender_scripts/landmark_city.py`); the core only reads what the
//! siege battle needs (lot L3, ADR 0026): the walls with their gates, the
//! streets and the `siege.battle` block naming the walls of the besieged
//! town, the attacked gate and the streets kept as lanes. Unknown fields are
//! ignored; the full file is validated by `data/schemas/landmark.schema.json`.

use serde::{Deserialize, Serialize};

/// A gate of a wall, at a plan point (metres, x east, y true north).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandmarkGate {
    pub name: String,
    pub at: [f64; 2],
}

/// A town wall of the plan (polyline, closed or ending on a river).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandmarkWall {
    pub id: String,
    pub points: Vec<[f64; 2]>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub gates: Vec<LandmarkGate>,
}

/// A main street of the plan (polyline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandmarkStreet {
    pub id: String,
    pub points: Vec<[f64; 2]>,
}

/// `siege.battle`: the besieged town of the siege battle drawn from the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandmarkBattle {
    /// Wall ids, chained in this order into the ring.
    pub walls: Vec<String>,
    /// Name of the attacked gate.
    pub gate: String,
    /// Street ids kept free of houses.
    #[serde(default)]
    pub streets: Vec<String>,
    /// Central square (default: centroid of the ring).
    #[serde(default)]
    pub square: Option<[f64; 2]>,
    /// Mean radius of the ring on the battlefield (default: the generic town's).
    #[serde(default)]
    pub radius_m: Option<f64>,
}

/// The `siege` block (only `battle` matters to the core).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LandmarkSiege {
    #[serde(default)]
    pub battle: Option<LandmarkBattle>,
}

/// A landmark city, as far as the core is concerned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub id: String,
    pub name: String,
    /// Settlement whose siege battles use this plan.
    pub settlement: String,
    #[serde(default)]
    pub siege: Option<LandmarkSiege>,
    #[serde(default)]
    pub walls: Vec<LandmarkWall>,
    #[serde(default)]
    pub streets: Vec<LandmarkStreet>,
}

impl Landmark {
    /// The `siege.battle` block, if the plan describes its besieged town.
    pub fn battle(&self) -> Option<&LandmarkBattle> {
        self.siege.as_ref().and_then(|s| s.battle.as_ref())
    }
}
