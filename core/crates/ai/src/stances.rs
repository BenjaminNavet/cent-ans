//! The AI's stances and encounter detours (lot CV3-6, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 5), called by
//! `campaign::plan_armies` for each army.
//!
//! - **Ambush** ([`ambush_orders`]): an army weaker than an enemy army
//!   marching on its lands (the enemy's `planned_path`, what the core shows
//!   of a march spanning several turns) lies in wait in cover within the
//!   zone of control of that route: on the spot, or after a move that leaves
//!   it the movement the ambush needs. It keeps the stance while the threat
//!   lasts and it is not discovered ([`keep_ambush`]).
//! - **Forced march** ([`forced_march_orders`]): to relieve a besieged place
//!   or join a siege out of normal reach but within reach with the bonus,
//!   never to end next to a stronger enemy.
//! - **Entrenched camp** ([`should_entrench`]): an idle army outnumbered on
//!   a threatened border, outside any place.
//! - **Encounters** ([`encounter_detour`]): an army with nothing urgent walks
//!   to a site it reaches this turn, with a low chance.
//!
//! Every order is checked against the core's own rules before it is given
//! (`posture::validate_stance_change`, `preview_march_to_point`): the AI
//! never issues a stance order the core refuses. Tuning: `postures` and
//! `encounters` of `data/ai/grid.json` ([`data_model::AiPostures`],
//! [`data_model::AiEncounters`]); the chances are pure rolls of the seed,
//! turn and army ([`crate::alignment::campaign_roll`]), so the plan stays
//! deterministic.

use data_model::util::{dist, segment_distance};
use data_model::{FactionId, GameData, ProvinceId, SettlementId, PLAIN_COST};
use sim_campaign::posture;
use sim_campaign::{
    ArmyId, CampaignState, Cell, MoveOrderTarget, Order, Place, Stance, StopReason,
};

/// Salts of the rolls.
const AMBUSH_SALT: u64 = 0xA3B0;
const DETOUR_SALT: u64 = 0xD370;
/// Sampling step (km) of a route when looking for the lands it crosses.
const ROUTE_SAMPLE_KM: f32 = 4.0;
/// Ambush cells tried with a full march preview at most.
const AMBUSH_CANDIDATES: usize = 4;

/// Distance from `p` to the polyline `route`.
fn route_distance(p: [f32; 2], route: &[[f32; 2]]) -> f32 {
    match route {
        [] => f32::INFINITY,
        [only] => dist(p, *only),
        _ => route
            .windows(2)
            .map(|w| segment_distance(p, w[0], w[1]))
            .fold(f32::INFINITY, f32::min),
    }
}

/// `route` cut after `length` pixels.
fn truncate_route(route: &[[f32; 2]], length: f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    let Some(first) = route.first() else {
        return out;
    };
    out.push(*first);
    let mut left = length;
    for w in route.windows(2) {
        let d = dist(w[0], w[1]);
        if d >= left {
            let t = if d > 0.0 { left / d } else { 0.0 };
            out.push([
                w[0][0] + t * (w[1][0] - w[0][0]),
                w[0][1] + t * (w[1][1] - w[0][1]),
            ]);
            return out;
        }
        left -= d;
        out.push(w[1]);
    }
    out
}

/// Pure roll in 0..1000 for `army` this turn (`salt` tells the decisions
/// apart).
fn roll(state: &CampaignState, faction: &FactionId, army: &ArmyId, salt: u64) -> u64 {
    let mut key = salt ^ (u64::from(state.turn) << 24);
    for b in army.as_str().bytes() {
        key = key.rotate_left(7) ^ u64::from(b);
    }
    crate::alignment::campaign_roll(state, faction, key)
}

/// Province under map point `point` (the nearest settlement's without the
/// raster), like `CampaignState::army_province` for a field army.
fn point_province(state: &CampaignState, data: &GameData, point: [f32; 2]) -> Option<ProvinceId> {
    data.province_at_point(point[0], point[1])
        .filter(|p| state.provinces.contains_key(*p))
        .cloned()
        .or_else(|| {
            sim_campaign::march::nearest_settlement(data, point)
                .and_then(|s| state.settlement_province(&s).cloned())
        })
}

