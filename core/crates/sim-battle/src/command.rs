//! Player (and AI) commands, validated by the simulation (spec § 1).

use serde::{Deserialize, Serialize};

use crate::unit::Formation;

/// An order to one or more regiments. Serialised as
/// `{"type": "move", "units": [..], ...}` for the GDExtension bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Move to `(x, z)`. With several units, they keep their relative
    /// positions, or spread along a line perpendicular to `facing` when given.
    Move {
        units: Vec<u32>,
        x: f64,
        z: f64,
        #[serde(default)]
        run: bool,
        #[serde(default)]
        facing: Option<f64>,
    },
    /// Close with (or shoot at) an enemy regiment.
    Attack {
        units: Vec<u32>,
        target: u32,
        #[serde(default = "default_true")]
        run: bool,
    },
    /// Stop where they stand.
    Halt {
        units: Vec<u32>,
    },
    Formation {
        units: Vec<u32>,
        kind: Formation,
    },
    FireAtWill {
        units: Vec<u32>,
        enabled: bool,
    },
    /// Leave the field by the side's own edge; survivors are saved.
    Withdraw {
        units: Vec<u32>,
    },
    /// Siege battles: engines batter wall piece `piece` (index in the
    /// `siege.pieces` of the terrain).
    TargetWall {
        units: Vec<u32>,
        piece: usize,
    },
}

fn default_true() -> bool {
    true
}

impl Command {
    pub fn units(&self) -> &[u32] {
        match self {
            Command::Move { units, .. }
            | Command::Attack { units, .. }
            | Command::Halt { units }
            | Command::Formation { units, .. }
            | Command::FireAtWill { units, .. }
            | Command::Withdraw { units }
            | Command::TargetWall { units, .. } => units,
        }
    }
}

/// Why a command was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    NoUnits,
    UnknownUnit(u32),
    NotYours(u32),
    Unavailable(u32),
    UnknownTarget(u32),
    FriendlyTarget(u32),
    InvalidFormation { unit: u32, formation: Formation },
    NoMissile(u32),
    OutsideField,
    Finished,
    NotASiege,
    UnknownPiece(usize),
    NotAnEngine(u32),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NoUnits => write!(f, "aucune unité sélectionnée"),
            CommandError::UnknownUnit(id) => write!(f, "unité inconnue : {id}"),
            CommandError::NotYours(id) => write!(f, "l'unité {id} n'est pas sous vos ordres"),
            CommandError::Unavailable(id) => {
                write!(f, "l'unité {id} ne répond plus (déroute ou hors du champ)")
            }
            CommandError::UnknownTarget(id) => write!(f, "cible inconnue : {id}"),
            CommandError::FriendlyTarget(id) => write!(f, "l'unité {id} est une unité alliée"),
            CommandError::InvalidFormation { unit, formation } => write!(
                f,
                "l'unité {unit} ne peut pas se former en {}",
                formation.label_fr()
            ),
            CommandError::NoMissile(id) => write!(f, "l'unité {id} n'a pas d'armes de trait"),
            CommandError::OutsideField => write!(f, "destination hors du champ de bataille"),
            CommandError::Finished => write!(f, "la bataille est terminée"),
            CommandError::NotASiege => write!(f, "il n'y a pas de murailles dans cette bataille"),
            CommandError::UnknownPiece(i) => write!(f, "pan de muraille inconnu : {i}"),
            CommandError::NotAnEngine(id) => {
                write!(f, "l'unité {id} ne peut pas battre les murailles")
            }
        }
    }
}

impl std::error::Error for CommandError {}
