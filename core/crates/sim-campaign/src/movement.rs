//! Province graph, pathfinding and the movement phase of `end_turn`.
//!
//! Land edges come from `GameData::province_geometry[..].neighbors` (falling
//! back to `Province::neighbors` when no polygon exists); sea edges from the
//! `sea_neighbors` property of the geometry, usable only when both provinces
//! have a port. Entering a province costs 1 movement point (2 for mountains
//! and marshes), a sea crossing costs [`SEA_EDGE_COST`].

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use data_model::{FactionId, GameData, ProvinceId, Terrain};

use crate::battle_auto::{resolve_auto, BattleContext, BattleUnit, Side, Winner};
use crate::dynasty;
use crate::events::{EventKind, GameEvent};
use crate::orders::OrderError;
use crate::skills;
use crate::state::{Army, ArmyId, CampaignState};

/// Movement points spent on a port-to-port crossing.
pub const SEA_EDGE_COST: u32 = 2;

/// Cost of entering a province by land.
pub fn terrain_cost(terrain: Terrain) -> u32 {
    match terrain {
        Terrain::Mountains | Terrain::Marsh => 2,
        _ => 1,
    }
}

/// Land neighbours of a province (geometry graph first, entity data otherwise).
pub fn land_neighbors<'a>(data: &'a GameData, id: &ProvinceId) -> &'a [ProvinceId] {
    if let Some(geometry) = data.province_geometry.get(id) {
        return &geometry.neighbors;
    }
    data.provinces
        .get(id)
        .map_or(&[], |province| province.neighbors.as_slice())
}

/// Provinces reachable by sea from `id` (ignoring ports; see [`edge_cost`]).
pub fn sea_neighbors(data: &GameData, id: &ProvinceId) -> Vec<ProvinceId> {
    data.province_geometry
        .get(id)
        .and_then(|geometry| geometry.extra.get("sea_neighbors"))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .filter_map(|raw| ProvinceId::new(raw).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Movement cost from `from` to the adjacent `to`, or `None` when not connected.
pub fn edge_cost(data: &GameData, from: &ProvinceId, to: &ProvinceId) -> Option<u32> {
    if from == to {
        return None;
    }
    let to_province = data.provinces.get(to)?;
    if land_neighbors(data, from).contains(to) {
        return Some(terrain_cost(to_province.terrain));
    }
    let from_province = data.provinces.get(from)?;
    if from_province.has_port() && to_province.has_port() && sea_neighbors(data, from).contains(to)
    {
        return Some(SEA_EDGE_COST);
    }
    None
}

/// All edges leaving `from`, with their costs, in deterministic order.
pub fn edges(data: &GameData, from: &ProvinceId) -> Vec<(ProvinceId, u32)> {
    let mut result: Vec<(ProvinceId, u32)> = land_neighbors(data, from)
        .iter()
        .chain(sea_neighbors(data, from).iter())
        .filter_map(|to| edge_cost(data, from, to).map(|cost| (to.clone(), cost)))
        .collect();
    result.sort();
    result.dedup();
    result
}

/// Checks that `path` is a chain of connected provinces starting next to `from`.
pub(crate) fn validate_path(
    data: &GameData,
    from: &ProvinceId,
    path: &[ProvinceId],
) -> Result<(), OrderError> {
    let mut current = from;
    for next in path {
        if !data.provinces.contains_key(next) {
            return Err(OrderError::UnknownProvince(next.clone()));
        }
        if edge_cost(data, current, next).is_none() {
            return Err(OrderError::NotAdjacent {
                from: current.clone(),
                to: next.clone(),
            });
        }
        current = next;
    }
    Ok(())
}

/// Dijkstra result: cost to reach each province and the province before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    pub cost: u32,
    pub previous: Option<ProvinceId>,
}

/// Shortest paths from `start` for an army of `faction`.
///
/// Expansion stops at provinces holding an army hostile to `faction` (an army
/// entering such a province must fight and stop). `budget` bounds the total
/// cost when given.
pub fn dijkstra(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    start: &ProvinceId,
    budget: Option<u32>,
) -> BTreeMap<ProvinceId, Reach> {
    let mut best: BTreeMap<ProvinceId, Reach> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(
        start.clone(),
        Reach {
            cost: 0,
            previous: None,
        },
    );
    heap.push(Reverse((0u32, start.clone())));
    while let Some(Reverse((cost, current))) = heap.pop() {
        if best.get(&current).is_some_and(|r| r.cost < cost) {
            continue;
        }
        if &current != start && !state.hostile_armies_in(faction, &current).is_empty() {
            continue;
        }
        for (next, edge) in edges(data, &current) {
            let total = cost + edge;
            if budget.is_some_and(|b| total > b) {
                continue;
            }
            let better = best.get(&next).is_none_or(|r| total < r.cost);
            if better {
                best.insert(
                    next.clone(),
                    Reach {
                        cost: total,
                        previous: Some(current.clone()),
                    },
                );
                heap.push(Reverse((total, next)));
            }
        }
    }
    best
}

/// Rebuilds the path (start excluded, target included) from a Dijkstra table.
pub fn path_to(
    table: &BTreeMap<ProvinceId, Reach>,
    target: &ProvinceId,
) -> Option<Vec<ProvinceId>> {
    let mut path = vec![target.clone()];
    let mut current = table.get(target)?;
    while let Some(previous) = &current.previous {
        path.push(previous.clone());
        current = table.get(previous)?;
    }
    path.pop(); // the start province
    path.reverse();
    Some(path)
}

impl CampaignState {
    /// Provinces `army` can reach this turn with its remaining movement points.
    pub fn reachable(&self, data: &GameData, army: &ArmyId) -> BTreeMap<ProvinceId, u32> {
        let Some(army) = self.armies.get(army) else {
            return BTreeMap::new();
        };
        dijkstra(
            self,
            data,
            &army.faction,
            &army.location,
            Some(army.movement_points),
        )
        .into_iter()
        .filter(|(id, _)| id != &army.location)
        .map(|(id, reach)| (id, reach.cost))
        .collect()
    }

    /// Cheapest path for `army` to `target` (may span several turns).
    pub fn find_path(
        &self,
        data: &GameData,
        army: &ArmyId,
        target: &ProvinceId,
    ) -> Option<Vec<ProvinceId>> {
        let army = self.armies.get(army)?;
        if &army.location == target {
            return Some(Vec::new());
        }
        let table = dijkstra(self, data, &army.faction, &army.location, None);
        path_to(&table, target)
    }
}

/// Phase 2 of `end_turn`: every army advances along its path, one province per
/// step, all armies interleaved; entering a province with a hostile army
/// triggers an auto-resolved battle and stops both.
pub(crate) fn resolve_movement(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    loop {
        let mut progressed = false;
        let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
        for id in ids {
            let Some(army) = state.armies.get(&id) else {
                continue;
            };
            let Some(next) = army.path.first().cloned() else {
                continue;
            };
            let from = army.location.clone();
            let faction = army.faction.clone();
            let Some(cost) = edge_cost(data, &from, &next) else {
                state.armies.get_mut(&id).expect("exists").path.clear();
                continue;
            };
            if cost > army.movement_points {
                continue;
            }
            {
                let army = state.armies.get_mut(&id).expect("exists");
                army.location = next.clone();
                army.movement_points -= cost;
                army.path.remove(0);
            }
            move_general(state, &id);
            progressed = true;

            let hostiles = state.hostile_armies_in(&faction, &next);
            if let Some(defender) = strongest(state, &hostiles) {
                fight(state, data, &id, &defender, &from, events);
                if let Some(army) = state.armies.get_mut(&id) {
                    army.path.clear();
                }
                if let Some(army) = state.armies.get_mut(&defender) {
                    army.path.clear();
                }
            }
        }
        if !progressed {
            break;
        }
    }
}

fn strongest(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    ids.iter()
        .max_by_key(|id| (state.armies[*id].total_strength(), Reverse((*id).clone())))
        .cloned()
}

fn move_general(state: &mut CampaignState, army_id: &ArmyId) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let location = army.location.clone();
    if let Some(general) = army.general.clone() {
        if let Some(character) = state.characters.get_mut(&general) {
            character.location = Some(location);
        }
    }
}

