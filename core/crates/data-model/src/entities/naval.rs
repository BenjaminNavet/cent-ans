//! Naval warfare data (lot NV1, ADR 0028): ship classes
//! (`data/naval/ships/*.json`, `ship_class.schema.json`), naval rules
//! (`data/naval/rules.json`, `naval_rules.schema.json`) and the fleets of
//! 1337 (`data/naval/fleets.json`, `naval_fleets.schema.json`).
//!
//! No rule lives here: `sim-battle::naval` fights the battles and
//! `sim-campaign::naval` applies them to the campaign.

use crate::key_enum;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::ids::{FactionId, SeaZoneId, SettlementId, ShipClassId, UnitTypeId};
use crate::load::{load_entities, read_json, DataError};

/// How a ship moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Propulsion {
    /// Square sail only (cog, nef): fast with the wind, helpless against it.
    Sail,
    /// Oars (galley): independent of the wind, rowers tire.
    Oars,
    /// Sail and a few sweeps (barge, balinger).
    Mixed,
}

/// A class of ship (cog, nef, galley, barge).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipClass {
    pub id: ShipClassId,
    pub name: LocalizedName,
    pub propulsion: Propulsion,
    /// Length over all, metres.
    pub length_m: f64,
    /// Beam, metres.
    pub beam_m: f64,
    /// Height of the main deck above the water, metres.
    pub freeboard_m: f64,
    /// Height of the forecastle platform above the deck (0 = none).
    #[serde(default)]
    pub forecastle_m: f64,
    /// Height of the aftcastle platform above the deck (0 = none).
    #[serde(default)]
    pub aftcastle_m: f64,
    /// Shooters the castles hold (they shoot from the castle height).
    #[serde(default)]
    pub castle_capacity: u32,
    /// Hull points.
    pub hull: u32,
    /// Soldiers the ship carries.
    pub soldiers: u32,
    /// Sailors (they fight fires, cut grapples and handle the ship).
    pub sailors: u32,
    /// Rowers (galleys); they do not fight.
    #[serde(default)]
    pub rowers: u32,
    /// Best speed under sail, metres per second (0 = no sail).
    #[serde(default)]
    pub sail_speed: f64,
    /// Speed under oars, metres per second (0 = no oars).
    #[serde(default)]
    pub oar_speed: f64,
    /// Turn rate, degrees per second.
    pub turn_rate: f64,
    /// Share of missiles stopped by the bulwarks and pavises along the side.
    pub bulwark: f64,
    /// Ram or spur (galleys): hull damage per metre per second of impact.
    #[serde(default)]
    pub ram: f64,
    /// Share of fire damage the hull shrugs off (0-1).
    #[serde(default)]
    pub fire_resistance: f64,
    /// Campaign cost of one ship (livres tournois).
    #[serde(default)]
    pub cost: u32,
    /// Rendering model (`cog`, `nef`, `galley`, `barge`).
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl ShipClass {
    /// Height of the highest fighting platform above the water.
    pub fn castle_height(&self) -> f64 {
        self.freeboard_m + self.forecastle_m.max(self.aftcastle_m)
    }

    pub fn has_castles(&self) -> bool {
        self.forecastle_m > 0.0 || self.aftcastle_m > 0.0
    }
}

