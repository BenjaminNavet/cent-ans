//! Shooting events for the renderer (lot BV1, « bataille vivante »).
//!
//! Every volley resolved by the simulation (a regiment's shot at another
//! regiment or at a wall piece) is recorded as a [`ShotEvent`]: who shot,
//! from where, at what, how many missiles left the bows and how many men the
//! volley killed. The rules stay in [`crate::BattleSim`]; the events only
//! describe what happened so that the Godot side can draw the arrows, the
//! arrows stuck in pavises and stakes, and the blood of the hits, without
//! guessing it from the ammunition counters.

use serde::Serialize;

/// Kind of missile a volley throws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissileKind {
    Arrow,
    Bolt,
    /// Bombard stone or iron ball.
    Ball,
    /// Trebuchet or mangonel stone.
    Stone,
    /// Handheld firearm bullet (couleuvriniers, lot UR2): drawn with
    /// ignition smoke at the shooter.
    Bullet,
    /// Thrown javelin (jinetes, lot UR2): no arc drop like a bow shot.
    Javelin,
}

impl MissileKind {
    pub fn key(self) -> &'static str {
        match self {
            MissileKind::Arrow => "arrow",
            MissileKind::Bolt => "bolt",
            MissileKind::Ball => "ball",
            MissileKind::Stone => "stone",
            MissileKind::Bullet => "bullet",
            MissileKind::Javelin => "javelin",
        }
    }
}

/// What the missiles strike besides men and earth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotCover {
    None,
    /// The target stands behind pavises (Genoese crossbowmen, pavise order).
    Pavise,
    /// The target stands behind planted stakes (English archers).
    Stakes,
    /// The target stands on a wall (merlons).
    Wall,
}

impl ShotCover {
    pub fn key(self) -> &'static str {
        match self {
            ShotCover::None => "none",
            ShotCover::Pavise => "pavise",
            ShotCover::Stakes => "stakes",
            ShotCover::Wall => "wall",
        }
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
}

/// Shots kept for the renderer between two reads; older ones are dropped
/// (a headless simulation nobody reads must not grow without bound).
pub const MAX_PENDING_SHOTS: usize = 1024;
