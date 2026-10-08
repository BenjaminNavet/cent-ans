//! CB4: active abilities used by the battle AI (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4). One
//! simple rule per kind, its thresholds in the `ai` block of each file of
//! `data/battle_abilities/` (an ability without it is left to the player):
//!
//! - **pavise**: a crossbow regiment standing to shoot that took missile
//!   casualties within `under_fire_within` seconds (the rule of the former
//!   leader's order, also in sieges);
//! - **aimed shot**: an enemy (mounted with `mounted_enemy`) within
//!   `enemy_within` metres;
//! - **banner rally**: morale below `morale_below`, no enemy within
//!   `no_enemy_within` metres, not charging nor engaged; the AI then leaves
//!   the regiment at its banner while it reforms;
//! - **close ranks**: on the defensive (`when_defensive`), an enemy foot or
//!   horse (not a shooter) within `enemy_within` metres in front;
//! - **planted pikes**: an enemy horse within `enemy_within` metres in
//!   front, the regiment standing.

use data_model::{AbilityKind, BattleAbility, UnitCategory};

use super::View;
use crate::command::Command;
use crate::sim::attack_angle;
use crate::unit::UnitState;

/// Is `id` already given a move (or attack, withdraw) in this step?
fn ordered_to_move(view: &View, id: u32) -> bool {
    view.commands.iter().any(|c| {
        matches!(
            c,
            Command::Move { .. } | Command::Attack { .. } | Command::Withdraw { .. }
        ) && c.units().contains(&id)
    })
}

/// Does the AI rule of `ability` want regiment `i` to use it now?
fn wants(view: &View, i: usize, ability: &BattleAbility, defensive: bool) -> bool {
    let Some(ai) = &ability.ai else {
        return false;
    };
    if (ai.when_defensive && !defensive) || (ai.when_attacking && defensive) {
        return false;
    }
    let u = &view.units[i];
    let within = |reach: Option<f64>, mounted: bool, front: bool| {
        let Some(reach) = reach else {
            return true;
        };
        view.nearest_enemy(i, |e| {
            (!mounted || e.mounted)
                && (!front || attack_angle(u, e.x, e.z) == 0)
                && (ability.kind != AbilityKind::CloseRanks
                    || (!e.can_shoot() || e.ammo == 0) && e.category != UnitCategory::Siege)
        })
        .is_some_and(|(_, d)| d < reach)
    };
    match ability.kind {
        AbilityKind::Pavise => {
            let window = ai.under_fire_within.unwrap_or(f64::INFINITY);
            u.missile_timer <= window
                && u.destination.is_none()
                && matches!(u.state, UnitState::Idle | UnitState::Shooting)
        }
        AbilityKind::AimedShot => {
            u.state != UnitState::Melee
                && u.ammo > 0
                && within(ai.enemy_within, ai.mounted_enemy, ai.in_front)
        }
        AbilityKind::BannerRally => {
            ai.morale_below.is_some_and(|m| u.morale < m)
                && !matches!(u.state, UnitState::Melee | UnitState::Charging)
                && ai.no_enemy_within.is_none_or(|reach| {
                    view.nearest_enemy(i, |_| true)
                        .is_none_or(|(_, d)| d >= reach)
                })
        }
        AbilityKind::CloseRanks => {
            u.destination.is_none()
                && matches!(u.state, UnitState::Idle | UnitState::Melee)
                && within(ai.enemy_within, ai.mounted_enemy, ai.in_front)
        }
        AbilityKind::PlantedPikes => {
            u.destination.is_none()
                && u.state == UnitState::Idle
                && within(ai.enemy_within, ai.mounted_enemy, ai.in_front)
        }
    }
}

/// Abilities of the side's regiments for this decision step (after the
/// movement plan and the leader's orders).
pub(super) fn plan_abilities(view: &mut View, defensive: bool, siege: bool) {
    let sim = view.sim;
    // Regiments reforming at their banner are left there while it lasts.
    let rallying: Vec<u32> = view
        .own
        .iter()
        .map(|&i| &view.units[i])
        .filter(|u| u.ability_state.active_kind() == Some(AbilityKind::BannerRally))
        .map(|u| u.id)
        .collect();
    if !rallying.is_empty() {
        hold(view, &rallying);
    }
    let mut catalogue: Vec<&BattleAbility> = sim
        .ability_catalog()
        .iter()
        .filter(|a| a.ai.as_ref().is_some_and(|ai| ai.in_sieges || !siege))
        .collect();
    if catalogue.is_empty() {
        return;
    }
    catalogue.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
    for ability in catalogue {
        let mut units = Vec::new();
        for k in 0..view.own.len() {
            let i = view.own[k];
            let u = &view.units[i];
            if u.ability_state.active.is_some()
                || !crate::abilities::eligible(ability, u)
                || sim.ability_unavailable(u, ability).is_some()
                || ordered_to_move(view, u.id)
                || !wants(view, i, ability, defensive)
            {
                continue;
            }
            units.push(u.id);
        }
        if units.is_empty() {
            continue;
        }
        if ability.kind == AbilityKind::BannerRally {
            // The regiment stays at its banner while it reforms.
            hold(view, &units);
        }
        view.commands.push(Command::UseAbility {
            units,
            ability: ability.id.clone(),
        });
    }
}

/// Takes `ids` out of the moves and attacks of this step (commands left
/// without regiments are dropped).
fn hold(view: &mut View, ids: &[u32]) {
    view.commands.retain_mut(|c| match c {
        Command::Move { units, .. } | Command::Attack { units, .. } => {
            units.retain(|id| !ids.contains(id));
            !units.is_empty()
        }
        _ => true,
    });
}
