//! OMR R1: read-only indexes over a [`CampaignState`] shared while a
//! planning scope holds the state borrowed.
//!
//! The AI planner reads the same state for a whole faction turn and asks
//! the same questions many times (the military weight of a faction, whether
//! two factions are neighbours), each an O(armies + settlements) or
//! O(provinces) walk. [`CampaignState::planning_scope`] computes their
//! answers once and registers them under the state's address; the accessors
//! ([`CampaignState::faction_power`], [`CampaignState::are_neighbors`], …)
//! read them while the scope lives, and walk the state as before otherwise.
//!
//! Soundness: the guard borrows the state, so the state can neither change
//! nor move while its entry is registered (the state has no interior
//! mutability), and the entry is removed when the last guard for that
//! address drops. A clone lives at another address and is never served
//! these answers. The answers are those of the plain walks (equality tests
//! `omr_r1_planning_scope.rs`), so no decision changes.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use data_model::{FactionId, GameData};

use crate::state::CampaignState;

/// Answers computed once per scope.
pub(crate) struct Derived {
    /// [`CampaignState::faction_power`] parts per faction: field armies,
    /// garrisons (summed like the plain walk: `u32`, then `f64`).
    power: BTreeMap<FactionId, (u32, u32)>,
    /// [`CampaignState::neighbour_factions`] of every faction, built on the
    /// first question with the data it was asked with (`data` address).
    neighbours: OnceLock<(usize, BTreeMap<FactionId, BTreeSet<FactionId>>)>,
}

impl Derived {
    fn new(state: &CampaignState) -> Self {
        let mut power: BTreeMap<FactionId, (u32, u32)> = BTreeMap::new();
        for army in state.armies.values() {
            let strength: u32 = army.units.iter().map(|u| u.strength).sum();
            let entry = power.entry(army.faction.clone()).or_default();
            entry.0 += strength;
        }
        for settlement in state.settlements.values() {
            let entry = power.entry(settlement.controller.clone()).or_default();
            entry.1 += settlement.garrison_strength();
        }
        Self {
            power,
            neighbours: OnceLock::new(),
        }
    }

    pub(crate) fn faction_power(&self, faction: &FactionId) -> f64 {
        let (armies, garrisons) = self.power.get(faction).copied().unwrap_or_default();
        f64::from(armies) + f64::from(garrisons) / 2.0
    }

    /// Neighbours of every faction, or `None` when the index was built with
    /// other data.
    pub(crate) fn neighbours(
        &self,
        state: &CampaignState,
        data: &GameData,
    ) -> Option<&BTreeMap<FactionId, BTreeSet<FactionId>>> {
        let key = std::ptr::from_ref(data) as usize;
        let (built_for, map) = self
            .neighbours
            .get_or_init(|| (key, neighbour_index(state, data)));
        (*built_for == key).then_some(map)
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

struct Entry {
    address: usize,
    guards: usize,
    derived: Arc<Derived>,
}

/// Registered scopes (a handful at most: one per state being planned).
static SCOPES: RwLock<Vec<Entry>> = RwLock::new(Vec::new());
/// Number of registered entries: the accessors skip the lock when zero.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

fn address(state: &CampaignState) -> usize {
    std::ptr::from_ref(state) as usize
}

/// Guard of a planning scope (see the module documentation).
#[must_use = "the scope ends when the guard drops"]
pub struct PlanningScope<'a> {
    state: &'a CampaignState,
}

impl Drop for PlanningScope<'_> {
    fn drop(&mut self) {
        let key = address(self.state);
        let mut scopes = SCOPES.write().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = scopes.iter().position(|e| e.address == key) {
            scopes[i].guards -= 1;
            if scopes[i].guards == 0 {
                scopes.swap_remove(i);
                ACTIVE.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
}

impl CampaignState {
    /// Opens a planning scope on this state (see [`crate::planning_scope`]):
    /// until the guard drops, repeated read-only questions are answered from
    /// indexes built once. Scopes nest.
    pub fn planning_scope(&self) -> PlanningScope<'_> {
        let key = address(self);
        {
            let mut scopes = SCOPES.write().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = scopes.iter_mut().find(|e| e.address == key) {
                entry.guards += 1;
                return PlanningScope { state: self };
            }
        }
        // Built outside the lock (other states' planners keep reading).
        let derived = Arc::new(Derived::new(self));
        let mut scopes = SCOPES.write().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = scopes.iter_mut().find(|e| e.address == key) {
            entry.guards += 1;
        } else {
            scopes.push(Entry {
                address: key,
                guards: 1,
                derived,
            });
            ACTIVE.fetch_add(1, Ordering::SeqCst);
        }
        PlanningScope { state: self }
    }

    /// The indexes of the scope open on this state, if any.
    pub(crate) fn derived(&self) -> Option<Arc<Derived>> {
        if ACTIVE.load(Ordering::Relaxed) == 0 {
            return None;
        }
        let key = address(self);
        let scopes = SCOPES.read().unwrap_or_else(|e| e.into_inner());
        scopes
            .iter()
            .find(|e| e.address == key)
            .map(|e| Arc::clone(&e.derived))
    }
}
