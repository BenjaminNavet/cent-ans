//! Naval battle results and events (lot NV1).

use serde::{Deserialize, Serialize};

use crate::setup::SideId;

/// What became of one ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipFate {
    /// Still afloat and in its side's hands.
    Kept,
    /// Got away from the battle.
    Escaped,
    /// Taken by the enemy.
    Captured,
    /// Burnt or holed, gone to the bottom.
    Sunk,
}

impl ShipFate {
    pub fn label_fr(self) -> &'static str {
        match self {
            ShipFate::Kept => "tenu",
            ShipFate::Escaped => "échappé",
            ShipFate::Captured => "pris",
            ShipFate::Sunk => "coulé",
        }
    }
}

/// One ship in the result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipResult {
    /// Index in the side's `ships` setup.
    pub index: usize,
    pub name: String,
    /// Ship class id (`ship_cog`).
    pub class: String,
    pub fate: ShipFate,
}

/// Result of one fleet.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NavalSideResult {
    /// Men lost per embarked regiment (index of `NavalSideSetup::units`):
    /// killed, drowned and taken.
    pub unit_losses: Vec<u32>,
    pub men_start: u32,
    pub men_lost: u32,
    pub drowned: u32,
    pub prisoners: u32,
    pub ships: Vec<ShipResult>,
    /// Enemy ships this side took (class ids).
    pub prizes: Vec<String>,
}

impl NavalSideResult {
    pub fn count(&self, fate: ShipFate) -> usize {
        self.ships.iter().filter(|s| s.fate == fate).count()
    }
}

/// Result of a naval battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavalOutcome {
    /// `None`: nobody held the sea (both fleets drew off).
    pub winner: Option<SideId>,
    /// Simulated seconds.
    pub duration: f64,
    pub attacker: NavalSideResult,
    pub defender: NavalSideResult,
    /// Auto-resolved (no real-time battle).
    #[serde(default)]
    pub auto: bool,
}

impl NavalOutcome {
    pub fn side(&self, side: SideId) -> &NavalSideResult {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }
}

/// What happened, for the renderer and the battle log.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NavalEventKind {
    /// Grapples hold between `ship` and `other`.
    Grapple {
        ship: u32,
        other: u32,
    },
    /// Grapples cut.
    Cut {
        ship: u32,
        other: u32,
    },
    /// The boarders of `ship` are on the deck of `other`.
    Board {
        ship: u32,
        other: u32,
    },
    /// `ship` struck its colours to `by`.
    Capture {
        ship: u32,
        by: SideId,
    },
    /// Fire broke out aboard.
    Ignite {
        ship: u32,
    },
    /// A fireship drove onto `other`.
    Fireship {
        ship: u32,
        other: u32,
    },
    /// The crew leapt into the sea.
    Abandon {
        ship: u32,
    },
    /// Going under.
    Sinking {
        ship: u32,
    },
    Sunk {
        ship: u32,
    },
    /// A galley's spur struck.
    Ram {
        ship: u32,
        other: u32,
        damage: f64,
    },
    /// Broke off and fled.
    Flee {
        ship: u32,
    },
    Escaped {
        ship: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NavalEvent {
    pub time: f64,
    #[serde(flatten)]
    pub kind: NavalEventKind,
}
