//! Settlement graph, movement allowance, field battles and the retreat of
//! the beaten.
//!
//! Lot M2 (`docs/design/2026-09-24-mouvement-libre.md`): armies march
//! freely on the navigation grid (`march.rs`, `navigation.rs`). The
//! settlement graph (`GameData::movement_graph`, lot C3/C4) remains the
//! skeleton of the AI's strategic planning, of the agents' walks and of the
//! sea crossings (port to port). Allowances are computed here in
//! kilometres of plain (`season_points` of C7a) and converted to grid
//! costs by `CampaignState::army_grid_allowance`.
//!
//! Battles are fought at once when an army attacks another within
//! `engage_radius_km`; the loser falls back on the grid (§ 3.3).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use data_model::{FactionId, GameData, ProvinceId, SettlementId, Terrain};

use crate::battle_auto::{resolve_field, BattleContext, BattleUnit, Side, Winner};
use crate::dynasty;
use crate::events::{EventKind, GameEvent};
use crate::march::{km_to_grid_points, px_per_km};
use crate::navigation::{self, Cell};
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState};

/// Province steps of a port-to-port crossing (v1 value; the fallback graph
/// reads `MovementRules::sea_crossing_steps`).
pub const SEA_EDGE_COST: u32 = 2;
/// Strength lost (percent) when landing in hostile territory (doubled in winter).
pub const LANDING_LOSS_PERCENT: u32 = 5;
/// Winter multiplier of [`LANDING_LOSS_PERCENT`].
pub const LANDING_WINTER_FACTOR: u32 = 2;
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

/// A landing on a hostile shore (M10 balance): the army is spent for the turn
/// and pays in men and morale for the disembarkation.
pub(crate) fn land_on_hostile_shore(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    province: Option<&ProvinceId>,
    events: &mut Vec<GameEvent>,
) {
    let winter = state.season == crate::state::Season::Winter;
    let percent = if winter {
        LANDING_WINTER_FACTOR * LANDING_LOSS_PERCENT
    } else {
        LANDING_LOSS_PERCENT
    };
    let name = state.army_name(data, army_id);
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    army.movement_left = 0;
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
        format!("Débarquement en terre hostile : {name} perd {lost} hommes."),
    )
    .army(army_id)
    .faction(&faction);
    if let Some(province) = province {
        event = event.province(province);
    }
    events.push(event);
}

pub(crate) fn strongest(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    ids.iter()
        .max_by_key(|id| (state.armies[*id].total_strength(), Reverse((*id).clone())))
        .cloned()
}

/// Keeps the province of an army's general in step with the army.
pub(crate) fn move_general(state: &mut CampaignState, data: &GameData, army_id: &ArmyId) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let Some(location) = state.army_province(data, army) else {
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
        // Lot C7a: the season covers `season_scale` of the v1 steps.
        (steps.max(1.0) * points_per_step(data) * data.movement_rules().season_scale).round() as u32
    }

    /// Movement points of a fresh army at full pace this season (no
    /// technology, general or siege train): the season's steps times
    /// `points_per_step` and `season_scale` (lot C7a).
    pub fn season_movement_points(&self, data: &GameData) -> u32 {
        let rules = data.movement_rules();
        (f64::from(self.season.movement_steps()) * rules.points_per_step * rules.season_scale)
            .round() as u32
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

/// A field battle between two armies within reach of each other: deferred
/// to the player when interactive battles are on and the player takes part
/// (M7, see `battle_request`), auto-resolved otherwise. Neither side moves
/// again this turn (lot M2).
pub(crate) fn fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    for id in [attacker_id, defender_id] {
        if let Some(army) = state.armies.get_mut(id) {
            army.movement_left = 0;
            army.clear_plan();
        }
    }
    if crate::battle_request::defer_player_battle(state, data, attacker_id, defender_id, events) {
        return;
    }
    auto_fight(state, data, attacker_id, defender_id, events);
}

