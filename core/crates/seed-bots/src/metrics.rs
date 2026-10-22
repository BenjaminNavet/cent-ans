//! Balance metrics of one game, and their aggregation per bot (oracle 4).
//!
//! Sources, all read from the real model:
//! - `revolts`: [`EventKind::Revolt`] events returned by `end_turn_with`, whoever the province
//!   belongs to (both the garrison weakened and the province handed to the rebels count).
//! - `bankruptcies`: [`EventKind::Bankruptcy`] events (a faction starts a turn with a negative
//!   treasury), all factions.
//! - `war_turns_mean`: mean duration, in turns, of the wars between living factions; a war is a
//!   pair of factions with `at_war_with`, seen after each end of turn, closed when the pair
//!   makes peace or when the game ends. 0 when no war broke out.
//! - `factions_eliminated`: factions seen alive that are no longer alive (`FactionState::alive`),
//!   which is more reliable than counting `FactionDestroyed` events (two code paths emit them).
//! - `ai_refused_orders` (B1): AI orders the core refused (counted by `CampaignState`).
//! - `leader_territory_share`: provinces owned (`province_owner`) by the faction owning the most,
//!   `fac_rebels` excluded, over all provinces of the map, at the end of the game.

use std::collections::{BTreeMap, BTreeSet};

use data_model::FactionId;
use serde::{Deserialize, Serialize};
use sim_campaign::{CampaignState, EventKind, GameEvent};

/// Names of the metrics, as keys of the bands file.
pub const METRIC_NAMES: [&str; 10] = [
    "revolts_per_game",
    "bankruptcies_per_game",
    "war_turns_mean",
    "factions_eliminated",
    "leader_territory_share",
    "revolts_occupied_per_game",
    "rebel_takeovers_per_game",
    "seasons_above_revolt_threshold_per_game",
    "ai_refused_orders",
    "ai_declined_proposals",
];

/// The metrics of one game.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GameMetrics {
    /// Revolt events.
    pub revolts: u32,
    /// Bankruptcy events.
    pub bankruptcies: u32,
    /// Mean duration of the wars, in turns.
    pub war_turns_mean: f64,
    /// Factions that died during the game.
    pub factions_eliminated: u32,
    /// Share (0-1) of the map owned by the biggest faction at the end.
    pub leader_territory_share: f64,
    /// Revolts in a province held by a faction other than its owner (occupied).
    pub revolts_occupied: u32,
    /// Provinces handed to the rebels (`fac_rebels`).
    pub rebel_takeovers: u32,
    /// Province-seasons spent with the revolt counter running (unrest above the threshold).
    pub seasons_above_revolt_threshold: u32,
    /// B1: AI orders refused by the core (planner proposed an order the core rejected).
    #[serde(default)]
    pub ai_refused_orders: u32,
    /// B1: treaty proposals of the AI declined by their target (normal outcome).
    #[serde(default)]
    pub ai_declined_proposals: u32,
}

impl GameMetrics {
    /// The values by metric name (same names as [`METRIC_NAMES`]).
    pub fn values(&self) -> [(&'static str, f64); 10] {
        [
            (METRIC_NAMES[0], f64::from(self.revolts)),
            (METRIC_NAMES[1], f64::from(self.bankruptcies)),
            (METRIC_NAMES[2], self.war_turns_mean),
            (METRIC_NAMES[3], f64::from(self.factions_eliminated)),
            (METRIC_NAMES[4], self.leader_territory_share),
            (METRIC_NAMES[5], f64::from(self.revolts_occupied)),
            (METRIC_NAMES[6], f64::from(self.rebel_takeovers)),
            (
                METRIC_NAMES[7],
                f64::from(self.seasons_above_revolt_threshold),
            ),
            (METRIC_NAMES[8], f64::from(self.ai_refused_orders)),
            (METRIC_NAMES[9], f64::from(self.ai_declined_proposals)),
        ]
    }
}

/// Watches a game turn by turn.
pub struct MetricsTracker {
    revolts: u32,
    revolts_occupied: u32,
    rebel_takeovers: u32,
    seasons_above_revolt_threshold: u32,
    bankruptcies: u32,
    turns: u32,
    /// War pairs in progress, with the turn they were first seen.
    wars: BTreeMap<(FactionId, FactionId), u32>,
    durations: Vec<u32>,
    alive: BTreeSet<FactionId>,
    eliminated: BTreeSet<FactionId>,
}

fn alive_factions(state: &CampaignState) -> BTreeSet<FactionId> {
    state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect()
}

fn war_pairs(state: &CampaignState) -> BTreeSet<(FactionId, FactionId)> {
    let mut pairs = BTreeSet::new();
    for (id, faction) in state.factions.iter().filter(|(_, f)| f.alive) {
        for other in &faction.at_war_with {
            if state.factions.get(other).is_some_and(|f| f.alive) {
                pairs.insert(if id < other {
                    (id.clone(), other.clone())
                } else {
                    (other.clone(), id.clone())
                });
            }
        }
    }
    pairs
}

impl MetricsTracker {
    /// Starts watching a game from its initial state.
    pub fn new(state: &CampaignState) -> Self {
        MetricsTracker {
            revolts: 0,
            revolts_occupied: 0,
            rebel_takeovers: 0,
            seasons_above_revolt_threshold: 0,
            bankruptcies: 0,
            turns: 0,
            wars: BTreeMap::new(),
            durations: Vec::new(),
            alive: alive_factions(state),
            eliminated: BTreeSet::new(),
        }
    }

