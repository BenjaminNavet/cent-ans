//! Frontier provinces (P1): the single classification shared by the 1337
//! start (garrison sizes, [`crate::setup_1337`]) and the strategic AI
//! (garrison kept when the excess becomes a field army, recruitment sites,
//! value of fortifications).
//!
//! A province is a *frontier* of `faction` when it has a port (raids and
//! landings from the sea) or when one of its land neighbours (geometry graph
//! first, entity data otherwise, see [`crate::movement::land_neighbors`]) is
//! controlled by another faction, allied or not. Before P1 the setup read the
//! entity `neighbors` (often empty) and counted ports, while the AI only
//! counted neighbours at war: a port such as `prov_normandie_ouest` started
//! with a frontier garrison of 3 that the AI judged too large and split into
//! a field army without a general on the first turn.

use data_model::{FactionId, GameData, ProvinceId};

use crate::movement::land_neighbors;
use crate::state::CampaignState;

/// Role of a province in its holder's defence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarrisonRole {
    Capital,
    Frontier,
    Interior,
}

impl GarrisonRole {
    /// Garrison units of the 1337 start. The AI keeps at least one unit
    /// less before turning the excess into a field army.
    pub fn garrison_size(self) -> usize {
        match self {
            GarrisonRole::Capital => 4,
            GarrisonRole::Frontier => 3,
            GarrisonRole::Interior => 2,
        }
    }
}

impl CampaignState {
    /// Whether `province` is a frontier of `faction` (see the module doc).
    pub fn is_frontier(&self, data: &GameData, faction: &FactionId, province: &ProvinceId) -> bool {
        data.provinces.get(province).is_some_and(|p| p.has_port())
            || land_neighbors(data, province).iter().any(|neighbour| {
                // Lot C4: the controller of a province is its city's.
                self.province_controller(neighbour)
                    .is_some_and(|controller| controller != faction)
            })
    }

    /// Role of `province` for `faction`: its capital, a frontier or the
    /// interior.
    pub fn garrison_role(
        &self,
        data: &GameData,
        faction: &FactionId,
        province: &ProvinceId,
    ) -> GarrisonRole {
        if self
            .factions
            .get(faction)
            .is_some_and(|f| &f.capital == province)
        {
            GarrisonRole::Capital
        } else if self.is_frontier(data, faction, province) {
            GarrisonRole::Frontier
        } else {
            GarrisonRole::Interior
        }
    }
}
