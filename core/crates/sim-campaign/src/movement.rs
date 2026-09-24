//! Settlement graph, pathfinding and the movement phase of `end_turn`
//! (lot C4, spec § 4.4).
//!
//! Armies stand on settlements and walk along the edges of
//! `GameData::movement_graph` (the C3 graph, or the fallback graph of
//! `data_model::movement_graph`). Costs and movement points are in
//! kilometres of plain; a season grants `Season::movement_steps` ×
//! `MovementRules::points_per_step` points, so that an army covers about the
//! same distance per season as in v1 (3 province steps). An edge dearer than
//! an army's full allowance costs exactly that allowance (one whole season).
//!
//! Entering a settlement held by an enemy stops the march (siege, or the
//! capture of an unfortified village); entering a settlement holding an
//! enemy army triggers a battle.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use data_model::{FactionId, GameData, ProvinceId, SettlementId, SettlementKind, Terrain};

use crate::battle_auto::{resolve_auto, BattleContext, BattleUnit, Side, Winner};
use crate::dynasty;
use crate::events::{EventKind, GameEvent};
use crate::orders::OrderError;
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, CampaignState};

/// Province steps of a port-to-port crossing (v1 value; the fallback graph
/// reads `MovementRules::sea_crossing_steps`).
pub const SEA_EDGE_COST: u32 = 2;
/// Strength lost (percent) when landing in hostile territory (doubled in winter).
pub const LANDING_LOSS_PERCENT: u32 = 5;
/// Morale lost by every unit landing in hostile territory.
pub const LANDING_MORALE_LOSS: u8 = 10;

/// True when `from` → `to` is a sea crossing rather than a land step.
pub fn is_sea_crossing(data: &GameData, from: &SettlementId, to: &SettlementId) -> bool {
    data.movement_graph
        .edge(from, to)
        .is_some_and(|edge| edge.sea)
}

/// Cost multiplier of a terrain (v1: 2 for mountains and marshes).
pub fn terrain_cost(terrain: Terrain) -> u32 {
    data_model::terrain_cost(terrain) as u32
}

/// Land neighbours of a province (geometry graph first, entity data otherwise).
pub fn land_neighbors<'a>(data: &'a GameData, id: &ProvinceId) -> &'a [ProvinceId] {
    data.province_land_neighbors(id)
}

/// Provinces reachable by sea from `id` (`sea_neighbors` of the geometry).
pub fn sea_neighbors(data: &GameData, id: &ProvinceId) -> Vec<ProvinceId> {
    data.province_sea_neighbors(id)
}

/// Movement points one v1 province step is worth.
pub fn points_per_step(data: &GameData) -> f64 {
    data.movement_rules().points_per_step
}

/// Movement cost from `from` to the adjacent `to`, or `None` when not connected.
pub fn edge_cost(data: &GameData, from: &SettlementId, to: &SettlementId) -> Option<u32> {
    if from == to {
        return None;
    }
    data.movement_graph
        .edge(from, to)
        .map(|edge| (edge.cost.round() as u32).max(1))
}

/// All edges leaving `from`, with their costs, in id order.
pub fn edges(data: &GameData, from: &SettlementId) -> Vec<(SettlementId, u32)> {
    data.movement_graph
        .edges(from)
        .iter()
        .map(|edge| (edge.to.clone(), (edge.cost.round() as u32).max(1)))
        .collect()
}

/// Checks that `path` is a chain of connected settlements starting next to `from`.
pub(crate) fn validate_path(
    data: &GameData,
    from: &SettlementId,
    path: &[SettlementId],
) -> Result<(), OrderError> {
    let mut current = from;
    for next in path {
        if !data.settlements.contains_key(next) {
            return Err(OrderError::UnknownSettlement(next.clone()));
        }
        if edge_cost(data, current, next).is_none() {
            return Err(OrderError::NotAdjacent {
                from: current.to_string(),
                to: next.to_string(),
            });
        }
        current = next;
    }
    Ok(())
}

/// Dijkstra result: cost to reach each settlement and the settlement before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    pub cost: u32,
    pub previous: Option<SettlementId>,
}

