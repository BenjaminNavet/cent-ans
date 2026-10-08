//! Lot TW2-T2 (ADR 0102): recruitment pools, Medieval II style.
//!
//! Every settlement holds, for every unit type, a reserve of recruitable
//! units capped by its kind and buildings (`data/rules/replenishment.json`,
//! `recruit_pool`). Each recruit draws one unit from it and the reserve
//! refills every season by a rate that grows with the settlement's kind,
//! fortification level and military buildings, and shrinks for knights and
//! siege engines. The player and the AI are bound alike: the check sits in
//! the recruitment blocker of `orders`.
//!
//! Storage: [`crate::state::SettlementState::recruit_pool`] keeps only the
//! reserves below their cap, in thousandths of a unit; a unit type absent
//! from the map is full. Old saves (no map) and the 1337 start therefore
//! begin with every reserve full.

use data_model::EffectKind;
use data_model::{GameData, SettlementId, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::state::CampaignState;

/// Thousandths per unit.
pub const MILLI: u32 = 1000;

/// State of one reserve, as the recruitment panel shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PoolView {
    /// Whole units recruitable now.
    pub available: u32,
    /// Units held at most.
    pub cap: u32,
    /// Current reserve in thousandths of a unit.
    pub milli: u32,
    /// Thousandths gained each season.
    pub rate_milli: u32,
    /// Seasons before the next whole unit; `None` when full or never.
    pub seasons_to_next: Option<u32>,
}

impl PoolView {
    /// French line of the recruitment panel: « 2 disponibles, +1 dans 2 saisons ».
    pub fn label_fr(&self) -> String {
        let available = match self.available {
            0 => "aucune disponible".to_owned(),
            1 => "1 disponible".to_owned(),
            n => format!("{n} disponibles"),
        };
        match self.seasons_to_next {
            None if self.available >= self.cap => format!("{available} (réserve pleine)"),
            None => format!("{available}, réserve tarie"),
            Some(1) => format!("{available}, +1 dans 1 saison"),
            Some(k) => format!("{available}, +1 dans {k} saisons"),
        }
    }
}

impl CampaignState {
    /// `true` when `settlement` is the city of its controller's capital.
    fn is_capital_city(&self, settlement: &SettlementId) -> bool {
        let Some(state) = self.settlements.get(settlement) else {
            return false;
        };
        self.factions.get(&state.controller).is_some_and(|f| {
            f.capital == state.province && self.province_city_id(&f.capital) == Some(settlement)
        })
    }

    /// `recruit_slots` points of the settlement's own buildings, governor
    /// and edict (muster field, stables, armoury...).
    fn pool_slot_points(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        self.settlement_effects(data, settlement)[EffectKind::RecruitSlots]
            .flat
            .max(0.0) as u32
    }

    /// Units of `unit_type` `settlement` holds at most.
    pub fn recruit_pool_cap(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        _unit_type: &UnitTypeId,
    ) -> u32 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0;
        };
        let rules = &data.replenishment_rules.recruit_pool;
        let mut cap = rules.base_cap.get(state.kind);
        if self.is_capital_city(settlement) {
            cap += rules.capital_cap_bonus;
        }
        cap + rules.recruit_slot_cap_bonus * self.pool_slot_points(data, settlement)
    }

    /// Thousandths of a unit of `unit_type` `settlement` gains each season.
    pub fn recruit_pool_rate_milli(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) -> u32 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0;
        };
        let rules = &data.replenishment_rules.recruit_pool;
        let mut rate = rules.base_rate_milli.get(state.kind)
            + rules.fortification_rate_milli_per_level * u32::from(state.fortification_level)
            + rules.recruit_slot_rate_milli * self.pool_slot_points(data, settlement);
        if self.is_capital_city(settlement) {
            rate += rules.capital_rate_milli;
        }
        let category = data
            .unit_types
            .get(unit_type)
            .map_or(100, |t| rules.category_rate_percent.get(t.category));
        rate * category / 100
    }

    /// The reserve of `unit_type` in `settlement`.
    pub fn recruit_pool(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) -> PoolView {
        let cap = self.recruit_pool_cap(data, settlement, unit_type);
        let full = cap * MILLI;
        let milli = self
            .settlements
            .get(settlement)
            .and_then(|s| s.recruit_pool.get(unit_type).copied())
            .map_or(full, |m| m.min(full));
        let rate_milli = self.recruit_pool_rate_milli(data, settlement, unit_type);
        let seasons_to_next = if milli >= full || rate_milli == 0 {
            None
        } else {
            let next = (milli / MILLI + 1) * MILLI;
            Some((next - milli).div_ceil(rate_milli))
        };
        PoolView {
            available: milli / MILLI,
            cap,
            milli,
            rate_milli,
            seasons_to_next,
        }
    }

    /// Draws one unit of `unit_type` from the reserve of `settlement`
    /// (checked by the recruitment blocker beforehand).
    pub(crate) fn draw_recruit_pool(
        &mut self,
        data: &GameData,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) {
        let milli = self.recruit_pool(data, settlement, unit_type).milli;
        if let Some(state) = self.settlements.get_mut(settlement) {
            state
                .recruit_pool
                .insert(unit_type.clone(), milli.saturating_sub(MILLI));
        }
    }
}

/// New season: every reserve below its cap refills by its rate; a full
/// reserve leaves the map. A besieged settlement does not refill.
pub fn resolve_recruit_pools(state: &mut CampaignState, data: &GameData) {
    /// New reserve of each unit type (`None`: full or unknown, removed).
    type PoolChanges = Vec<(UnitTypeId, Option<u32>)>;
    let updates: Vec<(SettlementId, PoolChanges)> = state
        .settlements
        .iter()
        .filter(|(_, s)| !s.recruit_pool.is_empty())
        .map(|(id, s)| {
            let besieged = s.siege.is_some();
            let changes = s
                .recruit_pool
                .keys()
                .map(|unit_type| {
                    if !data.unit_types.contains_key(unit_type) {
                        return (unit_type.clone(), None);
                    }
                    let view = state.recruit_pool(data, id, unit_type);
                    let rate = if besieged { 0 } else { view.rate_milli };
                    let next = view.milli + rate;
                    let full = view.cap * MILLI;
                    (unit_type.clone(), (next < full).then_some(next))
                })
                .collect();
            (id.clone(), changes)
        })
        .collect();
    for (id, changes) in updates {
        let Some(settlement) = state.settlements.get_mut(&id) else {
            continue;
        };
        for (unit_type, milli) in changes {
            match milli {
                Some(milli) => {
                    settlement.recruit_pool.insert(unit_type, milli);
                }
                None => {
                    settlement.recruit_pool.remove(&unit_type);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_like_the_spec() {
        let view = PoolView {
            available: 2,
            cap: 3,
            milli: 2500,
            rate_milli: 250,
            seasons_to_next: Some(2),
        };
        assert_eq!(view.label_fr(), "2 disponibles, +1 dans 2 saisons");
        let full = PoolView {
            available: 3,
            cap: 3,
            milli: 3000,
            rate_milli: 250,
            seasons_to_next: None,
        };
        assert_eq!(full.label_fr(), "3 disponibles (réserve pleine)");
        let empty = PoolView {
            available: 0,
            cap: 1,
            milli: 0,
            rate_milli: 0,
            seasons_to_next: None,
        };
        assert_eq!(empty.label_fr(), "aucune disponible, réserve tarie");
    }
}
