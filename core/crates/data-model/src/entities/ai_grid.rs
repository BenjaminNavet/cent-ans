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
    /// CV3-6: when the AI takes the ambush, forced march and entrenched
    /// stances (spec `docs/design/2026-09-27-campagne-vivante.md` § 5).
    #[serde(default)]
    pub postures: AiPostures,
    /// CV3-6: detours of idle AI armies towards map encounters.
    #[serde(default)]
    pub encounters: AiEncounters,
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
            postures: AiPostures::default(),
            encounters: AiEncounters::default(),
            description: None,
        }
    }
}

/// CV3-6: the AI's stance tuning (`postures` of `data/ai/grid.json`).
/// Without it, the AI takes none of the CV3 stances.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPostures {
    #[serde(default)]
    pub ambush: AiAmbush,
    #[serde(default)]
    pub forced_march: AiForcedMarch,
    #[serde(default)]
    pub entrenched: AiEntrenched,
}

/// CV3-6: an AI army weaker than an enemy army marching on its lands lies
/// in wait in cover by the enemy's foreseeable route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiAmbush {
    /// Chance (per mille) that an army in a fitting situation lies in wait,
    /// for a faction of aggression 50. 0 turns the AI's ambushes off.
    pub weight_permille: u32,
    /// Change of that chance (per mille) per point of aggression above 50
    /// (negative: cautious realms lie in wait more, bold ones give battle).
    pub weight_per_aggression: f64,
    /// The army lies in wait only when its power is below this share of the
    /// enemy army's (it would rather attack otherwise)...
    pub max_power_ratio: f64,
    /// ... and at least this share (a handful of men is no ambush).
    pub min_power_ratio: f64,
    /// The ambush cell lies within this share of the zone of control radius
    /// of the enemy's foreseeable route (its march must enter the zone).
    pub route_zoc_share: f64,
    /// Part of the enemy's route watched: what it marches in this many of
    /// its full turns.
    pub route_turns: f64,
    /// Enemy armies considered: within this distance (km) of the army.
    pub watch_radius_km: f64,
}

impl Default for AiAmbush {
    fn default() -> Self {
        Self {
            weight_permille: 0,
            weight_per_aggression: 0.0,
            max_power_ratio: 0.9,
            min_power_ratio: 0.35,
            route_zoc_share: 0.75,
            route_turns: 1.0,
            watch_radius_km: 80.0,
        }
    }
}

/// CV3-6: forced march to relieve a besieged place or join a siege out of
/// normal reach but within reach with the bonus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiForcedMarch {
    /// `false`: the AI never force-marches.
    pub enabled: bool,
    /// Share of the forced march allowance the route may cost (the route
    /// table is on the settlement graph, the march on the grid).
    pub reach_margin: f64,
    /// Enemy armies within this distance (km) of the target count as
    /// waiting there.
    pub danger_radius_km: f64,
    /// No forced march when the enemy power waiting there exceeds the
    /// army's power times this ratio.
    pub danger_ratio: f64,
}

impl Default for AiForcedMarch {
    fn default() -> Self {
        Self {
            enabled: false,
            reach_margin: 0.9,
            danger_radius_km: 12.0,
            danger_ratio: 1.0,
        }
    }
}

/// CV3-6: entrenched camp of an idle army outnumbered on a threatened
/// border, outside any place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiEntrenched {
    /// `false`: the AI never entrenches.
    pub enabled: bool,
    /// The army entrenches when the hostile power around its province
    /// exceeds its own times this ratio.
    pub inferiority_ratio: f64,
}

impl Default for AiEntrenched {
    fn default() -> Self {
        Self {
            enabled: false,
            inferiority_ratio: 1.0,
        }
    }
}

/// CV3-6: an AI army without urgent business walks to a map encounter it
/// can reach this turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiEncounters {
    /// Chance (per mille) of the detour, per army and turn. 0: never.
    pub detour_permille: u32,
    /// Sites considered: within this distance (km) of the army (what it
    /// sees).
    pub sight_km: f64,
}

impl Default for AiEncounters {
    fn default() -> Self {
        Self {
            detour_permille: 0,
            sight_km: 30.0,
        }
    }
}

fn default_trespass_route_factor() -> f64 {
    1.0
}
