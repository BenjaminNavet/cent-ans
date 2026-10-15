//! Mouse to order conversions (pure, unit-tested): the geometry of the right-drag line, pivot on
//! the spot, locked-group rigid places and the `sim_battle::Command` each gesture becomes.
//!
//! Ports game/scripts/battle/formation_drag.gd and the order builders of battle_input.gd.
//! No game rule here: the retained width, the ranks and the sharing of a group order are
//! computed by the core (`sim-battle/src/formation_width.rs`). Conventions of the core:
//! `facing = atan2(dx, dz)`, forward = `(sin f, cos f)`, right = `(cos f, -sin f)`.

use std::collections::HashMap;

use bevy::prelude::*;
use sim_battle::Command;

use super::picking::UnitView;

/// A drag shorter than this (m) is a plain click (no width).
pub const MIN_DRAG_M: f32 = 2.0;
/// A right-drag longer than this (px) is a line, not a click.
pub const DRAG_PX: f32 = 20.0;
/// Pivot on the spot: a drag whose middle falls within this radius (m) of the selection centre.
pub const PIVOT_RADIUS_M: f32 = 4.0;

/// The line of a right-drag from `p0` to `p1` (ground points), front turned away from the
/// camera at `cam`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragLine {
    pub mid: Vec3,
    pub facing: f32,
    pub width: f32,
}

pub fn drag_line(p0: Vec3, p1: Vec3, cam: Vec3) -> Option<DragLine> {
    let dir = Vec2::new(p1.x - p0.x, p1.z - p0.z);
    if dir.length() < MIN_DRAG_M {
        return None;
    }
    let mut normal = Vec2::new(-dir.y, dir.x).normalize();
    let mid = (p0 + p1) * 0.5;
    if normal.dot(Vec2::new(mid.x - cam.x, mid.z - cam.z)) < 0.0 {
        normal = -normal;
    }
    Some(DragLine {
        mid,
        facing: normal.x.atan2(normal.y),
        width: dir.length(),
    })
}

/// Circular mean of facings.
pub fn mean_facing(facings: &[f32]) -> f32 {
    let (s, c) = facings
        .iter()
        .fold((0.0, 0.0), |(s, c), f| (s + f.sin(), c + f.cos()));
    if s.abs() + c.abs() > 1e-9 {
        s.atan2(c)
    } else {
        0.0
    }
}

