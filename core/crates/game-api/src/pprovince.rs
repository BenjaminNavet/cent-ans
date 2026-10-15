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

fn effect_view(effect: &data_model::Effect) -> NodeEffectView {
    NodeEffectView {
        key: sim_campaign::preview::effect_key(effect)
            .split(':')
            .next()
            .unwrap_or_default()
            .to_owned(),
        class: effect
            .class
            .map_or_else(String::new, |c| c.key().to_owned()),
        value: effect.value,
        percent: effect.mode == data_model::EffectMode::Percent,
    }
}

/// Effects of a data block as flat rows (`key`, class, value, percent).
pub fn effect_views(effects: &[data_model::Effect]) -> Vec<NodeEffectView> {
    effects.iter().map(effect_view).collect()
}

// ----- La Table (province diets) ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DietOptionView {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Effective cost this season (livres).
    pub cost: i64,
    pub available: bool,
    pub reasons: Vec<String>,
    pub current: bool,
    pub effects: Vec<NodeEffectView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableView {
    pub diet: String,
    pub name: String,
    pub description: String,
    pub cost: i64,
    /// One change per province and per turn.
    pub changed_this_turn: bool,
    /// Spring: meat and dairy diets are penalised.
    pub lent: bool,
    pub options: Vec<DietOptionView>,
}

/// The diet of `province` and every option; `None` for an unknown province.
pub fn table_view(
    data: &GameData,
    state: &CampaignState,
    province: &data_model::ProvinceId,
) -> Option<TableView> {
    let entry = state.province_state(province)?;
    let diet = state.province_diet(province);
    let definition = data.diets.get(&diet);
    let changed_this_turn = entry.diet.as_ref().is_some_and(|choice| {
        choice.turn == state.turn() && state.province_controller(province) == Some(&choice.faction)
    });
    let options = state
        .diet_options(data, province)
        .into_iter()
        .filter_map(|option| {
            let diet = data.diets.get(&option.diet)?;
            Some(DietOptionView {
                id: diet.id.as_str().to_owned(),
                name: option.name,
                description: diet.description.clone(),
                cost: option.cost,
                available: option.available,
                reasons: option.reasons,
                current: option.current,
                effects: effect_views(&diet.effects),
            })
        })
        .collect();
    Some(TableView {
        diet: diet.as_str().to_owned(),
        name: definition.map_or_else(|| diet.as_str().to_owned(), |d| d.name.display.clone()),
        description: definition.map_or_else(String::new, |d| d.description.clone()),
        cost: state.diet_cost(data, province, &diet),
        changed_this_turn,
        lent: state.is_lent(),
        options,
    })
}

// ----- Regional edicts -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdictOptionView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub delay_turns: u32,
    pub cost_money: i64,
    pub cost_prestige: i32,
    /// `(text, met)`.
    pub requirements: Vec<(String, bool)>,
    pub effects: Vec<NodeEffectView>,
    pub available: bool,
    pub reason: String,
    /// Chosen (even if still pending).
    pub current: bool,
    /// In effect right now.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdictView {
    pub edict: String,
    pub name: String,
    pub description: String,
    pub pending: bool,
    pub pending_name: String,
    pub turns_left: u32,
    pub options: Vec<EdictOptionView>,
}

/// The edict of `province` and every option; `None` for an unknown province.
pub fn edict_view(
    data: &GameData,
    state: &CampaignState,
    province: &data_model::ProvinceId,
) -> Option<EdictView> {
    state.province_state(province)?;
    let edict = state.province_edict(data, province);
    let definition = data.edicts.get(&edict);
    let pending = state.pending_edict(data, province);
    let options = state
        .edict_options(data, province)
        .into_iter()
        .filter_map(|option| {
            let edict = data.edicts.get(&option.edict)?;
            Some(EdictOptionView {
                id: edict.id.as_str().to_owned(),
                name: option.name,
                description: edict.description.clone(),
                delay_turns: edict.delay_turns,
                cost_money: option.cost_money,
                cost_prestige: option.cost_prestige,
                requirements: option.requirements,
                effects: effect_views(&edict.effects),
                available: option.available,
                reason: option.reason.unwrap_or_default(),
                current: option.current,
                active: option.active,
            })
        })
        .collect();
    Some(EdictView {
        edict: edict.as_str().to_owned(),
        name: definition.map_or_else(|| edict.as_str().to_owned(), |e| e.name.display.clone()),
        description: definition.map_or_else(String::new, |e| e.description.clone()),
        pending: pending.is_some(),
        pending_name: pending
            .as_ref()
            .and_then(|(id, _)| data.edicts.get(id))
            .map_or_else(String::new, |e| e.name.display.clone()),
        turns_left: pending.as_ref().map_or(0, |(_, turns)| *turns),
        options,
    })
}

