//! CPU picking on the positions of the simulation (no physics, no meshes): a unit is picked by
//! the screen distance to its centre or banner top, the ground by marching the camera ray
//! against the height grid.
//!
//! Ports `pick_unit`, `ground_point` and the box selection of game/scripts/battle/battle_scene.gd
//! and battle_input.gd. The projection is injected as a closure so everything here is pure and
//! unit-tested.

use bevy::prelude::*;
use data_model::UnitCategory;
use sim_battle::{BattleSim, Battlefield, SideId, UnitState};

/// Smallest picking radius around a regiment, on screen (px).
pub const PICK_RADIUS_PX: f32 = 26.0;
/// Height of the banner top above the regiment's feet (m, `BattleBannerLayer.BANNER_HEIGHT`).
pub const BANNER_HEIGHT: f32 = 7.0;
/// Margin kept from the field edges for a picked ground point (m).
const EDGE_MARGIN: f32 = 5.0;

/// What the orders need to know about a regiment, copied from the simulation each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub id: u32,
    pub side: SideId,
    /// Feet of the regiment centre (y = standing height).
    pub pos: Vec3,
    pub facing: f32,
    pub width: f32,
    pub depth: f32,
    pub present: bool,
    pub routing: bool,
    pub withdrawing: bool,
    pub can_shoot: bool,
    pub cavalry: bool,
    pub idle: bool,
    pub unit_type: String,
}

impl UnitView {
    /// Present on the field and not in rout or withdrawing: can be given orders.
    pub fn available(&self) -> bool {
        self.present && !self.routing && !self.withdrawing
    }
}

/// Every real regiment (no synthetic ones) of the battle.
pub fn views(sim: &BattleSim) -> Vec<UnitView> {
    sim.units()
        .iter()
        .filter(|unit| !unit.synthetic)
        .map(|unit| {
            let (width, depth) = unit.extent();
            UnitView {
                id: unit.id,
                side: unit.side,
                pos: Vec3::new(
                    unit.x as f32,
                    sim.standing_height(unit, unit.x, unit.z) as f32,
                    unit.z as f32,
                ),
                facing: unit.facing as f32,
                width: width as f32,
                depth: depth as f32,
                present: unit.present(),
                routing: unit.state == UnitState::Routing,
                withdrawing: unit.withdrawing,
                can_shoot: unit.can_shoot(),
                cavalry: unit.category == UnitCategory::Cavalry,
                idle: unit.state == UnitState::Idle
                    && unit.destination.is_none()
                    && unit.target.is_none(),
                unit_type: unit.unit_type.to_string(),
            }
        })
        .collect()
}

/// Point of the ground under a camera ray (iterated on the height of the field, as the Godot
/// scene does), kept inside the field.
pub fn ground_point(origin: Vec3, dir: Vec3, field: &Battlefield) -> Vec3 {
    let height = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    if dir.y.abs() < 1e-4 {
        return Vec3::new(origin.x, 0.0, origin.z);
    }
    let mut t = (height(origin.x, origin.z) - origin.y) / dir.y;
    for _ in 0..8 {
        let p = origin + dir * t;
        let next = (height(p.x, p.z) - origin.y) / dir.y;
        let done = (next - t).abs() < 0.05;
        t = next;
        if done {
            break;
        }
    }
    let mut point = origin + dir * t.max(0.0);
    point.x = point.x.clamp(EDGE_MARGIN, field.width as f32 - EDGE_MARGIN);
    point.z = point.z.clamp(EDGE_MARGIN, field.depth as f32 - EDGE_MARGIN);
    point.y = height(point.x, point.z);
    point
}

/// Screen projection of a world point; `None` when behind the camera.
pub type Project<'a> = &'a dyn Fn(Vec3) -> Option<Vec2>;

