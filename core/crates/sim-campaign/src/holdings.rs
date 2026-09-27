//! Settlements quick-access list (touche B), lot HL1.
//! `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 1.
//!
//! [`holdings_overview`] is a pure query, without side effect: it never
//! changes `CampaignState`. It reuses the same building blocks as
//! `settlement_detail` in `godot-bridge/src/campaign_sim_settlements.rs`
//! (`settlement_tax`, `buildable`, `garrison_cap`) so both stay in sync.

use data_model::{FactionId, GameData, ProvinceId, SettlementId, SettlementKind};

use crate::state::CampaignState;

/// Why a settlement is [`SettlementRow::endangered`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DangerReason {
    Siege,
    Occupied,
    RevoltCountdown,
    UnrestAboveThreshold,
}

/// One buildable or promotable option offered by a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BuildOptionRow {
    pub building: data_model::BuildingId,
    pub name: String,
    pub cost: u32,
    pub is_upgrade: bool,
}

/// A construction under way in a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ConstructionRow {
    pub building: data_model::BuildingId,
    pub name: String,
    pub turns_left: u32,
}

/// A siege under way against a settlement.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SiegeRow {
    pub attacker: FactionId,
    pub turns_left: u32,
}

/// One settlement of a [`ProvinceRow`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SettlementRow {
    pub id: SettlementId,
    pub name: String,
    pub kind: SettlementKind,
    pub is_city: bool,
    pub occupied: bool,
    pub income: i64,
    pub construction: Option<ConstructionRow>,
    pub idle: bool,
    pub options_available: Vec<BuildOptionRow>,
    pub is_upgrade: bool,
    pub upgrade_available: bool,
    pub recruit_queue_len: usize,
    pub garrison_strength: u32,
    pub garrison_free: i64,
    pub siege: Option<SiegeRow>,
    pub endangered: bool,
    pub danger_reasons: Vec<DangerReason>,
}

/// One province holding at least one settlement of [`HoldingsOverview`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProvinceRow {
    pub province: ProvinceId,
    pub name: String,
    pub income: i64,
    pub unrest: i64,
    pub revolt_seasons: u32,
    pub revolt_seasons_needed: u32,
    pub revolt_threshold: f64,
    pub devastation: u8,
    pub slots_busy: usize,
    pub slots_total: usize,
    pub settlements: Vec<SettlementRow>,
}

/// Settlements quick-access overview of `faction` (touche B).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HoldingsOverview {
    pub treasury: i64,
    pub net_income_last_turn: i64,
    pub settlement_income_total: i64,
    pub count_idle: usize,
    pub count_upgrade: usize,
    pub count_endangered: usize,
    pub provinces: Vec<ProvinceRow>,
}

/// Builds the settlements quick-access overview of `faction`: the
/// settlements it controls, plus those it owns but does not control
/// (`occupied`), grouped by their province. Pure, no side effect. Empty for
/// an unknown faction.
pub fn holdings_overview(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> HoldingsOverview {
    let _ = (state, data, faction);
    todo!()
}
