//! `CampaignSim` possession reads (lot RJ-c, ADR 0175): what is the
//! viewer's, what is occupied, and by whom — the statuses the province and
//! settlement panels and the map hover explain. No rule here: the
//! classification comes from `sim_campaign::possession`.

use data_model::{FactionId, ProvinceId, SettlementId};
use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};

impl CampaignSim {
    /// `viewer`, or the player's faction when it is empty or malformed.
    fn possession_viewer(&self, viewer: &GString) -> Option<FactionId> {
        let state = self.state.as_ref()?;
        let text = viewer.to_string();
        if text.is_empty() {
            return Some(state.player_faction().clone());
        }
        FactionId::new(text)
            .ok()
            .or_else(|| Some(state.player_faction().clone()))
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Possession of province `id` for `viewer` ("" = the player):
    /// `{status, owner, controller, city, city_name, occupied, held, total,
    /// whole_holder, settlements: [{id, name, kind, is_city, status, owner,
    /// controller, occupied}]}`. `status` is `own`, `own_occupied`,
    /// `occupied_by_viewer`, `foreign` or `foreign_occupied` (that of the
    /// city, which gives the province's control); `held` = places of the
    /// province the viewer controls, `total` = all its places; `whole_holder`
    /// = faction controlling them all (full-province bonus), "" if none.
    /// `occupied`: owner and controller differ — the place goes back to its
    /// owner at their peace unless ceded. Empty for an unknown id.
    #[func]
    fn province_possession(&self, id: GString, viewer: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(viewer) = self.possession_viewer(&viewer) else {
            return VarDictionary::new();
        };
        let Ok(province) = ProvinceId::new(id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(summary) = state.province_possession(&viewer, &province) else {
            return VarDictionary::new();
        };
        let mut places = VarArray::new();
        for (settlement_id, live) in state.settlements_of(&province) {
            let status = sim_campaign::possession::PossessionStatus::of(
                &viewer,
                &live.owner,
                &live.controller,
            );
            let name = data
                .settlements
                .get(settlement_id)
                .map_or(settlement_id.as_str(), |s| s.name.display.as_str());
            places.push(
                &vdict! {
                    "id" => settlement_id.as_str(),
                    "name" => name,
                    "kind" => live.kind.key(),
                    "is_city" => settlement_id == &summary.city,
                    "status" => status.key(),
                    "owner" => live.owner.as_str(),
                    "controller" => live.controller.as_str(),
                    "occupied" => status.is_occupied(),
                }
                .to_variant(),
            );
        }
        let city_name = data
            .settlements
            .get(&summary.city)
            .map_or(summary.city.as_str(), |s| s.name.display.as_str());
        vdict! {
            "status" => summary.status.key(),
            "owner" => summary.owner.as_str(),
            "controller" => summary.controller.as_str(),
            "city" => summary.city.as_str(),
            "city_name" => city_name,
            "occupied" => summary.status.is_occupied(),
            "held" => summary.held_by_viewer as i64,
            "total" => summary.settlements_total as i64,
            "whole_holder" => summary.whole_province_holder.as_ref().map_or("", |f| f.as_str()),
            "settlements" => &places,
        }
    }

    /// Possession of settlement `id` for `viewer` ("" = the player):
    /// `{status, owner, controller, occupied, is_city, city_name,
    /// province_name}` (`status` as in `province_possession`; `city_name` =
    /// the city of its province). Empty for an unknown id.
    #[func]
    fn settlement_possession(&self, id: GString, viewer: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(viewer) = self.possession_viewer(&viewer) else {
            return VarDictionary::new();
        };
        let Ok(settlement) = SettlementId::new(id.to_string()) else {
            return VarDictionary::new();
        };
        let (Some(live), Some(status)) = (
            state.settlement_state(&settlement),
            state.settlement_possession(&viewer, &settlement),
        ) else {
            return VarDictionary::new();
        };
        let city = state.province_city_id(&live.province);
        let city_name = city
            .and_then(|c| data.settlements.get(c))
            .map_or("", |s| s.name.display.as_str());
        let province_name = data
            .provinces
            .get(&live.province)
            .map_or(live.province.as_str(), |p| p.name.display.as_str());
        vdict! {
            "status" => status.key(),
            "owner" => live.owner.as_str(),
            "controller" => live.controller.as_str(),
            "occupied" => status.is_occupied(),
            "is_city" => city == Some(&settlement),
            "city_name" => city_name,
            "province_name" => province_name,
        }
    }
}
