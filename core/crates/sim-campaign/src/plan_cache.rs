//! [`PlanCache`]: read-only answers over a [`CampaignState`] shared while the
//! AI plans a faction turn.
//!
//! The planner reads one state for a whole turn and asks the same questions
//! many times (the military weight of a faction, whether two factions are
//! neighbours, a faction's income or rivals), each an O(armies + settlements)
//! or O(provinces) walk. The cache borrows the state, so it can never serve a
//! stale answer, and computes each answer lazily, once. Its answers are those
//! of the pure walks on [`CampaignState`] (`faction_power`, `are_neighbors`,
//! `neighbour_factions`, `faction_income`, [`crate::diplomacy::rivals`]), so
//! no decision changes. Callers outside the AI use those walks directly.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use data_model::{FactionId, GameData};

use crate::state::CampaignState;

/// Lazily computed answers over one borrowed state.
pub struct PlanCache<'a> {
    state: &'a CampaignState,
    /// `(armies, garrisons)` strength per faction, summed like the plain walk
    /// (`u32`, then `f64`).
    power: OnceLock<BTreeMap<FactionId, (u32, u32)>>,
    /// Neighbouring factions of every faction, built with the data of the
    /// first question (`data` address); other data is answered by the walk.
    neighbours: OnceLock<(usize, BTreeMap<FactionId, BTreeSet<FactionId>>)>,
    /// Income per `(data address, faction)` asked so far.
    income: Mutex<BTreeMap<(usize, FactionId), i64>>,
    /// Rivals per faction asked so far.
    rivals: Mutex<BTreeMap<FactionId, BTreeSet<FactionId>>>,
}

/// `map[key]`, computed by `compute` (outside the lock) when missing.
fn memo<K: Ord, V: Clone>(map: &Mutex<BTreeMap<K, V>>, key: K, compute: impl FnOnce() -> V) -> V {
    let known = map
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .cloned();
    known.unwrap_or_else(|| {
        let value = compute();
        map.lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, value.clone());
        value
    })
}

fn address(data: &GameData) -> usize {
    std::ptr::from_ref(data) as usize
}

impl<'a> PlanCache<'a> {
    pub fn new(state: &'a CampaignState) -> Self {
        Self {
            state,
            power: OnceLock::new(),
            neighbours: OnceLock::new(),
            income: Mutex::new(BTreeMap::new()),
            rivals: Mutex::new(BTreeMap::new()),
        }
    }

    /// The state the answers are about.
    pub fn state(&self) -> &'a CampaignState {
        self.state
    }

    /// [`CampaignState::faction_power`].
    pub fn faction_power(&self, faction: &FactionId) -> f64 {
        let power = self.power.get_or_init(|| {
            let mut power: BTreeMap<FactionId, (u32, u32)> = BTreeMap::new();
            for army in self.state.armies.values() {
                let strength: u32 = army.units.iter().map(|u| u.strength).sum();
                power.entry(army.faction.clone()).or_default().0 += strength;
            }
            for settlement in self.state.settlements.values() {
                power.entry(settlement.controller.clone()).or_default().1 +=
                    settlement.garrison_strength();
            }
            power
        });
        let (armies, garrisons) = power.get(faction).copied().unwrap_or_default();
        f64::from(armies) + f64::from(garrisons) / 2.0
    }

    /// Power of `faction` plus that of its coalition (see
    /// [`CampaignState::coalition_power`]).
    pub fn coalition_power(&self, faction: &FactionId) -> f64 {
        self.faction_power(faction)
            + self
                .state
                .coalition_members(faction)
                .iter()
                .map(|a| self.faction_power(a))
                .sum::<f64>()
    }

    fn neighbour_index(
        &self,
        data: &GameData,
    ) -> Option<&BTreeMap<FactionId, BTreeSet<FactionId>>> {
        let key = address(data);
        let (built_for, index) = self
            .neighbours
            .get_or_init(|| (key, neighbour_index(self.state, data)));
        (*built_for == key).then_some(index)
    }

    /// [`CampaignState::are_neighbors`].
    pub fn are_neighbors(&self, data: &GameData, a: &FactionId, b: &FactionId) -> bool {
        match self.neighbour_index(data) {
            Some(index) => index.get(a).is_some_and(|set| set.contains(b)),
            None => self.state.are_neighbors(data, a, b),
        }
    }

    /// [`CampaignState::neighbour_factions`].
    pub fn neighbour_factions(&self, data: &GameData, a: &FactionId) -> BTreeSet<FactionId> {
        match self.neighbour_index(data) {
            Some(index) => index.get(a).cloned().unwrap_or_default(),
            None => self.state.neighbour_factions(data, a),
        }
    }

    /// [`CampaignState::faction_income`].
    pub fn faction_income(&self, data: &GameData, faction: &FactionId) -> i64 {
        memo(&self.income, (address(data), faction.clone()), || {
            self.state.faction_income(data, faction)
        })
    }

    /// [`crate::diplomacy::rivals`].
    pub fn rivals(&self, faction: &FactionId) -> BTreeSet<FactionId> {
        memo(&self.rivals, faction.clone(), || {
            crate::diplomacy::rivals(self.state, faction)
        })
    }
}

/// [`CampaignState::neighbour_factions`] of every faction in one pass.
fn neighbour_index(
    state: &CampaignState,
    data: &GameData,
) -> BTreeMap<FactionId, BTreeSet<FactionId>> {
    let mut out: BTreeMap<FactionId, BTreeSet<FactionId>> = BTreeMap::new();
    for (id, province) in &state.provinces {
        let Some(city) = state.settlements.get(&province.city) else {
            continue;
        };
        let set = out.entry(city.controller.clone()).or_default();
        for n in crate::movement::land_neighbors(data, id) {
            if let Some(controller) = state.province_controller(n) {
                if !set.contains(controller) {
                    set.insert(controller.clone());
                }
            }
        }
    }
    out
}
