//! The retreat of the beaten and the aftermath of a battle.

use std::collections::BTreeMap;

use super::{move_general, points_per_step};
use crate::events::{EventKind, GameEvent};
use crate::march::{km_to_grid_points, px_per_km};
use crate::navigation::{self, Cell};
use crate::state::{ArmyId, ArmyPosition, CampaignState};
use data_model::{CharacterId, FactionId, GameData, ProvinceId, SettlementId};

/// How a beaten army leaves the battlefield (rule on the grid, spec § 3.3).
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
pub(crate) fn retreat_beaten_army(
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
            if state
                .armies
                .get(army_id)
                .is_some_and(|a| a.units.is_empty())
            {
                disperse_army(state, data, army_id, events);
                return;
            }
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

/// F1: a general taken in battle (made `captive` by [`apply_outcome`]) is
/// held by `captor`; without a captor he could never be ransomed. No-op when
/// the general was not captured or already has a captor.
pub(crate) fn assign_captor(
    state: &mut CampaignState,
    general: Option<&CharacterId>,
    captor: &FactionId,
) {
    if let Some(c) = general
        .and_then(|g| state.characters.get_mut(g))
        .filter(|c| c.captive && c.captor.is_none())
    {
        c.captor = Some(captor.clone());
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
    // TW2-T2: no replenishment this season.
    crate::replenish::mark_fought(state, army_id);
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

    let taken = outcome.general_captured
        && general
            .as_ref()
            .is_none_or(|g| crate::ransom::capture_allowed(state, data, g, outcome.routed));
    if taken {
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
