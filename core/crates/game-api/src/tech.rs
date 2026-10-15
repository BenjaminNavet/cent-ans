//! Technology screen (UX5 lot T): the three trees of a faction with states, effective costs,
//! prerequisites, unlocks and effects, plus current research, queue and reserve.
//!
//! Replaces `get_tech_tree`, `get_research`, `get_research_queue`, `get_research_reserve` and
//! `get_research_points` of `godot-bridge` (`campaign_sim_tech.rs`). Choosing a research is an
//! order (`Order::Research` / `QueueResearch` / `DequeueResearch`) submitted by the client.

use data_model::{Effect, FactionId, GameData, Technology};
use serde::Serialize;
use sim_campaign::research::{effective_cost, tech_progress, tech_status};
use sim_campaign::CampaignState;

/// One effect of a technology (`kind` = serde key such as `tax_income`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EffectView {
    pub kind: String,
    pub value: f64,
    /// `add` or `percent`.
    pub mode: String,
    /// `infantry`, `ranged`, `cavalry`, `siege` or empty.
    pub unit_category: String,
    pub class: String,
}

/// An entity (unit type or building) unlocked by a technology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnlockView {
    pub id: String,
    pub name: String,
}

/// A prerequisite and whether the faction already knows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrerequisiteView {
    pub id: String,
    pub name: String,
    pub met: bool,
}

/// One node of a tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TechNode {
    pub id: String,
    pub name: String,
    /// `military`, `civil` or `medicine`.
    pub branch: String,
    pub tier: u32,
    pub cost: u32,
    /// Cost with the anachronism surcharge.
    pub effective_cost: u32,
    pub prerequisites: Vec<PrerequisiteView>,
    pub unit_unlocks: Vec<UnlockView>,
    pub building_unlocks: Vec<UnlockView>,
    pub effects: Vec<EffectView>,
    pub description: String,
    /// 0 when unknown.
    pub historical_year: i32,
    pub historical_uncertain: bool,
    pub historical_note: String,
    /// `known`, `available`, `locked` or `researching`.
    pub state: String,
    pub progress: u32,
    /// 1-based position in the research queue, 0 when not queued.
    pub queue_position: usize,
    /// Faction statistics moved by the technology: `(key, before, after)`.
    pub before_after: Vec<(String, f64, f64)>,
}

/// Current research (`get_research`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResearchView {
    pub technology: String,
    pub name: String,
    pub progress: u32,
    pub cost: u32,
    pub points_per_turn: u32,
    /// `None` when no point is produced.
    pub turns_left: Option<u32>,
}

/// A queued technology (`get_research_queue`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueEntry {
    pub id: String,
    pub name: String,
    pub cost: u32,
}

/// Everything the technology screen shows for one faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TechTree {
    pub faction: String,
    pub points_per_turn: u32,
    pub research: Option<ResearchView>,
    pub queue: Vec<QueueEntry>,
    /// Points piled up while no research runs.
    pub reserve_points: u32,
    pub reserve_cap: u32,
    pub queue_max: u32,
    /// Sorted by branch, tier, id.
    pub techs: Vec<TechNode>,
}

fn key_of<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn effect_view(effect: &Effect) -> EffectView {
    EffectView {
        kind: key_of(&effect.effect),
        value: effect.value,
        mode: key_of(&effect.mode),
        unit_category: effect.unit_category.map(|c| key_of(&c)).unwrap_or_default(),
        class: effect.class.map_or("", |c| c.key()).to_owned(),
    }
}

