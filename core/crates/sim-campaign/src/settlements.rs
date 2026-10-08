//! Lot C4: settlement queries and changes of hands
//! (`docs/design/2026-09-24-echelle-colonies.md` § 2 and § 4.3).
//!
//! The province keeps the land and the people; its owner and controller are
//! those of its city. Everything that used to read `ProvinceState::owner`,
//! `controller`, `garrison`, `siege` or `buildings` goes through the helpers
//! below.

use data_model::{BuildingId, FactionId, GameData, ProvinceId, SettlementId, SettlementKind};

use crate::state::{CampaignState, SettlementState};

/// Normalised share (0-1) of its province that `settlement` carries
/// (`Settlement::weight` divided by the sum of the province's weights,
/// precomputed at load); 0 for an unknown settlement.
pub fn weight_share(data: &GameData, settlement: &SettlementId) -> f64 {
    data.settlement_weight_share
        .get(settlement)
        .copied()
        .unwrap_or(0.0)
}

impl CampaignState {
    /// Id of the city of `province`.
    pub fn province_city_id(&self, province: &ProvinceId) -> Option<&SettlementId> {
        self.provinces.get(province).map(|p| &p.city)
    }

    /// State of the city of `province`.
    pub fn city_state(&self, province: &ProvinceId) -> Option<&SettlementState> {
        self.province_city_id(province)
            .and_then(|id| self.settlements.get(id))
    }

    /// Mutable state of the city of `province` (debug helpers, tests).
    pub fn city_state_mut(&mut self, province: &ProvinceId) -> Option<&mut SettlementState> {
        let id = self.provinces.get(province)?.city.clone();
        self.settlements.get_mut(&id)
    }

    /// De jure owner of `province`: the owner of its city.
    pub fn province_owner(&self, province: &ProvinceId) -> Option<&FactionId> {
        self.city_state(province).map(|s| &s.owner)
    }

    /// Faction controlling `province`: the controller of its city.
    pub fn province_controller(&self, province: &ProvinceId) -> Option<&FactionId> {
        self.city_state(province).map(|s| &s.controller)
    }

    /// `true` when `faction` controls the city of `province`.
    pub fn controls_province(&self, faction: &FactionId, province: &ProvinceId) -> bool {
        self.province_controller(province) == Some(faction)
    }

    /// `true` when `faction` owns and controls the city of `province`.
    pub fn holds_province(&self, faction: &FactionId, province: &ProvinceId) -> bool {
        self.city_state(province)
            .is_some_and(|s| &s.owner == faction && &s.controller == faction)
    }

    /// Lot LR-15: the place that stands for the faction's capital when it
    /// musters, recruits or receives a reward — the city of its capital
    /// province when it controls it, else the city of its first controlled
    /// province (id order), else its first controlled place other than a
    /// village, else any place it controls; `None` when it controls
    /// nothing. Never another realm's city: the capital province of a
    /// cityless faction (ADR 0165) belongs to someone else.
    pub fn faction_seat(&self, faction: &FactionId) -> Option<SettlementId> {
        let capital = &self.factions.get(faction)?.capital;
        if self.controls_province(faction, capital) {
            return self.province_city_id(capital).cloned();
        }
        let controlled = |s: &SettlementState| &s.controller == faction;
        self.provinces
            .values()
            .find(|p| self.settlements.get(&p.city).is_some_and(controlled))
            .map(|p| p.city.clone())
            .or_else(|| {
                self.settlements
                    .iter()
                    .find(|(_, s)| controlled(s) && s.kind != SettlementKind::Village)
                    .or_else(|| self.settlements.iter().find(|(_, s)| controlled(s)))
                    .map(|(id, _)| id.clone())
            })
    }

    /// Province of `settlement`.
    pub fn settlement_province(&self, settlement: &SettlementId) -> Option<&ProvinceId> {
        self.settlements.get(settlement).map(|s| &s.province)
    }

