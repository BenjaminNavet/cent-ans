//! TW retreat: a beaten army sounds the retreat in open country.
//!
//! When the share of its soldiers still fit to fight and its power relative
//! to the enemy's both fall under the thresholds of `data/rules/battle_ai.json`
//! (`retreat_share`, `retreat_ratio`, after `retreat_min_time` seconds) and
//! its general lives and it loses more men than the enemy, the AI orders `Command::Withdraw`: the regiments first,
//! the general last (once no other regiment is left on the field). The order
//! is sticky: once a regiment withdraws the retreat goes on.

use super::*;

/// Plans the retreat of a field battle; `true` when the side is retreating
/// (no other order is given this step).
pub(super) fn plan_retreat(view: &mut View) -> bool {
    let rules = tuning();
    if rules.retreat_share <= 0.0 {
        return false;
    }
    let side = view.side;
    let retreating = view.units.iter().any(|u| {
        u.side == side
            && !u.synthetic
            && u.present()
            && u.withdrawing
            && u.state != UnitState::Routing
    });
    if !retreating {
        let elapsed = view.sim.elapsed() / view.sim.ai_patience_factor();
        if elapsed < rules.retreat_min_time || !view.sim.general_alive(side) {
            return false;
        }
        let ratio = view.power(true) / view.power(false).max(1.0);
        let threshold = match side {
            SideId::Attacker => rules.retreat_share,
            SideId::Defender => rules.retreat_share_defender,
        };
        if view.sim.fighting_share(side) >= threshold || ratio >= rules.retreat_ratio {
            return false;
        }
        // An army that bleeds less than its enemy holds on (the outnumbered
        // English on their ground at Crécy or Azincourt).
        let (own_losses, enemy_losses) = (
            side_losses(view.units, side),
            side_losses(view.units, side.other()),
        );
        if own_losses <= enemy_losses + tuning().duel_loss_margin {
            return false;
        }
    }
    let units = view.units;
    let regiments: Vec<u32> = view
        .own
        .iter()
        .filter(|&&i| !units[i].is_general)
        .map(|&i| units[i].id)
        .collect();
    let command = if regiments.is_empty() {
        view.own
            .iter()
            .filter(|&&i| units[i].is_general)
            .map(|&i| units[i].id)
            .collect()
    } else {
        regiments
    };
    if command.is_empty() {
        return false;
    }
    view.commands.push(Command::Withdraw { units: command });
    true
}
