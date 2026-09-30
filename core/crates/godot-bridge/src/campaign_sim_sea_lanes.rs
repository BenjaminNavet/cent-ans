//! `CampaignSim` sea lane API (lot SL1, ADR 0139): the lanes of
//! `data/naval/sea_lanes.json` resolved against the current turn, for the
//! map layer (`SeaLaneLayer`) and its tooltip. Read only; the geometry
//! (`data/map/sea_lanes_px.json`) is read by Godot directly.

use godot::prelude::*;
use sim_campaign::sea_lanes::{sea_lanes, SeaLaneView};

use crate::campaign_sim::CampaignSim;

fn lane_dict(data: &data_model::GameData, lane: &SeaLaneView) -> VarDictionary {
    let settlement_name = |id: &data_model::SettlementId| {
        data.settlements
            .get(id)
            .map_or_else(|| id.to_string(), |s| s.name.display.clone())
    };
    let control_name = lane.control_faction.as_ref().map_or_else(String::new, |f| {
        data.factions
            .get(f)
            .map_or_else(|| f.to_string(), |x| x.short_or_display_name().to_owned())
    });
    let trade: PackedStringArray = lane
        .trade_routes
        .iter()
        .map(|r| GString::from(r.as_str()))
        .collect();
    vdict! {
        "id" => lane.id.as_str(),
        "name" => lane.name.as_str(),
        "from" => lane.from.as_str(),
        "from_name" => settlement_name(&lane.from).as_str(),
        "to" => lane.to.as_str(),
        "to_name" => settlement_name(&lane.to).as_str(),
        "sea" => lane.sea.as_str(),
        "sea_name" => lane.sea_name.as_str(),
        "kind" => match lane.kind {
            data_model::SeaLaneKind::Coastal => "coastal",
            data_model::SeaLaneKind::OpenSea => "open_sea",
        },
        "kind_name" => lane.kind_name.as_str(),
        "length_km" => lane.length_km,
        "control_faction" => lane.control_faction.as_ref().map_or("", |f| f.as_str()),
        "control_name" => control_name.as_str(),
        "control_level" => i64::from(lane.control_level),
        "hostile" => lane.hostile,
        "blockade" => lane.blockade,
        "storm_loss_percent" => lane.storm_loss_percent,
        "trade_season_factor" => lane.trade_season_factor,
        "trade_routes" => &trade,
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Every sea lane seen by the player: `{id, name, from, from_name, to,
    /// to_name, sea, sea_name, kind ("coastal" | "open_sea"), kind_name,
    /// length_km, control_faction, control_name, control_level, hostile,
    /// blockade, storm_loss_percent, trade_season_factor, trade_routes}`.
    #[func]
    fn get_sea_lanes(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        sea_lanes(state, data, &state.player_faction)
            .iter()
            .map(|lane| lane_dict(data, lane).to_variant())
            .collect()
    }

    /// Whether an army standing in port `from` can embark for `to` (a sea
    /// edge of the graph: lane or short passage).
    #[func]
    fn is_sea_link(&self, from: GString, to: GString) -> bool {
        let Some(data) = &self.data else {
            return false;
        };
        let (Ok(from), Ok(to)) = (
            data_model::SettlementId::new(from.to_string()),
            data_model::SettlementId::new(to.to_string()),
        ) else {
            return false;
        };
        sim_campaign::movement::is_sea_crossing(data, &from, &to)
    }
}
