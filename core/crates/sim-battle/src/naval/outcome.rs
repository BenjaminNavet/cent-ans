//! Naval battle results.

use serde::{Deserialize, Serialize};

use super::setup::NavalSetup;
use super::ship::{Ship, ShipStatus};
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
}

impl NavalOutcome {
    pub fn side(&self, side: SideId) -> &NavalSideResult {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }
}

/// What became of `ship` when `winner` holds the sea: afloat ships of the
/// loser that did not get away and abandoned hulks go to the winner as prizes.
fn ship_fate(ship: &Ship, winner: Option<SideId>) -> ShipFate {
    let lost_side = winner.is_some_and(|w| w != ship.side);
    match ship.status {
        ShipStatus::Sunk | ShipStatus::Sinking { .. } => ShipFate::Sunk,
        ShipStatus::Captured { .. } => ShipFate::Captured,
        ShipStatus::Escaped => ShipFate::Escaped,
        ShipStatus::Abandoned => {
            if ship.fire > 0.3 || ship.hull <= 0.0 || winner.is_none() {
                ShipFate::Sunk
            } else if lost_side {
                ShipFate::Captured
            } else {
                ShipFate::Kept
            }
        }
        ShipStatus::Afloat => {
            if ship.fleeing {
                ShipFate::Escaped
            } else if lost_side {
                ShipFate::Captured
            } else {
                ShipFate::Kept
            }
        }
    }
}

/// Result of one fleet from the final state of its ships. Swimmers are
/// picked up by their side if it holds the sea, taken or drowned otherwise.
fn side_result(
    setup: &NavalSetup,
    ships: &[Ship],
    side: SideId,
    winner: Option<SideId>,
) -> NavalSideResult {
    let fleet = setup.side(side);
    let mut result = NavalSideResult {
        unit_losses: vec![0; fleet.units.len()],
        men_start: fleet.men(),
        ..Default::default()
    };
    let mut losses = vec![0.0; fleet.units.len()];
    let mut drowned = 0.0;
    let mut prisoners = 0.0;
    // In the water: saved by the side that holds the sea.
    let rescued = winner == Some(side) || winner.is_none();
    for ship in ships.iter().filter(|s| s.side == side) {
        let fate = ship_fate(ship, winner);
        for (k, crew) in ship.crew.iter().enumerate() {
            let mut lost =
                (crew.initial - crew.men - ship.swimmers[k] - ship.prisoners[k]).max(0.0);
            if !rescued {
                lost += ship.swimmers[k];
                prisoners += ship.swimmers[k] * 0.5;
                drowned += ship.swimmers[k] * 0.5;
            }
            lost += ship.prisoners[k];
            prisoners += ship.prisoners[k];
            drowned += ship.drowned[k];
            // Men still aboard a ship taken at the end.
            if fate == ShipFate::Captured && ship.is_afloat() {
                lost += crew.men;
                prisoners += crew.men;
            }
            if let Some(slot) = losses.get_mut(crew.unit) {
                *slot += lost;
            }
        }
        result.ships.push(ShipResult {
            index: ship.index,
            name: ship.name.clone(),
            class: ship.class.id.to_string(),
            fate,
        });
    }
    for ((slot, lost), unit) in result.unit_losses.iter_mut().zip(&losses).zip(&fleet.units) {
        *slot = (lost.round() as u32).min(unit.soldiers);
    }
    result.men_lost = result.unit_losses.iter().sum();
    result.drowned = drowned.round() as u32;
    result.prisoners = prisoners.round() as u32;
    result
}

impl NavalOutcome {
    /// Builds the result of a battle from the ships' final state; captured
    /// ships go to the other side as prizes.
    pub(crate) fn from_ships(
        setup: &NavalSetup,
        ships: &[Ship],
        winner: Option<SideId>,
        duration: f64,
    ) -> NavalOutcome {
        let mut attacker = side_result(setup, ships, SideId::Attacker, winner);
        let mut defender = side_result(setup, ships, SideId::Defender, winner);
        let prizes_of = |side: &NavalSideResult| -> Vec<String> {
            side.ships
                .iter()
                .filter(|s| s.fate == ShipFate::Captured)
                .map(|s| s.class.clone())
                .collect()
        };
        attacker.prizes = prizes_of(&defender);
        defender.prizes = prizes_of(&attacker);
        NavalOutcome {
            winner,
            duration,
            attacker,
            defender,
        }
    }
}