/// Armies fighting on `lead`'s side (F1): `lead` first, then every other
/// army of `lead`'s faction or of an ally of it, at war with `enemy`, and
/// standing in the same settlement or within `engage_radius_km` of `lead`
/// (lot M2), in id order.
pub fn battle_coalition(
    state: &CampaignState,
    data: &GameData,
    lead: &ArmyId,
    enemy: &FactionId,
) -> Vec<ArmyId> {
    let Some(lead_army) = state.armies.get(lead) else {
        return Vec::new();
    };
    let engage = data.free_movement_rules().engage_radius_km;
    let mut ids = vec![lead.clone()];
    ids.extend(
        state
            .armies
            .iter()
            .filter(|(id, army)| {
                *id != lead
                    && state.is_allied(&lead_army.faction, &army.faction)
                    && state.is_at_war(&army.faction, enemy)
                    && match (army.settlement(), lead_army.settlement()) {
                        (Some(a), Some(b)) => a == b,
                        _ => state.army_distance_km(data, army, lead_army) <= engage,
                    }
            })
            .map(|(id, _)| id.clone()),
    );
    ids
}

/// Armies of `lead`'s side stationed in its settlement (sieges, G1):
/// `lead` first, then the armies of its faction or allies there at war with
/// `enemy`, in id order.
pub fn settlement_coalition(
    state: &CampaignState,
    lead: &ArmyId,
    enemy: &FactionId,
) -> Vec<ArmyId> {
    let Some(lead_army) = state.armies.get(lead) else {
        return Vec::new();
    };
    let Some(place) = lead_army.settlement() else {
        return vec![lead.clone()];
    };
    let mut ids = vec![lead.clone()];
    ids.extend(
        state
            .armies
            .iter()
            .filter(|(id, army)| {
                *id != lead
                    && army.is_at(place)
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

/// Auto-resolves a field battle between two armies within reach of each
/// other; the allied armies nearby join either side (F1).
pub(crate) fn auto_fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return;
    };
    let province = state
        .army_province(data, defender)
        .and_then(|p| data.provinces.get(&p));
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
    let attackers = battle_coalition(state, data, attacker_id, &defender.faction);
    let defenders = battle_coalition(state, data, defender_id, &attacker.faction);
    let mut attacker_side = coalition_side(state, data, &attackers);
    let mut defender_side = coalition_side(state, data, &defenders);
    // DF1: the AI's morale against the player follows the difficulty.
    state.apply_difficulty_morale(
        data,
        &mut attacker_side,
        state.coalition_has_player(&attackers),
        &mut defender_side,
        state.coalition_has_player(&defenders),
    );
    // N1: phased auto-resolve on the province's terrain, season and weather.
    let result = resolve_field(
        state,
        data,
        &attackers,
        &defenders,
        &attacker_side,
        &defender_side,
        &context,
        province,
    );
    apply_battle_result(state, data, &attackers, &defenders, &result, events);
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
    let Some(province_id) = state.army_province(data, defender) else {
        return;
    };
    let battlefield = state.army_point(data, defender);
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

    // Lot M5b: strength before the battle, to weigh the defeat of the losers.
    let strength_before: BTreeMap<ArmyId, u32> = attackers
        .iter()
        .chain(defenders)
        .filter_map(|id| Some((id.clone(), state.armies.get(id)?.total_strength())))
        .collect();
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
        let before = strength_before.get(loser_id).copied().unwrap_or(0);
        let after = state.armies.get(loser_id).map_or(0, |a| a.total_strength());
        let losses_percent = if before == 0 {
            0
        } else {
            (u64::from(before.saturating_sub(after)) * 100 / u64::from(before)) as u32
        };
        retreat_beaten_army(state, data, loser_id, battlefield, losses_percent, events);
    }
}

/// How a beaten army leaves the battlefield (lot C7a rule on the grid, spec
/// § 3.3).
#[derive(Debug, Clone, PartialEq)]
pub enum Retreat {
    /// To a friendly settlement (own or allied) within the friendly radius.
    Friendly(SettlementId),
    /// Back `retreat_fallback_km` away from the victor, losing stragglers
    /// (map-pixel point).
    Fallback([f32; 2]),
    /// Nowhere to go: a rout. The survivors rally at the given friendly
    /// settlement (any distance) or disperse (`None`).
    Rout(Option<SettlementId>),
}

/// Directions tried for the fallback, in degrees off the line away from the
/// victor.
const FALLBACK_ANGLES: [f32; 5] = [0.0, 45.0, -45.0, 90.0, -90.0];

/// Where `army_id`, just beaten at `battlefield` (map pixels), falls back
/// (pure, lot M2 on the C7a rule):
///
/// 1. the nearest settlement held by the army's faction or an ally, free of
///    enemy armies, within `friendly_radius_steps` × `points_per_step`
///    kilometres of march on the grid, without crossing an enemy zone of
///    control (those around the battlefield excepted) nor an enemy place;
/// 2. otherwise, when the army still has a refuge (lot M5b, the C7a
///    « neutral » rule on the grid: a settlement no enemy holds, free of
///    enemy armies, within `neutral_radius_steps` of march by the same
///    rules), a point `retreat_fallback_km` away from the victor (or
///    slightly aside), reachable the same way;
/// 3. otherwise a rout: the survivors rally at the nearest friendly
///    settlement of the same land mass, at any distance.
///
/// Ties are broken by cell and settlement id: the result is deterministic.
/// The battle losses are not weighed here: see [`retreat_target_after`].
pub fn retreat_target(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    battlefield: [f32; 2],
) -> Option<Retreat> {
    retreat_target_after(state, data, army_id, battlefield, 0)
}

/// [`retreat_target`] for an army that lost `losses_percent` of its men in
/// the battle: at `heavy_defeat_losses_percent` or more, without a friendly
/// place within reach, it cannot fall back in order and routs (lot M5b).
pub fn retreat_target_after(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    battlefield: [f32; 2],
    losses_percent: u32,
) -> Option<Retreat> {
    let army = state.armies.get(army_id)?;
    let rules = data.retreat_rules();
    let free = data.free_movement_rules();
    let grid = data.navgrid();
    let start_point = state.army_point(data, army);
    let start = Cell::of_point(grid, start_point);
    let zoc_px = free.zoc_radius_km as f32 * px_per_km(data);
    let dist = |a: [f32; 2], b: [f32; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    let enemies: Vec<[f32; 2]> = state
        .armies
        .iter()
        .filter(|(id, a)| *id != army_id && state.is_at_war(&army.faction, &a.faction))
        .map(|(_, a)| state.army_point(data, a))
        .filter(|p| dist(*p, start_point) > zoc_px && dist(*p, battlefield) > zoc_px)
        .collect();
    // Hostile places and the zones of control of the enemies away from the
    // battlefield, as a set of blocked cells.
    let mut blocker = state.hostile_blocker(data, &army.faction);
    let zoc_cells = (zoc_px / grid.scale as f32).ceil() as i64 + 1;
    for enemy in &enemies {
        let centre = Cell::of_point(grid, *enemy);
        for dy in -zoc_cells..=zoc_cells {
            for dx in -zoc_cells..=zoc_cells {
                let (x, y) = (i64::from(centre.x) + dx, i64::from(centre.y) + dy);
                if !grid.contains(x, y) {
                    continue;
                }
                let cell = Cell::new(x as u32, y as u32);
                if dist(cell.center(grid), *enemy) <= zoc_px {
                    blocker.insert(cell);
                }
            }
        }
    }
    let blocker = &blocker;
    let friendly: std::collections::BTreeMap<Cell, SettlementId> = state
        .settlements
        .keys()
        .filter(|id| {
            !army.is_at(id)
                && state.is_friendly_settlement(&army.faction, id)
                && state.hostile_armies_at(&army.faction, id).is_empty()
        })
        .filter_map(|id| {
            data.settlement_point(id)
                .map(|p| (Cell::of_point(grid, p), id.clone()))
        })
        .rev()
        .collect();
    let budget = km_to_grid_points(data, rules.friendly_radius_steps * points_per_step(data));
    if let Some((cell, _)) =
        navigation::bounded_dijkstra(grid, start, budget, blocker, |cell, _| {
            friendly.contains_key(&cell)
        })
    {
        return Some(Retreat::Friendly(friendly[&cell].clone()));
    }
    // Lot M5b: a crushed army, or one with no refuge left in reach (deep in
    // enemy land, or hemmed in by enemy zones of control), routs.
    let heavy = losses_percent >= rules.heavy_defeat_losses_percent;
    let refuges: std::collections::BTreeSet<Cell> = state
        .settlements
        .keys()
        .filter(|id| {
            !army.is_at(id)
                && !state.is_hostile_settlement(&army.faction, id)
                && state.hostile_armies_at(&army.faction, id).is_empty()
        })
        .filter_map(|id| data.settlement_point(id).map(|p| Cell::of_point(grid, p)))
        .collect();
    let refuge_budget = km_to_grid_points(data, rules.neutral_radius_steps * points_per_step(data));
    let has_refuge = !heavy
        && navigation::bounded_dijkstra(grid, start, refuge_budget, blocker, |cell, _| {
            refuges.contains(&cell)
        })
        .is_some();
    // Away from the victor.
    let (dx, dy) = (
        start_point[0] - battlefield[0],
        start_point[1] - battlefield[1],
    );
    let length = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if length > 1e-3 {
        (dx / length, dy / length)
    } else {
        (1.0, 0.0)
    };
    let reach = free.retreat_fallback_km as f32 * px_per_km(data);
    let fallback_budget = km_to_grid_points(data, 2.0 * free.retreat_fallback_km);
    for angle in FALLBACK_ANGLES.iter().filter(|_| has_refuge) {
        let (sin, cos) = angle.to_radians().sin_cos();
        let (vx, vy) = (ux * cos - uy * sin, ux * sin + uy * cos);
        let target = [start_point[0] + vx * reach, start_point[1] + vy * reach];
        let cell = Cell::of_point(grid, target);
        if blocker.contains(cell) {
            continue;
        }
        if navigation::find_path(grid, start, cell, blocker, Some(fallback_budget)).is_some() {
            return Some(Retreat::Fallback(cell.center(grid)));
        }
    }
    // Rout: rally at the nearest friendly place of the same land mass.
    let (sx, sy) = (i64::from(start.x), i64::from(start.y));
    let component = grid.component(sx, sy);
    let rally = crate::march::nearest_settlement_where(data, start_point, |id| {
        state.is_friendly_settlement(&army.faction, id)
            && state.hostile_armies_at(&army.faction, id).is_empty()
            && data.settlement_point(id).is_some_and(|p| {
                let c = Cell::of_point(grid, p);
                grid.component(i64::from(c.x), i64::from(c.y)) == component
            })
    });
    Some(Retreat::Rout(rally))
}

/// Applies the retreat rule to a beaten army (see [`retreat_target`]).
fn retreat_beaten_army(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    battlefield: [f32; 2],
    losses_percent: u32,
    events: &mut Vec<GameEvent>,
) {
    let Some(retreat) = retreat_target_after(state, data, army_id, battlefield, losses_percent)
    else {
        return;
    };
    let rules = data.retreat_rules();
    match retreat {
        Retreat::Friendly(target) => {
            move_beaten_army(state, data, army_id, ArmyPosition::Settlement(target))
        }
        Retreat::Fallback(point) => {
            let lost = decimate(state, army_id, rules.neutral_loss_percent);
            push_retreat_event(
                state,
                data,
                army_id,
                format!(
                    "{}, coupé de ses places, recule et perd {lost} traînards.",
                    crate::events::capitalize(&state.army_name(data, army_id))
                ),
                events,
            );
            move_beaten_army(state, data, army_id, ArmyPosition::field(point));
        }
        Retreat::Rout(rally) => {
            let lost = decimate(state, army_id, rules.rout_loss_percent);
            let (strength, max) = state.armies.get(army_id).map_or((0, 0), |a| {
                (
                    a.total_strength(),
                    a.units.iter().map(|u| u.max_strength).sum::<u32>(),
                )
            });
            let broken = strength == 0
                || u64::from(strength) * 100
                    < u64::from(rules.rout_dissolve_below_percent) * u64::from(max);
            match rally.filter(|_| !broken) {
                Some(target) => {
                    push_retreat_event(
                        state,
                        data,
                        army_id,
                        format!(
                            "Débandade : {}, coupé de ses places, perd {lost} hommes avant de se rallier.",
                            state.army_name(data, army_id)
                        ),
                        events,
                    );
                    move_beaten_army(state, data, army_id, ArmyPosition::Settlement(target));
                }
                None => disperse_army(state, data, army_id, events),
            }
        }
    }
}

/// Moves a beaten army to `position`, spent for the turn.
fn move_beaten_army(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    position: ArmyPosition,
) {
    if let Some(army) = state.armies.get_mut(army_id) {
        army.position = position;
        army.movement_left = 0;
        army.clear_plan();
        move_general(state, data, army_id);
    }
}
/// Removes `percent` of every unit of `army_id` (rounded up); units under
/// 5 % of their maximum are disbanded, as after a battle. Returns the men lost.
fn decimate(state: &mut CampaignState, army_id: &ArmyId, percent: u32) -> u32 {
    let Some(army) = state.armies.get_mut(army_id) else {
        return 0;
    };
    let mut lost = 0;
    for unit in &mut army.units {
        let casualties = (unit.strength * percent).div_ceil(100).min(unit.strength);
        unit.strength -= casualties;
        lost += casualties;
    }
    army.units
        .retain(|unit| unit.strength > 0 && unit.strength * 20 >= unit.max_strength);
    lost
}

fn push_retreat_event(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    text: String,
    events: &mut Vec<GameEvent>,
) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let mut event = GameEvent::new(EventKind::Attrition, text)
        .army(army_id)
        .faction(&army.faction);
    if let Some(province) = state.army_province(data, army) {
        event = event.province(&province);
    }
    events.push(event);
}

/// A routed army with nowhere to rally melts away; its general escapes.
fn disperse_army(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let faction = army.faction.clone();
    let province = state.army_province(data, army);
    let name = state.army_name(data, army_id);
    if let Some(general) = army.general.clone() {
        state.detach_general(&general);
    }
    state.armies.remove(army_id);
    let mut event = GameEvent::new(
        EventKind::ArmyDestroyed,
        format!("Débandade : {name}, coupé de ses places, se disperse."),
    )
    .army(army_id)
    .faction(&faction);
    if let Some(province) = province {
        event = event.province(&province);
    }
    events.push(event);
}

pub(crate) fn apply_outcome(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    outcome: &crate::battle_auto::SideOutcome,
    events: &mut Vec<GameEvent>,
) {
    // H4: barber-surgeons tend the wounded of the surviving units.
    let recovery = crate::medicine::army_wound_recovery(state, data, army_id);
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
    let faction = army.faction.clone();
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let location = state
        .army_province(data, army)
        .unwrap_or_else(province_placeholder);

    if tended > 0 && faction == state.player_faction {
        events.push(
            GameEvent::new(
                EventKind::Medicine,
                format!(
                    "{tended} blessés soignés rejoignent les rangs de {}.",
                    state.army_name(data, army_id)
                ),
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
        let name = crate::events::capitalize(&state.army_name(data, army_id));
        if let Some(general) = &general {
            state.detach_general(general);
        }
        state.armies.remove(army_id);
        events.push(
            GameEvent::new(EventKind::ArmyDestroyed, format!("{name} est anéanti."))
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