fn node(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    tech: &Technology,
) -> TechNode {
    let queue = state.factions.get(faction).map(|f| &f.research_queue);
    let name_of = |id: &str, found: Option<&str>| UnlockView {
        id: id.to_owned(),
        name: found.unwrap_or(id).to_owned(),
    };
    let own_state = tech_status(state, faction, tech);
    TechNode {
        id: tech.id.as_str().to_owned(),
        name: tech.name.display.clone(),
        branch: tech.branch.key().to_owned(),
        tier: tech.tier.into(),
        cost: tech.cost,
        effective_cost: effective_cost(tech, state.year()),
        prerequisites: tech
            .prerequisites
            .iter()
            .map(|p| PrerequisiteView {
                id: p.as_str().to_owned(),
                name: data
                    .technologies
                    .get(p)
                    .map_or(p.as_str(), |t| t.name.display.as_str())
                    .to_owned(),
                met: state
                    .factions
                    .get(faction)
                    .is_some_and(|f| f.technologies.contains(p)),
            })
            .collect(),
        unit_unlocks: tech
            .unlocks
            .units
            .iter()
            .map(|u| {
                name_of(
                    u.as_str(),
                    data.unit_types.get(u).map(|t| t.name.display.as_str()),
                )
            })
            .collect(),
        building_unlocks: tech
            .unlocks
            .buildings
            .iter()
            .map(|b| {
                name_of(
                    b.as_str(),
                    data.buildings.get(b).map(|t| t.name.display.as_str()),
                )
            })
            .collect(),
        effects: tech.effects.iter().map(effect_view).collect(),
        description: tech.description.clone().unwrap_or_default(),
        historical_year: tech
            .historical_year
            .as_ref()
            .and_then(|d| d.year())
            .map_or(0, i32::from),
        historical_uncertain: tech.historical_year.as_ref().is_some_and(|d| d.uncertain),
        historical_note: tech
            .historical_year
            .as_ref()
            .and_then(|d| d.note.clone())
            .unwrap_or_default(),
        state: own_state.key().to_owned(),
        progress: tech_progress(state, faction, &tech.id),
        queue_position: queue
            .and_then(|q| q.iter().position(|id| id == &tech.id))
            .map_or(0, |i| i + 1),
        before_after: state
            .technology_before_after(data, faction, &tech.id)
            .into_iter()
            .map(|(key, [before, after])| (key, before, after))
            .collect(),
    }
}

/// The technology screen of `faction`; `None` for a faction unknown to the campaign.
pub fn tech_tree(data: &GameData, state: &CampaignState, faction: &FactionId) -> Option<TechTree> {
    let f = state.factions.get(faction)?;
    let mut techs: Vec<&Technology> = data.technologies.values().collect();
    techs.sort_by_key(|t| (t.branch.key(), t.tier, t.id.clone()));
    let research = state.research_info(data, faction).map(|info| ResearchView {
        name: data
            .technologies
            .get(&info.technology)
            .map_or(info.technology.as_str(), |t| t.name.display.as_str())
            .to_owned(),
        technology: info.technology.as_str().to_owned(),
        progress: info.progress,
        cost: info.cost,
        points_per_turn: info.points_per_turn,
        turns_left: (info.turns_left != u32::MAX).then_some(info.turns_left),
    });
    let (reserve_points, reserve_cap) = state.research_reserve(data, faction);
    Some(TechTree {
        faction: faction.as_str().to_owned(),
        points_per_turn: state.research_points_per_turn(data, faction),
        research,
        queue: f
            .research_queue
            .iter()
            .filter_map(|id| data.technologies.get(id))
            .map(|t| QueueEntry {
                id: t.id.as_str().to_owned(),
                name: t.name.display.clone(),
                cost: effective_cost(t, state.year()),
            })
            .collect(),
        reserve_points,
        reserve_cap,
        queue_max: data.economy_rules.research_queue_max,
        techs: techs
            .into_iter()
            .map(|t| node(data, state, faction, t))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::Order;

    #[test]
    fn france_tree_and_research_order() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let faction = FactionId::new("fac_france").unwrap();
        let tree = tech_tree(data, &state, &faction).unwrap();
        assert!(tree.techs.len() > 20);
        for branch in ["military", "civil", "medicine"] {
            assert!(tree.techs.iter().any(|t| t.branch == branch), "{branch}");
        }
        assert!(tree.techs.iter().any(|t| t.state == "known"));
        assert!(tree.research.is_none() || tree.techs.iter().any(|t| t.state == "researching"));
        // Prerequisite names resolve and `met` agrees with the state of known techs.
        for t in &tree.techs {
            for p in &t.prerequisites {
                assert_ne!(p.name, "");
                if t.state == "known" {
                    assert!(p.met, "{} requires {}", t.id, p.id);
                }
            }
            assert!(t.effective_cost >= t.cost);
        }
        let available = tree
            .techs
            .iter()
            .find(|t| t.state == "available")
            .expect("an available technology")
            .clone();
        state
            .submit_order(
                data,
                Order::Research {
                    technology: data_model::TechnologyId::new(available.id.clone()).unwrap(),
                },
            )
            .unwrap();
        let after = tech_tree(data, &state, &faction).unwrap();
        let research = after.research.clone().expect("research set");
        assert_eq!(research.technology, available.id);
        assert_eq!(research.cost, available.effective_cost);
        assert!(after
            .techs
            .iter()
            .any(|t| t.id == available.id && t.state == "researching"));
        assert!(tech_tree(data, &state, &FactionId::new("fac_nowhere").unwrap()).is_none());
        assert!(serde_json::to_string(&after).is_ok());
    }
}
