//! BuildingKit models of the battle site (port of `_build_buildings`, `_build_props`,
//! `_build_churchyards` and the horse lines of game/scripts/battle/battle_decor.gd): the houses,
//! mills and church of the field, the props of the site and of the camps, the dry-stone walls and
//! lychgate of the churchyards and the tethered horses are the Blender / generated models of
//! `game/assets/models/buildings/` (manifest read by `campaign::towns::kit`), placed on their
//! footprint, instead of the coloured volumes of `site.rs` (which stay the fallback of a kind
//! the kit does not have).
//!
//! The placement is pure ([`plan`]): model name + transform, tested without assets.

use bevy::prelude::*;
use sim_battle::{Area, AreaKind, Battlefield, DecorProp, DecorPropKind, House, HouseKind, SideId};

use super::ground::polyline_distance;
use super::hash01;
use crate::campaign::towns::kit::Kit;
use crate::campaign::towns::rng::Rng;

/// Length of a wall run of the kit (m).
pub const WALL_RUN_M: f32 = 10.0;
/// Half-width of the lychgate opening (m).
const GATE_HALF_M: f32 = 1.6;

/// One kit model to place.
#[derive(Debug, Clone, PartialEq)]
pub struct KitPiece {
    pub model: String,
    pub transform: Transform,
}

/// What the kit takes over, and what the volumes keep.
#[derive(Debug, Default)]
pub struct KitPlan {
    pub pieces: Vec<KitPiece>,
    /// Indices of `decor.buildings` drawn as kit models.
    pub houses: Vec<usize>,
    /// Indices of the props (site, then camp items and convoys in order) drawn as kit models.
    pub props: Vec<usize>,
    pub wall_runs: usize,
    pub lychgates: usize,
    pub horses: usize,
    /// Horse lines (the posts and the rope stay volumes).
    pub horse_lines: usize,
    /// Indices in `pieces` of the horses (animated by `camp_horse`).
    pub horse_pieces: Vec<usize>,
    /// `(index in pieces, side)` of the tents and pavilions of the camps (knocked down when the
    /// camp is looted, `_loot`).
    pub camp_tents: Vec<(usize, SideId)>,
}

/// Kit kind of a house of the decor (`BattleDecor.kit_kind`).
pub fn kit_kind(kind: HouseKind, length: f32) -> &'static str {
    match kind {
        HouseKind::Timbered => "timber",
        HouseKind::Stone => "stonehouse",
        HouseKind::Barn => "barn",
        HouseKind::Church => "church",
        HouseKind::Windmill => "windmill",
        HouseKind::Watermill => "watermill",
        _ => {
            if length > 11.0 {
                "longere"
            } else {
                "cottage"
            }
        }
    }
}

/// Kit kind of a prop kind (its name in the manifest), `None` for the horse line.
pub fn prop_kit_kind(kind: DecorPropKind) -> Option<&'static str> {
    Some(match kind {
        DecorPropKind::Haystack => "haystack",
        DecorPropKind::Cart => "cart",
        DecorPropKind::Tent => "tent",
        DecorPropKind::Pavilion => "pavilion",
        DecorPropKind::Wagon => "wagon",
        DecorPropKind::Campfire => "campfire",
        DecorPropKind::Graves => "graves",
        DecorPropKind::Well => "well",
        DecorPropKind::Woodpile => "woodpile",
        _ => return None,
    })
}

/// Lowest and highest ground under an oriented footprint (centre included).
pub fn ground_span(
    ground: &impl Fn(f64, f64) -> f32,
    x: f64,
    z: f64,
    length: f64,
    width: f64,
    yaw: f64,
) -> (f32, f32) {
    let (sin, cos) = yaw.sin_cos();
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    let corners = [
        (-length, -width),
        (length, -width),
        (length, width),
        (-length, width),
        (0.0, 0.0),
    ];
    for (cx, cz) in corners {
        let (cx, cz) = (cx * 0.5, cz * 0.5);
        let h = ground(x + cx * cos - cz * sin, z + cx * sin + cz * cos);
        low = low.min(h);
        high = high.max(h);
    }
    (low, high)
}