/// `province` is land of `faction` (owned or controlled).
fn is_own_land(state: &CampaignState, faction: &FactionId, province: &ProvinceId) -> bool {
    state.province_owner(province) == Some(faction)
        || state.province_controller(province) == Some(faction)
}

/// Kilometres of `points` grid movement points (plain cells).
fn points_km(data: &GameData, points: u32) -> f64 {
    f64::from(points) / f64::from(PLAIN_COST) * data.navgrid().cell_km
}

/// The foreseeable route of `enemy` (its point, then the corners of its
/// multi-turn march) over `route_turns` full turns, when it marches on the
/// lands of `faction` (owned; also occupied ones with `occupied_lands`).
fn threatening_route(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    enemy: &ArmyId,
) -> Option<Vec<[f32; 2]>> {
    let army = state.armies.get(enemy)?;
    if army.planned_path.is_empty() || army.destination.is_none() {
        return None;
    }
    let grid = data.navgrid();
    let px_per_km = sim_campaign::march::px_per_km(data);
    let mut route = vec![state.army_point(data, army)];
    route.extend(army.planned_path.iter().map(|c: &Cell| c.center(grid)));
    let turns = data.ai_grid.postures.ambush.route_turns;
    let km = points_km(data, state.army_base_grid_allowance(data, army)) * turns;
    let route = truncate_route(&route, km as f32 * px_per_km);
    // Does it cross our lands? Sampled every few kilometres.
    let step = ROUTE_SAMPLE_KM * px_per_km;
    let crosses = route.windows(2).any(|w| {
        let d = dist(w[0], w[1]);
        let n = (d / step).ceil().max(1.0) as usize;
        (0..=n).any(|i| {
            let t = i as f32 / n as f32;
            let p = [
                w[0][0] + t * (w[1][0] - w[0][0]),
                w[0][1] + t * (w[1][1] - w[0][1]),
            ];
            point_province(state, data, p).is_some_and(|p| {
                state.province_owner(&p) == Some(faction)
                    || (data.ai_grid.postures.ambush.occupied_lands
                        && state.province_controller(&p) == Some(faction))
            })
        })
    });
    crosses.then_some(route)
}

