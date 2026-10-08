//! Building: a province improvement (`building.schema.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

use crate::common::{Cost, Effect, LocalizedName, Sources};
use crate::entities::settlement::SettlementKind;
use crate::ids::{BuildingId, ResourceId, TechnologyId, UnitTypeId};

key_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    Production => "production",
    Commerce => "commerce",
    Military => "military",
    Religious => "religious",
    Sanitary => "sanitary",
    Fortification => "fortification",
}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Building {
    pub id: BuildingId,
    pub name: LocalizedName,
    pub category: BuildingCategory,
    pub tier: u8,
    /// Previous step of the upgrade chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrades_from: Option<BuildingId>,
    pub cost: Cost,
    pub build_time_turns: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upkeep: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_technology: Option<TechnologyId>,
    /// Other building that must be present (outside the upgrade chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_building: Option<BuildingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_resource: Option<ResourceId>,
    #[serde(default)]
    pub requires_coastal: bool,
    #[serde(default)]
    pub requires_river: bool,
    #[serde(default)]
    pub unique_per_faction: bool,
    /// Settlement kinds where it can be built (lot C4); `None`: every kind
    /// but `village`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_kinds: Option<Vec<SettlementKind>>,
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enables_units: Vec<UnitTypeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Building {
    /// `true` when the building may stand in a settlement of `kind`
    /// (`settlement_kinds`, default: anything but a village).
    pub fn allowed_in(&self, kind: SettlementKind) -> bool {
        match &self.settlement_kinds {
            Some(kinds) => kinds.contains(&kind),
            None => kind != SettlementKind::Village,
        }
    }
}

impl crate::load::GameData {
    /// `true` when `present` is `required` or a later step of its upgrade
    /// chain (EQ2: a fair still counts as the market it replaced, so a
    /// building or unit requiring the market is not lost by upgrading).
    pub fn building_satisfies(&self, present: &BuildingId, required: &BuildingId) -> bool {
        let mut current = Some(present);
        // The chain is short; the bound guards against a cycle in the data.
        for _ in 0..16 {
            match current {
                Some(id) if id == required => return true,
                Some(id) => {
                    current = self
                        .buildings
                        .get(id)
                        .and_then(|b| b.upgrades_from.as_ref());
                }
                None => return false,
            }
        }
        false
    }

    /// `true` when one of `buildings` satisfies `required`
    /// ([`GameData::building_satisfies`]).
    pub fn has_building(&self, buildings: &[BuildingId], required: &BuildingId) -> bool {
        buildings
            .iter()
            .any(|present| self.building_satisfies(present, required))
    }

    /// `buildings` with one step per upgrade chain: every building another
    /// one of the list upgrades (directly or not) is dropped, and of two
    /// branches of the same chain the higher tier is kept (then the
    /// costlier: the cathedral over the abbey; then the first listed) (EQ2: starting settlements, where the data once stacked the
    /// market, the guild hall and the fair).
    pub fn normalize_building_tiers(&self, buildings: &[BuildingId]) -> Vec<BuildingId> {
        let chain_root = |id: &BuildingId| -> BuildingId {
            let mut current = id.clone();
            for _ in 0..16 {
                match self
                    .buildings
                    .get(&current)
                    .and_then(|b| b.upgrades_from.clone())
                {
                    Some(base) => current = base,
                    None => break,
                }
            }
            current
        };
        let rank = |id: &BuildingId| {
            self.buildings
                .get(id)
                .map_or((0, 0), |b| (b.tier, b.cost.money))
        };
        let mut kept: Vec<BuildingId> = Vec::new();
        for id in buildings {
            if kept.contains(id) {
                continue;
            }
            if buildings
                .iter()
                .any(|other| other != id && self.building_satisfies(other, id))
            {
                continue;
            }
            let root = chain_root(id);
            if let Some(index) = kept
                .iter()
                .position(|k| *k != root && chain_root(k) == root && root != *id)
            {
                if rank(id) > rank(&kept[index]) {
                    kept[index] = id.clone();
                }
                continue;
            }
            kept.push(id.clone());
        }
        kept
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<BuildingCategory>();
    }
}