// ----- Settlement vocation ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TierRowView {
    pub min_buildings: u32,
    pub label: String,
    pub reached: bool,
    pub effects: Vec<NodeEffectView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VocationOptionView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub current: bool,
    pub cost: u32,
    pub available: bool,
    pub reason: String,
    /// Concerned buildings already standing.
    pub fit: u32,
    pub effect_bonus_percent: f64,
    pub cost_reduction_percent: f64,
    pub off_vocation_surcharge_percent: f64,
    pub tiers: Vec<TierRowView>,
    pub preview_effects: Vec<NodeEffectView>,
    pub lock_turns: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VocationView {
    /// Empty without vocation.
    pub vocation: String,
    pub name: String,
    pub description: String,
    pub count: u32,
    pub reached_tiers: u32,
    pub tiers: Vec<TierRowView>,
    /// Buildings still missing for the next tier (`None`: top tier or no vocation).
    pub missing_for_next: Option<u32>,
    pub effect_bonus_percent: f64,
    pub cost_reduction_percent: f64,
    pub off_vocation_surcharge_percent: f64,
    pub active_effects: Vec<NodeEffectView>,
    pub locked_until: Option<u32>,
    pub options: Vec<VocationOptionView>,
}

fn tier_row(tier: &sim_campaign::vocations::TierView) -> TierRowView {
    TierRowView {
        min_buildings: tier.min_buildings,
        label: tier.label.clone(),
        reached: tier.reached,
        effects: effect_views(&tier.effects),
    }
}

/// Vocation of `settlement` with the change menu; `None` for an unknown id.
pub fn vocation_view(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
) -> Option<VocationView> {
    let view = state.settlement_vocation(data, settlement)?;
    let options = state
        .vocation_options(data, settlement)
        .iter()
        .map(|o| VocationOptionView {
            id: o.id.clone(),
            name: o.name.clone(),
            description: o.description.clone(),
            current: o.current,
            cost: o.cost,
            available: o.available,
            reason: o.reason.clone().unwrap_or_default(),
            fit: o.fit,
            effect_bonus_percent: o.effect_bonus_percent,
            cost_reduction_percent: o.cost_reduction_percent,
            off_vocation_surcharge_percent: o.off_vocation_surcharge_percent,
            tiers: o.tiers.iter().map(tier_row).collect(),
            preview_effects: effect_views(&o.preview_effects),
            lock_turns: o.lock_turns,
        })
        .collect();
    Some(VocationView {
        vocation: view.vocation.clone().unwrap_or_default(),
        name: view.name.clone(),
        description: view.description.clone(),
        count: view.count,
        reached_tiers: view.reached_tiers,
        missing_for_next: view
            .next_tier
            .as_ref()
            .map(|t| t.min_buildings.saturating_sub(view.count).max(1)),
        tiers: view.tiers.iter().map(tier_row).collect(),
        effect_bonus_percent: view.effect_bonus_percent,
        cost_reduction_percent: view.cost_reduction_percent,
        off_vocation_surcharge_percent: view.off_vocation_surcharge_percent,
        active_effects: effect_views(&view.active_effects),
        locked_until: view.locked_until,
        options,
    })
}

// ----- Possession ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PossessionView {
    /// `own`, `own_occupied`, `occupied_by_viewer`, `foreign` or `foreign_occupied`.
    pub status: String,
    pub owner: String,
    pub owner_name: String,
    pub controller: String,
    pub controller_name: String,
    pub city_name: String,
    pub occupied: bool,
    /// Settlements of the province held by the viewer / in total.
    pub held: usize,
    pub total: usize,
    /// Faction holding every settlement (empty if none).
    pub whole_holder: String,
    /// Stance category of the current holder: `self`, `friend`, `enemy` or `other`.
    pub cue: String,
}

