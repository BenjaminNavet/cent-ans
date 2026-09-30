//! Player (and AI) commands, validated by the simulation (spec § 1).

use serde::{Deserialize, Serialize};

use crate::modes::UnitMode;
use crate::setup::SideId;
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
        /// CB-M3: after the regiments' current orders (Shift + right click)
        /// instead of replacing them. Left out of the JSON when false, so
        /// replays recorded before CB read and write the same.
        #[serde(default, skip_serializing_if = "is_false")]
        queue: bool,
        /// CB1: frontage in metres set by a right-drag (the whole line for
        /// a group, shared in proportion to strength): the regiments form a
        /// Line that wide, ranks within `data/rules/formation_width.json`.
        /// `None` (a plain click) leaves the formation as it is. Left out
        /// of the JSON when unset, like `queue`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        /// CB1: the regiments of the order (or of `group_tag`) walk at the
        /// pace of the slowest of them.
        #[serde(default, skip_serializing_if = "is_false")]
        match_speed: bool,
        /// CB1: tag shared by the individual moves of a locked group (for
        /// `match_speed`); without it, the order's own regiments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_tag: Option<u32>,
    },
    /// Close with (or shoot at) an enemy regiment.
    Attack {
        units: Vec<u32>,
        target: u32,
        #[serde(default = "default_true")]
        run: bool,
        /// CB-M3: after the regiments' current orders (see `Move::queue`).
        #[serde(default, skip_serializing_if = "is_false")]
        queue: bool,
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
    /// Siege battles (S2): put a torch to house `house` (index in
    /// `siege.houses`) or, with `gate`, to the town gate. One of the
    /// regiments must stand close enough (the garrison reaches its suburbs
    /// from anywhere).
    Burn {
        units: Vec<u32>,
        #[serde(default)]
        house: Option<usize>,
        #[serde(default)]
        gate: bool,
    },
    /// A leader's order from `data/battle_orders/` (war cry, no quarter,
    /// dismount, pavise, rally). `units` names the regiments of a
    /// `selected`-scope order (every eligible one when empty); `side`
    /// defaults to the issuing side, then to the side of the first unit.
    LeaderOrder {
        #[serde(default)]
        side: Option<SideId>,
        order: String,
        #[serde(default)]
        units: Vec<u32>,
    },
    /// CB2: turn a persistent mode of the regiments on or off (run, guard,
    /// skirmish, melee, breach; see [`crate::modes`]). A new variant: replays
    /// recorded before CB2 read the same.
    SetMode {
        units: Vec<u32>,
        mode: UnitMode,
        enabled: bool,
    },
    /// CB4: an active ability of the regiments (`data/battle_abilities/`),
    /// used by those of `units` that have it; a second use by regiments
    /// that all have it on lifts it. Additive: replays recorded before CB4
    /// read unchanged.
    UseAbility {
        units: Vec<u32>,
        ability: String,
    },
    /// The side quits the battle (« Quitter la bataille »): every regiment
    /// still in order withdraws and the battle ends at once, lost. Allowed
    /// during deployment. `side` defaults to the issuing side. Additive:
    /// older replays read unchanged.
    Concede {
        #[serde(default)]
        side: Option<SideId>,
    },
}

