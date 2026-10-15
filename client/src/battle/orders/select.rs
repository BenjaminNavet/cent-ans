//! Pure selection helpers (quick selection, idle cycling, pursuit assignment).
//!
//! Ports `select_same_type_of`, `next_idle`, `_order_pursue` and the Ctrl+A family of
//! game/scripts/battle/battle_input.gd.

use sim_battle::SideId;

use super::picking::UnitView;

/// Every present regiment of `side` with the type of `unit_id` (double click).
pub fn same_type_of(units: &[UnitView], side: SideId, unit_id: u32) -> Vec<u32> {
    let Some(kind) = units.iter().find(|u| u.id == unit_id).map(|u| &u.unit_type) else {
        return Vec::new();
    };
    units
        .iter()
        .filter(|u| u.side == side && u.present && &u.unit_type == kind)
        .map(|u| u.id)
        .collect()
}

/// Ctrl+A: every regiment of `side` present and not in rout.
pub fn all_of(units: &[UnitView], side: SideId, keep: impl Fn(&UnitView) -> bool) -> Vec<u32> {
    units
        .iter()
        .filter(|u| u.side == side && u.present && !u.routing && keep(u))
        .map(|u| u.id)
        .collect()
}

/// Next (`dir` = 1) or previous (-1) idle regiment of `side` after the last selected one,
/// wrapping around.
pub fn next_idle(units: &[UnitView], side: SideId, selected: &[u32], dir: i32) -> Option<u32> {
    let count = units.len() as i32;
    if count == 0 {
        return None;
    }
    let anchor = units
        .iter()
        .rposition(|u| selected.contains(&u.id))
        .map_or(-1, |i| i as i32);
    let start = if anchor < 0 && dir < 0 {
        count - 1
    } else {
        anchor + dir
    };
    (0..count)
        .map(|step| &units[(start + step * dir).rem_euclid(count) as usize])
        .find(|u| u.side == side && u.present && u.idle && !u.withdrawing)
        .map(|u| u.id)
}

/// Pursuit: each of `ids` chases the nearest routing enemy; `(target, chasers)` pairs.
pub fn pursuit_groups(units: &[UnitView], side: SideId, ids: &[u32]) -> Vec<(u32, Vec<u32>)> {
    let routed: Vec<&UnitView> = units
        .iter()
        .filter(|u| u.side != side && u.present && u.routing)
        .collect();
    let mut groups: Vec<(u32, Vec<u32>)> = Vec::new();
    for unit in units.iter().filter(|u| ids.contains(&u.id)) {
        let Some(foe) = routed.iter().min_by(|a, b| {
            let da = a.pos.distance(unit.pos);
            let db = b.pos.distance(unit.pos);
            da.total_cmp(&db)
        }) else {
            return Vec::new();
        };
        match groups.iter_mut().find(|(t, _)| *t == foe.id) {
            Some((_, chasers)) => chasers.push(unit.id),
            None => groups.push((foe.id, vec![unit.id])),
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;

    fn view(id: u32, side: SideId, kind: &str, x: f32) -> UnitView {
        UnitView {
            id,
            side,
            pos: Vec3::new(x, 0.0, 0.0),
            facing: 0.0,
            width: 20.0,
            depth: 10.0,
            present: true,
            routing: false,
            withdrawing: false,
            can_shoot: false,
            cavalry: false,
            idle: true,
            unit_type: kind.into(),
        }
    }

    #[test]
    fn double_click_takes_the_same_type() {
        let units = [
            view(0, SideId::Attacker, "knights", 0.0),
            view(1, SideId::Attacker, "archers", 10.0),
            view(2, SideId::Attacker, "knights", 20.0),
            view(3, SideId::Defender, "knights", 30.0),
        ];
        assert_eq!(same_type_of(&units, SideId::Attacker, 0), vec![0, 2]);
        assert!(same_type_of(&units, SideId::Attacker, 99).is_empty());
    }

    #[test]
    fn idle_cycling_wraps_and_skips_busy_units() {
        let mut units = [
            view(0, SideId::Attacker, "a", 0.0),
            view(1, SideId::Attacker, "a", 10.0),
            view(2, SideId::Defender, "a", 20.0),
            view(3, SideId::Attacker, "a", 30.0),
        ];
        units[1].idle = false;
        assert_eq!(next_idle(&units, SideId::Attacker, &[], 1), Some(0));
        assert_eq!(next_idle(&units, SideId::Attacker, &[0], 1), Some(3));
        assert_eq!(next_idle(&units, SideId::Attacker, &[3], 1), Some(0));
        assert_eq!(next_idle(&units, SideId::Attacker, &[], -1), Some(3));
        assert_eq!(next_idle(&units, SideId::Attacker, &[0], -1), Some(3));
        units[0].idle = false;
        units[3].idle = false;
        assert_eq!(next_idle(&units, SideId::Attacker, &[], 1), None);
    }

    #[test]
    fn pursuit_picks_the_nearest_routing_enemy() {
        let mut units = [
            view(0, SideId::Attacker, "horse", 0.0),
            view(1, SideId::Attacker, "horse", 100.0),
            view(2, SideId::Defender, "foot", 10.0),
            view(3, SideId::Defender, "foot", 90.0),
        ];
        assert!(pursuit_groups(&units, SideId::Attacker, &[0, 1]).is_empty());
        units[2].routing = true;
        units[3].routing = true;
        let groups = pursuit_groups(&units, SideId::Attacker, &[0, 1]);
        assert_eq!(groups, vec![(2, vec![0]), (3, vec![1])]);
    }
}
