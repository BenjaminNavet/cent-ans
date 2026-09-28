//! IB5 (ADR 0109, spec IB § 2.2): "before → after" values of the statistics an
//! effect moves, and the state of each requirement, for the rich tooltips.
//!
//! Every value is read on the real state and on a copy where the thing is
//! applied the way the turn applies it (a building completed, a technology
//! acquired), with the same functions the turn uses: nothing is re-derived.
//! An effect whose statistic has no exact value in its context (battle
//! modifiers, garrison levies, piety...) gets no entry.

use std::collections::BTreeMap;

use data_model::{BuildingId, Effect, FactionId, GameData, SettlementId, TechnologyId, UnitTypeId};

use crate::state::CampaignState;

/// `{effect key: [before, after]}` (see [`effect_key`]).
pub type BeforeAfter = BTreeMap<String, [f64; 2]>;

/// State of one requirement of a building, a unit or a technology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The entity required (building, technology, resource id), or a tag:
    /// `coastal`, `river`, `enabling_building`.
    pub id: String,
    pub met: bool,
}

/// Key of an effect in [`BeforeAfter`]: its kind (`"health"`), followed by
/// `":<class>"` or `":<unit category>"` when it targets one.
pub fn effect_key(effect: &Effect) -> String {
    let _ = effect;
    String::new()
}

impl CampaignState {
    /// Province statistics moved by completing `building` in `settlement`.
    pub fn building_before_after(
        &self,
        _data: &GameData,
        _settlement: &SettlementId,
        _building: &BuildingId,
    ) -> BeforeAfter {
        BeforeAfter::new()
    }

    /// Faction statistics moved by acquiring `technology`.
    pub fn technology_before_after(
        &self,
        _data: &GameData,
        _faction: &FactionId,
        _technology: &TechnologyId,
    ) -> BeforeAfter {
        BeforeAfter::new()
    }

    /// Requirements of `building` in `settlement`, in data order.
    pub fn building_requirements(
        &self,
        _data: &GameData,
        _settlement: &SettlementId,
        _building: &BuildingId,
    ) -> Vec<Requirement> {
        Vec::new()
    }

    /// Requirements of recruiting `unit_type` in `settlement`.
    pub fn recruit_requirements(
        &self,
        _data: &GameData,
        _settlement: &SettlementId,
        _unit_type: &UnitTypeId,
    ) -> Vec<Requirement> {
        Vec::new()
    }

    /// Prerequisites of `technology` for `faction`.
    pub fn technology_requirements(
        &self,
        _data: &GameData,
        _faction: &FactionId,
        _technology: &TechnologyId,
    ) -> Vec<Requirement> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {}