fn default_true() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
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
            | Command::TargetWall { units, .. }
            | Command::Burn { units, .. }
            | Command::LeaderOrder { units, .. }
            | Command::SetMode { units, .. }
            | Command::UseAbility { units, .. } => units,
            Command::Concede { .. } => &[],
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
    InvalidFormation {
        unit: u32,
        formation: Formation,
    },
    NoMissile(u32),
    OutsideField,
    Finished,
    NotASiege,
    UnknownPiece(usize),
    NotAnEngine(u32),
    /// No such order in the battle's catalogue.
    UnknownOrder(String),
    /// The order cannot be given now (French reason: cooldown, no general...).
    OrderUnavailable {
        order: String,
        reason: String,
    },
    /// The side giving a leader's order cannot be determined.
    NoSide,
    /// A leader's order for the other side.
    WrongSide,
    /// Deployment phase (F5a): only formation and fire-at-will orders, and
    /// `deploy_unit`, until `start_battle`.
    Deploying,
    /// `deploy_unit` / `start_battle` outside the deployment phase.
    NotDeploying,
    /// The position is outside the side's deployment zone (or, in a siege,
    /// on the wrong side of the walls).
    OutsideZone(u32),
    /// `burn` (S2): no such house.
    UnknownHouse(usize),
    /// `burn`: the house or the gate already burns (or is a ruin).
    AlreadyBurning,
    /// `burn`: neither a house nor the gate named (or both).
    NothingToBurn,
    /// `burn`: no regiment of the order stands close enough.
    TooFarToBurn(u32),
    /// RS-F (`BattleSim::burn_choice`): every house and the gate already
    /// burn or are ruins.
    NothingLeftToBurn,
    /// RS-F (`BattleSim::burn_choice`): no house nor the gate within torch
    /// reach of the regiments.
    NothingInReach,
    /// CB-M3: regiment `unit` already has `max` orders waiting.
    QueueFull {
        unit: u32,
        max: u32,
    },
    /// CB2: regiment `unit` cannot take `mode` (no missiles, not an engine,
    /// not a siege...).
    ModeUnavailable {
        unit: u32,
        mode: UnitMode,
    },
    /// CB4: no ability of that id in the battle's catalogue.
    UnknownAbility(String),
    /// CB4: none of the regiments could use the ability (`ability` is its
    /// French name).
    AbilityUnavailable {
        ability: String,
        reason: String,
    },
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
            CommandError::UnknownOrder(id) => write!(f, "ordre inconnu : {id}"),
            CommandError::OrderUnavailable { order, reason } => {
                write!(f, "« {order} » impossible : {reason}")
            }
            CommandError::NoSide => write!(f, "impossible de savoir quel camp donne cet ordre"),
            CommandError::WrongSide => write!(f, "cet ordre ne concerne pas votre armée"),
            CommandError::Deploying => {
                write!(
                    f,
                    "déploiement en cours : placez vos troupes puis lancez la bataille"
                )
            }
            CommandError::NotDeploying => write!(f, "le déploiement est terminé"),
            CommandError::UnknownHouse(i) => write!(f, "maison inconnue : {i}"),
            CommandError::AlreadyBurning => write!(f, "le feu y a déjà pris"),
            CommandError::NothingToBurn => {
                write!(f, "désignez une maison ou la porte à incendier")
            }
            CommandError::TooFarToBurn(id) => {
                write!(f, "l'unité {id} est trop loin pour y mettre le feu")
            }
            CommandError::NothingLeftToBurn => {
                write!(
                    f,
                    "toutes les maisons et la porte brûlent déjà ou sont en ruine"
                )
            }
            CommandError::NothingInReach => {
                write!(
                    f,
                    "aucune maison ni la porte à portée de torche : approchez un régiment"
                )
            }
            CommandError::QueueFull { unit, max } => {
                write!(
                    f,
                    "file d'ordres pleine : l'unité {unit} a déjà {max} ordres en attente"
                )
            }
            CommandError::ModeUnavailable { unit, mode } => {
                write!(
                    f,
                    "l'unité {unit} ne peut pas passer en mode {}",
                    mode.label_fr()
                )
            }
            CommandError::UnknownAbility(id) => write!(f, "capacité inconnue : {id}"),
            CommandError::AbilityUnavailable { ability, reason } => {
                write!(f, "« {ability} » impossible : {reason}")
            }
            CommandError::OutsideZone(id) => {
                write!(
                    f,
                    "l'unité {id} doit être placée dans votre zone de déploiement"
                )
            }
        }
    }
}

impl std::error::Error for CommandError {}