/// Transform of a house on its footprint (the facade towards +Z of the model).
pub fn house_transform(
    kit: &Kit,
    model: &str,
    house: &House,
    ground: &impl Fn(f64, f64) -> f32,
) -> Transform {
    let (low, high) = ground_span(
        ground,
        house.x,
        house.z,
        house.length,
        house.width,
        house.yaw,
    );
    let y = high.min(low + 1.4) - 0.05;
    Transform::from_translation(Vec3::new(house.x as f32, y, house.z as f32))
        .with_rotation(Quat::from_rotation_y(-(house.yaw as f32)))
        .with_scale(kit.fit_scale(model, house.length as f32, house.width as f32))
}

/// Transform of a prop (`BuildingKit.prop_transform`).
fn prop_transform(prop: &DecorProp, ground: &impl Fn(f64, f64) -> f32) -> Transform {
    Transform::from_translation(Vec3::new(
        prop.x as f32,
        ground(prop.x, prop.z) - 0.03,
        prop.z as f32,
    ))
    .with_rotation(Quat::from_rotation_y(-(prop.yaw as f32)))
}

/// Places every kit model of the field. `ground` is the ground height.
pub fn plan(field: &Battlefield, kit: &Kit, ground: &impl Fn(f64, f64) -> f32) -> KitPlan {
    let mut plan = KitPlan::default();
    if kit.is_empty() {
        return plan;
    }
    for (index, house) in field.decor.buildings.iter().enumerate() {
        let wanted = kit_kind(house.kind, house.length as f32);
        let mut rng = Rng::new((house.x * 31.0 + house.z * 17.0) as i64 as u32);
        let Some(entry) = kit.pick(wanted, house.length as f32, house.width as f32, &mut rng)
        else {
            continue;
        };
        plan.pieces.push(KitPiece {
            model: entry.name.clone(),
            transform: house_transform(kit, &entry.name, house, ground),
        });
        plan.houses.push(index);
    }
    let camp_items = field.decor.camps.iter().flat_map(|c| {
        c.items
            .iter()
            .chain(c.convoy.iter())
            .map(move |prop| (prop, Some(c.side)))
    });
    let all_props = field
        .decor
        .props
        .iter()
        .map(|p| (p, None))
        .chain(camp_items);
    for (index, (prop, camp_side)) in all_props.enumerate() {
        if prop.kind == DecorPropKind::HorseLine {
            plan.horse_lines += 1;
            horses(&mut plan, kit, prop, ground);
            continue;
        }
        let Some(kind) = prop_kit_kind(prop.kind) else {
            continue;
        };
        let Some(entry) = kit.prop_model(kind, index as u32) else {
            continue;
        };
        if let Some(side) = camp_side
            && matches!(prop.kind, DecorPropKind::Tent | DecorPropKind::Pavilion)
        {
            plan.camp_tents.push((plan.pieces.len(), side));
        }
        plan.pieces.push(KitPiece {
            model: entry.name.clone(),
            transform: prop_transform(prop, ground),
        });
        plan.props.push(index);
    }
    let road_distance = |p: Vec2| {
        field
            .roads
            .iter()
            .map(|r| polyline_distance(&r.points, f64::from(p.x), f64::from(p.y)))
            .fold(f64::MAX, f64::min)
    };
    churchyards(&mut plan, &field.decor.areas, kit, ground, &road_distance);
    plan
}

/// Horses at the picket line: on both sides of the rope, head towards it (the model's nose is
/// towards -X).
fn horses(plan: &mut KitPlan, kit: &Kit, prop: &DecorProp, ground: &impl Fn(f64, f64) -> f32) {
    let models = kit.models_of("horse");
    if models.is_empty() {
        return;
    }
    let centre = Vec2::new(prop.x as f32, prop.z as f32);
    let yaw = prop.yaw as f32;
    let axis = Vec2::new(yaw.cos(), yaw.sin());
    let front = Vec2::new(-yaw.sin(), yaw.cos());
    let length = prop.length as f32;
    let (a, b) = (centre - axis * length * 0.5, centre + axis * length * 0.5);
    let count = if prop.count == 0 { 6 } else { prop.count } as usize;
    for i in 0..count {
        let t = (i as f32 + 0.5) / count as f32;
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let p = a.lerp(b, t) + front * side * 1.5;
        let nose = -front * side;
        let jitter = hash01(i as f32, centre.x, 71.0) * 0.5 - 0.25;
        let angle = (-nose.y).atan2(-nose.x) + jitter;
        let model = models[(i + centre.x as i32 as usize) % models.len()];
        plan.horse_pieces.push(plan.pieces.len());
        plan.pieces.push(KitPiece {
            model: model.name.clone(),
            transform: Transform::from_translation(Vec3::new(
                p.x,
                ground(f64::from(p.x), f64::from(p.y)) - 0.02,
                p.y,
            ))
            .with_rotation(Quat::from_rotation_y(-angle)),
        });
        plan.horses += 1;
    }
}