/// Builds the pure battle description of an army.
pub(crate) fn side_from_army(state: &CampaignState, data: &GameData, army: &Army) -> Side {
    let general_command = army
        .general
        .as_ref()
        .and_then(|id| state.characters.get(id))
        .map_or(0, |c| c.skills.command);
    let general_effects = army
        .general
        .as_ref()
        .map(|id| skills::character_effects(state, data, id))
        .unwrap_or_default();
    Side {
        units: army
            .units
            .iter()
            .map(|unit| {
                let stats = data.unit_types.get(&unit.unit_type);
                BattleUnit {
                    strength: unit.strength,
                    max_strength: unit.max_strength,
                    experience: unit.experience,
                    morale: unit.morale,
                    melee: stats.map_or(30, |t| t.stats.melee),
                    ranged: stats.map_or(0, |t| t.stats.ranged),
                    armor: stats.map_or(20, |t| t.stats.armor),
                    is_ranged: stats.is_some_and(|t| {
                        matches!(
                            t.category,
                            data_model::UnitCategory::Ranged | data_model::UnitCategory::Siege
                        )
                    }),
                }
            })
            .collect(),
        general_command,
        supply: army.supply,
        general_morale_bonus: general_effects.army_morale.apply(0.0),
        general_charge_percent: general_effects.battle_charge.apply(0.0),
        general_ranged_percent: general_effects.battle_ranged.apply(0.0),
        general_defense_percent: general_effects.battle_defense.apply(0.0),
    }
}