/// Enemy armies of `faction` marching on its lands near `army_id`, with
/// their foreseeable route, nearest first.
fn threats(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
) -> Vec<(ArmyId, Vec<[f32; 2]>)> {
    let Some(army) = state.armies.get(army_id) else {
        return Vec::new();
    };
    let rules = &data.ai_grid.postures.ambush;
    let here = state.army_point(data, army);
    let watch = rules.watch_radius_km as f32 * sim_campaign::march::px_per_km(data);
    let mut out: Vec<(f32, ArmyId, Vec<[f32; 2]>)> = state
        .armies
        .iter()
        .filter(|(_, e)| state.is_at_war(faction, &e.faction) && !e.units.is_empty())
        .filter(|(_, e)| !posture::is_hidden_from(state, data, e, faction))
        .filter_map(|(id, e)| {
            let d = dist(here, state.army_point(data, e));
            if d > watch {
                return None;
            }
            let route = threatening_route(state, data, faction, id)?;
            Some((d, id.clone(), route))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out.into_iter().map(|(_, id, route)| (id, route)).collect()
}

/// Chance (per mille) that `faction` lies in wait (personality).
fn ambush_weight(data: &GameData, aggression: i32) -> u64 {
    let rules = &data.ai_grid.postures.ambush;
    let weight =
        f64::from(rules.weight_permille) + rules.weight_per_aggression * f64::from(aggression - 50);
    weight.round().clamp(0.0, 1000.0) as u64
}

/// CV3-6: orders laying `army_id` in ambush (a move to a covered cell by
/// the route of a stronger enemy marching on its lands, then `SetStance
/// Ambush`), or `None`.
pub fn ambush_orders(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
    aggression: i32,
) -> Option<Vec<Order>> {
    let rules = &data.ai_grid.postures.ambush;
    let weight = ambush_weight(data, aggression);
    let army = state.armies.get(army_id)?;
    if weight == 0
        || army.units.is_empty()
        || army.movement_left == 0
        || matches!(army.stance, Stance::Ambush | Stance::ForcedMarch)
    {
        return None;
    }
    let power = state.army_power(data, army_id);
    let px_per_km = sim_campaign::march::px_per_km(data);
    let zoc = data.free_movement_rules().zoc_radius_km as f32 * px_per_km;
    let near = rules.route_zoc_share as f32 * zoc;
    // Stronger enemies, but not overwhelming ones.
    let targets: Vec<([f32; 2], Vec<[f32; 2]>)> = threats(state, data, faction, army_id)
        .into_iter()
        .filter(|(enemy, _)| {
            let theirs = state.army_power(data, enemy);
            power < rules.max_power_ratio * theirs && power >= rules.min_power_ratio * theirs
        })
        .map(|(enemy, route)| (state.army_point(data, &state.armies[&enemy]), route))
        .collect();
    if targets.is_empty() || roll(state, faction, army_id, AMBUSH_SALT) >= weight {
        return None;
    }
    let fits = |point: [f32; 2]| {
        targets
            .iter()
            .any(|(at, route)| dist(point, *at) > zoc && route_distance(point, route) <= near)
    };
    let posture_rules = &data.posture_rules;
    let covered = |point: [f32; 2]| {
        data.cover_class_at(point, &posture_rules.cover)
            .is_covered()
    };
    // On the spot.
    let here = state.army_point(data, army);
    if army.settlement().is_none()
        && fits(here)
        && posture::validate_stance_change(state, data, army_id, Stance::Ambush).is_ok()
    {
        return Some(vec![Order::SetStance {
            army: army_id.clone(),
            stance: Stance::Ambush,
        }]);
    }
    // After a move leaving the movement the ambush needs.
    let base = state.army_base_grid_allowance(data, army);
    let needed =
        (f64::from(base) * posture_rules.ambush.min_movement_left_percent / 100.0).ceil() as u32;
    let budget = army.movement_left.saturating_sub(needed.max(1));
    if budget == 0 {
        return None;
    }
    let grid = data.navgrid();
    let own_cell = state.army_cell(data, army);
    let mut cells: Vec<(f32, u32, Cell)> = state
        .reachable_area(data, army_id)
        .into_iter()
        .filter(|(cell, cost)| *cost <= budget && *cell != own_cell)
        .filter_map(|(cell, cost)| {
            let point = cell.center(grid);
            if !fits(point) || !covered(point) {
                return None;
            }
            let d = targets
                .iter()
                .map(|(_, route)| route_distance(point, route))
                .fold(f32::INFINITY, f32::min);
            Some((d, cost, cell))
        })
        .collect();
    cells.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    cells
        .into_iter()
        .take(AMBUSH_CANDIDATES)
        .find_map(|(_, _, cell)| {
            let point = cell.center(grid);
            let (cost, stop) = state.preview_march_to_point(data, army_id, point)?;
            (stop == StopReason::Arrived && cost <= budget).then(|| {
                vec![
                    Order::move_to_point(army_id.clone(), point),
                    Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Ambush,
                    },
                ]
            })
        })
}

/// CV3-6: an army in ambush holds it while an enemy it may surprise still
/// marches on its lands by its hiding place and no enemy has discovered it.
pub fn keep_ambush(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
) -> bool {
    let Some(army) = state.armies.get(army_id) else {
        return false;
    };
    if army.stance != Stance::Ambush {
        return false;
    }
    let px_per_km = sim_campaign::march::px_per_km(data);
    let zoc = data.free_movement_rules().zoc_radius_km as f32 * px_per_km;
    let here = state.army_point(data, army);
    threats(state, data, faction, army_id)
        .into_iter()
        .any(|(enemy, route)| {
            let e = &state.armies[&enemy];
            dist(here, state.army_point(data, e)) > zoc
                && route_distance(here, &route) <= zoc
                && posture::is_hidden_from(state, data, army, &e.faction)
        })
}

/// CV3-6: `SetStance ForcedMarch` then the march, when `target` (a place of
/// ours or an ally's under siege, or a place our side besieges) is beyond
/// the army's normal reach (`cost` on the settlement graph, `cap` its
/// allowance, both in `(cost, cap)`) but within reach with the bonus, and no stronger enemy waits
/// there. `march` builds the march orders for a given cap.
pub fn forced_march_orders(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
    target: &SettlementId,
    (cost, cap): (u32, u32),
    march: impl Fn(u32) -> Vec<Order>,
) -> Option<Vec<Order>> {
    let rules = &data.ai_grid.postures.forced_march;
    if !rules.enabled {
        return None;
    }
    let army = state.armies.get(army_id)?;
    let place = state.settlements.get(target)?;
    let siege = place.siege.as_ref()?;
    let ours = state.is_allied(faction, &siege.attacker);
    let relief = !ours && state.is_friendly_settlement(faction, target);
    if !(ours || relief) {
        return None;
    }
    let bonus = data.posture_rules.forced_march.movement_bonus_percent;
    let forced_cap = (f64::from(cap) * (1.0 + bonus / 100.0)).round() as u32;
    let limit = (f64::from(forced_cap) * rules.reach_margin).round() as u32;
    if cost <= cap || cost > limit {
        return None;
    }
    if posture::validate_stance_change(state, data, army_id, Stance::ForcedMarch).is_err() {
        return None;
    }
    // Never to end beside a stronger enemy.
    let point = data.settlement_point(target)?;
    let radius = rules.danger_radius_km as f32 * sim_campaign::march::px_per_km(data);
    let waiting: f64 = state
        .armies
        .iter()
        .filter(|(_, e)| state.is_at_war(faction, &e.faction))
        .filter(|(_, e)| dist(state.army_point(data, e), point) <= radius)
        .map(|(id, _)| state.army_power(data, id))
        .sum();
    if waiting > state.army_power(data, army_id) * rules.danger_ratio {
        return None;
    }
    let mut orders = march(forced_cap);
    if orders.is_empty() || army.settlement() == Some(target) {
        return None;
    }
    // A forced march does not enter a hostile place: it stops at the walls
    // of the besieged one and joins the siege next turn; no sea crossing.
    for order in &mut orders {
        match order {
            Order::MoveArmy {
                target: MoveOrderTarget::Place(Place::Settlement(s)),
                ..
            } if state.is_hostile_settlement(faction, s) => {
                if s != target {
                    return None;
                }
                *order = Order::move_to_point(army_id.clone(), point);
            }
            Order::MoveArmy { .. } => {}
            _ => return None,
        }
    }
    let mut out = vec![Order::SetStance {
        army: army_id.clone(),
        stance: Stance::ForcedMarch,
    }];
    out.append(&mut orders);
    Some(out)
}

/// CV3-6: an idle army in the field, on a threatened border of its own
/// lands, outnumbered by `threat` (hostile power around its province),
/// entrenches (or stays entrenched).
pub fn should_entrench(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
    threat: f64,
) -> bool {
    let rules = &data.ai_grid.postures.entrenched;
    let Some(army) = state.armies.get(army_id) else {
        return false;
    };
    if !rules.enabled || army.settlement().is_some() || army.units.is_empty() || threat <= 0.0 {
        return false;
    }
    let Some(province) = state.army_province(data, army) else {
        return false;
    };
    if !is_own_land(state, faction, &province) || !state.is_frontier(data, faction, &province) {
        return false;
    }
    if threat < state.army_power(data, army_id) * rules.inferiority_ratio {
        return false;
    }
    army.stance == Stance::Entrenched
        || posture::validate_stance_change(state, data, army_id, Stance::Entrenched).is_ok()
}

/// NT6c: what a weakened army does about resting this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestPlan {
    /// Not resting (and not in a rest camp): the usual plan applies.
    None,
    /// Stay put; `entrench` when it stands in the open (camp retranché).
    Rest { entrench: bool },
    /// Rested enough: leave the rest camp.
    Leave,
}