/// Dry-stone walls round each churchyard (10 m runs fitted to each side) and a lychgate on the
/// side nearest a road.
fn churchyards(
    plan: &mut KitPlan,
    areas: &[Area],
    kit: &Kit,
    ground: &impl Fn(f64, f64) -> f32,
    road_distance: &impl Fn(Vec2) -> f64,
) {
    let runs = kit.models_of("wall_run");
    if runs.is_empty() {
        return;
    }
    let gate = kit.models_of("lychgate").into_iter().next();
    for area in areas.iter().filter(|a| a.kind == AreaKind::Church) {
        let centre = Vec2::new(area.x as f32, area.z as f32);
        let yaw = area.yaw as f32;
        let (hl, hw) = (area.length as f32 * 0.5, area.width as f32 * 0.5);
        let corners = [
            Vec2::new(-hl, -hw),
            Vec2::new(hl, -hw),
            Vec2::new(hl, hw),
            Vec2::new(-hl, hw),
        ];
        let rotate = |v: Vec2| {
            Vec2::new(
                v.x * yaw.cos() - v.y * yaw.sin(),
                v.x * yaw.sin() + v.y * yaw.cos(),
            )
        };
        let mut gate_side = 2;
        let mut best = f64::MAX;
        for s in 0..4 {
            let mid = centre + rotate((corners[s] + corners[(s + 1) % 4]) * 0.5);
            let d = road_distance(mid);
            if d < best {
                best = d;
                gate_side = s;
            }
        }
        for s in 0..4 {
            let a = centre + rotate(corners[s]);
            let b = centre + rotate(corners[(s + 1) % 4]);
            let seg = a.distance(b);
            let dir = (b - a).normalize_or_zero();
            let heading = Quat::from_rotation_y(-dir.y.atan2(dir.x));
            let mut spans = vec![(0.0, seg)];
            if s == gate_side
                && let Some(gate) = gate
            {
                let mid = seg * 0.5;
                spans = vec![(0.0, mid - GATE_HALF_M), (mid + GATE_HALF_M, seg)];
                let gp = a + dir * mid;
                plan.pieces.push(KitPiece {
                    model: gate.name.clone(),
                    transform: Transform::from_translation(Vec3::new(
                        gp.x,
                        ground(f64::from(gp.x), f64::from(gp.y)) - 0.05,
                        gp.y,
                    ))
                    .with_rotation(heading),
                });
                plan.lychgates += 1;
            }
            for (from, to) in spans {
                let length = to - from;
                if length < 1.0 {
                    continue;
                }
                let n = ((length / WALL_RUN_M).ceil() as usize).max(1);
                let run_len = length / n as f32;
                for i in 0..n {
                    let q = a + dir * (from + run_len * (i as f32 + 0.5));
                    plan.pieces.push(KitPiece {
                        model: runs[(i + s) % runs.len()].name.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            q.x,
                            ground(f64::from(q.x), f64::from(q.y)) - 0.1,
                            q.y,
                        ))
                        .with_rotation(heading)
                        .with_scale(Vec3::new(
                            run_len / WALL_RUN_M,
                            1.0,
                            1.0,
                        )),
                    });
                    plan.wall_runs += 1;
                }
            }
        }
    }
}

/// Length of a stone / wood bay of the bridge kit (m), and the width of their deck
/// (`battle_bridges.gd`: dimensions of `building_kit.py`).
const STONE_BAY_M: f32 = 9.0;
const STONE_WIDTH_M: f32 = 6.0;
const WOOD_BAY_M: f32 = 4.0;
const WOOD_WIDTH_M: f32 = 5.0;

