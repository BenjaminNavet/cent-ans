//! `CampaignSim` technology API (M6, `docs/design/m6-technologies.md` § 3),
//! kept in a secondary `#[godot_api]` impl block so that parallel milestones
//! do not edit the same file.
//!
//! The `research { technology }` order goes through the existing
//! `submit_order`.

use data_model::{FactionId, TechBranch, Technology};
use godot::prelude::*;
use sim_campaign::research::{effective_cost, tech_progress, tech_status};

use crate::campaign_sim::{effects_array, ids, CampaignSim};

fn tech_branch_key(branch: TechBranch) -> &'static str {
    match branch {
        TechBranch::Military => "military",
        TechBranch::Civil => "civil",
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Both technology trees as seen by `faction`, sorted by branch, tier,
    /// then id: `[{id, name, branch, tier, cost, effective_cost,
    /// prerequisites[], unlocks{units[], buildings[]}, effects[{kind, value,
    /// mode}], description, historical_year, state, progress}]` where `state`
    /// is `"known" | "available" | "locked" | "researching"`.
    #[func]
    fn get_tech_tree(&self, faction: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        let mut techs: Vec<&Technology> = data.technologies.values().collect();
        techs.sort_by_key(|t| (tech_branch_key(t.branch), t.tier, t.id.clone()));
        techs
            .into_iter()
            .map(|tech| {
                let unlocks = vdict! {
                    "units" => &ids(tech.unlocks.units.iter()),
                    "buildings" => &ids(tech.unlocks.buildings.iter()),
                };
                let unit_names: PackedStringArray = tech
                    .unlocks
                    .units
                    .iter()
                    .map(|u| {
                        GString::from(
                            data.unit_types
                                .get(u)
                                .map_or(u.as_str(), |t| t.name.display.as_str()),
                        )
                    })
                    .collect();
                let building_names: PackedStringArray = tech
                    .unlocks
                    .buildings
                    .iter()
                    .map(|b| {
                        GString::from(
                            data.buildings
                                .get(b)
                                .map_or(b.as_str(), |t| t.name.display.as_str()),
                        )
                    })
                    .collect();
                let historical_year = tech
                    .historical_year
                    .as_ref()
                    .and_then(|d| d.year())
                    .map_or(0, i64::from);
                vdict! {
                    "id" => tech.id.as_str(),
                    "name" => tech.name.display.as_str(),
                    "branch" => tech_branch_key(tech.branch),
                    "tier" => i64::from(tech.tier),
                    "cost" => i64::from(tech.cost),
                    "effective_cost" => i64::from(effective_cost(tech, state.year())),
                    "prerequisites" => &ids(tech.prerequisites.iter()),
                    "unlocks" => &unlocks,
                    "unlock_names" => &vdict! {
                        "units" => &unit_names,
                        "buildings" => &building_names,
                    },
                    "effects" => &effects_array(&tech.effects),
                    "description" => tech.description.as_deref().unwrap_or(""),
                    "historical_year" => historical_year,
                    "historical_uncertain" => tech.historical_year.as_ref().is_some_and(|d| d.uncertain),
                    "state" => tech_status(state, &faction, tech).key(),
                    "progress" => i64::from(tech_progress(state, &faction, &tech.id)),
                }
                .to_variant()
            })
            .collect()
    }

    /// `{technology, name, progress, cost, points_per_turn, turns_left}` of
    /// `faction`'s current research; empty when idle (`get_research_points`
    /// gives the rate then). `turns_left` is -1 when no points are produced.
    #[func]
    fn get_research(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(info) = state.research_info(data, &faction) else {
            return VarDictionary::new();
        };
        let name = data
            .technologies
            .get(&info.technology)
            .map_or(info.technology.as_str(), |t| t.name.display.as_str());
        let turns_left = if info.turns_left == u32::MAX {
            -1
        } else {
            i64::from(info.turns_left)
        };
        vdict! {
            "technology" => info.technology.as_str(),
            "name" => name,
            "progress" => i64::from(info.progress),
            "cost" => i64::from(info.cost),
            "points_per_turn" => i64::from(info.points_per_turn),
            "turns_left" => turns_left,
        }
    }

    /// Research points `faction` produces per turn (shown when idle).
    #[func]
    fn get_research_points(&self, faction: GString) -> i64 {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return 0;
        };
        FactionId::new(faction.to_string())
            .map_or(0, |f| i64::from(state.research_points_per_turn(data, &f)))
    }
}