/// Shortest paths from `start` for an army of `faction`.
///
/// Expansion stops at settlements holding an army hostile to `faction` or
/// held by a faction at war with it (an army entering them must fight or
/// besiege, and stops). `budget` bounds the total cost when given; `cap`
/// clamps each edge (an army's full seasonal allowance).
pub fn dijkstra(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    start: &SettlementId,
    budget: Option<u32>,
    cap: Option<u32>,
) -> BTreeMap<SettlementId, Reach> {
    let mut best: BTreeMap<SettlementId, Reach> = BTreeMap::new();
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
        if &current != start
            && (state.is_hostile_settlement(faction, &current)
                || !state.hostile_armies_at(faction, &current).is_empty())
        {
            continue;
        }
        for (next, edge) in edges(data, &current) {
            let edge = cap.map_or(edge, |c| edge.min(c.max(1)));
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
    table: &BTreeMap<SettlementId, Reach>,
    target: &SettlementId,
) -> Option<Vec<SettlementId>> {
    let mut path = vec![target.clone()];
    let mut current = table.get(target)?;
    while let Some(previous) = &current.previous {
        path.push(previous.clone());
        current = table.get(previous)?;
    }
    path.pop(); // the start settlement
    path.reverse();
    Some(path)
}

impl CampaignState {
    /// Settlements `army` can reach this turn with its remaining movement points.
    pub fn reachable(&self, data: &GameData, army: &ArmyId) -> BTreeMap<SettlementId, u32> {
        let Some(army) = self.armies.get(army) else {
            return BTreeMap::new();
        };
        let cap = self.army_movement_allowance(data, army);
        dijkstra(
            self,
            data,
            &army.faction,
            &army.location,
            Some(army.movement_points),
            Some(cap),
        )
        .into_iter()
        .filter(|(id, _)| id != &army.location)
        .map(|(id, reach)| (id, reach.cost))
        .collect()
    }

    /// Provinces `army` can reach this turn: the cheapest reachable
    /// settlement of each province other than the army's own.
    pub fn reachable_provinces(&self, data: &GameData, army: &ArmyId) -> BTreeMap<ProvinceId, u32> {
        let own = self
            .armies
            .get(army)
            .and_then(|a| self.settlement_province(&a.location))
            .cloned();
        let mut result: BTreeMap<ProvinceId, u32> = BTreeMap::new();
        for (settlement, cost) in self.reachable(data, army) {
            let Some(province) = self.settlement_province(&settlement) else {
                continue;
            };
            if Some(province) == own.as_ref() {
                continue;
            }
            let entry = result.entry(province.clone()).or_insert(cost);
            *entry = (*entry).min(cost);
        }
        result
    }

    /// Cheapest path for `army` to `target` (may span several turns).
    pub fn find_path(
        &self,
        data: &GameData,
        army: &ArmyId,
        target: &SettlementId,
    ) -> Option<Vec<SettlementId>> {
        let army = self.armies.get(army)?;
        if &army.location == target {
            return Some(Vec::new());
        }
        let cap = self.army_movement_allowance(data, army);
        let table = dijkstra(self, data, &army.faction, &army.location, None, Some(cap));
        path_to(&table, target)
    }

    /// Cheapest path for `army` to the city of `province` (empty when the
    /// army already stands on it).
    pub fn find_path_to_province(
        &self,
        data: &GameData,
        army: &ArmyId,
        province: &ProvinceId,
    ) -> Option<Vec<SettlementId>> {
        let city = self.province_city_id(province)?.clone();
        self.find_path(data, army, &city)
    }
}

/// Phase 2 of `end_turn`: every army advances along its path, one settlement
/// per step, all armies interleaved. Entering a settlement with a hostile
/// army triggers a battle and stops both; entering a settlement held by an
/// enemy stops the march (an unfortified village is taken at once).
pub(crate) fn resolve_movement(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let allowances: BTreeMap<ArmyId, u32> = state
        .armies
        .iter()
        .map(|(id, army)| (id.clone(), state.army_movement_allowance(data, army)))
        .collect();
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
            let cost = allowances
                .get(&id)
                .map_or(cost, |full| cost.min((*full).max(1)));
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
            let next_province = state.settlement_province(&next).cloned();
            if is_sea_crossing(data, &from, &next)
                && next_province
                    .as_ref()
                    .is_some_and(|p| state.is_hostile_territory(&faction, p))
            {
                land_on_hostile_shore(state, &id, next_province.as_ref(), events);
            }

            let hostiles = state.hostile_armies_at(&faction, &next);
            if let Some(defender) = strongest(state, &hostiles) {
                fight(state, data, &id, &defender, &from, events);
                if let Some(army) = state.armies.get_mut(&id) {
                    army.path.clear();
                }
                if let Some(army) = state.armies.get_mut(&defender) {
                    army.path.clear();
                }
                continue;
            }
            if state.is_hostile_settlement(&faction, &next) {
                if let Some(army) = state.armies.get_mut(&id) {
                    army.path.clear();
                }
                if state.settlement_kind(&next) == SettlementKind::Village {
                    crate::siege::enter_village(state, data, &id, events);
                }
            }
        }
        if !progressed {
            break;
        }
    }
}