    /// Settlements of `province` with their state, the city first.
    pub fn settlements_of<'a>(
        &'a self,
        province: &ProvinceId,
    ) -> impl Iterator<Item = (&'a SettlementId, &'a SettlementState)> + 'a {
        self.provinces
            .get(province)
            .into_iter()
            .flat_map(|p| p.settlements.iter())
            .filter_map(move |id| self.settlements.get_key_value(id))
    }

    /// Ids of the provinces whose city `faction` controls, in id order.
    pub fn controlled_provinces<'a>(
        &'a self,
        faction: &'a FactionId,
    ) -> impl Iterator<Item = &'a ProvinceId> + 'a {
        self.provinces
            .keys()
            .filter(move |id| self.controls_province(faction, id))
    }

    /// Ids of the provinces whose city `faction` owns, in id order.
    pub fn owned_provinces(&self, faction: &FactionId) -> Vec<ProvinceId> {
        self.provinces
            .keys()
            .filter(|id| self.province_owner(id) == Some(faction))
            .cloned()
            .collect()
    }

    /// `true` when `faction` controls every settlement of `province`
    /// (full-province bonus, spec § 4.3).
    pub fn holds_whole_province(&self, faction: &FactionId, province: &ProvinceId) -> bool {
        let mut any = false;
        for (_, s) in self.settlements_of(province) {
            if &s.controller != faction {
                return false;
            }
            any = true;
        }
        any
    }

    /// Completed buildings of every settlement of `province` (province-wide
    /// effects: population, religion, table, research...).
    pub fn province_buildings(&self, province: &ProvinceId) -> Vec<BuildingId> {
        self.settlements_of(province)
            .flat_map(|(_, s)| s.buildings.iter().cloned())
            .collect()
    }

    /// Men in the garrisons of the settlements held by the province's
    /// controller.
    pub fn province_garrison_strength(&self, province: &ProvinceId) -> u32 {
        let Some(controller) = self.province_controller(province) else {
            return 0;
        };
        self.settlements_of(province)
            .filter(|(_, s)| &s.controller == controller)
            .map(|(_, s)| s.garrison_strength())
            .sum()
    }

    /// Men of [`CampaignState::province_garrison_strength`], each place weighing
    /// its kind's `province_effect_percent` (lot RS-B, ADR 0100: the garrison
    /// that keeps a province in order is the city's; the dense map's castles and
    /// towns count like their buildings, for half).
    pub fn weighted_garrison_strength(&self, data: &GameData, province: &ProvinceId) -> u32 {
        let Some(controller) = self.province_controller(province) else {
            return 0;
        };
        let weighted: u64 = self
            .settlements_of(province)
            .filter(|(_, s)| &s.controller == controller)
            .map(|(_, s)| {
                u64::from(s.garrison_strength())
                    * u64::from(crate::buildings::province_effect_percent(data, s.kind))
            })
            .sum();
        u32::try_from(weighted / 100).unwrap_or(u32::MAX)
    }

    /// `true` while the city of `province` is besieged.
    pub fn province_besieged(&self, province: &ProvinceId) -> bool {
        self.city_state(province).is_some_and(|s| s.siege.is_some())
    }

    /// Kind of `settlement` (a village when unknown).
    pub fn settlement_kind(&self, settlement: &SettlementId) -> SettlementKind {
        self.settlements
            .get(settlement)
            .map_or(SettlementKind::Village, |s| s.kind)
    }

    /// Hands every settlement of `province` held (owned) by `from` — every
    /// settlement when `from` is `None` — to `to`, as owner and controller
    /// (peace treaty, ransom, revolt...). Sieges end; garrisons, recruits and
    /// constructions of the ceded places are lost. Returns the ceded ids.
    pub(crate) fn cede_province(
        &mut self,
        province: &ProvinceId,
        from: Option<&FactionId>,
        to: &FactionId,
    ) -> Vec<SettlementId> {
        let ids: Vec<SettlementId> = self
            .settlements_of(province)
            .filter(|(_, s)| from.is_none_or(|f| &s.owner == f))
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            let s = self.settlements.get_mut(id).expect("listed above");
            s.owner = to.clone();
            s.hand_over(to);
            s.garrison.clear();
        }
        ids
    }

    /// Settlements of `province` owned by `owner` and controlled by `holder`.
    pub fn occupied_settlements(
        &self,
        province: &ProvinceId,
        owner: &FactionId,
        holder: &FactionId,
    ) -> Vec<SettlementId> {
        self.settlements_of(province)
            .filter(|(_, s)| &s.owner == owner && &s.controller == holder)
            .map(|(id, _)| id.clone())
            .collect()
    }
}
