//! Historical battle maps (lot EP7, ADR 0035): Crécy (26 August 1346),
//! Poitiers (19 September 1356), Agincourt (25 October 1415).
//!
//! A map (`data/battle_maps/<id>.json`, schema
//! `data/schemas/battle_map.schema.json`) describes:
//!
//! - **the site**: the real relief of the place (heights baked from the
//!   Copernicus DEM by `cent-ans geo battle-site`, laid on the height grid
//!   of the field), the woods, marshes, brooks, roads, hedges and ditches,
//!   and the period decor as an EP6 [`DecorPlan`] (windmill, villages,
//!   vineyards, ploughland, camps);
//! - **the scenario**: the two orders of battle regiment by regiment, their
//!   historical positions and facings, the successive French "battles"
//!   (waves held back until released, [`Wave`]), the English posted on
//!   their ground ([`RegimentBlock::hold`]), the weather (with its changes
//!   during the battle) and the starting hour (EP8
//!   [`BattleSim::set_start_hour`]).
//!
//! Field coordinates as everywhere in the battle: metres, `x` from 0 to the
//! width, `z` from 0 (the attacker's edge) to the depth. The real site is
//! rotated so that the attacker's advance (+z) points at
//! [`SiteAnchor::bearing_deg`] on the compass.
//!
//! A campaign battle fought in the province of a map, within its years
//! ([`CampaignEntry`]), is fought on the historical site (relief, water,
//! woods, decor) with the campaign armies deployed as usual
//! ([`HistoricalMap::apply_site`]).

use std::collections::BTreeMap;

use data_model::{BattleOrder, BattleStandardRules, Terrain, UnitType, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::decor::DecorPlan;
use crate::field::{Battlefield, Weather, Zone};
use crate::hydro::{Road, Stream};
use crate::scale::{BattleScale, FieldSize, GRID_RESOLUTION};
use crate::setup::{BattleSeason, BattleSetup, GeneralSetup, SideId, SideSetup, UnitSetup};
use crate::sim::BattleSim;
use crate::site::{Ground, Obstacle};
use crate::unit::Formation;

/// Where the map lies on the real map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteAnchor {
    /// Longitude of the centre of the field (degrees east, WGS 84).
    pub lon: f64,
    /// Latitude of the centre of the field (degrees north).
    pub lat: f64,
    /// Compass bearing (degrees, 0 = north, clockwise) of +z, the
    /// attacker's advance.
    pub bearing_deg: f64,
    /// Name of the place ("Crécy-en-Ponthieu").
    pub place: String,
}

/// Size of the field and layout of the deployment lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapField {
    pub width_m: f64,
    pub depth_m: f64,
    pub line_gap_m: f64,
    pub zone_depth_m: f64,
    pub max_regiments_per_side: usize,
}

/// Real relief of the site: heights (decimetres above the lowest point)
/// on a grid of `nx` × `nz` points every `step_m` metres, row-major (rows
/// of constant z, from z = 0), baked by `cent-ans geo battle-site`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapRelief {
    pub step_m: f64,
    pub nx: usize,
    pub nz: usize,
    /// Real altitude (m) of height 0.
    #[serde(default)]
    pub base_m: f64,
    /// Exaggeration of the relief on the field (1 = true relief).
    #[serde(default = "one")]
    pub vertical_scale: f64,
    pub heights_dm: Vec<i32>,
    #[serde(default)]
    pub source: String,
}

fn one() -> f64 {
    1.0
}

impl MapRelief {
    /// Height (m, before exaggeration) at field point (x, z), bilinear,
    /// clamped to the grid.
    pub fn sample(&self, x: f64, z: f64) -> f64 {
        if self.nx < 2 || self.nz < 2 || self.heights_dm.len() < self.nx * self.nz {
            return 0.0;
        }
        let fx = (x / self.step_m).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.step_m).clamp(0.0, (self.nz - 1) as f64);
        let (ix, iz) = (
            (fx as usize).min(self.nx - 2),
            (fz as usize).min(self.nz - 2),
        );
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let at = |i: usize, j: usize| f64::from(self.heights_dm[j * self.nx + i]) * 0.1;
        let top = at(ix, iz) * (1.0 - tx) + at(ix + 1, iz) * tx;
        let bottom = at(ix, iz + 1) * (1.0 - tx) + at(ix + 1, iz + 1) * tx;
        top * (1.0 - tz) + bottom * tz
    }
}

