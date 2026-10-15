//! Army sheet.
//!
//! Replaces `CampaignSim.get_army_ids` and `get_army`.

use data_model::GameData;
use serde::Serialize;
use sim_campaign::{ArmyId, CampaignState};

use crate::common::{unit_views, UnitView};

/// Another army of the same faction the army can merge with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergeTarget {
    pub id: String,
    pub name: String,
    pub units: usize,
}

/// Everything the army panel and plate show.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmySheet {
    pub id: String,
    pub name: String,
    pub faction: String,
    pub faction_name: String,
    pub general: Option<String>,
    pub general_name: String,
    pub units: Vec<UnitView>,
    /// Sum of the units' head counts.
    pub strength: u32,
    pub max_strength: u32,
    /// Map-pixel position.
    pub position: [f32; 2],
    /// Settlement the army stands in, `None` in the field.
    pub settlement: Option<String>,
    /// Settlement the army stands in or the nearest one.
    pub location: Option<String>,
    pub location_province: Option<String>,
    /// Movement points left this turn (10 = one plain cell) and this turn's allowance.
    pub movement_left: u32,
    pub movement_max: u32,
    /// Movement left in km of plain.
    pub movement_km: f64,
    pub supply: u8,
    /// `stance` key (`march`, `siege`, ...).
    pub stance: String,
    /// A march spans several turns.
    pub marching: bool,
    /// No order, movement left, not besieging (the "idle army" alert condition).
    pub idle: bool,
    /// Player's own armies only: why recruiting is refused (empty = enabled).
    pub recruit_blocker: Option<String>,
    pub merge_targets: Vec<MergeTarget>,
}

/// Sorted ids of every army.
pub fn army_ids(state: &CampaignState) -> Vec<String> {
    state.armies().keys().map(ToString::to_string).collect()
}

/// Sheet of army `id`; `None` for an unknown id.
pub fn army_sheet(data: &GameData, state: &CampaignState, id: &ArmyId) -> Option<ArmySheet> {
    let army = state.army(id)?;
    let point = state.army_point(data, army);
    let grid = data.navgrid();
    let location = state.army_anchor(data, army);
    let marching = army.destination.is_some() && !army.planned_path.is_empty();
    let own = &army.faction == state.player_faction();
    let (recruit_blocker, merge_targets) = if own {
        let merges = state
            .armies()
            .iter()
            .filter(|(other_id, other)| {
                *other_id != id
                    && other.faction == army.faction
                    && state.armies_together(data, army, other)
                    && army.units.len() + other.units.len() <= data.army_rules.cap()
            })
            .map(|(other_id, other)| MergeTarget {
                id: other_id.to_string(),
                name: state.army_name(data, other_id),
                units: other.units.len(),
            })
            .collect();
        (
            Some(state.army_recruit_blocker(data, id).unwrap_or_default()),
            merges,
        )
    } else {
        (None, Vec::new())
    };
    Some(ArmySheet {
        id: id.to_string(),
        name: state.army_name(data, id),
        faction: army.faction.as_str().to_owned(),
        faction_name: data.faction_name(&army.faction),
        general: army.general.as_ref().map(|g| g.as_str().to_owned()),
        general_name: army
            .general
            .as_ref()
            .map_or_else(String::new, |g| state.character_name(data, g)),
        units: unit_views(data, &army.units),
        strength: army.units.iter().map(|u| u.strength).sum(),
        max_strength: army.units.iter().map(|u| u.max_strength).sum(),
        position: point,
        settlement: army.settlement().map(|s| s.as_str().to_owned()),
        location: location.map(|s| s.as_str().to_owned()),
        location_province: state
            .army_province(data, army)
            .map(|p| p.as_str().to_owned()),
        movement_left: army.movement_left,
        movement_max: state.army_grid_allowance(data, army),
        movement_km: f64::from(army.movement_left) / f64::from(data_model::PLAIN_COST)
            * grid.cell_km,
        supply: army.supply,
        stance: army.stance.key().to_owned(),
        marching,
        idle: is_idle(army),
        recruit_blocker,
        merge_targets,
    })
}

/// The `idle army` condition of the Godot alerts (`CampaignAlerts.army_is_idle`).
pub(crate) fn is_idle(army: &sim_campaign::Army) -> bool {
    army.stance.key() != "siege" && army.planned_path.is_empty() && army.movement_left > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_army_sheet() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let id = sim_campaign::test_support::main_army(&state, "fac_france");
        let sheet = army_sheet(data, &state, &id).unwrap();
        assert_eq!(sheet.faction, "fac_france");
        assert!(!sheet.units.is_empty());
        assert_eq!(
            sheet.strength,
            sheet.units.iter().map(|u| u.strength).sum::<u32>()
        );
        assert!(sheet.movement_max > 0 && sheet.movement_left <= sheet.movement_max);
        assert!(sheet.recruit_blocker.is_some());
        assert!(army_ids(&state).contains(&sheet.id));
        assert!(army_sheet(data, &state, &ArmyId::parse("army_9999").unwrap()).is_none());
        assert!(serde_json::to_string(&sheet).is_ok());
    }
}
