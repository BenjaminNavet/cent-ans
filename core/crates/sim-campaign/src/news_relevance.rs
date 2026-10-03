//! News relevance (audit A6, U6/U7): does a journal entry, a letter or an
//! alert concern the player? Rule (lives here, the UI only reads it):
//! an entry concerns the player when it involves their faction, their
//! suzerain or vassals, their allies, the factions they are at war with, or a
//! province bordering their lands. Everything else is "world" news: kept in
//! the folded « Monde » tab of the journal, never pushed as a letter.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, ProvinceId};

use crate::movement::land_neighbors;
use crate::state::CampaignState;

/// How much an entry concerns the player, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relevance {
    /// Unrelated to the player: « Monde » tab only.
    Far,
    /// A bordering faction or province.
    Neighbor,
    /// Ally, vassal, suzerain or enemy at war.
    Related,
    /// The player's own faction or provinces.
    Player,
}

impl Relevance {
    /// Stable key used by the bridge.
    pub fn as_str(self) -> &'static str {
        match self {
            Relevance::Far => "far",
            Relevance::Neighbor => "neighbor",
            Relevance::Related => "related",
            Relevance::Player => "player",
        }
    }

    /// Whether the entry belongs to the player's own journal (not « Monde »).
    pub fn concerns_player(self) -> bool {
        self != Relevance::Far
    }
}

/// Snapshot of the player's ties, built once per batch of events.
#[derive(Debug, Clone, Default)]
pub struct NewsRelevance {
    player: Option<FactionId>,
    related: BTreeSet<FactionId>,
    neighbor_factions: BTreeSet<FactionId>,
    near_provinces: BTreeSet<ProvinceId>,
    owners: BTreeMap<ProvinceId, FactionId>,
}

impl NewsRelevance {
    /// Reads the player's allies, suzerain, vassals, enemies and borders.
    pub fn new(state: &CampaignState, data: &GameData, player: &FactionId) -> Self {
        let mut relevance = NewsRelevance {
            player: Some(player.clone()),
            ..Default::default()
        };
        for (id, faction) in &state.factions {
            if !faction.alive || id == player {
                continue;
            }
            if state.is_allied(player, id)
                || state.is_at_war(player, id)
                || state.is_at_war(id, player)
            {
                relevance.related.insert(id.clone());
            }
        }
        for id in state.provinces.keys() {
            if let Some(owner) = state.province_owner(id) {
                relevance.owners.insert(id.clone(), owner.clone());
            }
        }
        for id in state.provinces.keys() {
            if state.province_owner(id) != Some(player) && !state.controls_province(player, id) {
                continue;
            }
            relevance.near_provinces.insert(id.clone());
            for neighbor in land_neighbors(data, id) {
                relevance.near_provinces.insert(neighbor.clone());
                if let Some(owner) = state.province_owner(neighbor) {
                    if owner != player {
                        relevance.neighbor_factions.insert(owner.clone());
                    }
                }
            }
        }
        relevance
    }

    fn faction_level(&self, faction: &FactionId) -> Relevance {
        if self.player.as_ref() == Some(faction) {
            Relevance::Player
        } else if self.related.contains(faction) {
            Relevance::Related
        } else if self.neighbor_factions.contains(faction) {
            Relevance::Neighbor
        } else {
            Relevance::Far
        }
    }

    /// Relevance of an entry from its faction, province and `public` flag
    /// (public news reach everyone as at least `Related`, JR5).
    pub fn classify(
        &self,
        faction: Option<&FactionId>,
        province: Option<&ProvinceId>,
        public: bool,
    ) -> Relevance {
        let mut level = if public {
            Relevance::Related
        } else {
            Relevance::Far
        };
        if let Some(faction) = faction {
            level = level.max(self.faction_level(faction));
        }
        if let Some(province) = province {
            if let Some(owner) = self.owners.get(province) {
                level = level.max(self.faction_level(owner));
            }
            if self.near_provinces.contains(province) {
                level = level.max(Relevance::Neighbor);
            }
        }
        level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fid(name: &str) -> FactionId {
        FactionId::new(name.to_owned()).unwrap()
    }

    fn pid(name: &str) -> ProvinceId {
        ProvinceId::new(name.to_owned()).unwrap()
    }

    fn sample() -> NewsRelevance {
        let mut relevance = NewsRelevance {
            player: Some(fid("fac_a")),
            ..Default::default()
        };
        relevance.related.insert(fid("fac_ally"));
        relevance.neighbor_factions.insert(fid("fac_next"));
        relevance.near_provinces.insert(pid("prov_border"));
        relevance.owners.insert(pid("prov_far"), fid("fac_far"));
        relevance.owners.insert(pid("prov_ally"), fid("fac_ally"));
        relevance
    }

    #[test]
    fn own_ally_neighbor_and_far_are_told_apart() {
        let r = sample();
        assert_eq!(
            r.classify(Some(&fid("fac_a")), None, false),
            Relevance::Player
        );
        assert_eq!(
            r.classify(Some(&fid("fac_ally")), None, false),
            Relevance::Related
        );
        assert_eq!(
            r.classify(Some(&fid("fac_next")), None, false),
            Relevance::Neighbor
        );
        assert_eq!(
            r.classify(Some(&fid("fac_far")), None, false),
            Relevance::Far
        );
        assert!(!r
            .classify(Some(&fid("fac_far")), Some(&pid("prov_far")), false)
            .concerns_player());
    }

    #[test]
    fn province_and_public_flag_raise_the_level() {
        let r = sample();
        assert_eq!(
            r.classify(Some(&fid("fac_far")), Some(&pid("prov_border")), false),
            Relevance::Neighbor
        );
        assert_eq!(
            r.classify(None, Some(&pid("prov_ally")), false),
            Relevance::Related
        );
        assert_eq!(
            r.classify(Some(&fid("fac_far")), None, true),
            Relevance::Related
        );
        assert_eq!(r.classify(None, None, false), Relevance::Far);
    }
}
