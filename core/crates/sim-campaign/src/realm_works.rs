//! Realm-wide construction overview (`docs/design/chantiers-royaume.md`).
//!
//! [`realm_works`] is a pure query: one row per colony of the faction with its
//! free slots, queue and buildable options. [`plan_works_batch`] and
//! [`apply_works_batch`] run a list of `(settlement, building)` through
//! `Order::Build`, so every work obeys the rules of a single order; the plan
//! runs on a copy of the state and tells what the real run would do.

use data_model::{BuildingId, FactionId, GameData, ProvinceId, SettlementId, SettlementKind};

use crate::buildings::BuildOption;
use crate::orders::{Order, Place};
use crate::state::CampaignState;

/// One colony of [`RealmWorks`].
#[derive(Debug, Clone, PartialEq)]
pub struct WorkRow {
    pub settlement: SettlementId,
    pub name: String,
    pub province: ProvinceId,
    pub province_name: String,
    pub kind: SettlementKind,
    pub tier: u8,
    pub slots_used: usize,
    /// Cap of the kind (`None`: uncapped).
    pub slots_max: Option<usize>,
    /// Works under way plus queued.
    pub queue_len: usize,
    pub queue_size: usize,
    /// Buildings of the colony, available or not (`available` + `reason`).
    pub options: Vec<BuildOption>,
}

impl WorkRow {
    /// Free slots left (`None`: uncapped).
    pub fn slots_free(&self) -> Option<usize> {
        self.slots_max
            .map(|max| max.saturating_sub(self.slots_used))
    }

    /// `true` when the queue cannot take another work.
    pub fn queue_full(&self) -> bool {
        self.queue_len >= self.queue_size
    }

    /// `true` when something can be started here, money aside.
    pub fn can_start(&self) -> bool {
        !self.queue_full() && self.options.iter().any(|o| o.available)
    }
}

/// Overview of the colonies `faction` owns and controls.
#[derive(Debug, Clone, PartialEq)]
pub struct RealmWorks {
    pub treasury: i64,
    /// Sum of the free slots of the capped colonies.
    pub free_slots: usize,
    /// Works under way or queued, all colonies.
    pub works_in_progress: usize,
    pub rows: Vec<WorkRow>,
}

/// Outcome of one entry of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchEntry {
    pub settlement: SettlementId,
    pub building: BuildingId,
    /// Money paid, or why the order was refused.
    pub outcome: Result<u32, String>,
}

/// Outcome of a batch, in the order of the entries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BatchReport {
    pub entries: Vec<BatchEntry>,
}

impl BatchReport {
    pub fn started(&self) -> usize {
        self.entries.iter().filter(|e| e.outcome.is_ok()).count()
    }

    pub fn total_cost(&self) -> i64 {
        self.entries
            .iter()
            .filter_map(|e| e.outcome.as_ref().ok())
            .map(|cost| i64::from(*cost))
            .sum()
    }
}

/// The colonies of `faction`, by province then settlement id. Empty for an
/// unknown faction.
pub fn realm_works(state: &CampaignState, data: &GameData, faction: &FactionId) -> RealmWorks {
    let Some(faction_state) = state.factions.get(faction) else {
        return RealmWorks {
            treasury: 0,
            free_slots: 0,
            works_in_progress: 0,
            rows: Vec::new(),
        };
    };
    let queue_size = data.economy_rules.construction_queue_size.max(1) as usize;
    let mut rows = Vec::new();
    for (province_id, province) in &state.provinces {
        for settlement_id in &province.settlements {
            let Some(live) = state.settlements.get(settlement_id) else {
                continue;
            };
            if &live.owner != faction
                || &live.controller != faction
                || crate::capture::is_ruined(state, settlement_id)
            {
                continue;
            }
            let usage = state.slot_usage(data, settlement_id);
            rows.push(WorkRow {
                settlement: settlement_id.clone(),
                name: data
                    .settlements
                    .get(settlement_id)
                    .map_or_else(|| settlement_id.to_string(), |s| s.name.display.clone()),
                province: province_id.clone(),
                province_name: data.province_name(province_id),
                kind: live.kind,
                tier: state.settlement_tier(data, settlement_id),
                slots_used: usage.used,
                slots_max: usage.max,
                queue_len: usize::from(live.construction.is_some()) + live.build_queue.len(),
                queue_size,
                options: state.buildable(data, settlement_id),
            });
        }
    }
    RealmWorks {
        treasury: faction_state.treasury,
        free_slots: rows.iter().filter_map(WorkRow::slots_free).sum(),
        works_in_progress: rows.iter().map(|r| r.queue_len).sum(),
        rows,
    }
}

/// What [`apply_works_batch`] would do, run on a copy of `state`.
pub fn plan_works_batch(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    entries: &[(SettlementId, BuildingId)],
) -> BatchReport {
    apply_works_batch(&mut state.clone(), data, faction, entries)
}

/// Starts every `(settlement, building)` in order through `Order::Build`.
/// A refused entry (money, full queue, prerequisite) does not stop the others.
pub fn apply_works_batch(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    entries: &[(SettlementId, BuildingId)],
) -> BatchReport {
    let mut report = BatchReport::default();
    for (settlement, building) in entries {
        let treasury_before = state.factions.get(faction).map_or(0, |f| f.treasury);
        let result = state.apply_order(
            data,
            faction,
            Order::Build {
                settlement: Place::Settlement(settlement.clone()),
                building: building.clone(),
            },
        );
        let treasury_after = state.factions.get(faction).map_or(0, |f| f.treasury);
        report.entries.push(BatchEntry {
            settlement: settlement.clone(),
            building: building.clone(),
            outcome: result
                .map(|()| u32::try_from(treasury_before - treasury_after).unwrap_or(0))
                .map_err(|error| error.to_string()),
        });
    }
    report
}
