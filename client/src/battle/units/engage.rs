//! Melee legibility (lot B2): in contact, every soldier faces the nearest soldier of the enemy
//! regiment he fights, so that blows land on someone instead of in the air. Render only: the
//! core decides who fights whom (`Unit::state`, `Unit::target`); this module only reads the
//! positions of the figures and sets their yaw before the followers smooth it.

use std::collections::HashMap;

use sim_battle::{BattleSim, Unit, UnitState};

use super::{Pose, drawn_units};

/// Regiment that `unit` fights: its explicit target when it stands on the other side, else the
/// nearest enemy regiment within `range` metres of centres plus the two half-depths (`None` when
/// there is none).
fn opponent(sim: &BattleSim, unit: &Unit, range: f64) -> Option<u32> {
    let enemy = |u: &&Unit| u.side != unit.side && u.present() && !u.left_field;
    if let Some(target) = unit
        .target
        .and_then(|id| sim.units().iter().find(|u| u.id == id))
        .filter(enemy)
    {
        let close = (target.x - unit.x).hypot(target.z - unit.z) < range * 6.0;
        if close {
            return Some(target.id);
        }
    }
    drawn_units(sim)
        .filter(|u| enemy(u))
        .map(|u| (u.id, (u.x - unit.x).hypot(u.z - unit.z)))
        .filter(|(_, d)| *d < range * 6.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Pairs (fighter, opponent) of the regiments in melee.
pub fn pairs(sim: &BattleSim, range_m: f32) -> Vec<(u32, u32)> {
    drawn_units(sim)
        .filter(|u| u.state == UnitState::Melee && u.present() && !u.on_wall)
        .filter(|u| u.climbing.is_none())
        .filter_map(|u| opponent(sim, u, f64::from(range_m)).map(|o| (u.id, o)))
        .collect()
}

/// Turns each figure of the fighters towards the nearest figure of its opponent when that one is
/// within `range_m`; the others keep the regiment's facing. Returns the figures turned.
pub fn face_nearest(
    places: &mut HashMap<u32, Vec<Pose>>,
    pairs: &[(u32, u32)],
    range_m: f32,
) -> usize {
    let mut turned = 0;
    for &(fighter, enemy) in pairs {
        let Some(foes) = places.get(&enemy).cloned() else {
            continue;
        };
        if foes.is_empty() {
            continue;
        }
        let Some(own) = places.get_mut(&fighter) else {
            continue;
        };
        let reach2 = range_m * range_m;
        for pose in own.iter_mut() {
            let nearest = foes
                .iter()
                .map(|f| {
                    let (dx, dz) = (f[0] - pose[0], f[2] - pose[2]);
                    (dx * dx + dz * dz, dx, dz)
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((d2, dx, dz)) = nearest
                && d2 <= reach2
                && d2 > 1e-4
            {
                pose[3] = dx.atan2(dz);
                turned += 1;
            }
        }
    }
    turned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fighter_faces_the_nearest_foe_in_reach_and_others_keep_their_facing() {
        let mut places: HashMap<u32, Vec<Pose>> = HashMap::new();
        places.insert(1, vec![[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, -50.0, 0.25]]);
        places.insert(2, vec![[3.0, 0.0, 0.0, 2.5], [30.0, 0.0, 0.0, 2.5]]);
        let turned = face_nearest(&mut places, &[(1, 2)], 7.0);
        assert_eq!(turned, 1);
        let first = places[&1][0];
        assert!(
            (first[3] - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "foe at +x is yaw pi/2: {first:?}"
        );
        assert_eq!(places[&1][1][3], 0.25, "out of reach keeps the facing");
        assert_eq!(
            places[&2][0][3], 2.5,
            "the foe is not turned by the other pair"
        );
    }

    #[test]
    fn a_missing_opponent_changes_nothing() {
        let mut places: HashMap<u32, Vec<Pose>> = HashMap::new();
        places.insert(1, vec![[0.0, 0.0, 0.0, 0.5]]);
        assert_eq!(face_nearest(&mut places, &[(1, 9)], 7.0), 0);
        assert_eq!(places[&1][0][3], 0.5);
    }
}