/// A landing on a hostile shore (M10 balance): the army is spent for the turn
/// and pays in men and morale for the disembarkation.
fn land_on_hostile_shore(
    state: &mut CampaignState,
    army_id: &ArmyId,
    province: Option<&ProvinceId>,
    events: &mut Vec<GameEvent>,
) {
    let winter = state.season == crate::state::Season::Winter;
    let percent = if winter {
        2 * LANDING_LOSS_PERCENT
    } else {
        LANDING_LOSS_PERCENT
    };
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    army.movement_points = 0;
    let mut lost = 0;
    for unit in &mut army.units {
        let casualties = (unit.strength * percent)
            .div_ceil(100)
            .min(unit.strength.saturating_sub(1));
        unit.strength -= casualties;
        unit.morale = unit.morale.saturating_sub(LANDING_MORALE_LOSS);
        lost += casualties;
    }
    let faction = army.faction.clone();
    let mut event = GameEvent::new(
        EventKind::Attrition,
        format!("Débarquement en terre hostile : l'armée {army_id} perd {lost} hommes."),
    )
    .army(army_id)
    .faction(&faction);
    if let Some(province) = province {
        event = event.province(province);
    }
    events.push(event);
}

fn strongest(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    ids.iter()
        .max_by_key(|id| (state.armies[*id].total_strength(), Reverse((*id).clone())))
        .cloned()
}

pub(crate) fn move_general(state: &mut CampaignState, army_id: &ArmyId) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let Some(location) = state.settlement_province(&army.location).cloned() else {
        return;
    };
    if let Some(general) = army.general.clone() {
        if let Some(character) = state.characters.get_mut(&general) {
            character.location = Some(location);
        }
    }
}

/// Pace (per cent) of an army hauling siege engines (F1): trebuchets and
/// bombards slow the march unless `Movement` effects for the `siege` family
/// (field artillery) make up for it.
pub const SIEGE_TRAIN_PACE_PERCENT: f64 = -20.0;

impl CampaignState {
    /// Movement points `army` receives at the start of a turn (F1, lot C4):
    /// the season's steps scaled by the pace of its slowest unit family
    /// (technologies' `Movement` percents, global or per family; siege
    /// trains [`SIEGE_TRAIN_PACE_PERCENT`]), plus the flat `Movement` steps
    /// of its general (admiral, chevauchée) and of the faction's
    /// technologies, the whole times `MovementRules::points_per_step`.
    pub fn army_movement_allowance(&self, data: &GameData, army: &Army) -> u32 {
        let base = f64::from(self.season.movement_steps());
        let tech = research::faction_tech_effects(self, data, &army.faction);
        let general = army
            .general
            .as_ref()
            .map(|id| skills::character_effects(self, data, id))
            .unwrap_or_default();
        let common = tech.movement.percent + general.movement.percent;
        let pace = army
            .units
            .iter()
            .filter_map(|unit| data.unit_types.get(&unit.unit_type))
            .map(|unit_type| {
                let mut percent = common
                    + tech
                        .unit_categories
                        .get(unit_type.category)
                        .movement
                        .percent;
                if unit_type.category == data_model::UnitCategory::Siege {
                    percent += SIEGE_TRAIN_PACE_PERCENT;
                }
                percent
            })
            .fold(common, f64::min);
        let steps = (base * (1.0 + pace / 100.0) + 1e-9).floor()
            + tech.movement.flat
            + general.movement.flat;
        (steps.max(1.0) * points_per_step(data)).round() as u32
    }
}

