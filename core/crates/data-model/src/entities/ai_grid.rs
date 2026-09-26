//! Tuning of the AI armies on the navigation grid (lot M3), mirroring
//! `data/schemas/ai_grid.schema.json` (file `data/ai/grid.json`). Without
//! the file, [`AiGrid::default`] applies.

use serde::{Deserialize, Serialize};

/// Contents of `data/ai/grid.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiGrid {
    /// An AI army attacks an enemy army in its bubble when its side's power
    /// exceeds the enemy's (with the walls it stands behind) by this factor.
    pub attack_ratio: f64,
    /// An enemy army is a candidate target when its straight-line distance
    /// is at most this share of the kilometres the army may still march.
    pub attack_reach_share: f64,
    /// An enemy army stronger than this multiple of the army's power is
    /// avoided: its zone of control is kept out of the planned route.
    pub avoid_ratio: f64,
    /// Settlements within this distance (km) of an avoided army are left
    /// out of the route (the zone of control plus a margin).
    pub avoid_radius_km: f64,
    /// Settlement-graph legs an army is ordered to walk in one turn at most
    /// (one `MoveArmy` per leg; the march stops when its points run out).
    pub max_legs_per_turn: usize,
    /// EQ5: route cost multiplier of the roads into lands the AI may cross
    /// without right of passage (at war, by temper): the armies go round
    /// them when a road of their own or of the enemy is not much longer.
    #[serde(default = "default_trespass_route_factor")]
    pub trespass_route_factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for AiGrid {
    fn default() -> Self {
        Self {
            attack_ratio: 1.5,
            attack_reach_share: 0.8,
            avoid_ratio: 1.0,
            avoid_radius_km: 12.0,
            max_legs_per_turn: 6,
            trespass_route_factor: default_trespass_route_factor(),
            description: None,
        }
    }
}

fn default_trespass_route_factor() -> f64 {
    1.0
}