/// A change of weather during the battle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherChange {
    /// Battle seconds after the start.
    pub at_s: f64,
    pub weather: Weather,
}

/// Weather of the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapWeather {
    pub start: Weather,
    #[serde(default)]
    pub changes: Vec<WeatherChange>,
    /// French label of the weather for the loading card ("Averse d'orage").
    #[serde(default)]
    pub label: String,
}

/// One or several regiments of the same type side by side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegimentBlock {
    pub unit_type: String,
    /// Regiments in the block, laid side by side along the front.
    #[serde(default = "one_u32")]
    pub count: u32,
    /// Soldiers per regiment (the type's full strength when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soldiers: Option<u32>,
    /// Centre of the block.
    pub x: f64,
    pub z: f64,
    /// Facing (degrees, 0 = +z, 90 = +x).
    pub facing_deg: f64,
    /// Gap between two regiments of the block (m).
    #[serde(default = "default_gap")]
    pub gap_m: f64,
    /// Index of the wave ([`MapArmy::waves`]) the block belongs to.
    #[serde(default)]
    pub wave: usize,
    /// The block holds its ground: the AI moves it no farther than
    /// `leash_m` from its post (it still shoots, fights on contact and
    /// counter-charges within the leash).
    #[serde(default)]
    pub hold: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leash_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<Formation>,
    /// Men-at-arms fighting on foot (Poitiers, Agincourt).
    #[serde(default)]
    pub dismounted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub morale: Option<u8>,
    #[serde(default)]
    pub experience: u8,
    /// Missiles carried (the type's full load when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ammo: Option<u32>,
    /// Who they were ("Arbalétriers génois d'Ottone Doria").
    #[serde(default)]
    pub label: String,
}

fn one_u32() -> u32 {
    1
}

fn default_gap() -> f64 {
    8.0
}

/// A "battle" (division) of an army, held back until released.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub label: String,
    /// Released this many battle seconds after the start at the latest.
    #[serde(default)]
    pub release_s: f64,
    /// Released earlier as soon as wave `after` is engaged (half of its
    /// regiments in a melee, routing or gone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<usize>,
    /// ... or as soon as the enemy's wave `after_enemy` is engaged (the
    /// captal de Buch at Poitiers waits for the king's battle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_enemy: Option<usize>,
    /// Once released, the wave goes straight at the enemy (the AI no longer
    /// weighs whether to attack: the French "battles" of Crécy).
    #[serde(default)]
    pub assault: bool,
}

/// General of a historical army.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapGeneral {
    pub name: String,
    #[serde(default)]
    pub character: String,
    /// Index of the general's regiment, counting regiments block after
    /// block.
    pub regiment: usize,
    pub command: u8,
    #[serde(default)]
    pub sovereign: bool,
    #[serde(default)]
    pub morale_bonus: f64,
    #[serde(default)]
    pub charge_percent: f64,
    #[serde(default)]
    pub ranged_percent: f64,
    #[serde(default)]
    pub defense_percent: f64,
}

/// One army of the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapArmy {
    pub faction: String,
    pub faction_name: String,
    #[serde(default)]
    pub army: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general: Option<MapGeneral>,
    pub regiments: Vec<RegimentBlock>,
    /// Waves of the army; wave 0 is free from the start. Empty: one wave.
    #[serde(default)]
    pub waves: Vec<Wave>,
}

/// Both armies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapArmies {
    pub attacker: MapArmy,
    pub defender: MapArmy,
}

impl MapArmies {
    pub fn side(&self, side: SideId) -> &MapArmy {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }
}

/// When a campaign battle is fought on the historical site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignEntry {
    pub from_year: i32,
    pub to_year: i32,
}

