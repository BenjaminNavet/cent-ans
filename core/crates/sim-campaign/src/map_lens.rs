//! Campaign map filters (« filtres de carte », lot MF1): the per-province
//! values the map tints by, seen from one faction. Read only; computed in
//! one pass so the claims of every faction are gathered once, not per
//! province. The rendering (colours, legend) lives in Godot.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, ProvinceId};

use crate::diplomacy::claimed_provinces;
use crate::economy::{province_base_income, seasonal_supply_change};
use crate::population::weighted_unrest;
use crate::state::CampaignState;

/// Who claims a province, from the viewer's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimStance {
    /// No claim involves the viewer.
    None,
    /// The viewer claims a province it does not own.
    Ours,
    /// Another faction claims a province the viewer owns.
    AgainstUs,
    /// The viewer and a third faction both claim a province neither owns.
    Contested,
}

/// Map filter values of one province.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceLens {
    /// Seasonal base tax (livres) before tax bracket and buildings.
    pub income: f64,
    pub population: u64,
    /// Population-weighted unrest, 0-100.
    pub unrest: f64,
    /// Loyalty (0-100) of the owner when it is a vassal.
    pub vassal_loyalty: Option<u8>,
    /// Suzerain of the owner when it is a vassal.
    pub suzerain: Option<FactionId>,
    /// Seasonal supply change of a viewer army standing here, without a
    /// general: recovery (> 0) or attrition (< 0).
    pub supply_change: i8,
    pub claim: ClaimStance,
}

/// Map filter values of every province, seen from `viewer`.
pub fn map_lens(
    state: &CampaignState,
    data: &GameData,
    viewer: &FactionId,
) -> BTreeMap<ProvinceId, ProvinceLens> {
    let ours = claimed_provinces(state, viewer);
    let others: BTreeSet<ProvinceId> = state
        .factions
        .keys()
        .filter(|f| *f != viewer)
        .flat_map(|f| claimed_provinces(state, f))
        .collect();
    let season = state.season;
    state
        .provinces
        .iter()
        .map(|(id, province)| {
            let owner = state.province_owner(id);
            let vassal = owner
                .and_then(|o| state.factions.get(o))
                .filter(|f| f.suzerain.is_some());
            let owned = owner == Some(viewer);
            let claim = match (ours.contains(id), others.contains(id)) {
                (true, true) if !owned => ClaimStance::Contested,
                (true, _) if !owned => ClaimStance::Ours,
                (_, true) if owned => ClaimStance::AgainstUs,
                _ => ClaimStance::None,
            };
            let friendly = state.is_friendly_territory(viewer, id);
            let lens = ProvinceLens {
                income: province_base_income(data, province),
                population: province.population.total(),
                unrest: weighted_unrest(&province.population),
                vassal_loyalty: vassal.map(|f| f.loyalty),
                suzerain: vassal.and_then(|f| f.suzerain.clone()),
                supply_change: seasonal_supply_change(state, data, id, friendly, None, season),
                claim,
            };
            (id.clone(), lens)
        })
        .collect()
}
