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
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, CampaignState};

/// Movement points spent on a port-to-port crossing.
pub const SEA_EDGE_COST: u32 = 2;
/// Strength lost (percent) when landing in hostile territory (doubled in winter).
pub const LANDING_LOSS_PERCENT: u32 = 5;
/// Morale lost by every unit landing in hostile territory.
pub const LANDING_MORALE_LOSS: u8 = 10;

/// True when `from` → `to` is a sea crossing rather than a land step.
pub fn is_sea_crossing(data: &GameData, from: &ProvinceId, to: &ProvinceId) -> bool {
    !land_neighbors(data, from).contains(to) && sea_neighbors(data, from).contains(to)
}

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
            if is_sea_crossing(data, &from, &next) && state.is_hostile_territory(&faction, &next) {
                land_on_hostile_shore(state, &id, &next, events);
            }

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

/// A landing on a hostile shore (M10 balance): the army is spent for the turn
/// and pays in men and morale for the disembarkation.
fn land_on_hostile_shore(
    state: &mut CampaignState,
    army_id: &ArmyId,
    province: &ProvinceId,
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
    events.push(
        GameEvent::new(
            EventKind::Attrition,
            format!("Débarquement en terre hostile : l'armée {army_id} perd {lost} hommes."),
        )
        .army(army_id)
        .province(province)
        .faction(&faction),
    );
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

/// Pace (per cent) of an army hauling siege engines (F1): trebuchets and
/// bombards slow the march unless `Movement` effects for the `siege` family
/// (field artillery) make up for it.
pub const SIEGE_TRAIN_PACE_PERCENT: f64 = -20.0;

impl CampaignState {
    /// Movement points `army` receives at the start of a turn (F1): the
    /// season's allowance scaled by the pace of its slowest unit family
    /// (technologies' `Movement` percents, global or per family; siege
    /// trains [`SIEGE_TRAIN_PACE_PERCENT`]), plus the flat `Movement` of its
    /// general (admiral, chevauchée) and of the faction's technologies.
    pub fn army_movement_allowance(&self, data: &GameData, army: &Army) -> u32 {
        let base = f64::from(self.season.movement_points());
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
        let points = (base * (1.0 + pace / 100.0) + 1e-9).floor()
            + tech.movement.flat
            + general.movement.flat;
        points.max(1.0) as u32
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

/// A field battle between two armies standing in the same province: deferred
/// to the player when interactive battles are on and the player takes part
/// (M7, see `battle_request`), auto-resolved otherwise.
pub(crate) fn fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    attacker_origin: &ProvinceId,
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

/// Armies fighting on `lead`'s side in its province (F1): `lead` first, then
/// every other army of the province belonging to `lead`'s faction or to an
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

/// Auto-resolves a field battle between two armies standing in the same
/// province; the allied armies of the province join either side (F1).
pub(crate) fn auto_fight(
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
    attacker_origin: &ProvinceId,
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
    let province_id = attacker.location.clone();
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
            retreat_province(state, data, loser_id)
        };
        if let (Some(army), Some(target)) = (state.armies.get_mut(loser_id), retreat_to) {
            army.location = target;
            army.movement_points = 0;
            army.path.clear();
            move_general(state, loser_id);
        }
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
    let location = army.location.clone();
    let faction = army.faction.clone();
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
