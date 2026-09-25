//! Historical naval scenarios (`data/naval/scenarios/*.json`,
//! `naval_scenario.schema.json`), turned into a [`NavalSetup`] with the ship
//! classes and unit types of the game data.

use std::collections::BTreeMap;

use data_model::{NavalRules, ShipClass, ShipClassId, UnitType, UnitTypeId};
use serde::{Deserialize, Serialize};

use super::setup::{CrewSetup, NavalSetup, NavalSideSetup, ShipSetup};
use crate::setup::{BattleSeason, SideId, UnitSetup};

/// A regiment of a scenario fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioUnit {
    pub unit_type: UnitTypeId,
    /// Display name (the unit type's otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_morale")]
    pub morale: u8,
    #[serde(default)]
    pub experience: u8,
}

fn default_morale() -> u8 {
    70
}

/// Men of one scenario regiment aboard one ship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCrew {
    pub unit: usize,
    pub men: u32,
}

/// A ship of a scenario fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioShip {
    pub name: String,
    pub class: ShipClassId,
    pub crew: Vec<ScenarioCrew>,
    #[serde(default)]
    pub fireship: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<u32>,
    #[serde(default)]
    pub fire_arrows: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_deg: Option<f64>,
    #[serde(default)]
    pub flagship: bool,
}

/// A line of chained ships in defensive formation (lot NV2): the ships of
/// group `chain` without a `position` are lashed side by side along a line
/// through `origin`, perpendicular to their heading, in file order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioChain {
    pub chain: u32,
    pub origin: [f64; 2],
    #[serde(default = "default_chain_heading")]
    pub heading_deg: f64,
    #[serde(default = "default_chain_spacing")]
    pub spacing_m: f64,
}

fn default_chain_heading() -> f64 {
    180.0
}

fn default_chain_spacing() -> f64 {
    12.0
}

impl ScenarioChain {
    /// Position of ship `k` of the `count` ships of the line.
    pub fn slot(&self, k: usize, count: usize) -> [f64; 2] {
        let h = self.heading_deg.to_radians();
        let offset = (k as f64 - (count as f64 - 1.0) * 0.5) * self.spacing_m;
        [
            self.origin[0] + h.sin() * offset,
            self.origin[1] - h.cos() * offset,
        ]
    }
}

/// A scenario fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFleet {
    pub faction: String,
    pub faction_name: String,
    #[serde(default)]
    pub admiral: String,
    #[serde(default)]
    pub hold: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chains: Vec<ScenarioChain>,
    pub units: Vec<ScenarioUnit>,
    pub ships: Vec<ScenarioShip>,
}

/// A historical naval battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavalScenario {
    pub id: String,
    pub name: String,
    /// ISO date (Julian calendar).
    pub date: String,
    pub place_name: String,
    pub sea_zone: String,
    #[serde(default)]
    pub season: BattleSeason,
    #[serde(default)]
    pub rain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_to_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_strength: Option<f64>,
    #[serde(default)]
    pub shore: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_side: Option<SideId>,
    pub attacker: ScenarioFleet,
    pub defender: ScenarioFleet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
}

impl NavalScenario {
    /// The battle setup: ship classes and unit stats looked up in the data.
    pub fn to_setup(
        &self,
        ships: &BTreeMap<ShipClassId, ShipClass>,
        unit_types: &BTreeMap<UnitTypeId, UnitType>,
        rules: &NavalRules,
    ) -> Result<NavalSetup, String> {
        let fleet = |f: &ScenarioFleet| -> Result<NavalSideSetup, String> {
            let mut aboard = vec![0u32; f.units.len()];
            let mut out_ships = Vec::new();
            let mut in_chain: BTreeMap<u32, usize> = BTreeMap::new();
            for ship in &f.ships {
                // Place in its chained line, if the fleet draws one.
                let line = ship
                    .chain
                    .and_then(|c| f.chains.iter().find(|l| l.chain == c));
                let (position, heading_deg) = match line {
                    Some(line) if ship.position.is_none() => {
                        let k = in_chain.entry(line.chain).or_insert(0);
                        let count = f
                            .ships
                            .iter()
                            .filter(|s| s.chain == Some(line.chain) && s.position.is_none())
                            .count();
                        let slot = line.slot(*k, count);
                        *k += 1;
                        (Some(slot), ship.heading_deg.or(Some(line.heading_deg)))
                    }
                    _ => (ship.position, ship.heading_deg),
                };
                let class = ships
                    .get(&ship.class)
                    .ok_or_else(|| format!("classe de navire inconnue : {}", ship.class))?;
                let mut crew = Vec::new();
                for c in &ship.crew {
                    let slot = aboard
                        .get_mut(c.unit)
                        .ok_or_else(|| format!("{} : régiment inconnu n°{}", ship.name, c.unit))?;
                    *slot += c.men;
                    crew.push(CrewSetup {
                        unit: c.unit,
                        men: c.men,
                    });
                }
                out_ships.push(ShipSetup {
                    name: ship.name.clone(),
                    class: class.clone(),
                    crew,
                    fireship: ship.fireship,
                    chain: ship.chain,
                    fire_arrows: ship.fire_arrows,
                    position,
                    heading_deg,
                    flagship: ship.flagship,
                });
            }
            let mut units = Vec::new();
            for (unit, men) in f.units.iter().zip(&aboard) {
                let unit_type = unit_types
                    .get(&unit.unit_type)
                    .ok_or_else(|| format!("type d'unité inconnu : {}", unit.unit_type))?;
                let mut setup =
                    UnitSetup::from_unit_type(unit_type, *men, unit.morale, unit.experience);
                setup.max_soldiers = setup.max_soldiers.max(*men);
                if let Some(name) = &unit.name {
                    setup.name = name.clone();
                }
                units.push(setup);
            }
            Ok(NavalSideSetup {
                faction: f.faction.clone(),
                faction_name: f.faction_name.clone(),
                army: String::new(),
                admiral: f.admiral.clone(),
                units,
                ships: out_ships,
                hold: f.hold,
            })
        };
        Ok(NavalSetup {
            sea_zone: self.sea_zone.clone(),
            place_name: self.place_name.clone(),
            season: self.season,
            rain: self.rain,
            wind_to_deg: self.wind_to_deg,
            wind_strength: self.wind_strength,
            gauge: None,
            shore: self.shore,
            attacker: fleet(&self.attacker)?,
            defender: fleet(&self.defender)?,
            player_side: self.player_side,
            rules: rules.clone(),
        })
    }
}
