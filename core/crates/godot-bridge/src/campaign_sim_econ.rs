//! `CampaignSim` API of the readable economy (WH econ), in its own secondary
//! `#[godot_api]` block (ADR 0002): income by source and the tax bracket of a
//! province. The `SetProvinceTax` order goes through
//! `submit_order({"type": "set_province_tax", "province": …, "rate": "low"|"normal"|"high"|null})`.

use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};

/// Non-zero receipt lines of the coming season (`get_income_breakdown`).
pub(crate) fn income_lines(
    state: &sim_campaign::CampaignState,
    data: &data_model::GameData,
    faction: &data_model::FactionId,
    economy: &sim_campaign::economy::FactionEconomy,
) -> Vec<(&'static str, i64)> {
    let breakdown = state.faction_income_breakdown(data, faction);
    let mut lines: Vec<(&'static str, i64)> = breakdown.provinces.named_lines();
    lines.extend([
        ("embargo", breakdown.embargo),
        ("domain", breakdown.domain),
        ("difficulty", breakdown.difficulty),
        ("alms", breakdown.alms),
        ("seigniorage", economy.seigniorage),
        ("trade_routes", economy.trade_income),
    ]);
    lines.retain(|(_, value)| *value != 0);
    lines
}

pub(crate) fn lines_array(lines: &[(&str, i64)]) -> VarArray {
    lines
        .iter()
        .map(|(key, value)| vdict! { "key" => *key, "value" => *value }.to_variant())
        .collect()
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Receipts of the coming season by source: `{lines: [{key, value}],
    /// total}` (non-zero lines only; `total` = taxes + seigniorage + trade
    /// routes = the "Impôts et commerce" budget line). Keys: `poll_*` (per
    /// class), `production`, `tax_rate`, `building_tax`, `building_trade`,
    /// `devastation`, `full_province`, `embargo`, `domain`, `difficulty`,
    /// `alms`, `seigniorage`, `trade_routes`.
    #[func]
    fn get_income_breakdown(&self, faction: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(id) = data_model::FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(economy) = state.faction_economy(data, &id) else {
            return VarDictionary::new();
        };
        let lines = income_lines(state, data, &id, &economy);
        vdict! {
            "lines" => &lines_array(&lines),
            "total" => lines.iter().map(|(_, v)| v).sum::<i64>(),
        }
    }

    /// Tax bracket of `province`: `{rate, own, faction_rate}` (`own`: set for
    /// this province alone); empty for an unknown province.
    #[func]
    fn get_province_tax(&self, province: GString) -> VarDictionary {
        let Some(Ctx { state, .. }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(id) = data_model::ProvinceId::new(province.to_string()) else {
            return VarDictionary::new();
        };
        if state.province_state(&id).is_none() {
            return VarDictionary::new();
        }
        let faction_rate = state
            .province_controller(&id)
            .and_then(|f| state.factions.get(f))
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        vdict! {
            "rate" => state.effective_province_tax(&id).key(),
            "own" => state.has_province_tax(&id),
            "faction_rate" => faction_rate.key(),
        }
    }
}