/// Bridges of the field as kit bays over the water and ramp-shaped abutments on both banks
/// (`BattleBridges.build`). `fallback` lists the bridges whose models the kit does not have (the
/// volumes of `site::add_bridge` draw them).
#[derive(Debug, Default, PartialEq)]
pub struct BridgePlan {
    pub pieces: Vec<KitPiece>,
    pub bays: usize,
    pub ends: usize,
    pub fallback: Vec<usize>,
}

pub fn plan_bridges(bridges: &[sim_battle::Bridge], kit: &Kit) -> BridgePlan {
    let mut plan = BridgePlan::default();
    for (index, bridge) in bridges.iter().enumerate() {
        let kind = if bridge.stone { "stone" } else { "wood" };
        let bay = kit
            .models_of(&format!("bridge_{kind}_bay"))
            .first()
            .map(|e| e.name.clone());
        let end = kit
            .models_of(&format!("bridge_{kind}_end"))
            .first()
            .map(|e| e.name.clone());
        let (Some(bay_model), Some(end_model)) = (bay, end) else {
            plan.fallback.push(index);
            continue;
        };
        let (bay_len, deck_width) = if bridge.stone {
            (STONE_BAY_M, STONE_WIDTH_M)
        } else {
            (WOOD_BAY_M, WOOD_WIDTH_M)
        };
        let yaw = bridge.dir.1.atan2(bridge.dir.0) as f32;
        let dir = Vec2::new(yaw.cos(), yaw.sin());
        let centre = Vec2::new(bridge.x as f32, bridge.z as f32);
        let span = (bridge.span as f32).max(2.0);
        let count = ((span / bay_len).round() as i32).max(1);
        let stretch = span / (count as f32 * bay_len);
        let squeeze = bridge.width as f32 / deck_width;
        let deck = bridge.deck as f32;
        for k in 0..count {
            let along = -span * 0.5 + (k as f32 + 0.5) * span / count as f32;
            let p = centre + dir * along;
            plan.pieces.push(KitPiece {
                model: bay_model.clone(),
                transform: Transform::from_translation(Vec3::new(p.x, deck, p.y))
                    .with_rotation(Quat::from_rotation_y(-yaw))
                    .with_scale(Vec3::new(stretch, 1.0, squeeze)),
            });
            plan.bays += 1;
        }
        for sign in [-1.0f32, 1.0] {
            let p = centre + dir * (sign * span * 0.5);
            let turn = if sign < 0.0 {
                std::f32::consts::PI
            } else {
                0.0
            };
            plan.pieces.push(KitPiece {
                model: end_model.clone(),
                transform: Transform::from_translation(Vec3::new(p.x, deck, p.y))
                    .with_rotation(Quat::from_rotation_y(-yaw + turn))
                    .with_scale(Vec3::new(1.0, 1.0, squeeze)),
            });
            plan.ends += 1;
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kit() -> Kit {
        Kit::from_json(&json!({
            "cottage_0": {"kind": "cottage", "length": 8.0, "depth": 5.0, "height": 5.0},
            "wall_run_0": {"kind": "wall_run", "length": 10.0, "depth": 0.6, "height": 1.2},
            "lychgate_0": {"kind": "lychgate", "length": 3.2, "depth": 2.4, "height": 3.0},
            "horse_0": {"kind": "horse", "length": 2.4, "depth": 0.8, "height": 1.6},
        }))
    }

    #[test]
    fn houses_map_to_kit_kinds() {
        assert_eq!(kit_kind(HouseKind::Timbered, 8.0), "timber");
        assert_eq!(kit_kind(HouseKind::Cottage, 12.0), "longere");
        assert_eq!(kit_kind(HouseKind::Cottage, 8.0), "cottage");
        assert_eq!(kit_kind(HouseKind::Watermill, 8.0), "watermill");
    }

    #[test]
    fn a_house_sits_on_its_lowest_ground_plus_a_step() {
        let house = House {
            x: 10.0,
            z: 10.0,
            length: 8.0,
            width: 5.0,
            yaw: 0.0,
            kind: HouseKind::Cottage,
        };
        // 1 m of ground per m along x: the span of an 8 m house is 8 m, capped at low + 1.4.
        let ground = |x: f64, _z: f64| x as f32;
        let t = house_transform(&kit(), "cottage_0", &house, &ground);
        assert!(
            (t.translation.y - (6.0 + 1.4 - 0.05)).abs() < 1e-4,
            "{}",
            t.translation.y
        );
        assert!((t.scale.x - 1.0).abs() < 1e-5 && (t.scale.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn horse_lines_stand_horses_on_both_sides() {
        let prop = DecorProp {
            kind: DecorPropKind::HorseLine,
            x: 50.0,
            z: 50.0,
            yaw: 0.0,
            length: 12.0,
            depth: 3.0,
            count: 6,
        };
        let mut plan = KitPlan::default();
        horses(&mut plan, &kit(), &prop, &|_, _| 0.0);
        assert_eq!(plan.horses, 6);
        assert_eq!(plan.horse_pieces, (0..6).collect::<Vec<_>>());
        let north = plan
            .pieces
            .iter()
            .filter(|p| p.transform.translation.z > 50.0)
            .count();
        assert_eq!(north, 3);
    }

    #[test]
    fn a_churchyard_gets_runs_on_four_sides_and_one_gate() {
        let areas = [Area {
            kind: AreaKind::Church,
            x: 100.0,
            z: 100.0,
            length: 30.0,
            width: 20.0,
            yaw: 0.0,
            state: None,
        }];
        let mut plan = KitPlan::default();
        // The road runs along the north side (z = 85): the gate goes there.
        let road = |p: Vec2| f64::from((p.y - 85.0).abs());
        churchyards(&mut plan, &areas, &kit(), &|_, _| 0.0, &road);
        assert_eq!(plan.lychgates, 1);
        assert!(plan.wall_runs >= 8, "{}", plan.wall_runs);
        let gate = plan
            .pieces
            .iter()
            .find(|p| p.model == "lychgate_0")
            .expect("gate");
        assert!((gate.transform.translation.z - 90.0).abs() < 1e-3);
        assert!(
            plan.pieces
                .iter()
                .all(|p| p.transform.translation.is_finite())
        );
    }

    fn bridge(stone: bool, span: f64) -> sim_battle::Bridge {
        sim_battle::Bridge {
            x: 100.0,
            z: 50.0,
            dir: (1.0, 0.0),
            length: span + 10.0,
            width: 6.0,
            span,
            deck: 4.0,
            stone,
            arches: 2,
            stream: None,
        }
    }

    fn bridge_kit() -> Kit {
        Kit::from_json(&json!({
            "bridge_stone_bay_0": {"kind": "bridge_stone_bay", "length": 9.0, "depth": 8.8, "height": 8.1},
            "bridge_stone_end_0": {"kind": "bridge_stone_end", "length": 5.0, "depth": 6.0, "height": 8.5},
        }))
    }

    #[test]
    fn a_stone_bridge_is_bays_over_the_water_and_two_ends() {
        let plan = plan_bridges(&[bridge(true, 20.0)], &bridge_kit());
        assert_eq!((plan.bays, plan.ends), (2, 2));
        assert!(plan.fallback.is_empty());
        // Two bays of 10 m: stretched 10/9, centred 5 m either side of the bridge centre.
        let first = &plan.pieces[0].transform;
        assert!((first.scale.x - 10.0 / 9.0).abs() < 1e-5);
        assert!((first.translation.x - 95.0).abs() < 1e-4);
        assert_eq!(first.translation.y, 4.0);
        assert!((first.scale.z - 1.0).abs() < 1e-5);
        // The end at the start of the bridge is turned round.
        let west = &plan.pieces[2].transform;
        assert!((west.translation.x - 90.0).abs() < 1e-4);
        let facing = west.rotation * Vec3::X;
        assert!(facing.x < -0.99);
    }

    #[test]
    fn a_bridge_without_kit_models_falls_back_to_volumes() {
        // The wooden models are missing from this kit.
        let plan = plan_bridges(&[bridge(false, 8.0), bridge(true, 9.0)], &bridge_kit());
        assert_eq!(plan.fallback, vec![0]);
        assert_eq!(plan.bays, 1);
    }
}
