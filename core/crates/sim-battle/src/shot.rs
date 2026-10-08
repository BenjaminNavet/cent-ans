//! Shooting events for the renderer (lot BV1, « bataille vivante »).
//!
//! Every volley resolved by the simulation (a regiment's shot at another
//! regiment or at a wall piece) is recorded as a [`ShotEvent`]: who shot,
//! from where, at what, how many missiles left the bows and how many men the
//! volley killed. The rules stay in [`crate::BattleSim`]; the events only
//! describe what happened so that the Godot side can draw the arrows, the
//! arrows stuck in pavises and stakes, and the blood of the hits, without
//! guessing it from the ammunition counters.

use data_model::key_enum;
use serde::Serialize;

/// Seconds between two shots of a siege engine (SG2: the renderer winds the
/// trebuchet back over this time, `get_units().reload`).
pub const ENGINE_RELOAD: f64 = 12.0;
/// Seconds between two volleys of crossbowmen behind pavises.
pub const PAVISE_RELOAD: f64 = 9.0;
/// Seconds between two volleys of other shooters.
pub const VOLLEY_RELOAD: f64 = 6.0;

key_enum! {
/// Kind of missile a volley throws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissileKind {
    Arrow => "arrow",
    Bolt => "bolt",
    /// Bombard stone or iron ball.
    Ball => "ball",
    /// Trebuchet or mangonel stone.
    Stone => "stone",
    /// Handheld firearm bullet (couleuvriniers, lot UR2): drawn with
    /// ignition smoke at the shooter.
    Bullet => "bullet",
    /// Thrown javelin (jinetes, lot UR2): no arc drop like a bow shot.
    Javelin => "javelin",
}
}

key_enum! {
/// What the missiles strike besides men and earth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotCover {
    None => "none",
    /// The target stands behind pavises (Genoese crossbowmen, pavise order).
    Pavise => "pavise",
    /// The target stands behind planted stakes (English archers).
    Stakes => "stakes",
    /// The target stands on a wall (merlons).
    Wall => "wall",
}
}

/// One volley, as resolved by the simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShotEvent {
    /// Simulated time of the volley (seconds).
    pub time: f64,
    /// Shooting regiment id.
    pub shooter: u32,
    /// Target regiment id (`None`: a wall piece).
    pub target: Option<u32>,
    /// Centre of the shooting regiment.
    pub from: (f64, f64),
    /// Aim point (target regiment centre or wall piece midpoint).
    pub aim: (f64, f64),
    /// Missiles loosed (one per living shooter, one per engine).
    pub missiles: u32,
    /// Men of the target killed by this volley (fractional, as the rules count them).
    pub kills: f64,
    pub kind: MissileKind,
    /// Fire arrows or incendiary stones (siege attackers able to set fires).
    pub incendiary: bool,
    pub cover: ShotCover,
    /// R4: lobbed over a crest at a target the shooters do not see (the
    /// renderer may draw a higher arc).
    pub indirect: bool,
}

/// Shots kept for the renderer between two reads; older ones are dropped
/// (a headless simulation nobody reads must not grow without bound).
pub const MAX_PENDING_SHOTS: usize = 1024;

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<MissileKind>();
        assert_keys_match_serde::<ShotCover>();
    }
}