/// Coefficients of the naval battle, of its auto-resolve and of its
/// campaign consequences (`data/naval/rules.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavalRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // ----- shooting ---------------------------------------------------------
    /// Bow range at sea, metres (crossbows: `crossbow_range_m`).
    pub bow_range_m: f64,
    pub crossbow_range_m: f64,
    /// Seconds between two volleys of bows / crossbows.
    pub bow_reload_s: f64,
    pub crossbow_reload_s: f64,
    /// Men killed per shooter and per point of `ranged` / 100, per volley,
    /// at point-blank range.
    pub ranged_lethality: f64,
    /// Share of missiles stopped by 100 points of armour.
    pub armor_vs_ranged: f64,
    /// Accuracy bonus per metre of height of the shooter over the target.
    pub height_per_m: f64,
    /// Bounds of the height factor (1 ± this).
    pub height_cap: f64,
    /// Shooters downwind of the target lose this share of range and accuracy;
    /// upwind shooters gain it.
    pub wind_gauge: f64,
    /// Share of the fire the men in the castles take.
    pub castle_cover: f64,
    /// Fire added to the target per incendiary missile (fire arrows).
    pub fire_per_missile: f64,
    /// Share of a volley shot as fire arrows when the ship shoots fire.
    pub fire_arrow_share: f64,
    /// Wind alignment between the fleets above which a side holds the weather gauge (0-1).
    pub gauge_alignment: f64,
    /// Weakest wind drawn for a battle (0-1).
    pub wind_strength_min: f64,
    /// Strongest wind drawn for a battle (0-1).
    pub wind_strength_max: f64,
    /// The wind blows at least this many degrees off the axis between the fleets.
    pub gauge_angle_min_deg: f64,
    /// The wind blows at most this many degrees off the axis between the fleets.
    pub gauge_angle_max_deg: f64,
    /// Share of accuracy lost by a volley at the very end of its range (grows with the square of the distance).
    pub volley_falloff: f64,
    /// Accuracy factor of bows in the rain.
    pub rain_accuracy: f64,
    /// Largest share of the target's men sheltering in its castles (they take `castle_cover` of the fire).
    pub castle_shelter: f64,
    // ----- boarding ---------------------------------------------------------
    /// Men killed per second and per point of fighting power / 100.
    pub melee_lethality: f64,
    /// Share of melee blows stopped by 100 points of armour.
    pub armor_vs_melee: f64,
    /// Penalty per metre of deck height the boarders must climb (and bonus
    /// per metre they jump down).
    pub climb_per_m: f64,
    /// Bounds of the climb factor (1 ± this).
    pub climb_cap: f64,
    /// Bonus of defenders holding castles.
    pub castle_defense: f64,
    /// Bonus per free chained neighbour (reinforcements over the chains).
    pub chain_support: f64,
    /// Chained crews cannot run: their morale losses to volleys and to the
    /// loss of other ships are multiplied by this.
    pub chain_morale: f64,
    /// Share of their melee value that shooters with arrows left fight at.
    pub shooter_melee_share: f64,
    /// Melee power of one sailor, as a share of a man with 100 melee.
    pub sailor_melee: f64,
    /// Armour of the sailors (0-100).
    pub sailor_armor: f64,
    /// Melee power factor of a crew at zero morale (1 at full morale).
    pub melee_morale_floor: f64,
    // ----- morale and surrender --------------------------------------------
    pub morale_per_loss_percent: f64,
    /// Crew morale under which a ship strikes (or flees if free).
    pub surrender_morale: f64,
    /// A ship strikes when its fighting crew falls under this share.
    pub surrender_crew_share: f64,
    // ----- fire -------------------------------------------------------------
    /// Growth of a fire per second (share of itself) at full wind.
    pub fire_growth: f64,
    /// Fire put out per second by the whole sailing crew.
    pub fire_fighting: f64,
    /// Hull points burnt per second at full fire.
    pub fire_hull: f64,
    /// Share of the crew lost per second at full fire.
    pub fire_crew: f64,
    /// Morale lost per second aboard a ship at full fire.
    pub fire_morale: f64,
    /// Fire above which a ship is not boarded and an abandoned hulk is lost.
    pub burnt_fire: f64,
    /// Fire above which a burning ship sets its lashed neighbour alight.
    pub fire_spread_threshold: f64,
    /// Fire that spreads per second to a grappled or touching ship, × fire.
    pub fire_spread: f64,
    /// Above this fire the crew abandons the ship.
    pub fire_abandon: f64,
    /// Fire a fireship sets on the ship it grapples.
    pub fireship_fire: f64,
    /// Morale lost by a ship a fireship struck.
    pub fireship_morale: f64,
    /// Chance that a fireship of the fleet holding the weather gauge reaches its target.
    pub fireship_chance_gauge: f64,
    /// Chance that a fireship of the fleet downwind reaches its target.
    pub fireship_chance_lee: f64,
    /// Chance that a fireship reaches its target when nobody holds the gauge.
    pub fireship_chance_calm: f64,
    // ----- ramming and sinking ---------------------------------------------
    /// Ramming speed bonus (share of top speed).
    pub ram_speed: f64,
    /// Share of the ram's damage against a high-sided ship (see `ram_high_freeboard_m`).
    pub ram_high_factor: f64,
    /// Freeboard from which a ship counts as high-sided against a ram.
    pub ram_high_freeboard_m: f64,
    /// Morale lost by a ship struck by a ram.
    pub ram_morale: f64,
    /// Share of the men of a sinking ship who drown (+ armour / 100 × `drown_armor`).
    pub drown_base: f64,
    pub drown_armor: f64,
    // ----- auto-resolve -----------------------------------------------------
    /// Volleys before the ships close.
    pub auto_volleys: u32,
    /// Extra volleys of the side holding the weather gauge.
    pub auto_gauge_volleys: u32,
    /// Boarding rounds.
    pub auto_rounds: u32,
    /// Seconds of melee one boarding round stands for.
    pub auto_round_seconds: f64,
    /// Seconds of shooting one auto-resolve volley stands for.
    pub auto_volley_seconds: f64,
    /// Share of its range at which a crew shoots in the auto-resolve.
    pub auto_range_share: f64,
    /// Wind alignment given to volleys of the gauge side (and, negated, of the other).
    pub auto_volley_alignment: f64,
    /// Share of the melee morale loss that volleys inflict.
    pub auto_volley_morale: f64,
    /// Share of the chain support that counts when every chained ship is engaged.
    pub auto_chain_share: f64,
    /// Melee power floor when a ship picks the enemy it can climb onto best.
    pub auto_min_power: f64,
    /// A side wins when its standing strength exceeds the other's by this factor; the weather gauge decides otherwise.
    pub auto_decisive_margin: f64,
    /// Chance that a beaten galley gets away.
    pub escape_chance_oars: f64,
    /// Chance that a beaten sail ship holding the gauge gets away.
    pub escape_chance_gauge: f64,
    /// Chance that a beaten sail ship downwind gets away.
    pub escape_chance_other: f64,
    /// Random spread of the auto-resolve (± share).
    pub auto_jitter: f64,
    // ----- campaign ---------------------------------------------------------
    /// Chance that a hostile fleet intercepts a crossing, before sea control.
    pub intercept_base: f64,
    /// Added chance per point of the enemy's control of the sea zone / 100.
    pub intercept_per_control: f64,
    /// Sea control gained by the victor of a naval battle (0-100).
    pub control_victory: u32,
    /// Sea control lost per season by an idle holder.
    pub control_decay: u32,
    /// Control above which the ports of the other factions are blockaded.
    pub blockade_control: u32,
    /// Treasury lost per blockaded port and season (livres).
    pub blockade_toll: u32,
    /// Share of the ships of a faction's fleet sent to intercept (0-1).
    pub squadron_share: f64,
    /// Largest squadron and escort, ships.
    pub squadron_max: u32,
    /// Marines per intercepting ship when the fleet sails without an army:
    /// share of archers, the rest men-at-arms.
    pub marine_archer_share: f64,
}

