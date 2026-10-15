//! The town of a siege battle in BuildingKit models (port of `_build_kit_town`, `_kit_props`,
//! `_kit_place` and `ruin_site` of game/scripts/battle/battle_siege.gd).
//!
//! The core lays house blocks (`SiegeWorks::houses`: a rectangle of `length` x `depth`, one or
//! two rows, the parish church, the suburb houses) and street furniture (`props`, standing
//! against a house or on the market square). Each block becomes a row (or two rows back to back)
//! of kit buildings (town houses with the odd stone house or timber frame), the church the kit
//! church, a suburb house a rural model; the furniture is the kit's stalls, carts, barrels,
//! woodpiles and well. A house the fire burnt down gives way to the ruin model of its kind
//! (the furniture against it goes), see [`update_ruins`].
//!
//! The placement is pure ([`plan`]) and tested without assets; houses of a kind the kit does
//! not have keep the coloured block of `works.rs`.

use std::collections::HashMap;
use std::f32::consts::PI;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sim_battle::{FireState, SiegeWorks};

use super::{SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::campaign::towns::kit::{Kit, KitEntry};
use crate::campaign::towns::rng::Rng;

/// Seed of the town (the same in Godot).
const SEED: u32 = 1340;

/// One kit building or furniture of a house.
#[derive(Debug, Clone, PartialEq)]
pub struct TownPiece {
    /// Index of the house of `SiegeWorks::houses` (`None`: the market square).
    pub house: Option<usize>,
    pub model: String,
    /// Ruin model that replaces it when the house burns (buildings only).
    pub ruin: Option<String>,
    pub transform: Transform,
}

/// Ruin of the same kind with the best proportions.
pub fn pick_ruin(kit: &Kit, kind: &str, length: f32, width: f32) -> Option<String> {
    let target = length / width.max(0.1);
    kit.all_of(kind)
        .into_iter()
        .filter(|e| e.ruined)
        .min_by(|a, b| {
            let da = ((a.length / a.depth) / target).ln().abs();
            let db = ((b.length / b.depth) / target).ln().abs();
            da.total_cmp(&db)
        })
        .map(|e: &KitEntry| e.name.clone())
}

#[allow(clippy::too_many_arguments)]
fn place(
    pieces: &mut Vec<TownPiece>,
    kit: &Kit,
    ground: &impl Fn(f64, f64) -> f32,
    house: usize,
    kind: &str,
    p: Vec2,
    length: f32,
    width: f32,
    yaw: f32,
    rng: &mut Rng,
) {
    let Some(entry) = kit.pick(kind, length, width, rng) else {
        return;
    };
    let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
    for (cx, cz) in [
        (-length, -width),
        (length, -width),
        (length, width),
        (-length, width),
    ] {
        let (cx, cz) = (cx * 0.5, cz * 0.5);
        let q = Vec2::new(
            p.x + cx * yaw.cos() - cz * yaw.sin(),
            p.y + cx * yaw.sin() + cz * yaw.cos(),
        );
        let h = ground(f64::from(q.x), f64::from(q.y));
        low = low.min(h);
        high = high.max(h);
    }
    let transform = Transform::from_translation(Vec3::new(p.x, high.min(low + 1.4) - 0.05, p.y))
        .with_rotation(Quat::from_rotation_y(-yaw))
        .with_scale(kit.fit_scale(&entry.name, length, width));
    pieces.push(TownPiece {
        house: Some(house),
        model: entry.name.clone(),
        ruin: pick_ruin(kit, kind, length, width),
        transform,
    });
}

/// Every kit building and furniture piece of the town of `works`.
pub fn plan(works: &SiegeWorks, kit: &Kit, ground: &impl Fn(f64, f64) -> f32) -> Vec<TownPiece> {
    let mut pieces = Vec::new();
    if kit.is_empty() {
        return pieces;
    }
    let mut rng = Rng::new(SEED);
    for (index, house) in works.houses.iter().enumerate() {
        let p = Vec2::new(house.x as f32, house.z as f32);
        let (length, depth) = if house.length > 0.0 {
            (house.length as f32, house.depth as f32)
        } else {
            let r = house.radius as f32;
            (r * 1.7, r * 1.7)
        };
        let yaw = house.yaw as f32;
        if house.church {
            place(
                &mut pieces,
                kit,
                ground,
                index,
                "church",
                p,
                length,
                depth,
                yaw,
                &mut rng,
            );
        } else if house.suburb {
            let kinds = ["cottage", "timber", "longere", "barn", "cottage"];
            let kind = kinds[rng.int(0, 4) as usize];
            place(
                &mut pieces,
                kit,
                ground,
                index,
                kind,
                p,
                length,
                depth,
                yaw,
                &mut rng,
            );
        } else {
            let tangent = Vec2::new(yaw.cos(), yaw.sin());
            let front = Vec2::new(-yaw.sin(), yaw.cos());
            let rows = i32::from(house.rows.max(1));
            let row_depth = depth / rows as f32;
            for row in 0..rows {
                let (facing, row_yaw, row_tangent) = if row == 0 {
                    (front, yaw, tangent)
                } else {
                    (-front, yaw + PI, -tangent)
                };
                let row_centre = p + facing * if rows == 2 { row_depth * 0.5 } else { 0.0 };
                let n = ((length / 6.5).round() as i32).clamp(2, 5);
                let widths: Vec<f32> = (0..n).map(|_| rng.range(0.8, 1.25)).collect();
                let total: f32 = widths.iter().sum();
                let mut x = -length * 0.5;
                for w_share in widths {
                    let w = length * w_share / total;
                    let mut kind = "townhouse";
                    if n <= 3 && rng.randf() < 0.3 {
                        kind = ["stonehouse", "timber"][rng.int(0, 1) as usize];
                    }
                    let d = row_depth
                        * if kind == "townhouse" {
                            rng.range(0.85, 1.0)
                        } else {
                            0.9
                        };
                    let q =
                        row_centre + row_tangent * (x + w * 0.5) - facing * (row_depth - d) * 0.5;
                    place(
                        &mut pieces,
                        kit,
                        ground,
                        index,
                        kind,
                        q,
                        w,
                        d,
                        row_yaw,
                        &mut rng,
                    );
                    x += w;
                }
            }
        }
    }
    // Street furniture: against a house (goes with it) or on the paving of the square.
    let paving = ground(works.center.0, works.center.1) + 0.2;
    for (k, prop) in works.props.iter().enumerate() {
        let kind = prop.kind.key();
        let Some(entry) = kit.prop_model(kind, k as u32) else {
            continue;
        };
        let y = match prop.house {
            None => paving - if kind == "well" { 0.1 } else { 0.0 },
            Some(_) => ground(prop.x, prop.z) - 0.03,
        };
        pieces.push(TownPiece {
            house: prop.house,
            model: entry.name.clone(),
            ruin: None,
            transform: Transform::from_translation(Vec3::new(prop.x as f32, y, prop.z as f32))
                .with_rotation(Quat::from_rotation_y(-(prop.yaw as f32))),
        });
    }
    pieces
}

/// Kit piece of a house (intact), child of its [`KitHouse`].
#[derive(Component)]
pub struct KitPart {
    pub ruin: Option<String>,
    pub transform: Transform,
}

/// Parent of the kit pieces of one house.
#[derive(Component)]
pub struct KitHouse {
    pub index: usize,
    pub ruined: bool,
}

/// Kit scenes of the town, by model.
#[derive(Default)]
pub struct Scenes(HashMap<String, Handle<WorldAsset>>);

impl Scenes {
    pub fn get(&mut self, assets: &AssetServer, model: &str) -> Handle<WorldAsset> {
        self.0
            .entry(model.to_owned())
            .or_insert_with(|| {
                assets.load(GltfAssetLabel::Scene(0).from_asset(Kit::asset_path(model)))
            })
            .clone()
    }
}

/// Spawns the kit town and returns the houses it covers (the others keep their block).
pub fn spawn_town(
    commands: &mut Commands,
    assets: &AssetServer,
    works: &SiegeWorks,
    kit: &Kit,
    ground: &impl Fn(f64, f64) -> f32,
    stats: &mut SiegeStats,
) -> Vec<usize> {
    let pieces = plan(works, kit, ground);
    let mut scenes = Scenes::default();
    let mut covered = Vec::new();
    // One parent per house; square furniture hangs from a parent of its own.
    let mut parents: HashMap<Option<usize>, Entity> = HashMap::new();
    for piece in pieces {
        let parent = *parents.entry(piece.house).or_insert_with(|| {
            let mut entity = commands.spawn((
                Name::new(match piece.house {
                    Some(i) => format!("KitHouse{i}"),
                    None => "KitSquare".to_owned(),
                }),
                Transform::default(),
                Visibility::default(),
                DespawnOnExit(AppState::Battle),
            ));
            if let Some(index) = piece.house {
                entity.insert(KitHouse {
                    index,
                    ruined: false,
                });
            }
            entity.id()
        });
        let handle = scenes.get(assets, &piece.model);
        commands.spawn((
            Name::new(format!("Kit_{}", piece.model)),
            KitPart {
                ruin: piece.ruin.clone(),
                transform: piece.transform,
            },
            WorldAssetRoot(handle),
            piece.transform,
            ChildOf(parent),
        ));
        if piece.ruin.is_some()
            && let Some(index) = piece.house
            && !covered.contains(&index)
        {
            covered.push(index);
        }
        stats.kit_pieces += 1;
    }
    stats.kit_models = scenes.0.len();
    covered
}

/// A house the fire burnt down: its buildings give way to ruins, its furniture goes.
pub fn update_ruins(
    mut commands: Commands,
    assets: Res<AssetServer>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    mut houses: Query<(Entity, &mut KitHouse, Option<&Children>)>,
    parts: Query<(Entity, &KitPart)>,
) {
    let mut scenes = Scenes::default();
    for (entity, mut house, children) in &mut houses {
        if house.ruined {
            continue;
        }
        let burnt = frame
            .houses
            .get(house.index)
            .is_some_and(|f| f.state == FireState::Burnt);
        if !burnt {
            continue;
        }
        house.ruined = true;
        for child in children.into_iter().flat_map(|c| c.iter()) {
            let Ok((part_entity, part)) = parts.get(child) else {
                continue;
            };
            commands.entity(part_entity).despawn();
            if let Some(ruin) = &part.ruin {
                let handle = scenes.get(&assets, ruin);
                commands.spawn((
                    Name::new(format!("Ruin_{ruin}")),
                    WorldAssetRoot(handle),
                    part.transform,
                    ChildOf(entity),
                ));
                stats.kit_ruins += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kit() -> Kit {
        Kit::from_json(&json!({
            "townhouse_0": {"kind": "townhouse", "length": 6.0, "depth": 7.0, "height": 9.0},
            "townhouse_ruin_0": {"kind": "townhouse", "length": 6.0, "depth": 7.0, "ruined": true},
            "church_0": {"kind": "church", "length": 20.0, "depth": 10.0, "height": 12.0},
            "stall_0": {"kind": "stall", "length": 2.0, "depth": 2.0, "height": 2.0},
            "timber_0": {"kind": "timber", "length": 6.0, "depth": 6.0, "height": 8.0},
            "stonehouse_0": {"kind": "stonehouse", "length": 6.0, "depth": 6.0, "height": 8.0},
        }))
    }

    #[test]
    fn a_ruin_is_found_for_a_kind_with_one() {
        let kit = kit();
        assert_eq!(
            pick_ruin(&kit, "townhouse", 6.0, 7.0).as_deref(),
            Some("townhouse_ruin_0")
        );
        assert!(pick_ruin(&kit, "church", 6.0, 7.0).is_none());
    }

    #[test]
    fn the_real_kit_dresses_the_town_of_a_siege() {
        use crate::battle::scene::setup;
        use crate::paths;
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        let mut custom = setup::default_custom_battle(&game);
        custom.siege = true;
        custom.fortification = Some(sim_battle::CustomBattleRules::bundled().default_fortification);
        let sim = setup::build_sim(&game, &dir, &custom, 11).expect("siege builds");
        let works = sim.siege().expect("siege works").clone();
        let kit = Kit::load(&paths::asset_root(&root));
        let pieces = plan(&works, &kit, &|_, _| 0.0);
        assert!(!works.houses.is_empty());
        assert!(pieces.len() >= works.houses.len(), "{}", pieces.len());
        assert!(pieces.iter().all(|p| p.transform.translation.is_finite()));
        assert!(
            pieces.iter().any(|p| p.ruin.is_some()),
            "buildings have ruins"
        );
        assert!(plan(&works, &Kit::default(), &|_, _| 0.0).is_empty());
    }
}