/// Auto-resolves a field battle between two armies standing in the same province.
pub(crate) fn fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    attacker_origin: &ProvinceId,
    events: &mut Vec<GameEvent>,
) {
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return;
    };
    let province_id = attacker.location.clone();
    let province = data.provinces.get(&province_id);
    let context = BattleContext {
        defender_terrain_bonus: province.is_some_and(|p| {
            matches!(
                p.terrain,
                Terrain::Hills | Terrain::Forest | Terrain::Mountains
            )
        }),
        river_crossing: province.is_some_and(|p| !p.rivers.is_empty()),
        walls: false,
    };
    let attacker_side = side_from_army(state, data, attacker);
    let defender_side = side_from_army(state, data, defender);
    let attacker_faction = attacker.faction.clone();
    let defender_faction = defender.faction.clone();
    let attacker_general = attacker.general.clone();
    let defender_general = defender.general.clone();
    let result = resolve_auto(&attacker_side, &defender_side, &context, &mut state.rng);

    let province_name =
        province.map_or_else(|| province_id.to_string(), |p| p.name.display.clone());
    let faction_name = |id: &FactionId| -> String {
        data.factions
            .get(id)
            .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
    };
    let winner_faction = match result.winner {
        Winner::Attacker => &attacker_faction,
        Winner::Defender => &defender_faction,
    };
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Bataille de {province_name} : {} attaque {}. Vainqueur : {}. Pertes : {} contre {}.",
                faction_name(&attacker_faction),
                faction_name(&defender_faction),
                faction_name(winner_faction),
                result.attacker.total_losses,
                result.defender.total_losses
            ),
        )
        .province(&province_id)
        .army(attacker_id)
        .faction(winner_faction),
    );

    apply_outcome(state, data, attacker_id, &result.attacker, events);
    apply_outcome(state, data, defender_id, &result.defender, events);
    // M5 war score: a lopsided battle counts double.
    let (winner, loser, winner_losses, loser_losses) = match result.winner {
        Winner::Attacker => (
            &attacker_faction,
            &defender_faction,
            result.attacker.total_losses,
            result.defender.total_losses,
        ),
        Winner::Defender => (
            &defender_faction,
            &attacker_faction,
            result.defender.total_losses,
            result.attacker.total_losses,
        ),
    };
    state.record_battle(winner, loser, loser_losses > 2 * winner_losses.max(1));

    // Spec § 2: XP, `trait_veteran`, wounded and death chance for both
    // generals (only if they weren't captured, which already removed them
    // from command).
    if let Some(general) = &attacker_general {
        if state.characters.get(general).is_some_and(|c| c.alive) {
            dynasty::on_battle_resolved(
                state,
                data,
                general,
                result.winner == Winner::Attacker,
                events,
            );
        }
    }
    if let Some(general) = &defender_general {
        if state.characters.get(general).is_some_and(|c| c.alive) {
            dynasty::on_battle_resolved(
                state,
                data,
                general,
                result.winner == Winner::Defender,
                events,
            );
        }
    }

    let (loser_id, retreat_to) = match result.winner {
        Winner::Attacker => (defender_id, retreat_province(state, data, defender_id)),
        Winner::Defender => (attacker_id, Some(attacker_origin.clone())),
    };
    if let (Some(army), Some(target)) = (state.armies.get_mut(loser_id), retreat_to) {
        army.location = target;
        army.movement_points = 0;
        army.path.clear();
        move_general(state, loser_id);
    }
}

fn retreat_province(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
) -> Option<ProvinceId> {
    let army = state.armies.get(army_id)?;
    edges(data, &army.location)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| state.hostile_armies_in(&army.faction, id).is_empty())
        .find(|id| state.is_friendly_territory(&army.faction, id))
}

fn apply_outcome(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    outcome: &crate::battle_auto::SideOutcome,
    events: &mut Vec<GameEvent>,
) {
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    for (unit, losses) in army.units.iter_mut().zip(&outcome.losses) {
        unit.strength = unit.strength.saturating_sub(*losses);
        unit.morale = (i32::from(unit.morale) + outcome.morale_delta).clamp(0, 100) as u8;
        if outcome.morale_delta > 0 {
            unit.experience = (unit.experience + 1).min(10);
        }
    }
    army.units
        .retain(|unit| unit.strength > 0 && unit.strength * 20 >= unit.max_strength);
    let location = army.location.clone();
    let faction = army.faction.clone();
    let general = army.general.clone();
    let destroyed = army.units.is_empty();

    if outcome.general_captured {
        if let Some(general) = &general {
            state.detach_general(general);
            if let Some(character) = state.characters.get_mut(general) {
                character.captive = true;
            }
            let name = state.character_name(data, general);
            events.push(
                GameEvent::new(
                    EventKind::GeneralCaptured,
                    format!("{name} est fait prisonnier."),
                )
                .province(&location)
                .army(army_id)
                .faction(&faction),
            );
        }
    }
    if destroyed {
        if let Some(general) = &general {
            state.detach_general(general);
        }
        state.armies.remove(army_id);
        events.push(
            GameEvent::new(
                EventKind::ArmyDestroyed,
                format!("L'armée {army_id} est anéantie."),
            )
            .province(&location)
            .army(army_id)
            .faction(&faction),
        );
    }
}