/// A historical battle map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalMap {
    pub id: String,
    pub name: String,
    /// ISO date of the battle.
    pub date: String,
    pub year: i32,
    #[serde(default)]
    pub summary: String,
    pub province: String,
    pub province_name: String,
    pub terrain: Terrain,
    pub season: BattleSeason,
    pub site: SiteAnchor,
    pub field: MapField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relief: Option<MapRelief>,
    #[serde(default)]
    pub ground: Ground,
    pub weather: MapWeather,
    /// Hour of the day when the battle begins (EP8).
    pub start_hour: f64,
    #[serde(default)]
    pub woods: Vec<Zone>,
    #[serde(default)]
    pub mud: Vec<Zone>,
    /// Marsh pools (shallow water).
    #[serde(default)]
    pub pools: Vec<Zone>,
    #[serde(default)]
    pub streams: Vec<Stream>,
    #[serde(default)]
    pub roads: Vec<Road>,
    /// Hedges, fences and ditches.
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    #[serde(default)]
    pub decor: DecorPlan,
    pub armies: MapArmies,
    /// The side history gave the victory to.
    pub historical_winner: SideId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign: Option<CampaignEntry>,
    /// Horizon tile of the site (`game/assets/horizon/relief/index.json`).
    #[serde(default)]
    pub horizon: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// Leash of a regiment holding its ground when the map does not give one.
pub const DEFAULT_LEASH_M: f64 = 60.0;

impl HistoricalMap {
    /// Parses a map (`data/battle_maps/<id>.json`).
    pub fn from_json(text: &str) -> Result<HistoricalMap, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Size of the field.
    pub fn field_size(&self) -> FieldSize {
        FieldSize {
            width: self.field.width_m,
            depth: self.field.depth_m,
            line_gap: self.field.line_gap_m,
            zone_depth: self.field.zone_depth_m,
        }
    }

    /// Scale of the battle: the map's field, its regiment cap.
    pub fn scale(&self) -> BattleScale {
        BattleScale {
            key: "epic".to_owned(),
            field: self.field_size(),
            max_on_field: self.field.max_regiments_per_side,
        }
    }

    /// The date in French ("26 août 1346").
    pub fn date_fr(&self) -> String {
        const MONTHS: [&str; 12] = [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ];
        let parts: Vec<&str> = self.date.split('-').collect();
        match parts.as_slice() {
            [year, month, day] => {
                let month = month
                    .parse::<usize>()
                    .ok()
                    .and_then(|m| MONTHS.get(m.wrapping_sub(1)));
                let day = day.parse::<u32>().unwrap_or(0);
                match month {
                    Some(m) if day == 1 => format!("1er {m} {year}"),
                    Some(m) => format!("{day} {m} {year}"),
                    None => self.date.clone(),
                }
            }
            _ => self.date.clone(),
        }
    }

    /// Weather at the end of the scripted changes (the sky the battle is
    /// rendered under; a shower at the start falls from it).
    pub fn final_weather(&self) -> Weather {
        self.weather
            .changes
            .iter()
            .max_by(|a, b| a.at_s.total_cmp(&b.at_s))
            .map_or(self.weather.start, |c| c.weather)
    }

    /// Key of the horizon tile of the site.
    pub fn horizon_key(&self) -> String {
        if self.horizon.is_empty() {
            format!("hist_{}", self.id)
        } else {
            self.horizon.clone()
        }
    }

    /// Is a campaign battle in `province` in `year` fought on this site?
    pub fn matches_campaign(&self, province: &str, year: i32) -> bool {
        self.province == province
            && self
                .campaign
                .as_ref()
                .is_some_and(|c| (c.from_year..=c.to_year).contains(&year))
    }

