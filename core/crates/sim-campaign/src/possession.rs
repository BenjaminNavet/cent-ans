//! Lot RJ-c (ADR 0175): possession versus occupation, as a viewer sees it.
//!
//! The rule itself lives elsewhere and is unchanged (`settlements.rs`): the
//! city of a province gives its control; possession (`owner`) changes only by
//! treaty (cession, `diplomacy.rs`); at the peace between owner and occupier,
//! an occupied place that was not ceded goes back to its owner. This module
//! only classifies a place or a province for one viewer so the UI can explain
//! it without recomputing anything.

use data_model::{FactionId, ProvinceId, SettlementId};

use crate::state::CampaignState;

/// Status of a place (or of a province, through its city) for a viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PossessionStatus {
    /// The viewer owns and holds it.
    Own,
    /// The viewer owns it but another faction occupies it.
    OwnOccupied,
    /// The viewer occupies it; another faction owns it de jure.
    OccupiedByViewer,
    /// Another faction owns and holds it.
    Foreign,
    /// Another faction owns it and a third one occupies it.
    ForeignOccupied,
}

impl PossessionStatus {
    /// Classifies a place owned by `owner` and held by `controller` for
    /// `viewer`.
    pub fn of(viewer: &FactionId, owner: &FactionId, controller: &FactionId) -> Self {
        match (owner == viewer, controller == viewer) {
            (true, true) => Self::Own,
            (true, false) => Self::OwnOccupied,
            (false, true) => Self::OccupiedByViewer,
            (false, false) if owner == controller => Self::Foreign,
            (false, false) => Self::ForeignOccupied,
        }
    }

    /// Snake-case key used by the bridge and the UI.
    pub fn key(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::OwnOccupied => "own_occupied",
            Self::OccupiedByViewer => "occupied_by_viewer",
            Self::Foreign => "foreign",
            Self::ForeignOccupied => "foreign_occupied",
        }
    }

    /// `true` when the owner and the controller differ: the place goes back
    /// to its owner at their peace unless the treaty cedes it.
    pub fn is_occupied(self) -> bool {
        !matches!(self, Self::Own | Self::Foreign)
    }
}

/// Possession summary of a province for a viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvincePossession {
    /// Status of the province's city (which gives the province's control).
    pub status: PossessionStatus,
    pub owner: FactionId,
    pub controller: FactionId,
    pub city: SettlementId,
    /// Settlements of the province.
    pub settlements_total: usize,
    /// Settlements of the province the viewer controls.
    pub held_by_viewer: usize,
    /// Faction controlling every settlement of the province (full-province
    /// bonus), if any.
    pub whole_province_holder: Option<FactionId>,
}

impl CampaignState {
    /// Possession summary of `province` for `viewer`.
    pub fn province_possession(
        &self,
        viewer: &FactionId,
        province: &ProvinceId,
    ) -> Option<ProvincePossession> {
        let province_state = self.provinces.get(province)?;
        let city = self.settlements.get(&province_state.city)?;
        let mut total = 0;
        let mut held = 0;
        for (_, s) in self.settlements_of(province) {
            total += 1;
            if &s.controller == viewer {
                held += 1;
            }
        }
        let whole_province_holder = self
            .holds_whole_province(&city.controller, province)
            .then(|| city.controller.clone());
        Some(ProvincePossession {
            status: PossessionStatus::of(viewer, &city.owner, &city.controller),
            owner: city.owner.clone(),
            controller: city.controller.clone(),
            city: province_state.city.clone(),
            settlements_total: total,
            held_by_viewer: held,
            whole_province_holder,
        })
    }

    /// Status of `settlement` for `viewer`.
    pub fn settlement_possession(
        &self,
        viewer: &FactionId,
        settlement: &SettlementId,
    ) -> Option<PossessionStatus> {
        self.settlements
            .get(settlement)
            .map(|s| PossessionStatus::of(viewer, &s.owner, &s.controller))
    }
}
