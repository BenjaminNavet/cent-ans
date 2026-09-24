//! `CampaignSim` trade API (lot C5): routes for the map layer, tooltip and
//! the faction panel's « Commerce » section. Read only; agreements go
//! through `submit_order({"type": "propose_trade_agreement", ...})` /
//! `break_trade_agreement`, already generic (`Order` derives `Deserialize`).

use godot::prelude::*;
use sim_campaign::trade::{trade_routes, TradeMode, TradeRouteView};

use crate::campaign_sim::CampaignSim;

fn mode_key(mode: TradeMode) -> &'static str {
    match mode {
        TradeMode::Land => "land",
        TradeMode::Sea => "sea",
        TradeMode::Mixed => "mixed",
    }
}

fn route_dict(data: &data_model::GameData, route: &TradeRouteView) -> VarDictionary {
    let goods: PackedStringArray = route
        .goods
        .iter()
        .map(|g| GString::from(g.as_str()))
        .collect();
    let path: PackedStringArray = route
        .path
        .iter()
        .map(|s| GString::from(s.as_str()))
        .collect();
    let hub_name = |hub_id: &str| -> String {
        data.trade
            .as_ref()
            .and_then(|c| c.hub(hub_id))
            .map_or_else(|| hub_id.to_owned(), |h| h.name.display.clone())
    };
    vdict! {
        "id" => route.id.as_str(),
        "from_hub" => route.from_hub.as_str(),
        "from_hub_name" => hub_name(&route.from_hub).as_str(),
        "from_settlement" => route.from_settlement.as_str(),
        "to_hub" => route.to_hub.as_str(),
        "to_hub_name" => hub_name(&route.to_hub).as_str(),
        "to_settlement" => route.to_settlement.as_str(),
        "from_faction" => route.from_faction.as_ref().map_or("", |f| f.as_str()),
        "to_faction" => route.to_faction.as_ref().map_or("", |f| f.as_str()),
        "goods" => &goods,
        "mode" => mode_key(route.mode),
        "path" => &path,
        "agreement" => route.agreement,
        "security" => route.security,
        "cut" => route.cut,
        "cut_reason" => route.cut_reason.as_deref().unwrap_or(""),
        "value_from" => route.value_from,
        "value_to" => route.value_to,
        "total_value" => route.total_value(),
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Every trade route of `data/economy/trade.json`, resolved against the
    /// current turn (map layer, tooltip, « Commerce » panel). Respects no
    /// fog on its own: the caller (map layer) filters by `get_visible_provinces`.
    #[func]
    fn get_trade_routes(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        trade_routes(state, data)
            .iter()
            .map(|route| route_dict(data, route).to_variant())
            .collect()
    }
}