crate::bundled_rules!(NavalRules, "naval/rules.json", default);

/// Ships of one faction at the start of the campaign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionFleet {
    pub faction: FactionId,
    /// Ships per class.
    pub ships: BTreeMap<ShipClassId, u32>,
    /// Seas where the fleet musters (it may intercept there).
    pub seas: Vec<SeaZoneId>,
    /// Men the fleet embarks when it sails without an army.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marines: Option<Marines>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Unit types of a fleet's marines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marines {
    /// Shooters (archers, crossbowmen).
    pub archers: UnitTypeId,
    /// Boarders (men-at-arms).
    pub soldiers: UnitTypeId,
}

/// Contents of `data/naval/fleets.json`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavalFleets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fleets: Vec<FactionFleet>,
    /// French names of the seas (`sea_channel` → « la Manche »).
    #[serde(default)]
    pub sea_names: BTreeMap<SeaZoneId, String>,
    /// Waters of a crossing from or to a port, instead of the sea's name
    /// (`set_calais` → « le pas de Calais »), lot NV2.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub port_waters: BTreeMap<SettlementId, String>,
    /// Marines of fleets that give none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_marines: Option<Marines>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// Historical ship names of one faction (lot NV2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionShipNames {
    pub faction: FactionId,
    /// Names without a home port.
    pub names: Vec<String>,
    /// Names of the ships of each port (« la Marguerite de Rye »).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<SettlementId, Vec<String>>,
}

