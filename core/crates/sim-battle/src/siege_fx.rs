//! Siege assault events for the renderer (SG1, `docs/wip/sg1-sieges.md`).
//!
//! The battle journal ([`crate::BattleEvent`]) speaks French to the player;
//! these events speak to the 3D view: which engine shot which wall piece and
//! where the stone struck, each blow of the ram, the ladders raised against a
//! piece, a siege tower lowering its bridge, a regiment gaining the wall walk,
//! boiling oil poured from the gate, the gate giving way. They carry no rule:
//! the simulation applies its rules first, then records what happened, so a
//! renderer that ignores them loses nothing but the show. Deterministic: no
//! random draw (positions come from integer hashes of the tick and the unit).
//!
//! The only rules added with them (in `sim/siege_assault.rs`):
//! - the ram strikes in blows every [`RAM_PERIOD`] seconds instead of a
//!   continuous wear (same damage per second on average);
//! - [`OIL_PERIOD`]: boiling oil from the gate's machicolations while a
//!   defender guards the gate;
//! - once the gate gives way, the garrison's reserve falls back to the square.

use serde::{Deserialize, Serialize};

/// Seconds between two blows of the ram.
pub const RAM_PERIOD: f64 = 3.0;
/// Seconds between two pots of boiling oil over the gate.
pub const OIL_PERIOD: f64 = 30.0;
/// Reach of the oil in front of the gate (metres from the gate's outer face).
pub const OIL_REACH: f64 = 10.0;
/// A defender this close to the gate (inside) mans the machicolations.
pub const OIL_GUARD_RANGE: f64 = 50.0;
/// Soldiers of a foot regiment scalded by one pot (before armour).
pub const OIL_KILLS: f64 = 3.0;
/// Morale lost by a scalded regiment.
pub const OIL_MORALE: f64 = 4.0;
/// Servants of the ram scalded by one pot (its hide roof sheds most of it).
pub const OIL_RAM_KILLS: f64 = 0.5;
/// Regiments per opening kept to block it once the gate falls; the rest of
/// the garrison's foot regroups on the square.
pub const BLOCKERS_PER_OPENING: usize = 2;

/// Ladder foot distance from the outer face of the wall (metres).
pub const LADDER_LEAN: f64 = 2.8;
/// Soldiers seen on each ladder at once.
pub const CLIMBERS_PER_LADDER: usize = 4;
/// Soldiers seen crossing a siege tower's bridge at once.
pub const BRIDGE_CROSSERS: usize = 6;
/// How many times a soldier's ascent repeats over one full climb (the
/// regiment goes up man after man; a ladder takes ~7 s).
pub const CLIMB_WAVES: f64 = 6.0;

/// Ladders raised by a regiment of this frontage (metres).
pub fn ladder_count(width: f64) -> usize {
    ((width / 6.0) as usize).clamp(2, 7)
}

/// One thing worth showing, at simulated time `time`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiegeFx {
    pub time: f64,
    #[serde(flatten)]
    pub kind: SiegeFxKind,
}

/// What happened (unit ids are [`crate::Unit::id`], pieces index
/// [`crate::SiegeWorks::pieces`], towers index [`crate::SiegeWorks::towers`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SiegeFxKind {
    /// An engine's stone (or ball) struck wall piece `piece` at (x, z), at
    /// `height` (0-1 of the wall height); `breached` when it brought it down.
    EngineShot {
        unit: u32,
        piece: usize,
        x: f64,
        z: f64,
        height: f64,
        breached: bool,
    },
    /// One blow of the ram on the gate; `breached` on the last one.
    RamStrike {
        unit: u32,
        piece: usize,
        breached: bool,
    },
    /// A tower of the ring shot a volley of bolts at regiment `target`.
    TowerVolley { tower: usize, target: u32 },
    /// Regiment `unit` raised its ladders against piece `piece`.
    LaddersRaised { unit: u32, piece: usize },
    /// A siege tower docked against piece `piece` and lowered its bridge.
    TowerDocked { unit: u32, piece: usize },
    /// A siege tower left the wall (moved off, destroyed, or the wall fell).
    TowerUndocked { unit: u32, piece: usize },
    /// Regiment `unit` gained the wall walk of piece `piece`.
    OnWall { unit: u32, piece: usize },
    /// Boiling oil poured from the gate over (x, z); `targets` scalded.
    BoilingOil {
        piece: usize,
        x: f64,
        z: f64,
        targets: Vec<u32>,
    },
    /// The gate gave way (ram, engine or fire).
    GateBroken { piece: usize },
    /// A stretch of wall came down.
    WallBreached { piece: usize },
    /// The garrison's reserve falls back to the central square.
    DefendersFallBack,
}

/// Deterministic value in [0, 1) from two integers (no random stream used).
pub fn hash01(a: u64, b: u64) -> f64 {
    let mut h = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64
}