    /// The battle setup of the map: both armies from `unit_types`, the
    /// leader's orders and the standards of the campaign data. The site
    /// itself is laid by [`Self::start`] (the setup asks for a bare field).
    pub fn battle_setup(
        &self,
        unit_types: &BTreeMap<UnitTypeId, UnitType>,
        orders: Vec<BattleOrder>,
        standards: Option<BattleStandardRules>,
        player_side: Option<SideId>,
    ) -> Result<BattleSetup, String> {
        let side = |side: SideId| -> Result<SideSetup, String> {
            let army = self.armies.side(side);
            let mut units = Vec::new();
            for block in &army.regiments {
                let id = UnitTypeId::new(&block.unit_type).map_err(|e| e.to_string())?;
                let unit_type = unit_types
                    .get(&id)
                    .ok_or_else(|| format!("{}: unknown unit type {}", self.id, block.unit_type))?;
                let soldiers = block.soldiers.unwrap_or(unit_type.soldiers);
                let morale = block.morale.unwrap_or(unit_type.stats.morale);
                for _ in 0..block.count.max(1) {
                    let mut unit =
                        UnitSetup::from_unit_type(unit_type, soldiers, morale, block.experience);
                    if !block.label.is_empty() {
                        unit.name = block.label.clone();
                    }
                    units.push(unit);
                }
            }
            let general = army.general.as_ref().map(|g| GeneralSetup {
                character: if g.character.is_empty() {
                    format!("hist_{}_{}", self.id, side.key())
                } else {
                    g.character.clone()
                },
                name: g.name.clone(),
                command: g.command,
                unit_index: g.regiment.min(units.len().saturating_sub(1)),
                morale_bonus: g.morale_bonus,
                charge_percent: g.charge_percent,
                ranged_percent: g.ranged_percent,
                defense_percent: g.defense_percent,
                sovereign: g.sovereign,
            });
            Ok(SideSetup {
                faction: army.faction.clone(),
                faction_name: army.faction_name.clone(),
                army: army.army.clone(),
                units,
                general,
            })
        };
        Ok(BattleSetup {
            province: self.province.clone(),
            province_name: self.province_name.clone(),
            terrain: self.terrain,
            river: false,
            season: self.season,
            coastal: false,
            village: Some(false),
            attacker: side(SideId::Attacker)?,
            defender: side(SideId::Defender)?,
            player_side,
            siege: None,
            siege_layout: None,
            orders,
            standards,
            decor_plan: None,
        })
    }

    /// Lays the site on `field`: real relief, woods, marsh, brooks, roads,
    /// hedges and ditches, then the period decor. The field must have the
    /// map's size (a campaign battle on the site uses [`Self::scale`]).
    pub fn apply_site(&self, field: &mut Battlefield) {
        if let Some(relief) = &self.relief {
            let scale = relief.vertical_scale;
            let mut lowest = f64::INFINITY;
            for j in 0..field.nz {
                for i in 0..field.nx {
                    let h = relief.sample(i as f64 * GRID_RESOLUTION, j as f64 * GRID_RESOLUTION);
                    field.heights[j * field.nx + i] = h * scale;
                    lowest = lowest.min(h * scale);
                }
            }
            if lowest.is_finite() {
                for h in &mut field.heights {
                    *h -= lowest;
                }
            }
        }
        field.terrain = self.terrain;
        field.season = self.season;
        field.ground = self.ground;
        field.river = None;
        field.coast = None;
        field.village = None;
        field.oxbows.clear();
        field.bridges.clear();
        field.forests = self.woods.clone();
        field.forest_parts.clear();
        field.mud = self.mud.clone();
        field.mud_parts.clear();
        field.pools = self.pools.clone();
        field.streams = self.streams.clone();
        field.roads = self.roads.clone();
        field.obstacles = self.obstacles.clone();
        let mut plan = self.decor.clone();
        plan.clear = true;
        field.apply_decor_plan(&plan);
    }

    /// Starts the battle of the map: bare field of the map's size, site,
    /// historical deployment, weather, hour and the scenario (waves, posts,
    /// changes of weather).
    pub fn start(&self, setup: BattleSetup, seed: u64) -> Result<BattleSim, String> {
        let mut sim =
            BattleSim::new_scaled(setup, seed, self.scale()).map_err(|e| e.to_string())?;
        self.apply_site(sim.field_mut());
        sim.set_weather(self.weather.start);
        sim.set_start_hour(self.start_hour);
        let weather = if self.weather.label.is_empty() {
            self.weather.start.label_fr().to_owned()
        } else {
            self.weather.label.clone()
        };
        sim.set_opening_line(format!("{}, {} : {}.", self.name, self.site.place, weather));
        sim.deploy_historical(&self.armies, &self.weather.changes);
        Ok(sim)
    }
}

/// The side of `setup` by key.
pub fn side_of(key: &str) -> Option<SideId> {
    SideId::parse(key)
}