/// `data/naval/ship_names.json` (lot NV2).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavalShipNames {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub factions: Vec<FactionShipNames>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl NavalShipNames {
    pub fn of(&self, faction: &FactionId) -> Option<&FactionShipNames> {
        self.factions.iter().find(|f| &f.faction == faction)
    }
}

key_enum! {
/// Coastal navigation or open sea (lot SL1, ADR 0139).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeaLaneKind {
    /// Cabotage along the coast: short legs, sheltered.
    Coastal => "coastal",
    /// Open sea: long legs out of sight of land, exposed to gales but
    /// harder for a squadron to find.
    OpenSea => "open_sea",
}
}

/// One value per season (keys of the campaign seasons).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonValues {
    pub spring: f64,
    pub summer: f64,
    pub autumn: f64,
    pub winter: f64,
}

impl SeasonValues {
    /// Value of the season `index` (0 spring, 1 summer, 2 autumn, 3 winter).
    pub fn at(&self, index: usize) -> f64 {
        match index % 4 {
            0 => self.spring,
            1 => self.summer,
            2 => self.autumn,
            _ => self.winter,
        }
    }
}

/// Rules of one kind of sea lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaLaneKindRules {
    /// Display name (« haute mer »).
    pub name: String,
    /// Men lost (%) by the regiments of a crossing army, per season.
    pub storm_loss_percent: SeasonValues,
    /// Value factor of the trade using the lane, per season.
    pub trade_season_factor: SeasonValues,
    /// Factor of the interception chance of a crossing (NV1).
    pub intercept_factor: f64,
}

/// The two kinds of lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaLaneKinds {
    pub coastal: SeaLaneKindRules,
    pub open_sea: SeaLaneKindRules,
}

/// Rules of `data/naval/sea_lanes.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaLaneRules {
    /// Movement points per kilometre of lane.
    pub cost_per_km: f64,
    /// Floor of a lane's cost, in steps (× `points_per_step`).
    pub min_cost_steps: f64,
    /// Security a trade route loses per point of hostile control of a sea it
    /// crosses (× control / 100), below the blockade threshold.
    pub hostile_control_security: f64,
    pub kinds: SeaLaneKinds,
}

impl Default for SeaLaneRules {
    fn default() -> Self {
        let neutral = |name: &str| SeaLaneKindRules {
            name: name.to_owned(),
            storm_loss_percent: SeasonValues {
                spring: 0.0,
                summer: 0.0,
                autumn: 0.0,
                winter: 0.0,
            },
            trade_season_factor: SeasonValues {
                spring: 1.0,
                summer: 1.0,
                autumn: 1.0,
                winter: 1.0,
            },
            intercept_factor: 1.0,
        };
        SeaLaneRules {
            cost_per_km: 0.5,
            min_cost_steps: 2.0,
            hostile_control_security: 0.0,
            kinds: SeaLaneKinds {
                coastal: neutral("cabotage"),
                open_sea: neutral("haute mer"),
            },
        }
    }
}

impl SeaLaneRules {
    pub fn kind(&self, kind: SeaLaneKind) -> &SeaLaneKindRules {
        match kind {
            SeaLaneKind::Coastal => &self.kinds.coastal,
            SeaLaneKind::OpenSea => &self.kinds.open_sea,
        }
    }
}

/// A sea lane between two ports (lot SL1, ADR 0139).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaLane {
    pub id: String,
    /// Display name (« Route du vin de Gascogne »).
    pub name: String,
    pub from: SettlementId,
    pub to: SettlementId,
    /// Sea crossed: control, interception, blockade.
    pub sea: SeaZoneId,
    pub kind: SeaLaneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl SeaLane {
    /// Whether the lane links `a` and `b` (either way).
    pub fn links(&self, a: &SettlementId, b: &SettlementId) -> bool {
        (&self.from == a && &self.to == b) || (&self.from == b && &self.to == a)
    }
}

