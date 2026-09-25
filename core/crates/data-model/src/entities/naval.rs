//! Naval warfare data (lot NV1, ADR 0028): ship classes
//! (`data/naval/ships/*.json`, `ship_class.schema.json`), naval rules
//! (`data/naval/rules.json`, `naval_rules.schema.json`) and the fleets of
//! 1337 (`data/naval/fleets.json`, `naval_fleets.schema.json`).
//!
//! No rule lives here: `sim-battle::naval` fights the battles and
//! `sim-campaign::naval` applies them to the campaign.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::ids::{FactionId, SeaZoneId, ShipClassId, UnitTypeId};
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
    // ----- manoeuvre --------------------------------------------------------
    /// Wind speed factor bounds: a sail ship reaches `sail_speed × (min +
    /// (1 - min) × wind)`.
    pub wind_speed_min: f64,
    /// Sail ships cannot point closer to the wind than this, degrees off
    /// the wind's eye.
    pub close_hauled_deg: f64,
    /// Rowers' stamina in seconds at full stroke.
    pub rower_stamina_s: f64,
    /// Drift of a ship without way, metres per second at full wind.
    pub drift_speed: f64,
    // ----- grapples and boarding -------------------------------------------
    /// Distance between hull sides under which grapples can be thrown.
    pub grapple_gap_m: f64,
    /// Chance per second that the grapples hold.
    pub grapple_chance: f64,
    /// Chance per second (× sailors share) that a ship cuts itself free.
    pub cut_chance: f64,
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
    // ----- AI: general boarding (lot NV2) -----------------------------------
    /// The AI shares its targets out: at most this many of its ships board
    /// one enemy ship.
    #[serde(default = "default_boarders_per_target")]
    pub boarders_per_target: u32,
    /// A fleet calls the general boarding once most of its ships are within
    /// this distance of an enemy, metres.
    #[serde(default = "default_assault_range_m")]
    pub assault_range_m: f64,
    /// Seconds of volleys at that range before the general boarding.
    #[serde(default = "default_assault_softening_s")]
    pub assault_softening_s: f64,
    /// In the general boarding, a ship boards when its fighting power (after
    /// the climb) reaches this share of its target's.
    #[serde(default = "default_assault_odds")]
    pub assault_odds: f64,
    /// Chained crews cannot run: their morale losses to volleys and to the
    /// loss of other ships are multiplied by this.
    #[serde(default = "default_chain_morale")]
    pub chain_morale: f64,
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
    /// Fire that spreads per second to a grappled or touching ship, × fire.
    pub fire_spread: f64,
    /// Above this fire the crew abandons the ship.
    pub fire_abandon: f64,
    /// Fire a fireship sets on the ship it grapples.
    pub fireship_fire: f64,
    // ----- ramming and sinking ---------------------------------------------
    /// Ramming speed bonus (share of top speed).
    pub ram_speed: f64,
    /// Share of the ram's damage against a high-sided ship (freeboard ≥ 2.5 m).
    pub ram_high_factor: f64,
    /// Seconds a ship takes to go under.
    pub sink_seconds: f64,
    /// Share of the men of a sinking ship who drown (+ armour / 100 × `drown_armor`).
    pub drown_base: f64,
    pub drown_armor: f64,
    /// Distance from the centre past which a fleeing ship has escaped.
    pub escape_radius_m: f64,
    /// Longest battle, seconds.
    pub max_duration_s: f64,
    // ----- auto-resolve -----------------------------------------------------
    /// Volleys before the ships close.
    pub auto_volleys: u32,
    /// Extra volleys of the side holding the weather gauge.
    pub auto_gauge_volleys: u32,
    /// Boarding rounds.
    pub auto_rounds: u32,
    /// Seconds of melee one boarding round stands for.
    pub auto_round_seconds: f64,
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

fn default_boarders_per_target() -> u32 {
    2
}

fn default_assault_range_m() -> f64 {
    250.0
}

fn default_assault_softening_s() -> f64 {
    30.0
}

fn default_assault_odds() -> f64 {
    0.55
}

fn default_chain_morale() -> f64 {
    0.3
}

impl Default for NavalRules {
    fn default() -> Self {
        NavalRules {
            description: None,
            bow_range_m: 180.0,
            crossbow_range_m: 160.0,
            bow_reload_s: 6.0,
            crossbow_reload_s: 12.0,
            ranged_lethality: 0.09,
            armor_vs_ranged: 0.7,
            height_per_m: 0.1,
            height_cap: 0.5,
            wind_gauge: 0.2,
            castle_cover: 0.5,
            fire_per_missile: 0.012,
            fire_arrow_share: 0.15,
            wind_speed_min: 0.35,
            close_hauled_deg: 67.0,
            rower_stamina_s: 240.0,
            drift_speed: 0.4,
            grapple_gap_m: 6.0,
            grapple_chance: 0.35,
            cut_chance: 0.1,
            melee_lethality: 0.02,
            armor_vs_melee: 0.6,
            climb_per_m: 0.12,
            climb_cap: 0.45,
            castle_defense: 0.15,
            chain_support: 0.12,
            boarders_per_target: default_boarders_per_target(),
            assault_range_m: default_assault_range_m(),
            assault_softening_s: default_assault_softening_s(),
            assault_odds: default_assault_odds(),
            chain_morale: default_chain_morale(),
            morale_per_loss_percent: 1.1,
            surrender_morale: 18.0,
            surrender_crew_share: 0.12,
            fire_growth: 0.06,
            fire_fighting: 0.01,
            fire_hull: 1.6,
            fire_crew: 0.01,
            fire_spread: 0.05,
            fire_abandon: 0.75,
            fireship_fire: 0.45,
            ram_speed: 0.3,
            ram_high_factor: 0.4,
            sink_seconds: 25.0,
            drown_base: 0.35,
            drown_armor: 0.5,
            escape_radius_m: 1150.0,
            max_duration_s: 1800.0,
            auto_volleys: 3,
            auto_gauge_volleys: 2,
            auto_rounds: 4,
            auto_round_seconds: 45.0,
            auto_jitter: 0.1,
            intercept_base: 0.2,
            intercept_per_control: 0.5,
            control_victory: 40,
            control_decay: 10,
            blockade_control: 60,
            blockade_toll: 150,
            squadron_share: 0.5,
            squadron_max: 12,
            marine_archer_share: 0.4,
        }
    }
}

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
    /// Marines of fleets that give none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_marines: Option<Marines>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// Everything under `data/naval/`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavalData {
    pub ship_classes: BTreeMap<ShipClassId, ShipClass>,
    pub rules: NavalRules,
    pub fleets: NavalFleets,
}

/// Folder and files of the naval data, relative to `data/`.
pub mod files {
    pub const NAVAL: &str = "naval";
    pub const SHIPS: &str = "ships";
    pub const RULES: &str = "rules.json";
    pub const FLEETS: &str = "fleets.json";
    pub const SCENARIOS: &str = "scenarios";
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
            naval.rules = read_json(&rules)?;
        }
        let fleets = dir.join(files::FLEETS);
        if fleets.is_file() {
            naval.fleets = read_json(&fleets)?;
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

    /// The starting fleet of `faction`, if any.
    pub fn fleet_of(&self, faction: &FactionId) -> Option<&FactionFleet> {
        self.fleets.fleets.iter().find(|f| &f.faction == faction)
    }
}