/// Builds the pure battle description of an army.
pub fn side_from_army(state: &CampaignState, data: &GameData, army: &Army) -> Side {
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
                // M6: technology bonuses of the army's faction, per category.
                let tech = stats.map_or_else(Default::default, |t| {
                    research::tech_unit_bonus(state, data, &army.faction, t.category)
                });
                BattleUnit {
                    strength: unit.strength,
                    max_strength: unit.max_strength,
                    experience: unit.experience,
                    morale: research::boosted(unit.morale, tech.morale, 100),
                    melee: research::boosted(stats.map_or(30, |t| t.stats.melee), tech.melee, 255),
                    ranged: research::boosted(
                        stats.map_or(0, |t| t.stats.ranged),
                        tech.ranged + f64::from(unit.levy_ranged),
                        255,
                    ),
                    // G1: plus the levying province's buildings (armoury, butts).
                    armor: research::boosted(
                        stats.map_or(20, |t| t.stats.armor),
                        tech.armor + f64::from(unit.levy_armor),
                        100,
                    ),
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
        general_intrigue: general_effects.intrigue.apply(0.0),
    }
}

/// A field battle between two armies standing on the same settlement: deferred
/// to the player when interactive battles are on and the player takes part
/// (M7, see `battle_request`), auto-resolved otherwise.
pub(crate) fn fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    attacker_origin: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    if crate::battle_request::defer_player_battle(
        state,
        data,
        attacker_id,
        defender_id,
        attacker_origin,
        events,
    ) {
        return;
    }
    auto_fight(
        state,
        data,
        attacker_id,
        defender_id,
        attacker_origin,
        events,
    );
}

/// Armies fighting on `lead`'s side where it stands (F1): `lead` first, then
/// every other army on the same settlement belonging to `lead`'s faction or to an
/// ally of it, and at war with `enemy` (id order).
pub fn battle_coalition(state: &CampaignState, lead: &ArmyId, enemy: &FactionId) -> Vec<ArmyId> {
    let Some(lead_army) = state.armies.get(lead) else {
        return Vec::new();
    };
    let mut ids = vec![lead.clone()];
    ids.extend(
        state
            .armies
            .iter()
            .filter(|(id, army)| {
                *id != lead
                    && army.location == lead_army.location
                    && state.is_allied(&lead_army.faction, &army.faction)
                    && state.is_at_war(&army.faction, enemy)
            })
            .map(|(id, _)| id.clone()),
    );
    ids
}

/// The army of a coalition whose general commands it (F1): the best
/// general by command skill (ties: the first in coalition order).
pub fn coalition_commander(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    let mut best: Option<(&ArmyId, u8)> = None;
    for id in ids {
        let Some(command) = state
            .armies
            .get(id)
            .and_then(|a| a.general.as_ref())
            .and_then(|g| state.characters.get(g))
            .filter(|c| c.alive)
            .map(|c| c.skills.command)
        else {
            continue;
        };
        if best.is_none_or(|(_, b)| command > b) {
            best = Some((id, command));
        }
    }
    best.map(|(id, _)| id.clone())
}

/// One army standing for a whole coalition (F1): every regiment in
/// coalition order, the commander's general, the lead's faction and the
/// strength-weighted supply. Used for the 3D battle setup and to validate
/// its result.
pub(crate) fn coalition_army(state: &CampaignState, ids: &[ArmyId]) -> Option<Army> {
    let mut combined = state.armies.get(ids.first()?)?.clone();
    combined.general = coalition_commander(state, ids)
        .and_then(|id| state.armies.get(&id))
        .and_then(|a| a.general.clone());
    let mut weighted_supply = f64::from(combined.supply) * f64::from(combined.total_strength());
    let mut strength = f64::from(combined.total_strength());
    for id in &ids[1..] {
        let Some(army) = state.armies.get(id) else {
            continue;
        };
        weighted_supply += f64::from(army.supply) * f64::from(army.total_strength());
        strength += f64::from(army.total_strength());
        combined.units.extend(army.units.iter().cloned());
    }
    if strength > 0.0 {
        combined.supply = (weighted_supply / strength).round().clamp(0.0, 100.0) as u8;
    }
    Some(combined)
}