fn wrap_pi(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// One regiment's place in a locked group: lateral and forward offsets from the barycentre in
/// the group frame, and its facing relative to the reference facing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Offset {
    pub lateral: f32,
    pub forward: f32,
    pub turn: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LockShape {
    pub facing: f32,
    pub members: HashMap<u32, Offset>,
}

/// Shape of a locked group from the regiments (`id, x, z, facing`) at the moment of locking.
pub fn lock_shape(members: &[(u32, f32, f32, f32)]) -> LockShape {
    if members.is_empty() {
        return LockShape::default();
    }
    let facings: Vec<f32> = members.iter().map(|m| m.3).collect();
    let centre = members
        .iter()
        .fold(Vec2::ZERO, |acc, m| acc + Vec2::new(m.1, m.2))
        / members.len() as f32;
    let reference = mean_facing(&facings);
    let right = Vec2::new(reference.cos(), -reference.sin());
    let forward = Vec2::new(reference.sin(), reference.cos());
    let members = members
        .iter()
        .map(|&(id, x, z, facing)| {
            let d = Vec2::new(x, z) - centre;
            (
                id,
                Offset {
                    lateral: d.dot(right),
                    forward: d.dot(forward),
                    turn: wrap_pi(facing - reference),
                },
            )
        })
        .collect();
    LockShape {
        facing: reference,
        members,
    }
}

/// A regiment's destination: `(id, x, z, facing)`.
pub type Place = (u32, f32, f32, f32);

/// Places of a locked group moved to `point` (x, z) and turned to `facing` (`None`: its own
/// reference facing): rigid translation and rotation of the offsets, in the order of `ids`.
pub fn rigid_places(
    shape: &LockShape,
    ids: &[u32],
    point: Vec2,
    facing: Option<f32>,
) -> Vec<Place> {
    let f = facing.filter(|f| f.is_finite()).unwrap_or(shape.facing);
    let right = Vec2::new(f.cos(), -f.sin());
    let forward = Vec2::new(f.sin(), f.cos());
    ids.iter()
        .filter_map(|id| {
            let m = shape.members.get(id)?;
            let at = point + right * m.lateral + forward * m.forward;
            Some((*id, at.x, at.y, wrap_pi(f + m.turn)))
        })
        .collect()
}

/// Modifiers of a gesture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// A move order (`width` > 0: frontage of the drag, shared by the core).
#[allow(clippy::too_many_arguments)]
pub fn move_command(
    units: &[u32],
    point: Vec2,
    run: bool,
    facing: Option<f32>,
    width: Option<f32>,
    queued: bool,
) -> Command {
    Command::Move {
        units: units.to_vec(),
        x: f64::from(point.x),
        z: f64::from(point.y),
        run,
        facing: facing.map(f64::from),
        queue: queued,
        width: width.filter(|w| *w > 0.0).map(f64::from),
        match_speed: false,
        group_tag: None,
    }
}

pub fn attack_command(units: &[u32], target: u32, run: bool, queued: bool) -> Command {
    Command::Attack {
        units: units.to_vec(),
        target,
        run,
        queue: queued,
    }
}

/// Pivot on the spot: when `mid` is within [`PIVOT_RADIUS_M`] of the centre of the regiments
/// `ids`, one move per regiment to its own position with `facing`; otherwise empty.
pub fn pivot_orders(
    units: &[UnitView],
    ids: &[u32],
    mid: Vec3,
    facing: f32,
    queued: bool,
) -> Vec<Command> {
    let found: Vec<&UnitView> = units.iter().filter(|u| ids.contains(&u.id)).collect();
    if found.is_empty() {
        return Vec::new();
    }
    let centre = found
        .iter()
        .fold(Vec2::ZERO, |acc, u| acc + Vec2::new(u.pos.x, u.pos.z))
        / found.len() as f32;
    if centre.distance(Vec2::new(mid.x, mid.z)) > PIVOT_RADIUS_M {
        return Vec::new();
    }
    found
        .iter()
        .map(|u| {
            move_command(
                &[u.id],
                Vec2::new(u.pos.x, u.pos.z),
                false,
                Some(facing),
                None,
                queued,
            )
        })
        .collect()
}

/// A locked group's orders: one move per regiment to its rigid place, at the pace of the
/// slowest (`match_speed`) under a common `group_tag`.
pub fn locked_orders(places: &[Place], tag: u32, run: bool, queued: bool) -> Vec<Command> {
    places
        .iter()
        .map(|&(id, x, z, facing)| Command::Move {
            units: vec![id],
            x: f64::from(x),
            z: f64::from(z),
            run,
            facing: Some(f64::from(facing)),
            queue: queued,
            width: None,
            match_speed: true,
            group_tag: Some(tag),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_battle::SideId;

    fn view(id: u32, x: f32, z: f32) -> UnitView {
        UnitView {
            id,
            side: SideId::Attacker,
            pos: Vec3::new(x, 0.0, z),
            facing: 0.0,
            width: 20.0,
            depth: 10.0,
            present: true,
            routing: false,
            withdrawing: false,
            can_shoot: false,
            cavalry: false,
            idle: true,
            unit_type: "t".into(),
        }
    }

    #[test]
    fn a_short_drag_is_not_a_line() {
        assert!(
            drag_line(
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 50.0, -100.0)
            )
            .is_none()
        );
    }

    #[test]
    fn the_front_turns_away_from_the_camera() {
        // Drag along +x; the camera south (z = -100) looks north: the front faces +z (facing 0).
        let line = drag_line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(20.0, 50.0, -100.0),
        )
        .unwrap();
        assert!(line.facing.abs() < 1e-4, "{}", line.facing);
        assert!((line.width - 40.0).abs() < 1e-4);
        assert_eq!(line.mid, Vec3::new(20.0, 0.0, 0.0));
        // Camera north: the front faces -z.
        let line = drag_line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(20.0, 50.0, 100.0),
        )
        .unwrap();
        assert!((line.facing.abs() - std::f32::consts::PI).abs() < 1e-4);
    }

    #[test]
    fn move_command_serialises_like_the_godot_dictionary() {
        let command = move_command(
            &[3, 4],
            Vec2::new(100.0, 200.0),
            true,
            Some(0.5),
            Some(40.0),
            true,
        );
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(json["type"], "move");
        assert_eq!(json["units"], serde_json::json!([3, 4]));
        assert_eq!(json["x"], 100.0);
        assert_eq!(json["z"], 200.0);
        assert_eq!(json["run"], true);
        assert_eq!(json["queue"], true);
        assert_eq!(json["width"], 40.0);
        // Plain click: no width, no queue keys.
        let plain = serde_json::to_value(move_command(
            &[1],
            Vec2::ZERO,
            false,
            None,
            Some(0.0),
            false,
        ))
        .unwrap();
        assert!(plain.get("width").is_none() && plain.get("queue").is_none());
    }

    #[test]
    fn attack_command_runs_unless_walking() {
        let json = serde_json::to_value(attack_command(&[1, 2], 9, true, false)).unwrap();
        assert_eq!(json["type"], "attack");
        assert_eq!(json["target"], 9);
        assert_eq!(json["run"], true);
    }

    #[test]
    fn pivot_only_near_the_selection_centre() {
        let units = [view(1, 0.0, 0.0), view(2, 20.0, 0.0)];
        let near = pivot_orders(&units, &[1, 2], Vec3::new(10.0, 0.0, 1.0), 1.0, false);
        assert_eq!(near.len(), 2);
        let json = serde_json::to_value(&near[0]).unwrap();
        assert_eq!(json["x"], 0.0);
        assert_eq!(json["facing"], 1.0);
        assert_eq!(json["run"], false);
        assert!(pivot_orders(&units, &[1, 2], Vec3::new(50.0, 0.0, 0.0), 1.0, false).is_empty());
        assert!(pivot_orders(&units, &[7], Vec3::ZERO, 1.0, false).is_empty());
    }

    #[test]
    fn locked_group_keeps_its_shape_when_moved() {
        // Two regiments side by side facing +z, 20 m apart.
        let shape = lock_shape(&[(1, 0.0, 0.0, 0.0), (2, 20.0, 0.0, 0.0)]);
        let places = rigid_places(&shape, &[1, 2], Vec2::new(100.0, 100.0), None);
        assert!((places[0].1 - 90.0).abs() < 1e-3 && (places[0].2 - 100.0).abs() < 1e-3);
        assert!((places[1].1 - 110.0).abs() < 1e-3);
        // Turned a quarter turn (facing +x): the line now runs along z.
        let turned = rigid_places(
            &shape,
            &[1, 2],
            Vec2::new(100.0, 100.0),
            Some(std::f32::consts::FRAC_PI_2),
        );
        assert!((turned[0].1 - 100.0).abs() < 1e-3);
        assert!((turned[0].2 - 100.0).abs() > 9.0);
        // Orders carry match_speed and the tag.
        let orders = locked_orders(&places, 7, false, false);
        let json = serde_json::to_value(&orders[1]).unwrap();
        assert_eq!(json["match_speed"], true);
        assert_eq!(json["group_tag"], 7);
        assert_eq!(json["units"], serde_json::json!([2]));
    }
}
