//! `CampaignSim` mercenary companies (lot TW2-T3, ADR 0103): what an army
//! may hire where it stands (« Mercenaires » panel of the army sheet) and
//! the hiring order.

use godot::prelude::*;
use sim_campaign::{ArmyId, Order};

use crate::campaign_sim::{order_result, CampaignSim};

#[godot_api(secondary)]
impl CampaignSim {
    /// Companies `army_id` may hire where it stands: `{region, region_name,
    /// hires_left, blocked, premium_last_turn, options: [{unit_type, name,
    /// band, band_name, cost, upkeep, experience, available, reason,
    /// pool_available, pool_cap, pool_seasons_to_next, pool_label}]}`.
    /// The option rows carry the keys of `get_recruitable` (the panel reuses
    /// the recruitment rows); `upkeep` includes the mercenary premium,
    /// `blocked` is the French reason the army hires nothing here (`""`
    /// otherwise), `premium_last_turn` what the army's faction paid for its
    /// companies above the ordinary upkeep last season. Empty for an
    /// unknown army.
    #[func]
    fn get_mercenaries(&self, army_id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(market) = state.mercenary_market(data, &army) else {
            return VarDictionary::new();
        };
        let faction = &state.armies[&army].faction;
        let premium = state
            .mercenaries
            .premium_last_turn
            .get(faction)
            .copied()
            .unwrap_or(0);
        let options: VarArray = market
            .options
            .iter()
            .map(|option| {
                vdict! {
                    "unit_type" => option.unit_type.as_str(),
                    "name" => option.name.as_str(),
                    "band" => option.band.as_str(),
                    "band_name" => option.band_name.as_str(),
                    "cost" => i64::from(option.cost),
                    "upkeep" => i64::from(option.upkeep),
                    "experience" => i64::from(option.experience),
                    "available" => option.available,
                    "reason" => option.reason.as_deref().unwrap_or(""),
                    "pool_available" => i64::from(option.pool.available),
                    "pool_cap" => i64::from(option.pool.cap),
                    "pool_seasons_to_next" => option.pool.seasons_to_next.map_or(-1, i64::from),
                    "pool_label" => option.pool.label_fr().as_str(),
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "region" => market.region.as_deref().unwrap_or(""),
            "region_name" => market.region_name.as_str(),
            "hires_left" => i64::from(market.hires_left),
            "blocked" => market.blocked.as_deref().unwrap_or(""),
            "premium_last_turn" => premium,
            "options" => &options,
        }
    }

    /// `army_id` hires a company of `unit_type` (same as the
    /// `hire_mercenary` order): `{ok, error}`.
    #[func]
    fn hire_mercenary(&mut self, army_id: GString, unit_type: GString) -> VarDictionary {
        if self.refuse_while_turn_pending("hire_mercenary") {
            return order_result(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return order_result(Err(format!("armée inconnue : {army_id}")));
        };
        let Ok(unit) = data_model::UnitTypeId::new(unit_type.to_string()) else {
            return order_result(Err(format!("type d'unité inconnu : {unit_type}")));
        };
        let order = Order::HireMercenary { army, unit };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }
}
