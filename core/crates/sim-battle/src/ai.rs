//! Minimal battle AI (spec § 1; M9 will extend it).
//!
//! Evaluated once per simulated second for each AI side: the line advances,
//! shooters stay behind the infantry and shoot at range, cavalry charges the
//! nearest enemy within 150 m. The defender holds its ground until the enemy
//! comes within 250 m (or five minutes have passed).

use data_model::UnitCategory;

use crate::command::Command;
use crate::setup::SideId;
use crate::sim::BattleSim;
use crate::unit::UnitState;

/// Distance at which the cavalry charges.
pub const CAVALRY_CHARGE_DISTANCE: f64 = 150.0;
/// Distance at which the infantry closes in.
pub const INFANTRY_ENGAGE_DISTANCE: f64 = 180.0;
/// The defender waits until the enemy comes this close.
pub const DEFENDER_HOLD_DISTANCE: f64 = 250.0;

/// Commands of `side` for this second.
pub fn plan(sim: &BattleSim, side: SideId) -> Vec<Command> {
    let forward = match side {
        SideId::Attacker => 1.0,
        SideId::Defender => -1.0,
    };
    let units = sim.units();
    let enemies: Vec<usize> = (0..units.len())
        .filter(|&j| units[j].side != side && units[j].able())
        .collect();
    if enemies.is_empty() {
        return Vec::new();
    }
    let own: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == side && units[i].able())
        .collect();
    let nearest = |i: usize| -> (usize, f64) {
        let u = &units[i];
        enemies
            .iter()
            .map(|&j| {
                (
                    j,
                    ((units[j].x - u.x).powi(2) + (units[j].z - u.z).powi(2)).sqrt(),
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .expect("enemies not empty")
    };
    let closest_contact = own
        .iter()
        .map(|&i| nearest(i).1)
        .fold(f64::INFINITY, f64::min);
    let hold = side == SideId::Defender
        && sim.elapsed() < 300.0
        && closest_contact > DEFENDER_HOLD_DISTANCE;
    let infantry_front = own
        .iter()
        .filter(|&&i| units[i].category == UnitCategory::Infantry)
        .map(|&i| units[i].z * forward)
        .fold(f64::NEG_INFINITY, f64::max);
    let has_infantry = infantry_front.is_finite();

    let mut commands = Vec::new();
    for &i in &own {
        let unit = &units[i];
        if matches!(unit.state, UnitState::Melee | UnitState::Rallied) {
            continue;
        }
        let (enemy, distance) = nearest(i);
        let advance = unit.z * forward;
        let move_to = |target_advance: f64, run: bool| Command::Move {
            units: vec![unit.id],
            x: unit.x,
            z: (target_advance * forward).clamp(10.0, sim.field().depth - 10.0),
            run,
            facing: None,
        };
        let attack = |run: bool| Command::Attack {
            units: vec![unit.id],
            target: units[enemy].id,
            run,
        };
        let shooter = unit.can_shoot() && unit.ammo > 0;
        if shooter {
            let e = &units[enemy];
            let range = sim.effective_range(unit, e.x, e.z);
            if distance <= range * 0.95 {
                if unit.destination.is_some() || unit.target.is_some() {
                    commands.push(Command::Halt {
                        units: vec![unit.id],
                    });
                }
                continue;
            }
            if hold || unit.category == UnitCategory::Siege {
                continue;
            }
            let wanted = advance + distance - range * 0.85;
            let wanted = if has_infantry && !unit.mounted {
                wanted.min(infantry_front - 35.0)
            } else {
                wanted
            };
            if wanted - advance > 5.0 && unit.destination.is_none() {
                commands.push(move_to(wanted, false));
            }
            continue;
        }
        match unit.category {
            UnitCategory::Cavalry => {
                if distance < CAVALRY_CHARGE_DISTANCE {
                    if unit.target != Some(units[enemy].id) {
                        commands.push(attack(true));
                    }
                } else if !hold && unit.destination.is_none() && unit.target.is_none() {
                    let wanted = if has_infantry {
                        infantry_front - 15.0
                    } else {
                        advance + 60.0
                    };
                    if wanted - advance > 5.0 {
                        commands.push(move_to(wanted, false));
                    }
                }
            }
            _ => {
                if distance < INFANTRY_ENGAGE_DISTANCE {
                    let run = distance < 90.0;
                    if unit.target != Some(units[enemy].id) || unit.running != run {
                        commands.push(attack(run));
                    }
                } else if !hold && unit.destination.is_none() && unit.target.is_none() {
                    commands.push(move_to(advance + 60.0, false));
                }
            }
        }
    }
    commands
}
