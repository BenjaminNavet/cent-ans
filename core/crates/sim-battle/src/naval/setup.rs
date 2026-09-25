//! Naval battle setup (lot NV1): plain serde data built by the campaign
//! (`sim-campaign::naval`) or read from a scenario (`data/naval/scenarios/`).

use data_model::{NavalRules, ShipClass};
use serde::{Deserialize, Serialize};

use crate::setup::{BattleSeason, SideId, UnitSetup};

/// Men of one embarked regiment aboard one ship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrewSetup {
    /// Index in [`NavalSideSetup::units`].
    pub unit: usize,
    pub men: u32,
}

/// One ship and the men aboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipSetup {
    /// Ship name (« la Christophe », « le Saint-Denis »).
    pub name: String,
    pub class: ShipClass,
    pub crew: Vec<CrewSetup>,
    /// A fireship (brûlot): a few sailors steer it onto an enemy, set it
    /// alight and leave in the boat.
    #[serde(default)]
    pub fireship: bool,
    /// Chained ships of the same group are lashed together in line: they
    /// cannot move, but send men across the chains (l'Écluse, 1340).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<u32>,
    /// Shoots fire arrows (Castilians at La Rochelle, 1372).
    #[serde(default)]
    pub fire_arrows: bool,
    /// Starting position (metres, sea centred on 0); formation otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    /// Starting heading, degrees (0 = +x, 90 = +z); facing the enemy otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_deg: Option<f64>,
    /// The admiral's ship: its loss shakes the whole fleet.
    #[serde(default)]
    pub flagship: bool,
}

impl ShipSetup {
    pub fn soldiers(&self) -> u32 {
        self.crew.iter().map(|c| c.men).sum()
    }
}

/// One fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavalSideSetup {
    pub faction: String,
    pub faction_name: String,
    #[serde(default)]
    pub army: String,
    /// Admiral's name, for the UI.
    #[serde(default)]
    pub admiral: String,
    /// Embarked regiments (campaign units): losses come back per unit.
    pub units: Vec<UnitSetup>,
    pub ships: Vec<ShipSetup>,
    /// The fleet waits at anchor and never leaves its line (AI only).
    #[serde(default)]
    pub hold: bool,
}

impl NavalSideSetup {
    pub fn men(&self) -> u32 {
        self.ships.iter().map(ShipSetup::soldiers).sum()
    }
}

/// Full description of a naval battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavalSetup {
    /// Sea zone (`sea_channel`), for the campaign and the UI.
    #[serde(default)]
    pub sea_zone: String,
    /// Place shown in the UI (« devant l'Écluse »).
    #[serde(default)]
    pub place_name: String,
    #[serde(default)]
    pub season: BattleSeason,
    /// Rain slackens the bowstrings.
    #[serde(default)]
    pub rain: bool,
    /// Direction the wind blows towards, degrees (0 = +x); from the weather
    /// gauge otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_to_deg: Option<f64>,
    /// Wind strength 0-1; drawn from the seed otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_strength: Option<f64>,
    /// Side holding the weather gauge (upwind) when no wind is given; the
    /// seed decides otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauge: Option<SideId>,
    /// Shore behind the defender (estuary): drawn by the renderer; no rule.
    #[serde(default)]
    pub shore: bool,
    pub attacker: NavalSideSetup,
    pub defender: NavalSideSetup,
    /// Side commanded by the player; the other (both when `None`) is AI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_side: Option<SideId>,
    #[serde(default)]
    pub rules: NavalRules,
}

impl NavalSetup {
    pub fn side(&self, side: SideId) -> &NavalSideSetup {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }

    pub fn ship_count(&self) -> usize {
        self.attacker.ships.len() + self.defender.ships.len()
    }
}
