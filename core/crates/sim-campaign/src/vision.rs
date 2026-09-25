//! Line of sight on the campaign map (lot C1, light fog of war).
//!
//! A faction sees the provinces it controls, the provinces where its armies
//! stand, and every province within a few land steps of those (ranges from
//! `data/rules/vision.json`, [`VisionRules`]; the default when the file is
//! absent is one step). With `share_allied_vision`, allies (vassals and
//! overlords included, see [`CampaignState::is_allied`]) lend their provinces
//! and armies. The front end veils the other provinces and hides the foreign
//! armies standing in them; nothing else in the simulation depends on it.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, ProvinceId, VisionRules};

use crate::movement::land_neighbors;
use crate::state::CampaignState;

impl CampaignState {
    /// Provinces `faction` can see this turn (see the module doc).
    pub fn visible_provinces(&self, data: &GameData, faction: &FactionId) -> BTreeSet<ProvinceId> {
        let default_rules = VisionRules::default();
        let rules = data.vision_rules.as_ref().unwrap_or(&default_rules);
        let lends_sight = |other: &FactionId| {
            other == faction || (rules.share_allied_vision && self.is_allied(faction, other))
        };
        // Remaining land steps of sight from each province (the best seed wins).
        let mut range: BTreeMap<ProvinceId, u32> = BTreeMap::new();
        let mut seed = |province: &ProvinceId, steps: u32| {
            let best = range.entry(province.clone()).or_insert(steps);
            *best = (*best).max(steps);
        };
        for id in self.provinces.keys() {
            if self.province_controller(id).is_some_and(lends_sight) {
                seed(id, rules.controlled_range);
            }
        }
        for army in self.armies.values() {
            if lends_sight(&army.faction) {
                let bonus = if army.general.is_some() {
                    rules.general_bonus
                } else {
                    0
                };
                // Sight starts from the army's province (lot M2: the one
                // under it in the field).
                if let Some(province) = self.army_province(data, army) {
                    seed(&province, rules.army_range + bonus);
                }
            }
        }
        // C6: spies see around them; scouting keeps a province in sight.
        for (province, steps) in self.agent_sight(data, &lends_sight, faction) {
            seed(&province, steps);
        }
        spread_sight(data, range)
    }
}

/// Spreads each seed's remaining range along land edges (multi-source search
/// that only revisits a province when it is reached with more range left).
fn spread_sight(data: &GameData, mut range: BTreeMap<ProvinceId, u32>) -> BTreeSet<ProvinceId> {
    let mut frontier: Vec<(ProvinceId, u32)> = range
        .iter()
        .map(|(id, steps)| (id.clone(), *steps))
        .collect();
    while let Some((province, steps)) = frontier.pop() {
        if steps == 0 || range.get(&province).is_some_and(|best| *best > steps) {
            continue;
        }
        for neighbour in land_neighbors(data, &province) {
            let left = steps - 1;
            if range.get(neighbour).is_none_or(|best| *best < left) {
                range.insert(neighbour.clone(), left);
                frontier.push((neighbour.clone(), left));
            }
        }
    }
    range.into_keys().collect()
}