/// `data/naval/sea_lanes.json` (`sea_lanes.schema.json`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaLanes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub rules: SeaLaneRules,
    #[serde(default)]
    pub lanes: Vec<SeaLane>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// One lane of `data/map/sea_lanes_px.json` (generated by `cent-ans geo
/// sea-lanes`): the core only reads its length.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct SeaLaneGeometry {
    id: String,
    length_km: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct SeaLaneGeometryFile {
    lanes: Vec<SeaLaneGeometry>,
}

/// Everything under `data/naval/`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavalData {
    pub ship_classes: BTreeMap<ShipClassId, ShipClass>,
    /// Shared by every naval battle setup.
    pub rules: Arc<NavalRules>,
    pub fleets: NavalFleets,
    /// Ship names per faction and port (absent: « Nef n°1 »).
    pub ship_names: NavalShipNames,
    /// Sea lanes between ports (lot SL1).
    pub sea_lanes: SeaLanes,
    /// Routed length (km) of each lane, from `data/map/sea_lanes_px.json`;
    /// a lane missing there falls back to its great-circle length × 1.25.
    pub lane_lengths_km: BTreeMap<String, f64>,
}

/// Folder and files of the naval data, relative to `data/`.
pub mod files {
    pub const NAVAL: &str = "naval";
    pub const SHIPS: &str = "ships";
    pub const RULES: &str = "rules.json";
    pub const FLEETS: &str = "fleets.json";
    pub const SHIP_NAMES: &str = "ship_names.json";
    pub const SEA_LANES: &str = "sea_lanes.json";
    /// Inside `data/map/`: routed geometry of the lanes.
    pub const SEA_LANES_PX: &str = "sea_lanes_px.json";
}

impl NavalData {
    /// Loads `root/naval/` (`root` = the `data/` directory); defaults and no
    /// ship when the folder is absent.
    pub fn load(root: &Path) -> Result<NavalData, DataError> {
        let dir = root.join(files::NAVAL);
        let mut naval = NavalData::default();
        if !dir.is_dir() {
            return Ok(naval);
        }
        let ships = dir.join(files::SHIPS);
        if ships.is_dir() {
            naval.ship_classes = load_entities(&ships, |s: &ShipClass| &s.id)?;
        }
        let rules = dir.join(files::RULES);
        if rules.is_file() {
            naval.rules = Arc::new(read_json(&rules)?);
        }
        let fleets = dir.join(files::FLEETS);
        if fleets.is_file() {
            naval.fleets = read_json(&fleets)?;
        }
        let names = dir.join(files::SHIP_NAMES);
        if names.is_file() {
            naval.ship_names = read_json(&names)?;
        }
        let lanes = dir.join(files::SEA_LANES);
        if lanes.is_file() {
            naval.sea_lanes = read_json(&lanes)?;
        }
        let geometry = root.join("map").join(files::SEA_LANES_PX);
        if geometry.is_file() {
            let file: SeaLaneGeometryFile = read_json(&geometry)?;
            naval.lane_lengths_km = file
                .lanes
                .into_iter()
                .map(|lane| (lane.id, lane.length_km))
                .collect();
        }
        Ok(naval)
    }

    pub fn ship(&self, id: &str) -> Option<&ShipClass> {
        self.ship_classes.get(id)
    }

    /// French name of a sea (« la Manche »), its id otherwise.
    pub fn sea_name(&self, sea: &SeaZoneId) -> String {
        self.fleets
            .sea_names
            .get(sea)
            .cloned()
            .unwrap_or_else(|| sea.to_string())
    }

    /// The sea lane between two ports (either way), if any.
    pub fn lane_between(&self, a: &SettlementId, b: &SettlementId) -> Option<&SeaLane> {
        self.sea_lanes.lanes.iter().find(|lane| lane.links(a, b))
    }

    /// The starting fleet of `faction`, if any.
    pub fn fleet_of(&self, faction: &FactionId) -> Option<&FactionFleet> {
        self.fleets.fleets.iter().find(|f| &f.faction == faction)
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<SeaLaneKind>();
    }
}
