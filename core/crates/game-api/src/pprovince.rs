//! Construction views of the province panel: the building tree of a settlement (with
//! prerequisites, costs and lock reasons), cancel quotes and demolition previews.
//!
//! Replaces `CampaignSim.get_building_tree`, `settlement_cancel_quote` and
//! `settlement_demolition_preview` (the `buildable` list of `get_province_city` is superseded by
//! the tree, which carries the same cost / availability / reason per building).

use data_model::{GameData, SettlementId};
use serde::Serialize;
use sim_campaign::CampaignState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementView {
    /// `rank`, `chain`, `building`, `technology`, `resource`, `coast`, `river` or `unique`.
    pub kind: String,
    pub id: String,
    pub text: String,
    pub met: bool,
}

/// One effect of a building as written in the data.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeEffectView {
    /// Effect kind key (`tax_income`, `health`…).
    pub key: String,
    /// Targeted social class, or empty for the whole province.
    pub class: String,
    pub value: f64,
    /// `true` when `value` is a percentage rather than a flat amount.
    pub percent: bool,
}

/// A statistic moved by completing the building: `[before, after]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BeforeAfterView {
    pub key: String,
    pub before: f64,
    pub after: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildNodeView {
    pub building: String,
    pub name: String,
    pub category: String,
    pub tier: u8,
    pub parent: String,
    pub root: String,
    pub depth: u8,
    /// `built`, `building`, `queued`, `available` or `locked`.
    pub state: String,
    pub current: bool,
    pub reason: String,
    /// Refusal that is not a lock (treasury, queue, slots).
    pub blocked_now: String,
    pub can_build: bool,
    pub cost: u32,
    pub turns: u32,
    pub upkeep: u32,
    pub takes_slot: bool,
    pub turns_left: u32,
    pub total_turns: u32,
    pub queue_index: i64,
    pub allowed_here: bool,
    pub requirements: Vec<RequirementView>,
    pub effects: Vec<NodeEffectView>,
    /// Filled for the nodes that can still be built (`available`), empty otherwise.
    pub before_after: Vec<BeforeAfterView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueRowView {
    pub building: String,
    pub name: String,
    pub turns_left: u32,
    pub total_turns: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildingTreeView {
    pub settlement: String,
    pub kind: String,
    pub kind_label: String,
    pub slots_used: usize,
    /// `None`: no cap.
    pub slots_max: Option<usize>,
    pub queue_size: usize,
    pub queue: Vec<QueueRowView>,
    pub nodes: Vec<BuildNodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CancelQuoteView {
    pub paid: u32,
    pub refund: u32,
    pub elapsed: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DemolitionView {
    pub building: String,
    pub name: String,
    pub can_demolish: bool,
    pub reason: String,
    pub refund: i64,
    pub upkeep_saved: i64,
}

/// Construction tree of `settlement`; `None` for an unknown id.
pub fn building_tree(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
) -> Option<BuildingTreeView> {
    let tree = state.building_tree(data, settlement)?;
    Some(BuildingTreeView {
        settlement: tree.settlement.as_str().to_owned(),
        kind: tree.kind.key().to_owned(),
        kind_label: tree.kind_label.to_owned(),
        slots_used: tree.slots_used,
        slots_max: tree.slots_max,
        queue_size: tree.queue_size,
        queue: tree
            .queue
            .iter()
            .map(|row| QueueRowView {
                building: row.building.as_str().to_owned(),
                name: row.name.clone(),
                turns_left: row.turns_left,
                total_turns: row.total_turns,
            })
            .collect(),
        nodes: tree
            .nodes
            .iter()
            .map(|n| {
                let definition = data.buildings.get(&n.building);
                BuildNodeView {
                    building: n.building.as_str().to_owned(),
                    name: n.name.clone(),
                    category: n.category.to_owned(),
                    tier: n.tier,
                    parent: n
                        .parent
                        .as_ref()
                        .map_or_else(String::new, |p| p.as_str().to_owned()),
                    root: n.root.as_str().to_owned(),
                    depth: n.depth,
                    state: n.state.key().to_owned(),
                    current: n.current,
                    reason: n.reason.clone(),
                    blocked_now: n.blocked_now.clone(),
                    can_build: n.can_build,
                    cost: n.cost,
                    turns: n.turns,
                    upkeep: n.upkeep,
                    takes_slot: n.takes_slot,
                    turns_left: n.turns_left,
                    total_turns: n.total_turns,
                    queue_index: n.queue_index,
                    allowed_here: n.allowed_here,
                    requirements: n
                        .requirements
                        .iter()
                        .map(|r| RequirementView {
                            kind: r.kind.to_owned(),
                            id: r.id.clone(),
                            text: r.text.clone(),
                            met: r.met,
                        })
                        .collect(),
                    effects: definition.map_or_else(Vec::new, |b| {
                        b.effects
                            .iter()
                            .map(|e| NodeEffectView {
                                key: sim_campaign::preview::effect_key(e)
                                    .split(':')
                                    .next()
                                    .unwrap_or_default()
                                    .to_owned(),
                                class: e.class.map_or_else(String::new, |c| c.key().to_owned()),
                                value: e.value,
                                percent: e.mode == data_model::EffectMode::Percent,
                            })
                            .collect()
                    }),
                    before_after: if n.state.key() == "available" && n.allowed_here {
                        state
                            .building_before_after(data, settlement, &n.building)
                            .into_iter()
                            .map(|(key, [before, after])| BeforeAfterView { key, before, after })
                            .collect()
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect(),
    })
}

/// What cancelling a construction refunds now; `queue_index` -1 is the work in progress, `n >= 0`
/// the queued work `n`. `None` if there is none.
pub fn cancel_quote(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
    queue_index: i64,
) -> Option<CancelQuoteView> {
    let quote = if queue_index < 0 {
        state.cancel_build_quote(data, settlement)
    } else {
        state.cancel_queued_build_quote(data, settlement, queue_index as usize)
    }?;
    Some(CancelQuoteView {
        paid: quote.paid,
        refund: quote.refund,
        elapsed: quote.elapsed,
        total: quote.total,
    })
}

/// Demolition preview of every built building of `settlement`.
pub fn demolition_previews(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
) -> Vec<DemolitionView> {
    let Some(live) = state.settlement_state(settlement) else {
        return Vec::new();
    };
    live.buildings
        .iter()
        .map(|id| {
            let preview = sim_campaign::buildings::demolition_preview(state, data, settlement, id);
            DemolitionView {
                building: id.as_str().to_owned(),
                name: data
                    .buildings
                    .get(id)
                    .map_or_else(|| id.as_str().to_owned(), |b| b.name.display.clone()),
                can_demolish: preview.can_demolish,
                reason: preview.reason.unwrap_or_default(),
                refund: preview.refund,
                upkeep_saved: preview.upkeep_saved,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::FactionId;

    #[test]
    fn capital_building_tree() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let france = FactionId::new("fac_france").unwrap();
        let capital = state.factions[&france].capital.clone();
        let city = state.province_state(&capital).unwrap().city.clone();
        let tree = building_tree(data, &state, &city).unwrap();
        assert!(!tree.nodes.is_empty());
        assert!(tree
            .nodes
            .iter()
            .any(|n| n.state == "available" || n.state == "built"));
        assert!(serde_json::to_string(&tree).is_ok());
        assert!(cancel_quote(data, &state, &city, -1).is_none());
        let _ = demolition_previews(data, &state, &city);
        let unknown = SettlementId::new("set_nowhere").unwrap();
        assert!(building_tree(data, &state, &unknown).is_none());
    }
}