/// Possession of `province` seen by `viewer`; `None` for an unknown province.
pub fn possession_view(
    data: &GameData,
    state: &CampaignState,
    viewer: &data_model::FactionId,
    province: &data_model::ProvinceId,
) -> Option<PossessionView> {
    let possession = state.province_possession(viewer, province)?;
    let holder = match possession.status {
        sim_campaign::possession::PossessionStatus::Own
        | sim_campaign::possession::PossessionStatus::OccupiedByViewer => viewer,
        sim_campaign::possession::PossessionStatus::OwnOccupied
        | sim_campaign::possession::PossessionStatus::ForeignOccupied => &possession.controller,
        sim_campaign::possession::PossessionStatus::Foreign => &possession.owner,
    };
    let cue = if holder == viewer {
        "self"
    } else if state.is_at_war(viewer, holder) {
        "enemy"
    } else if state.is_allied(viewer, holder) {
        "friend"
    } else {
        "other"
    };
    Some(PossessionView {
        status: possession.status.key().to_owned(),
        owner: possession.owner.as_str().to_owned(),
        owner_name: data.faction_name(&possession.owner),
        controller: possession.controller.as_str().to_owned(),
        controller_name: data.faction_name(&possession.controller),
        city_name: data.settlement_name(&possession.city),
        occupied: possession.status.is_occupied(),
        held: possession.held_by_viewer,
        total: possession.settlements_total,
        whole_holder: possession
            .whole_province_holder
            .map_or_else(String::new, |f| f.as_str().to_owned()),
        cue: cue.to_owned(),
    })
}

// ----- Slot bar --------------------------------------------------------------------------------

/// One building slot of a colony (port of `CampaignSim.settlement_slots`, ADR 0185).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotView {
    pub root: String,
    /// Building standing in the slot (empty when none).
    pub built: String,
    pub built_name: String,
    pub level: u8,
    pub max_level: u8,
    pub category: String,
    pub upkeep: u32,
    /// `built`, `upgradable`, `empty` or `locked`.
    pub state: String,
    pub locked_reason: String,
    /// First next step (the root when empty, an upgrade otherwise), empty when none.
    pub next_building: String,
    pub next_name: String,
    pub next_cost: u32,
}

/// Slots of `settlement` (by category then id) and the cap of its kind (`None`: uncapped).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotBarView {
    pub settlement: String,
    pub slots: Vec<SlotView>,
    pub used: usize,
    pub max: Option<usize>,
}

/// The slot bar of `settlement`; `None` for an unknown id.
pub fn slot_bar(
    data: &GameData,
    state: &CampaignState,
    settlement: &SettlementId,
) -> Option<SlotBarView> {
    state.settlement_state(settlement)?;
    let usage = state.slot_usage(data, settlement);
    let slots = state
        .building_slots(data, settlement)
        .into_iter()
        .map(|slot| {
            let shown = slot.built.as_ref().unwrap_or(&slot.root);
            let building = data.buildings.get(shown);
            let slot_state = match (&slot.built, slot.locked) {
                (_, true) => "locked",
                (None, false) => "empty",
                (Some(_), false) if slot.next.is_empty() => "built",
                (Some(_), false) => "upgradable",
            };
            let next = slot.next.first();
            SlotView {
                root: slot.root.as_str().to_owned(),
                built: slot
                    .built
                    .as_ref()
                    .map_or_else(String::new, |b| b.as_str().to_owned()),
                built_name: building.map_or_else(String::new, |b| b.name.display.clone()),
                level: slot.level,
                max_level: slot.max_level,
                category: building.map_or_else(String::new, |b| b.category.key().to_owned()),
                upkeep: building.map_or(0, |b| b.upkeep.unwrap_or(0)),
                state: slot_state.to_owned(),
                locked_reason: slot.locked_reason.clone().unwrap_or_default(),
                next_building: next.map_or_else(String::new, |n| n.building.as_str().to_owned()),
                next_name: next.map_or_else(String::new, |n| n.name.clone()),
                next_cost: next.map_or(0, |n| n.cost),
            }
        })
        .collect();
    Some(SlotBarView {
        settlement: settlement.as_str().to_owned(),
        slots,
        used: usage.used,
        max: usage.max,
    })
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
        let table = table_view(data, &state, &capital).unwrap();
        assert!(!table.options.is_empty());
        assert!(edict_view(data, &state, &capital).is_some());
        let vocation = vocation_view(data, &state, &city).unwrap();
        assert!(!vocation.options.is_empty());
        let possession = possession_view(data, &state, &france, &capital).unwrap();
        assert_eq!(possession.status, "own");
        assert_eq!(possession.cue, "self");
        assert!(serde_json::to_string(&table).is_ok());
        let bar = slot_bar(data, &state, &city).unwrap();
        assert!(!bar.slots.is_empty());
        assert!(bar.slots.iter().all(|s| !s.state.is_empty()));
        assert!(serde_json::to_string(&bar).is_ok());
        let unknown = SettlementId::new("set_nowhere").unwrap();
        assert!(slot_bar(data, &state, &unknown).is_none());
        assert!(building_tree(data, &state, &unknown).is_none());
    }
}