    /// Takes one end of turn into account: the state after it and the events it returned.
    pub fn observe(&mut self, state: &CampaignState, events: &[GameEvent]) {
        self.turns += 1;
        for event in events {
            match event.kind {
                EventKind::Revolt => {
                    self.revolts += 1;
                    self.observe_revolt(state, event);
                }
                EventKind::Bankruptcy => self.bankruptcies += 1,
                _ => {}
            }
        }
        self.seasons_above_revolt_threshold += state
            .provinces
            .values()
            .filter(|province| province.revolt_seasons > 0)
            .count() as u32;
        let pairs = war_pairs(state);
        let ended: Vec<_> = self
            .wars
            .keys()
            .filter(|pair| !pairs.contains(*pair))
            .cloned()
            .collect();
        for pair in ended {
            if let Some(start) = self.wars.remove(&pair) {
                self.durations.push(self.turns - 1 - start);
            }
        }
        for pair in pairs {
            self.wars.entry(pair).or_insert(self.turns - 1);
        }
        let alive = alive_factions(state);
        for gone in self.alive.difference(&alive) {
            self.eliminated.insert(gone.clone());
        }
        self.alive = alive;
    }

    /// Splits a revolt event into garrison revolt (occupied or not) and rebel takeover.
    fn observe_revolt(&mut self, state: &CampaignState, event: &GameEvent) {
        let Some(province) = &event.province else {
            return;
        };
        if event
            .faction
            .as_ref()
            .is_some_and(|f| f.as_str() == "fac_rebels")
        {
            self.rebel_takeovers += 1;
        } else if state
            .province_owner(province)
            .zip(event.faction.as_ref())
            .is_some_and(|(owner, controller)| owner != controller)
        {
            self.revolts_occupied += 1;
        }
    }

    /// Closes the game and returns its metrics.
    pub fn finish(mut self, state: &CampaignState) -> GameMetrics {
        for (_, start) in std::mem::take(&mut self.wars) {
            self.durations.push(self.turns - start);
        }
        let war_turns_mean = if self.durations.is_empty() {
            0.0
        } else {
            f64::from(self.durations.iter().sum::<u32>()) / self.durations.len() as f64
        };
        for sample in state.ai_refused_order_samples() {
            eprintln!("ai refused order: {sample}");
        }
        GameMetrics {
            revolts: self.revolts,
            bankruptcies: self.bankruptcies,
            war_turns_mean,
            factions_eliminated: self.eliminated.len() as u32,
            leader_territory_share: leader_territory_share(state),
            revolts_occupied: self.revolts_occupied,
            rebel_takeovers: self.rebel_takeovers,
            seasons_above_revolt_threshold: self.seasons_above_revolt_threshold,
            ai_refused_orders: state.ai_refused_orders(),
            ai_declined_proposals: state.ai_declined_proposals(),
        }
    }
}

/// Share of the provinces owned by the biggest faction (rebels excluded).
pub fn leader_territory_share(state: &CampaignState) -> f64 {
    let mut counts: BTreeMap<&FactionId, u32> = BTreeMap::new();
    for province in state.provinces.keys() {
        if let Some(owner) = state.province_owner(province) {
            if !owner.is_rebels() {
                *counts.entry(owner).or_default() += 1;
            }
        }
    }
    let best = counts.values().copied().max().unwrap_or(0);
    if state.provinces.is_empty() {
        0.0
    } else {
        f64::from(best) / state.provinces.len() as f64
    }
}

/// Mean, min and max of one metric over the games of a bot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aggregate {
    /// Mean.
    pub mean: f64,
    /// Smallest value.
    pub min: f64,
    /// Largest value.
    pub max: f64,
}

/// A bot's metrics over its games.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotMetrics {
    /// Bot name.
    pub bot: String,
    /// Complete games counted.
    pub games: u32,
    /// Aggregate by metric name.
    pub metrics: BTreeMap<String, Aggregate>,
}