/// The regiment of `side` under `screen`, the nearest by distance to its centre or banner top
/// within a radius that grows with its apparent width.
pub fn pick_unit(screen: Vec2, units: &[UnitView], side: SideId, project: Project) -> Option<u32> {
    let mut best: Option<(u32, f32)> = None;
    for unit in units.iter().filter(|u| u.side == side && u.present) {
        let Some(centre) = project(unit.pos) else {
            continue;
        };
        let half = unit.width * 0.5;
        let edge = unit.pos + Vec3::new(unit.facing.cos() * half, 1.5, -unit.facing.sin() * half);
        let radius = project(edge)
            .map_or(PICK_RADIUS_PX, |p| p.distance(centre))
            .max(PICK_RADIUS_PX);
        let banner = project(unit.pos + Vec3::Y * BANNER_HEIGHT);
        let distance = banner.map_or(screen.distance(centre), |top| {
            screen.distance(centre).min(screen.distance(top))
        });
        if distance < radius && best.is_none_or(|(_, d)| distance < d) {
            best = Some((unit.id, distance));
        }
    }
    best.map(|(id, _)| id)
}

/// Regiments of `side` whose centre falls in the screen rectangle `[min, max]`.
pub fn box_select(
    min: Vec2,
    max: Vec2,
    units: &[UnitView],
    side: SideId,
    project: Project,
) -> Vec<u32> {
    units
        .iter()
        .filter(|u| u.side == side && u.present && !u.routing)
        .filter(|u| {
            project(u.pos)
                .is_some_and(|p| p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y)
        })
        .map(|u| u.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, side: SideId, x: f32, z: f32) -> UnitView {
        UnitView {
            id,
            side,
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
            unit_type: "unit_test".into(),
        }
    }

    /// Orthographic top-down projection: 10 px per metre, screen = (x, z) * 10.
    fn top_down(p: Vec3) -> Option<Vec2> {
        Some(Vec2::new(p.x, p.z) * 10.0)
    }

    #[test]
    fn picks_the_nearest_unit_of_the_side_within_the_radius() {
        let units = [
            unit(1, SideId::Attacker, 10.0, 10.0),
            unit(2, SideId::Attacker, 14.0, 10.0),
            unit(3, SideId::Defender, 10.0, 10.0),
        ];
        let at = |p| pick_unit(p, &units, SideId::Attacker, &top_down);
        assert_eq!(at(Vec2::new(105.0, 100.0)), Some(1));
        assert_eq!(at(Vec2::new(135.0, 100.0)), Some(2));
        assert_eq!(at(Vec2::new(900.0, 900.0)), None);
        assert_eq!(
            pick_unit(Vec2::new(100.0, 100.0), &units, SideId::Defender, &top_down),
            Some(3)
        );
    }

    #[test]
    fn a_wide_regiment_is_picked_by_its_flank() {
        let mut wide = unit(1, SideId::Attacker, 50.0, 10.0);
        wide.width = 60.0; // half 30 m = 300 px
        let units = [wide];
        let far_on_flank = Vec2::new(500.0 + 250.0, 100.0);
        assert_eq!(
            pick_unit(far_on_flank, &units, SideId::Attacker, &top_down),
            Some(1)
        );
    }

    #[test]
    fn gone_units_are_not_picked() {
        let mut gone = unit(1, SideId::Attacker, 10.0, 10.0);
        gone.present = false;
        assert_eq!(
            pick_unit(
                Vec2::new(100.0, 100.0),
                &[gone],
                SideId::Attacker,
                &top_down
            ),
            None
        );
    }

    #[test]
    fn box_selection_takes_own_units_inside_the_rectangle() {
        let mut routing = unit(4, SideId::Attacker, 12.0, 12.0);
        routing.routing = true;
        let units = [
            unit(1, SideId::Attacker, 10.0, 10.0),
            unit(2, SideId::Attacker, 30.0, 10.0),
            unit(3, SideId::Defender, 11.0, 11.0),
            routing,
        ];
        let ids = box_select(
            Vec2::new(0.0, 0.0),
            Vec2::new(200.0, 200.0),
            &units,
            SideId::Attacker,
            &top_down,
        );
        assert_eq!(ids, vec![1]);
    }
}
