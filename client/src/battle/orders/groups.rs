//! Selection groups (Ctrl+1..9 save, 1..9 recall) and locked groups (Ctrl+G).
//!
//! Ports game/scripts/battle/battle_groups.gd (the selection groups and the CB1 locks). Pure
//! presentation: no rule, orders always go through `BattleSim::issue_command`.

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::drag::{LockShape, lock_shape};
use super::picking::UnitView;

pub const MAX_GROUPS: u8 = 9;

#[derive(Debug, Clone, PartialEq)]
pub struct LockedGroup {
    pub ids: Vec<u32>,
    pub shape: LockShape,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct BattleGroups {
    /// Group number (1..9) to regiment ids.
    pub groups: BTreeMap<u8, Vec<u32>>,
    /// Lock tag to locked group.
    pub locks: BTreeMap<u32, LockedGroup>,
    next_tag: u32,
}

impl BattleGroups {
    /// Saves the selection under `number`; an empty selection clears the group.
    pub fn save(&mut self, number: u8, ids: &[u32]) {
        if number == 0 || number > MAX_GROUPS {
            return;
        }
        if ids.is_empty() {
            self.groups.remove(&number);
        } else {
            self.groups.insert(number, ids.to_vec());
        }
    }

    /// Ids of group `number` still present on the field, in saved order.
    pub fn recall(&self, number: u8, units: &[UnitView]) -> Vec<u32> {
        self.groups.get(&number).map_or_else(Vec::new, |ids| {
            ids.iter()
                .copied()
                .filter(|id| units.iter().any(|u| u.id == *id && u.present))
                .collect()
        })
    }

    /// Numbers of the groups holding `unit_id` (for the badges of the cards).
    pub fn numbers_of(&self, unit_id: u32) -> Vec<u8> {
        self.groups
            .iter()
            .filter(|(_, ids)| ids.contains(&unit_id))
            .map(|(n, _)| *n)
            .collect()
    }

    pub fn lock_of(&self, unit_id: u32) -> Option<u32> {
        self.locks
            .iter()
            .find(|(_, g)| g.ids.contains(&unit_id))
            .map(|(tag, _)| *tag)
    }

    /// Tag of the locked group that contains every one of `ids`.
    pub fn locked_group_for(&self, ids: &[u32]) -> Option<u32> {
        let tag = self.lock_of(*ids.first()?)?;
        ids.iter()
            .all(|id| self.lock_of(*id) == Some(tag))
            .then_some(tag)
    }

    /// Locks the selection, or unlocks it when it is exactly a locked group. Returns the new
    /// tag, `None` when unlocked (or fewer than two available regiments).
    pub fn toggle_lock(&mut self, ids: &[u32], units: &[UnitView]) -> Option<u32> {
        if let Some(tag) = self.locked_group_for(ids)
            && self.locks[&tag].ids.len() == ids.len()
        {
            self.locks.remove(&tag);
            return None;
        }
        let members: Vec<&UnitView> = units
            .iter()
            .filter(|u| ids.contains(&u.id) && u.available())
            .collect();
        for id in ids {
            self.leave(*id);
        }
        if members.len() < 2 {
            return None;
        }
        self.next_tag += 1;
        let tag = self.next_tag;
        let shape = lock_shape(
            &members
                .iter()
                .map(|u| (u.id, u.pos.x, u.pos.z, u.facing))
                .collect::<Vec<_>>(),
        );
        self.locks.insert(
            tag,
            LockedGroup {
                ids: members.iter().map(|u| u.id).collect(),
                shape,
            },
        );
        Some(tag)
    }

    /// Drops regiments in rout, destroyed or gone from the locked groups.
    pub fn prune_locks(&mut self, units: &[UnitView]) {
        if self.locks.is_empty() {
            return;
        }
        let gone: Vec<u32> = units
            .iter()
            .filter(|u| !u.available())
            .map(|u| u.id)
            .collect();
        for id in gone {
            self.leave(id);
        }
    }

    fn leave(&mut self, unit_id: u32) {
        let Some(tag) = self.lock_of(unit_id) else {
            return;
        };
        let group = self.locks.get_mut(&tag).expect("tag just found");
        group.ids.retain(|id| *id != unit_id);
        group.shape.members.remove(&unit_id);
        if group.ids.len() < 2 {
            self.locks.remove(&tag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_battle::SideId;

    fn view(id: u32, x: f32, present: bool) -> UnitView {
        UnitView {
            id,
            side: SideId::Attacker,
            pos: Vec3::new(x, 0.0, 0.0),
            facing: 0.0,
            width: 20.0,
            depth: 10.0,
            present,
            routing: false,
            withdrawing: false,
            can_shoot: false,
            cavalry: false,
            idle: true,
            unit_type: "t".into(),
        }
    }

    #[test]
    fn save_and_recall_skip_regiments_that_left() {
        let mut groups = BattleGroups::default();
        groups.save(3, &[1, 2, 5]);
        let units = [
            view(1, 0.0, true),
            view(2, 20.0, false),
            view(5, 40.0, true),
        ];
        assert_eq!(groups.recall(3, &units), vec![1, 5]);
        assert_eq!(groups.numbers_of(5), vec![3]);
        groups.save(3, &[]);
        assert!(groups.recall(3, &units).is_empty());
        groups.save(0, &[1]);
        groups.save(10, &[1]);
        assert!(groups.groups.is_empty());
    }

    #[test]
    fn lock_toggles_and_dissolves_when_a_member_is_lost() {
        let mut groups = BattleGroups::default();
        let mut units = vec![view(1, 0.0, true), view(2, 20.0, true), view(3, 40.0, true)];
        let tag = groups.toggle_lock(&[1, 2, 3], &units).unwrap();
        assert_eq!(groups.locked_group_for(&[1, 2, 3]), Some(tag));
        assert_eq!(groups.locked_group_for(&[1, 2]), Some(tag));
        units[2].present = false;
        groups.prune_locks(&units);
        assert_eq!(groups.locks[&tag].ids, vec![1, 2]);
        units[1].routing = true;
        groups.prune_locks(&units);
        assert!(groups.locks.is_empty());
        // A lone regiment cannot be locked; locking twice unlocks.
        assert_eq!(groups.toggle_lock(&[1], &units), None);
        let units = [view(1, 0.0, true), view(2, 20.0, true)];
        let tag = groups.toggle_lock(&[1, 2], &units).unwrap();
        assert!(tag > 0);
        assert_eq!(groups.toggle_lock(&[1, 2], &units), None);
        assert!(groups.locks.is_empty());
    }
}