/// Battle description of a coalition (F1): the regiments of every army
/// (each with its own faction's technologies), the commander's general and
/// the strength-weighted supply.
pub(crate) fn coalition_side(state: &CampaignState, data: &GameData, ids: &[ArmyId]) -> Side {
    let commander = coalition_commander(state, ids);
    let mut side = Side::default();
    let mut weighted_supply = 0.0;
    let mut strength = 0.0;
    for id in ids {
        let Some(army) = state.armies.get(id) else {
            continue;
        };
        let part = side_from_army(state, data, army);
        if commander.as_ref() == Some(id) {
            side.general_command = part.general_command;
            side.general_morale_bonus = part.general_morale_bonus;
            side.general_charge_percent = part.general_charge_percent;
            side.general_ranged_percent = part.general_ranged_percent;
            side.general_defense_percent = part.general_defense_percent;
            side.general_intrigue = part.general_intrigue;
        }
        let men = f64::from(army.total_strength());
        weighted_supply += f64::from(part.supply) * men;
        strength += men;
        side.units.extend(part.units);
    }
    if strength > 0.0 {
        side.supply = (weighted_supply / strength).round().clamp(0.0, 100.0) as u8;
    }
    side
}

/// Auto-resolves a field battle between two armies standing on the same
/// settlement; the allied armies there join either side (F1).
pub(crate) fn auto_fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    attacker_origin: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return;
    };
    let province = state
        .settlement_province(&attacker.location)
        .and_then(|p| data.provinces.get(p));
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
    let attackers = battle_coalition(state, attacker_id, &defender.faction);
    let defenders = battle_coalition(state, defender_id, &attacker.faction);
    let attacker_side = coalition_side(state, data, &attackers);
    let defender_side = coalition_side(state, data, &defenders);
    let result = resolve_auto(&attacker_side, &defender_side, &context, &mut state.rng);
    apply_battle_result(
        state,
        data,
        &attackers,
        &defenders,
        attacker_origin,
        &result,
        events,
    );
}

/// Splits a coalition's outcome into one outcome per army (F1): each army
/// takes the losses of its own regiments; only the commander's general can
/// be captured.
pub(crate) fn split_outcome(
    state: &CampaignState,
    ids: &[ArmyId],
    outcome: &crate::battle_auto::SideOutcome,
) -> Vec<(ArmyId, crate::battle_auto::SideOutcome)> {
    let commander = coalition_commander(state, ids);
    let mut offset = 0;
    ids.iter()
        .filter_map(|id| {
            let count = state.armies.get(id)?.units.len();
            let end = (offset + count).min(outcome.losses.len());
            let losses = outcome.losses[offset.min(end)..end].to_vec();
            offset += count;
            Some((
                id.clone(),
                crate::battle_auto::SideOutcome {
                    power: outcome.power,
                    total_losses: losses.iter().sum(),
                    losses,
                    morale_delta: outcome.morale_delta,
                    routed: outcome.routed,
                    general_captured: outcome.general_captured && commander.as_ref() == Some(id),
                },
            ))
        })
        .collect()
}

