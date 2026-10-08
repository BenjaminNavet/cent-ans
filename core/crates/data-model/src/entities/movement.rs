//! Free army movement on the navigation grid (lot M2), mirroring
//! `data/schemas/movement_rules.schema.json` (`data/movement/rules.json`).
//!
//! Grid costs have no unit: 10 is one plain cell of the navigation grid
//! (about 1.44 km); 255 is impassable.

use serde::{Deserialize, Serialize};

/// Cost of one navigation-grid cell per terrain (10 = plain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainCosts {
    pub plains: u8,
    pub hills: u8,
    pub forest: u8,
    pub marsh: u8,
    pub mountains: u8,
    /// OM3 (ADR 0116): dry grassland, plains pace by default.
    #[serde(default = "default_steppe_cost")]
    pub steppe: u8,
    /// OM3 (ADR 0116): sand and stone, slower than plains.
    #[serde(default = "default_desert_cost")]
    pub desert: u8,
}

fn default_steppe_cost() -> u8 {
    10
}

fn default_desert_cost() -> u8 {
    16
}

impl TerrainCosts {
    /// Cost of a province terrain relative to plains (1.0), for the
    /// terrains the grid pipeline marks per province (steppe, desert);
    /// `None` for the others, whose province-level factor stays the v1 one.
    pub fn province_factor(&self, terrain: crate::Terrain) -> Option<f64> {
        let plains = f64::from(self.plains.max(1));
        match terrain {
            crate::Terrain::Steppe => Some(f64::from(self.steppe) / plains),
            crate::Terrain::Desert => Some(f64::from(self.desert) / plains),
            _ => None,
        }
    }
}

crate::bundled_rules!(TerrainCosts, "movement/rules.json", at "/terrain_costs", default);

/// How much of the turn a port-to-port crossing costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbarkCost {
    /// The whole turn: the army must not have moved, and cannot move after.
    #[default]
    All,
}

/// Contents of `data/movement/rules.json` (lots M1 and M2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeMovementRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Radius of an army's zone of control: an enemy army entering it stops.
    pub zoc_radius_km: f64,
    /// Maximum distance between two armies for an attack.
    pub engage_radius_km: f64,
    /// Fallback of a beaten army away from the victor when no friendly
    /// place is within reach.
    pub retreat_fallback_km: f64,
    /// Line of sight around an army (lot M5).
    pub vision_army_km: f64,
    /// Line of sight around a settlement (lot M5).
    pub vision_settlement_km: f64,
    /// Grid cost per terrain; `plains` is the unit (10) of movement points.
    pub terrain_costs: TerrainCosts,
    /// Multiplier of the cost of a road cell.
    pub road_cost_factor: f64,
    /// Extra cost of a cell crossed by a minor river.
    pub minor_river_extra: u8,
    /// Slope above which a cell is impassable (pipeline only).
    pub slope_impassable_threshold: f64,
    /// Lot M5b (pipeline only): an Itiner-e road / major river crossing
    /// opens a passage only within this distance of a settlement or of a
    /// known bridge or ford; `None` keeps them all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub road_crossing_radius_km: Option<f64>,
    pub embark_cost: EmbarkCost,
    /// Lot EM (ADR 0167): sea edges a voyage may chain in one season, from
    /// the port of departure to the destination (1 = a single crossing).
    #[serde(default = "default_max_voyage_legs")]
    pub max_voyage_legs: u32,
}

fn default_max_voyage_legs() -> u32 {
    1
}

crate::bundled_rules!(FreeMovementRules, "movement/rules.json", default);
