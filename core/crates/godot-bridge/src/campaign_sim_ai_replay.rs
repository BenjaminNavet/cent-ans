//! `CampaignSim` record of the AI turn (lot CT1, ADR 0070): the AI armies'
//! moves of the last `end_turn`, replayed by `game/scripts/map/ai_turn_replay.gd`.

use godot::prelude::*;
use sim_campaign::AiMoveRecord;

use crate::campaign_sim::CampaignSim;

fn text(value: Option<&str>) -> &str {
    value.unwrap_or("")
}

fn move_dict(record: &AiMoveRecord) -> VarDictionary {
    let path: PackedVector2Array = record
        .path
        .iter()
        .map(|p| Vector2::new(p[0], p[1]))
        .collect();
    vdict! {
        "sequence" => i64::from(record.sequence),
        "army" => record.army.as_str(),
        "faction" => record.faction.as_str(),
        "path" => &path,
        "kind" => record.kind.as_str(),
        "settlement" => text(record.settlement.as_ref().map(|s| s.as_str())),
        "target_army" => text(record.target_army.as_ref().map(|a| a.as_str())),
        "target_faction" => text(record.target_faction.as_ref().map(|f| f.as_str())),
        "visible" => record.visible,
        "visible_from" => record.visible_from as i64,
        "visible_to" => record.visible_to as i64,
        "notable" => text(record.notable.map(|n| n.as_str())),
        "priority" => i64::from(record.notable.map_or(0, |n| n.priority())),
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Turns the record of the AI turn on or off (lot CT1). Off (the
    /// default, « Masquer »), `end_turn` records nothing and costs nothing
    /// more. `notable_radius_km`: a move ending that close to a player army
    /// or settlement concerns the player (`data/ui/ai_turn_replay.json`).
    #[func]
    fn set_ai_turn_recording(&mut self, enabled: bool, notable_radius_km: f64) {
        if let Some(state) = &mut self.state {
            state.set_ai_replay_recording(enabled, notable_radius_km);
        }
    }

    /// The AI army moves of the last `end_turn`, in play order:
    /// `[{sequence, army, faction, path: PackedVector2Array (map pixels,
    /// start included), kind: march|stationed|siege_started|settlement_taken|
    /// battle|landing, settlement, target_army, target_faction, visible,
    /// visible_from, visible_to, notable: ""|near_player|player_territory|
    /// siege|battle, priority: 0-4}]`. Empty when the record is off.
    #[func]
    fn get_ai_turn_moves(&self) -> VarArray {
        let Some(state) = &self.state else {
            return VarArray::new();
        };
        state
            .ai_turn_moves()
            .iter()
            .map(|record| move_dict(record).to_variant())
            .collect()
    }
}