/// Applies a battle result (auto-resolved or fought in 3D, M7): journal,
/// losses (spread over each coalition's armies, F1), captures, the
/// commanding generals' XP/traits (M4 hooks) and the losers' retreat.
/// `attackers` / `defenders` start with the two armies of the encounter.
pub(crate) fn apply_battle_result(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    defenders: &[ArmyId],
    attacker_origin: &SettlementId,
    result: &crate::battle_auto::BattleResult,
    events: &mut Vec<GameEvent>,
) {
    let (Some(attacker_id), Some(defender_id)) = (attackers.first(), defenders.first()) else {
        return;
    };
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return;
    };
    let Some(province_id) = state.settlement_province(&attacker.location).cloned() else {
        return;
    };
    let province = data.provinces.get(&province_id);
    let attacker_faction = attacker.faction.clone();
    let defender_faction = defender.faction.clone();
    let general_of = |state: &CampaignState, ids: &[ArmyId]| {
        coalition_commander(state, ids)
            .and_then(|id| state.armies.get(&id))
            .and_then(|a| a.general.clone())
    };
    let attacker_general = general_of(state, attackers);
    let defender_general = general_of(state, defenders);

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
    let allies_note = |ids: &[ArmyId]| {
        if ids.len() > 1 {
            format!(" (+{} armée(s) alliée(s))", ids.len() - 1)
        } else {
            String::new()
        }
    };
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Bataille {} : {}{} attaque {}{}. Vainqueur : {}. Pertes : {} contre {}.",
                crate::events::de(&province_name),
                faction_name(&attacker_faction),
                allies_note(attackers),
                faction_name(&defender_faction),
                allies_note(defenders),
                faction_name(winner_faction),
                result.attacker.total_losses,
                result.defender.total_losses
            ),
        )
        .province(&province_id)
        .army(attacker_id)
        .faction(winner_faction),
    );

    let attacker_parts = split_outcome(state, attackers, &result.attacker);
    let defender_parts = split_outcome(state, defenders, &result.defender);
    for (id, outcome) in attacker_parts.iter().chain(defender_parts.iter()) {
        apply_outcome(state, data, id, outcome, events);
    }
    // F1: a captured commander is held by the victor.
    for (general, captor) in [
        (&attacker_general, &defender_faction),
        (&defender_general, &attacker_faction),
    ] {
        if let Some(c) = general
            .as_ref()
            .and_then(|g| state.characters.get_mut(g))
            .filter(|c| c.captive && c.captor.is_none())
        {
            c.captor = Some(captor.clone());
        }
    }
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
    // commanding generals (only if they weren't captured, which already
    // removed them from command).
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

    let losers = match result.winner {
        Winner::Attacker => defenders,
        Winner::Defender => attackers,
    };
    for loser_id in losers {
        let retreat_to = if result.winner == Winner::Defender && loser_id == attacker_id {
            Some(attacker_origin.clone())
        } else {
            retreat_settlement(state, data, loser_id)
        };
        if let (Some(army), Some(target)) = (state.armies.get_mut(loser_id), retreat_to) {
            army.location = target;
            army.movement_points = 0;
            army.path.clear();
            move_general(state, loser_id);
        }
    }
}

/// Where a beaten army falls back: the first neighbouring settlement held
/// by a friend and free of enemy armies.
fn retreat_settlement(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
) -> Option<SettlementId> {
    let army = state.armies.get(army_id)?;
    edges(data, &army.location)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| state.hostile_armies_at(&army.faction, id).is_empty())
        .find(|id| state.is_friendly_settlement(&army.faction, id))
}

pub(crate) fn apply_outcome(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    outcome: &crate::battle_auto::SideOutcome,
    events: &mut Vec<GameEvent>,
) {
    // H4: barber-surgeons tend the wounded of the surviving units.
    let recovery = state.armies.get(army_id).map_or(0.0, |a| {
        crate::medicine::wound_recovery(state, data, &a.faction)
    });
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    let mut tended = 0;
    for (unit, losses) in army.units.iter_mut().zip(&outcome.losses) {
        let lost = (*losses).min(unit.strength);
        unit.strength -= lost;
        let survives = unit.strength > 0 && unit.strength * 20 >= unit.max_strength;
        if survives && recovery > 0.0 {
            let wounded = crate::medicine::recovered_wounded(lost, recovery);
            unit.strength = (unit.strength + wounded).min(unit.max_strength);
            tended += wounded;
        }
        unit.morale = (i32::from(unit.morale) + outcome.morale_delta).clamp(0, 100) as u8;
        if outcome.morale_delta > 0 {
            unit.experience = (unit.experience + 1).min(10);
        }
    }
    army.units
        .retain(|unit| unit.strength > 0 && unit.strength * 20 >= unit.max_strength);
    let settlement = army.location.clone();
    let faction = army.faction.clone();
    let location = state
        .settlements
        .get(&settlement)
        .map_or_else(province_placeholder, |s| s.province.clone());
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    if tended > 0 && faction == state.player_faction {
        events.push(
            GameEvent::new(
                EventKind::Medicine,
                format!("{tended} blessés soignés rejoignent les rangs de l'armée {army_id}."),
            )
            .province(&location)
            .army(army_id)
            .faction(&faction),
        );
    }
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

/// Province used in events when a settlement is unknown (never in practice).
fn province_placeholder() -> ProvinceId {
    ProvinceId::new("prov_unknown").expect("well-formed id")
}
