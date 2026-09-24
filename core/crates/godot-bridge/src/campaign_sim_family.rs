//! `CampaignSim` family tree (lot C3, court panel « Arbre familial »), read
//! only: no rule, only a walk over the parenthood links already kept in
//! `CampaignState::characters` (M4).

use std::collections::{BTreeMap, BTreeSet};

use data_model::{CharacterId, GameData};
use godot::prelude::*;
use sim_campaign::CampaignState;

use crate::campaign_sim::CampaignSim;

/// Hard cap on the number of nodes returned (the UI lays out every node).
const MAX_NODES: usize = 120;

#[godot_api(secondary)]
impl CampaignSim {
    /// Family tree around `character`: ancestors up to `up` generations,
    /// the root's siblings, descendants of the root and of its siblings down
    /// to `down` generations, and the spouse of every blood relative.
    ///
    /// Returns `{root, ruler, heir, nodes}` where `nodes` is
    /// `[{id, name, epithet, sex, alive, birth_year, death_year, age, house, faction,
    /// title, generation, blood, father, mother, spouse, children}]`
    /// (`generation` is relative to the root: negative above, positive
    /// below; `blood` is false for a spouse joined by marriage). Dead
    /// characters stay in the tree (`alive == false`). Empty dictionary for
    /// an unknown id or before a campaign starts.
    #[func]
    fn get_family_tree(&self, character: GString, up: i64, down: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(root) = CharacterId::new(character.to_string()) else {
            return VarDictionary::new();
        };
        let Some(root_state) = state.characters.get(&root) else {
            return VarDictionary::new();
        };
        let generations = family_generations(state, data, &root, up.max(0), down.max(0));
        let faction = state.factions.get(&root_state.faction);
        let opt = |id: Option<&CharacterId>| id.map_or(String::new(), |i| i.to_string());
        let nodes: VarArray = generations
            .iter()
            .filter_map(|(id, (generation, blood))| {
                node_dict(state, data, id, *generation, *blood).map(|d| d.to_variant())
            })
            .collect();
        vdict! {
            "root" => root.as_str(),
            "ruler" => opt(faction.and_then(|f| f.ruler.as_ref())).as_str(),
            "heir" => opt(faction.and_then(|f| f.heir.as_ref())).as_str(),
            "nodes" => &nodes,
        }
    }
}

/// `id → (generation relative to root, blood relative)` of every character
/// shown in the tree, in a deterministic order.
fn family_generations(
    state: &CampaignState,
    data: &GameData,
    root: &CharacterId,
    up: i64,
    down: i64,
) -> BTreeMap<CharacterId, (i64, bool)> {
    let mut out: BTreeMap<CharacterId, (i64, bool)> = BTreeMap::new();
    let known = |id: &CharacterId| state.characters.contains_key(id);
    out.insert(root.clone(), (0, true));

    // Ancestors, one generation at a time.
    let mut frontier = vec![root.clone()];
    for level in 1..=up {
        let mut next = Vec::new();
        for id in &frontier {
            let Some(c) = state.characters.get(id) else {
                continue;
            };
            for parent in [&c.father, &c.mother].into_iter().flatten() {
                if known(parent) && !out.contains_key(parent) && out.len() < MAX_NODES {
                    out.insert(parent.clone(), (-level, true));
                    next.push(parent.clone());
                }
            }
        }
        frontier = next;
    }

    // Siblings: children of the root's parents, else the static record.
    let mut siblings: BTreeSet<CharacterId> = BTreeSet::new();
    if let Some(c) = state.characters.get(root) {
        for parent in [&c.father, &c.mother].into_iter().flatten() {
            if let Some(p) = state.characters.get(parent) {
                siblings.extend(p.children.iter().cloned());
            }
        }
    }
    if let Some(family) = data.characters.get(root).and_then(|c| c.family.as_ref()) {
        siblings.extend(family.siblings.iter().cloned());
    }
    siblings.retain(|s| s != root && known(s));
    let mut frontier = vec![root.clone()];
    for sibling in siblings {
        if out.len() < MAX_NODES && !out.contains_key(&sibling) {
            out.insert(sibling.clone(), (0, true));
            frontier.push(sibling);
        }
    }

    // Descendants of the root and of its siblings.
    for level in 1..=down {
        let mut next = Vec::new();
        for id in &frontier {
            let Some(c) = state.characters.get(id) else {
                continue;
            };
            for child in &c.children {
                if known(child) && !out.contains_key(child) && out.len() < MAX_NODES {
                    out.insert(child.clone(), (level, true));
                    next.push(child.clone());
                }
            }
        }
        frontier = next;
    }

    // Spouses of every blood relative, on the same generation.
    let blood: Vec<(CharacterId, i64)> = out.iter().map(|(id, (g, _))| (id.clone(), *g)).collect();
    for (id, generation) in blood {
        let Some(spouse) = state.characters.get(&id).and_then(|c| c.spouse.clone()) else {
            continue;
        };
        if known(&spouse) && !out.contains_key(&spouse) && out.len() < MAX_NODES {
            out.insert(spouse, (generation, false));
        }
    }
    out
}

fn node_dict(
    state: &CampaignState,
    data: &GameData,
    id: &CharacterId,
    generation: i64,
    blood: bool,
) -> Option<VarDictionary> {
    let c = state.characters.get(id)?;
    let opt = |id: &Option<CharacterId>| id.as_ref().map_or(String::new(), |i| i.to_string());
    let children: PackedStringArray = c
        .children
        .iter()
        .filter(|child| state.characters.contains_key(*child))
        .map(|child| GString::from(child.as_str()))
        .collect();
    let epithet = data
        .characters
        .get(id)
        .and_then(|d| d.epithet.clone())
        .unwrap_or_default();
    Some(vdict! {
        "id" => id.as_str(),
        "name" => state.character_name(data, id).as_str(),
        "epithet" => epithet.as_str(),
        "sex" => match c.sex { data_model::Sex::Male => "male", data_model::Sex::Female => "female" },
        "alive" => c.alive,
        "birth_year" => i64::from(c.birth_year),
        "death_year" => i64::from(c.death_year.unwrap_or(0)),
        "age" => i64::from(c.age(state.year())),
        "house" => c.house.as_str(),
        "faction" => c.faction.as_str(),
        "title" => c.title.as_deref().unwrap_or(""),
        "generation" => generation,
        "blood" => blood,
        "father" => opt(&c.father).as_str(),
        "mother" => opt(&c.mother).as_str(),
        "spouse" => opt(&c.spouse).as_str(),
        "children" => &children,
    })
}