/// NT6c: a field army below `below_percent` of its full strength, on
/// friendly ground and with no hostile army within `watch_radius_km`, stays
/// put to be replenished (`replenish`): in a friendly place, or in an
/// entrenched camp in the open. It resumes once at `until_percent`
/// (hysteresis for the camp; a place has no marker, so it uses the
/// lower threshold there).
pub fn rest_plan(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
) -> RestPlan {
    let rules = &data.ai_grid.postures.rest;
    let Some(army) = state.armies.get(army_id) else {
        return RestPlan::None;
    };
    if !rules.enabled
        || army.units.is_empty()
        || !matches!(army.stance, Stance::Normal | Stance::Entrenched)
    {
        return RestPlan::None;
    }
    let camped = army.stance == Stance::Entrenched;
    let strength: u32 = army.units.iter().map(|u| u.strength).sum();
    let full: u32 = army.units.iter().map(|u| u.max_strength).sum();
    if full == 0 {
        return RestPlan::None;
    }
    let share = 100.0 * f64::from(strength) / f64::from(full);
    let limit = if camped {
        rules.until_percent
    } else {
        rules.below_percent
    };
    if share >= limit {
        return if camped {
            RestPlan::Leave
        } else {
            RestPlan::None
        };
    }
    use sim_campaign::Territory;
    if !matches!(
        state.army_territory(data, army_id),
        Territory::Own | Territory::Ally
    ) {
        return RestPlan::None;
    }
    let place = army.settlement();
    if place.is_some_and(|s| {
        state.settlements.get(s).is_some_and(|p| p.siege.is_some())
            || !state.is_friendly_settlement(faction, s)
    }) {
        return RestPlan::None;
    }
    let here = state.army_point(data, army);
    let radius = rules.watch_radius_km as f32 * sim_campaign::march::px_per_km(data);
    let menaced = state.armies.values().any(|other| {
        !other.units.is_empty()
            && state.is_at_war(faction, &other.faction)
            && dist(here, state.army_point(data, other)) <= radius
    });
    if menaced {
        return RestPlan::None;
    }
    // Nothing would come back (empty treasury, season already fought...).
    if !state
        .army_replenishment(data, army_id)
        .is_some_and(|p| p.men > 0)
    {
        return RestPlan::None;
    }
    if place.is_none()
        && !camped
        && posture::validate_stance_change(state, data, army_id, Stance::Entrenched).is_err()
    {
        return RestPlan::None;
    }
    RestPlan::Rest {
        entrench: place.is_none(),
    }
}

