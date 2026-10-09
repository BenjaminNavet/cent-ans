//! WH `diplob` (ADR 0282): the league against a hegemon.
//!
//! A faction that holds a large share of the provinces and out-weighs the
//! second power by far draws the distrust of all: everyone not bound to it
//! likes it less, enemies of the hegemon ally more easily, and war against
//! it needs no other casus belli. The league is a state of the world
//! ([`League`]), refreshed each season by [`update_league`].

use std::collections::BTreeMap;

use super::*;

/// The standing league: everyone not bound to `target` stands against it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct League {
    /// The hegemon.
    pub target: FactionId,
    /// Season the league formed.
    pub since_turn: u32,
    /// Season before which it does not dissolve.
    pub until_turn: u32,
}

/// Reason of the opinion every faction holds against the hegemon.
pub const LEAGUE_REASON: &str = "Puissance menaçante pour tous";
/// Casus belli against the hegemon.
pub const LEAGUE_CASUS_BELLI: &str = "ligue contre l'hégémon";

impl CampaignState {
    /// The faction that dominates the map (more than `league.province_share`
    /// of the provinces owned, and `league.power_ratio` times the power of
    /// the second power), if any. Pure.
    pub fn hegemon(&self, data: &GameData) -> Option<FactionId> {
        let rules = &data.ai_diplomacy.league;
        if !rules.enabled {
            return None;
        }
        let mut owned: BTreeMap<&FactionId, usize> = BTreeMap::new();
        for province in self.provinces.keys() {
            if let Some(owner) = self.province_owner(province) {
                *owned.entry(owner).or_default() += 1;
            }
        }
        let total: usize = owned.values().sum();
        let living = |f: &FactionId| {
            !f.is_rebels()
                && f.as_str() != PAPACY_FACTION
                && self.factions.get(f).is_some_and(|s| s.alive)
        };
        let (top, count) = owned
            .iter()
            .filter(|(f, _)| living(f))
            .filter(|(f, _)| rules.include_player || **f != &self.player_faction)
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(f, n)| ((*f).clone(), *n))?;
        if total == 0 || (count as f64) < rules.province_share * total as f64 {
            return None;
        }
        let cache = PlanCache::new(self);
        let second = self
            .factions
            .keys()
            .filter(|f| **f != top && living(f))
            .map(|f| cache.faction_power(f))
            .fold(0.0, f64::max);
        (cache.faction_power(&top) >= rules.power_ratio * second.max(1.0)).then_some(top)
    }

    /// The hegemon `faction` stands against: the league target when
    /// `faction` is neither the target nor bound to it (alliance, vassalage).
    pub fn league_target_of(&self, data: &GameData, faction: &FactionId) -> Option<&FactionId> {
        if !data.ai_diplomacy.league.enabled {
            return None;
        }
        let league = self.league.as_ref()?;
        (!self.is_allied(faction, &league.target)).then_some(&league.target)
    }

    /// `a` and `b` both stand in the league against the same hegemon.
    pub fn league_peers(&self, data: &GameData, a: &FactionId, b: &FactionId) -> bool {
        a != b
            && self
                .league_target_of(data, a)
                .is_some_and(|t| Some(t) == self.league_target_of(data, b))
    }
}

/// Season phase: forms, renews or dissolves the league, once per season.
pub(crate) fn update_league(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.ai_diplomacy.league;
    if !rules.enabled {
        state.league = None;
        return;
    }
    let turn = state.turn;
    let hegemon = state.hegemon(data);
    match (&state.league, hegemon) {
        (Some(league), Some(h)) if league.target == h => {
            if let Some(l) = state.league.as_mut() {
                l.until_turn = l.until_turn.max(turn + 1);
            }
        }
        (current, Some(h)) if current.as_ref().is_none_or(|l| l.until_turn <= turn) => {
            let text = format!(
                "Les princes se liguent contre {}, dont la puissance menace tous ses voisins.",
                data.faction_name(&h)
            );
            events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&h));
            state.league = Some(League {
                target: h,
                since_turn: turn,
                until_turn: turn + rules.duration_turns,
            });
        }
        (Some(league), None) if league.until_turn <= turn => {
            let text = format!(
                "La ligue contre {} se dissout : sa puissance ne menace plus.",
                data.faction_name(&league.target)
            );
            let target = league.target.clone();
            events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&target));
            state.league = None;
        }
        _ => {}
    }
}