/// Aggregates per-game metrics by bot, bots in name order.
pub fn aggregate(games: &[(String, GameMetrics)]) -> Vec<BotMetrics> {
    let mut by_bot: BTreeMap<&str, Vec<&GameMetrics>> = BTreeMap::new();
    for (bot, metrics) in games {
        by_bot.entry(bot).or_default().push(metrics);
    }
    by_bot
        .into_iter()
        .map(|(bot, played)| {
            let mut metrics = BTreeMap::new();
            for (index, name) in METRIC_NAMES.iter().enumerate() {
                let values: Vec<f64> = played.iter().map(|m| m.values()[index].1).collect();
                metrics.insert(
                    (*name).to_owned(),
                    Aggregate {
                        mean: values.iter().sum::<f64>() / values.len() as f64,
                        min: values.iter().copied().fold(f64::INFINITY, f64::min),
                        max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    },
                );
            }
            BotMetrics {
                bot: bot.to_owned(),
                games: played.len() as u32,
                metrics,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::FactionState;

    fn fid(name: &str) -> FactionId {
        FactionId::new(name).unwrap()
    }

    fn world() -> CampaignState {
        let mut state = CampaignState::new(1);
        for name in ["fac_a", "fac_b", "fac_c"] {
            let capital = data_model::ProvinceId::new("prov_x").unwrap();
            state.factions.insert(fid(name), FactionState::new(capital));
        }
        state
    }

    fn set_war(state: &mut CampaignState, a: &str, b: &str, war: bool) {
        for (x, y) in [(a, b), (b, a)] {
            let set = &mut state.factions.get_mut(&fid(x)).unwrap().at_war_with;
            if war {
                set.insert(fid(y));
            } else {
                set.remove(&fid(y));
            }
        }
    }

    #[test]
    fn tracker_counts_events_wars_and_eliminations() {
        let mut state = world();
        let mut tracker = MetricsTracker::new(&state);
        let events = vec![
            GameEvent::new(EventKind::Revolt, "r1"),
            GameEvent::new(EventKind::Revolt, "r2"),
            GameEvent::new(EventKind::Bankruptcy, "b"),
            GameEvent::new(EventKind::Battle, "autre"),
        ];
        // Turn 1: a-b at war.
        set_war(&mut state, "fac_a", "fac_b", true);
        tracker.observe(&state, &events);
        // Turn 2: still at war; c dies.
        state.factions.get_mut(&fid("fac_c")).unwrap().alive = false;
        tracker.observe(&state, &[]);
        // Turn 3: peace; then a-c "war" with c dead does not count.
        set_war(&mut state, "fac_a", "fac_b", false);
        set_war(&mut state, "fac_a", "fac_c", true);
        tracker.observe(&state, &[]);
        // Turn 4: a-b again, still on at the end of the game.
        set_war(&mut state, "fac_a", "fac_b", true);
        tracker.observe(&state, &[]);
        let metrics = tracker.finish(&state);
        assert_eq!((metrics.revolts, metrics.bankruptcies), (2, 1));
        assert_eq!(metrics.factions_eliminated, 1);
        // Wars: 2 turns, then 1 turn (closed by the end): mean 1.5.
        assert_eq!(metrics.war_turns_mean, 1.5);
        assert_eq!(metrics.leader_territory_share, 0.0);
    }

    #[test]
    fn no_war_means_zero() {
        let state = world();
        let mut tracker = MetricsTracker::new(&state);
        tracker.observe(&state, &[]);
        assert_eq!(tracker.finish(&state).war_turns_mean, 0.0);
    }

    #[test]
    fn leader_share_on_the_real_start() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let (data, _) = data_model::GameData::load(&root).expect("data");
        let state =
            CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1).unwrap();
        let share = leader_territory_share(&state);
        assert!(share > 0.0 && share < 1.0, "{share}");
    }

    fn game(revolts: u32, war: f64, share: f64) -> GameMetrics {
        GameMetrics {
            revolts,
            bankruptcies: 1,
            war_turns_mean: war,
            factions_eliminated: 0,
            leader_territory_share: share,
            ..Default::default()
        }
    }

    #[test]
    fn aggregates_mean_min_max_per_bot() {
        let games = vec![
            ("b".to_owned(), game(2, 4.0, 0.1)),
            ("a".to_owned(), game(1, 0.0, 0.2)),
            ("b".to_owned(), game(6, 8.0, 0.3)),
        ];
        let bots = aggregate(&games);
        assert_eq!(bots.len(), 2);
        assert_eq!(bots[0].bot, "a");
        let revolts = bots[1].metrics["revolts_per_game"];
        assert_eq!(bots[1].games, 2);
        assert_eq!((revolts.mean, revolts.min, revolts.max), (4.0, 2.0, 6.0));
        assert_eq!(bots[1].metrics["war_turns_mean"].mean, 6.0);
        assert_eq!(bots[0].metrics.len(), METRIC_NAMES.len());
    }

    #[test]
    fn values_follow_metric_names() {
        let values = game(3, 2.0, 0.5).values();
        for (index, (name, _)) in values.iter().enumerate() {
            assert_eq!(*name, METRIC_NAMES[index]);
        }
        assert_eq!(values[0].1, 3.0);
    }
}
