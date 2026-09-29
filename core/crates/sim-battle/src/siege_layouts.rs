//! NT1 (ADR 0126): kinds of besieged places other than the landmark cities.
//!
//! - [`PlaceKind::City`]: the ring town of [`SiegeWorks::generate`]
//!   (octagon, radial streets, rings of blocks), unchanged.
//! - [`PlaceKind::Borough`]: a fortified borough — a polygonal enceinte, a
//!   main street running from the attacked gate through the market square to
//!   a gatehouse on the far side, back lanes parallel to it, and houses in
//!   rows along the streets.
//! - [`PlaceKind::Castle`]: a tight polygonal enceinte, a square keep towards
//!   the back, the bailey (courtyard) at the centre and a few buildings
//!   against the curtain wall.
//!
//! The kind comes from a data rule (`places` of `data/rules/siege_town.json`:
//! settlement kind and fortification level, [`PlaceRules::place_kind`]).
//! Variations (number of sides, radius, rotation, gate and keep position,
//! street bend) are drawn from a hash of the province ([`place_seed`]), never
//! from the battle's random stream: a province always gets the same plan.
//! The side of the attacked gate always faces the attacker (−z).

use std::collections::BTreeMap;

use data_model::SettlementKind;
use serde::{Deserialize, Serialize};

use crate::rng::BattleRng;
use crate::siege::SiegeWorks;

/// Kind of besieged place (NT1).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// Ring town (the generic plan of BR3).
    #[default]
    City,
    /// Fortified borough: main street, market square, rows of houses.
    Borough,
    /// Castle: tight enceinte, keep, bailey, few buildings.
    Castle,
}

impl PlaceKind {
    /// Every kind.
    pub const ALL: [PlaceKind; 3] = [PlaceKind::City, PlaceKind::Borough, PlaceKind::Castle];

    /// The `snake_case` key used in JSON files and by the bridge.
    pub fn key(self) -> &'static str {
        match self {
            PlaceKind::City => "city",
            PlaceKind::Borough => "borough",
            PlaceKind::Castle => "castle",
        }
    }

    /// The default kind (serde `skip_serializing_if`).
    pub fn is_city(&self) -> bool {
        *self == PlaceKind::City
    }
}

/// Plan of the castle (`places.castle`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastleRules {
    pub sides: [u32; 2],
    pub radius_m: [f64; 2],
    pub radius_jitter: f64,
    pub rotation_deg: f64,
    pub gate_offset: f64,
    pub wall_height_bonus_m: f64,
    pub courtyard_m: f64,
    pub keep_side_m: [f64; 2],
    pub keep_distance: [f64; 2],
    pub keep_arc_deg: f64,
    pub buildings: [u32; 2],
    pub wall_walk_m: f64,
    pub lane_m: f64,
}

/// Plan of the fortified borough (`places.borough`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoroughRules {
    pub sides: [u32; 2],
    pub radius_m: [f64; 2],
    pub radius_jitter: f64,
    pub rotation_deg: f64,
    pub gate_offset: f64,
    pub market_m: f64,
    pub main_street_m: f64,
    pub street_bend_m: f64,
    pub back_lane_m: f64,
    pub back_lane_offset_m: [f64; 2],
    pub cross_lane_chance: f64,
}

/// `places` of `data/rules/siege_town.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceRules {
    pub by_settlement_kind: BTreeMap<SettlementKind, PlaceKind>,
    pub borough_to_city_from_fortification: u32,
    pub city_to_borough_up_to_fortification: u32,
    pub castle: CastleRules,
    pub borough: BoroughRules,
}

impl PlaceRules {
    /// The kind of place for a settlement of `kind` at `fortification`.
    pub fn place_kind(&self, kind: SettlementKind, fortification: u32) -> PlaceKind {
        let _ = (kind, fortification);
        PlaceKind::City
    }
}

/// Seed of the plan of a province (FNV-1a of its id).
pub fn place_seed(province: &str) -> u64 {
    let _ = province;
    0
}

impl SiegeWorks {
    /// The works of a place of `kind`: the generic ring town for a city, the
    /// borough or castle plan otherwise (variations from `seed`).
    pub fn generate_place(
        kind: PlaceKind,
        fortification: u32,
        breach: u8,
        seed: u64,
        rng: &mut BattleRng,
    ) -> Self {
        let _ = (kind, seed);
        SiegeWorks::generate(fortification, breach, rng)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "NT1 skeleton"]
    fn every_kind_is_valid() {}

    #[test]
    #[ignore = "NT1 skeleton"]
    fn deterministic_per_province() {}

    #[test]
    #[ignore = "NT1 skeleton"]
    fn provinces_differ() {}
}