/// CV3-6: a `MoveArmy` to an encounter site `army_id` reaches this turn
/// (low chance per turn), for an army without urgent business: the caller
/// checks that; here the site must be seen, in lands the army may enter.
pub fn encounter_detour(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    army_id: &ArmyId,
) -> Option<Order> {
    let rules = &data.ai_grid.encounters;
    let army = state.armies.get(army_id)?;
    if rules.detour_permille == 0
        || state.encounters.sites.is_empty()
        || army.movement_left == 0
        || army.units.is_empty()
        || !matches!(army.stance, Stance::Normal | Stance::Raid)
    {
        return None;
    }
    let grid = data.navgrid();
    let px_per_km = sim_campaign::march::px_per_km(data);
    let sight = rules.sight_km as f32 * px_per_km;
    let here = state.army_point(data, army);
    let mut sites: Vec<(f32, u32, [f32; 2])> = state
        .encounters
        .sites
        .iter()
        .filter(|s| s.claimed_by.is_none() && data.encounters.contains_key(&s.encounter))
        .filter(|s| {
            sim_campaign::passage::trespassed_owner(state, faction, &s.province).is_none()
                && !state.is_hostile_territory(faction, &s.province)
        })
        .map(|s| (dist(here, s.cell.center(grid)), s.id, s.cell.center(grid)))
        .filter(|(d, _, _)| *d <= sight)
        .collect();
    if sites.is_empty() {
        return None;
    }
    sites.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let salt = DETOUR_SALT ^ u64::from(sites[0].1);
    if roll(state, faction, army_id, salt) >= u64::from(rules.detour_permille) {
        return None;
    }
    sites.into_iter().find_map(|(_, _, point)| {
        let (cost, stop) = state.preview_march_to_point(data, army_id, point)?;
        (stop == StopReason::Arrived && cost <= army.movement_left)
            .then(|| Order::move_to_point(army_id.clone(), point))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_geometry() {
        let route = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
        assert!((route_distance([5.0, 3.0], &route) - 3.0).abs() < 1e-5);
        assert!((route_distance([12.0, 5.0], &route) - 2.0).abs() < 1e-5);
        let cut = truncate_route(&route, 15.0);
        assert_eq!(cut.len(), 3);
        assert!((cut[2][1] - 5.0).abs() < 1e-5);
        assert_eq!(truncate_route(&route, 100.0).len(), 3);
    }
}
