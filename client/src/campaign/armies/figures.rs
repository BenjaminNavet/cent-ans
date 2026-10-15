//! glTF figures of an army (leader + escort by composition, siege camp) or of a fleet at sea,
//! in the faction colours. Port of `ModelLibrary.dress_army_marker` / `escort_models` /
//! `tint_banner` (legacy glTF path of `army_figures.gd`: the skinned instanced figures of the
//! Godot map are not ported) and of the fleet part of `ArmyFigures` (cog per 900 men, at most 3).
//! Layout is pure and tested; the scenes are `WorldAssetRoot`s loaded from `game/assets/models/`.

use std::collections::HashMap;

use bevy::gltf::GltfMaterialName;
use bevy::prelude::*;
use data_model::UnitCategory;

/// Leader / escort scale (`ModelLibrary.ARMY_SCALE`).
pub const ARMY_SCALE: f32 = 3.8;
/// Ships (`ArmyFigures.SHIP_SCALE`).
pub const SHIP_SCALE: f32 = 5.5;
pub const SIEGE_CAMP_SCALE: f32 = ARMY_SCALE * 0.9;
/// Height of the figures in marker-local units (leader with its banner), for plates and picking.
pub const FIGURE_HEIGHT: f32 = 4.4;
/// Escort figures: `[max men (exclusive), count]` (`ModelLibrary.ESCORT_BY_MEN`).
pub const ESCORT_BY_MEN: [(u32, usize); 3] = [(400, 2), (1200, 3), (u32::MAX, 4)];
/// Escort places (the group looks at +X).
pub const ESCORT_SLOTS: [Vec3; 4] = [
    Vec3::new(-1.5, 0.0, 1.2),
    Vec3::new(-1.1, 0.0, -1.5),
    Vec3::new(-2.9, 0.0, 0.2),
    Vec3::new(-2.7, 0.0, 1.9),
];
pub const LEADER_SLOT: Vec3 = Vec3::new(0.6, 0.0, 0.0);
pub const SIEGE_CAMP_SLOT: Vec3 = Vec3::new(3.2, 0.0, 2.2);
/// Ship places (`SHIP_SLOTS`, marker frame).
pub const SHIP_SLOTS: [Vec2; 3] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(-7.5, 5.0),
    Vec2::new(-8.0, -5.5),
];
pub const MEN_PER_SHIP: u32 = 900;
pub const MAX_SHIPS: usize = 3;
/// Name of the livery material in the models (`ModelLibrary.tint_banner`).
pub const LIVERY_MATERIAL: &str = "Banner";

/// One glTF scene of the group.
#[derive(Debug, Clone, PartialEq)]
pub struct FigureSpec {
    /// Asset path relative to the asset root.
    pub path: &'static str,
    pub offset: Vec3,
    pub scale: f32,
    pub yaw: f32,
}

/// What an army looks like.
#[derive(Debug, Clone, PartialEq)]
pub struct Look<'a> {
    pub categories: &'a [UnitCategory],
    pub men: u32,
    pub sieging: bool,
    /// Camped in the field (not in a settlement, not moving) shows nothing extra in this port.
    pub at_sea: bool,
}

fn model_path(name: &str) -> &'static str {
    match name {
        "army" => "models/army.glb",
        "army_archer" => "models/army_archer.glb",
        "army_foot" => "models/army_foot.glb",
        "siege_camp" => "models/siege_camp.glb",
        _ => "models/fleet/cog.glb",
    }
}

/// Escort models by composition (`ModelLibrary.escort_models`, same arithmetic: largest
/// remainder, foot first). `categories` has one entry per regiment.
pub fn escort_models(categories: &[UnitCategory], men: u32) -> Vec<&'static str> {
    let order = ["army_foot", "army_archer", "army"];
    let mut weights: HashMap<&str, usize> = order.iter().map(|m| (*m, 0)).collect();
    for category in categories {
        let name = match category {
            UnitCategory::Ranged => "army_archer",
            UnitCategory::Cavalry => "army",
            _ => "army_foot",
        };
        *weights.entry(name).or_default() += 1;
    }
    let count = ESCORT_BY_MEN
        .iter()
        .find(|(limit, _)| men < *limit)
        .map_or(0, |(_, n)| *n);
    let total: usize = weights.values().sum();
    if total == 0 {
        return vec!["army_foot"; count];
    }
    let mut shares: HashMap<&str, usize> = HashMap::new();
    let mut remaining = count as isize;
    for name in order {
        let share = (weights[name] as f32 * count as f32 / total as f32).floor() as usize;
        shares.insert(name, share);
        remaining -= share as isize;
    }
    for name in order {
        if remaining <= 0 {
            break;
        }
        let exact = weights[name] as f32 * count as f32 / total as f32;
        if weights[name] > 0 && exact - shares[name] as f32 > 0.0 {
            *shares.get_mut(name).unwrap() += 1;
            remaining -= 1;
        }
    }
    let mut result: Vec<&'static str> = Vec::new();
    for name in order {
        for _ in 0..shares[name] {
            result.push(name);
        }
    }
    while result.len() < count {
        result.push("army_foot");
    }
    result
}

/// Deterministic jitter in `[-0.12, 0.12]` for escort `index` of the army `seed`.
fn jitter(seed: u32, index: usize) -> f32 {
    let h = seed
        .wrapping_mul(2654435761)
        .wrapping_add((index as u32).wrapping_mul(40503))
        % 1000;
    (h as f32 / 1000.0 - 0.5) * 0.24
}

/// Figures of an army, in the marker frame.
pub fn layout(look: &Look, seed: u32) -> Vec<FigureSpec> {
    if look.at_sea {
        let ships = (look.men.div_ceil(MEN_PER_SHIP) as usize).clamp(1, MAX_SHIPS);
        return SHIP_SLOTS
            .iter()
            .take(ships)
            .map(|slot| FigureSpec {
                path: model_path("cog"),
                offset: Vec3::new(slot.x, 0.0, slot.y),
                scale: SHIP_SCALE,
                yaw: 0.0,
            })
            .collect();
    }
    let mut specs = vec![FigureSpec {
        path: model_path("army"),
        offset: LEADER_SLOT,
        scale: ARMY_SCALE,
        yaw: 0.0,
    }];
    for (i, name) in escort_models(look.categories, look.men)
        .into_iter()
        .take(ESCORT_SLOTS.len())
        .enumerate()
    {
        specs.push(FigureSpec {
            path: model_path(name),
            offset: ESCORT_SLOTS[i],
            scale: ARMY_SCALE * if name == "army" { 0.8 } else { 1.0 },
            yaw: jitter(seed, i),
        });
    }
    if look.sieging {
        specs.push(FigureSpec {
            path: model_path("siege_camp"),
            offset: SIEGE_CAMP_SLOT,
            scale: SIEGE_CAMP_SCALE,
            yaw: 0.0,
        });
    }
    specs
}

// --- Skinned troop (port of the layout of `army_figures.gd`) ---------------------------------

/// Height of a man (1.8 m) in marker-local units (`ArmyFigures.FIGURE_SCALE`).
pub const SKINNED_SCALE: f32 = 2.3;
/// Escort figures: `[max men (inclusive), count]` (`ArmyFigures.ESCORT_BY_MEN`).
pub const SKINNED_ESCORT_BY_MEN: [(u32, usize); 5] =
    [(300, 2), (800, 3), (1500, 4), (3000, 5), (u32::MAX, 6)];
pub const SKINNED_LEADER_SLOT: Vec2 = Vec2::new(1.5, 0.2);
pub const SKINNED_BEARER_SLOT: Vec2 = Vec2::new(0.1, -1.4);
/// Advance of the enlarged lord per unit of enlargement (his mount grows forwards).
pub const LORD_ADVANCE: f32 = 2.6;
const ESCORT_ROWS_X: [f32; 2] = [-1.4, -3.0];
const ESCORT_FILE_Z: [f32; 3] = [-1.3, 0.0, 1.3];

/// A regiment as the figures see it: figure family, variant and strength.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub kind: &'static str,
    pub variant: u32,
    pub strength: u32,
}

/// One skinned figure of the group, in the marker frame (the group looks at +X).
#[derive(Debug, Clone, PartialEq)]
pub struct SkinnedSlot {
    pub kind: &'static str,
    pub variant: u32,
    pub slot: Vec2,
    pub yaw: f32,
    pub size: f32,
}

/// Escort figures proportional to the composition (`escort_roster`: largest remainder, ties by
/// key; horse first, then foot, then archers at the tail of the column).
pub fn skinned_escort(roster: &[RosterEntry]) -> Vec<(&'static str, u32)> {
    let men: u32 = roster.iter().map(|r| r.strength).sum();
    let count = SKINNED_ESCORT_BY_MEN
        .iter()
        .find(|(limit, _)| men <= *limit)
        .map_or(0, |(_, n)| *n);
    let mut weights: Vec<((&'static str, u32), u32)> = Vec::new();
    for entry in roster {
        let key = (entry.kind, entry.variant);
        let weight = entry.strength.max(1);
        match weights.iter_mut().find(|(k, _)| *k == key) {
            Some((_, w)) => *w += weight,
            None => weights.push((key, weight)),
        }
    }
    if weights.is_empty() {
        return vec![("infantry", 0); count];
    }
    let total: u32 = weights.iter().map(|(_, w)| *w).sum();
    let exact = |w: u32| w as f32 * count as f32 / total as f32;
    // Heaviest first, then by key.
    weights.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut shares: Vec<usize> = weights
        .iter()
        .map(|(_, w)| exact(*w).floor() as usize)
        .collect();
    let mut remaining = count as isize - shares.iter().sum::<usize>() as isize;
    let mut by_remainder: Vec<usize> = (0..weights.len()).collect();
    by_remainder.sort_by(|a, b| {
        let ra = exact(weights[*a].1) - shares[*a] as f32;
        let rb = exact(weights[*b].1) - shares[*b] as f32;
        if (ra - rb).abs() < 1e-5 {
            weights[*a].0.cmp(&weights[*b].0)
        } else {
            rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    for i in by_remainder {
        if remaining <= 0 {
            break;
        }
        shares[i] += 1;
        remaining -= 1;
    }
    let order = |kind: &str| match kind {
        "cavalry" => 0,
        "infantry" => 1,
        "archer" => 2,
        _ => 3,
    };
    let mut keyed: Vec<((&'static str, u32), usize)> = weights
        .iter()
        .map(|(k, _)| *k)
        .zip(shares.iter().copied())
        .collect();
    keyed.sort_by(|a, b| order(a.0.0).cmp(&order(b.0.0)).then(a.0.cmp(&b.0)));
    keyed
        .into_iter()
        .flat_map(|(key, n)| std::iter::repeat_n(key, n))
        .collect()
}

/// Place of the enlarged lord, advanced so that his mount does not cover the escort.
pub fn lord_slot(lord_scale: f32) -> Vec2 {
    SKINNED_LEADER_SLOT + Vec2::new(LORD_ADVANCE * (lord_scale - 1.0), 0.0)
}

/// Skinned figures of a troop: the lord (or leader and bearer) and the escort.
pub fn skinned_layout(roster: &[RosterEntry], lord_scale: f32) -> Vec<SkinnedSlot> {
    let mut slots = Vec::new();
    let lord = lord_scale > 1.001;
    if lord {
        slots.push(SkinnedSlot {
            kind: "cavalry",
            variant: 0,
            slot: lord_slot(lord_scale),
            yaw: 0.0,
            size: lord_scale,
        });
    } else {
        slots.push(SkinnedSlot {
            kind: "cavalry",
            variant: 0,
            slot: SKINNED_LEADER_SLOT,
            yaw: 0.0,
            size: 1.0,
        });
        slots.push(SkinnedSlot {
            kind: "infantry",
            variant: 0,
            slot: SKINNED_BEARER_SLOT,
            yaw: 0.0,
            size: 1.0,
        });
    }
    for (i, (kind, variant)) in skinned_escort(roster).into_iter().enumerate() {
        let row = i / ESCORT_FILE_Z.len();
        let file = i % ESCORT_FILE_Z.len();
        if row >= ESCORT_ROWS_X.len() {
            break;
        }
        let jitter = ((i * 37 + 11) % 17) as f32 / 17.0 - 0.5;
        let mut slot = Vec2::new(
            ESCORT_ROWS_X[row] + jitter * 0.4,
            ESCORT_FILE_Z[file] + jitter * 0.3,
        );
        if kind == "cavalry" {
            slot.x -= 0.6;
        }
        slots.push(SkinnedSlot {
            kind,
            variant,
            slot,
            yaw: jitter * 0.15,
            size: 1.0,
        });
    }
    slots
}

/// Faction colour of the livery of a figure scene (read by [`tint_liveries`]).
#[derive(Component, Debug, Clone, Copy)]
pub struct Livery(pub Color);

/// A figure scene of an army (root of the glTF scene).
#[derive(Component, Debug, Clone)]
pub struct ArmyFigure {
    pub path: &'static str,
}

/// Mesh primitive already looked at by [`tint_liveries`].
#[derive(Component)]
pub struct Tinted;

#[derive(Resource, Default)]
pub struct LiveryCache {
    cache: HashMap<(AssetId<StandardMaterial>, [u8; 3]), Handle<StandardMaterial>>,
    /// Livery materials replaced since the start (dump).
    pub tinted: usize,
}

fn rgb_key(color: Color) -> [u8; 3] {
    let c = color.to_srgba();
    [
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8,
    ]
}

/// Replaces the "Banner" material of every new figure mesh by a copy in the faction colour
/// (`tint_banner`). The livery is found on an ancestor.
#[allow(clippy::type_complexity)]
pub fn tint_liveries(
    mut commands: Commands,
    new: Query<(Entity, &GltfMaterialName, &MeshMaterial3d<StandardMaterial>), Without<Tinted>>,
    parents: Query<&ChildOf>,
    liveries: Query<&Livery>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<LiveryCache>,
) {
    for (entity, name, material) in &new {
        commands.entity(entity).insert(Tinted);
        if name.0 != LIVERY_MATERIAL {
            continue;
        }
        let mut cursor = entity;
        let mut color = None;
        for _ in 0..12 {
            if let Ok(livery) = liveries.get(cursor) {
                color = Some(livery.0);
                break;
            }
            match parents.get(cursor) {
                Ok(parent) => cursor = parent.parent(),
                Err(_) => break,
            }
        }
        let Some(color) = color else {
            continue;
        };
        let key = (material.0.id(), rgb_key(color));
        let handle = match cache.cache.get(&key) {
            Some(handle) => handle.clone(),
            None => {
                let Some(source) = materials.get(&material.0) else {
                    continue;
                };
                let mut tinted = source.clone();
                tinted.base_color = color;
                let handle = materials.add(tinted);
                cache.cache.insert(key, handle.clone());
                handle
            }
        };
        commands.entity(entity).insert(MeshMaterial3d(handle));
        cache.tinted += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use UnitCategory::*;

    #[test]
    fn escort_counts_follow_the_men() {
        assert_eq!(escort_models(&[Infantry], 300).len(), 2);
        assert_eq!(escort_models(&[Infantry], 800).len(), 3);
        assert_eq!(escort_models(&[Infantry], 5000).len(), 4);
    }

    #[test]
    fn escort_follows_the_composition() {
        let mix = escort_models(&[Infantry, Ranged, Cavalry, Infantry], 5000);
        assert_eq!(mix.len(), 4);
        assert!(mix.contains(&"army_archer") && mix.contains(&"army"));
        assert_eq!(mix.iter().filter(|m| **m == "army_foot").count(), 2);
        assert_eq!(escort_models(&[Siege], 100), vec!["army_foot"; 2]);
        assert_eq!(escort_models(&[], 100), vec!["army_foot"; 2]);
    }

    #[test]
    fn army_layout_has_leader_escort_and_camp() {
        let look = Look {
            categories: &[Infantry, Cavalry],
            men: 1500,
            sieging: true,
            at_sea: false,
        };
        let specs = layout(&look, 7);
        assert_eq!(specs[0].path, "models/army.glb");
        assert_eq!(specs.len(), 1 + 4 + 1);
        assert_eq!(specs.last().unwrap().path, "models/siege_camp.glb");
        assert_eq!(layout(&look, 7), specs);
    }

    #[test]
    fn fleet_is_one_ship_per_900_men_up_to_three() {
        let fleet = |men| {
            layout(
                &Look {
                    categories: &[],
                    men,
                    sieging: false,
                    at_sea: true,
                },
                1,
            )
            .len()
        };
        assert_eq!(fleet(100), 1);
        assert_eq!(fleet(901), 2);
        assert_eq!(fleet(20000), 3);
    }

    #[test]
    fn jitter_is_small_and_stable() {
        for i in 0..6 {
            assert!(jitter(99, i).abs() <= 0.12 + 1e-6);
        }
        assert_eq!(jitter(5, 2), jitter(5, 2));
    }

    fn entry(kind: &'static str, variant: u32, strength: u32) -> RosterEntry {
        RosterEntry {
            kind,
            variant,
            strength,
        }
    }

    #[test]
    fn skinned_escort_counts_and_order() {
        assert_eq!(skinned_escort(&[entry("infantry", 0, 200)]).len(), 2);
        assert_eq!(skinned_escort(&[entry("infantry", 0, 1000)]).len(), 4);
        assert_eq!(skinned_escort(&[entry("infantry", 0, 9000)]).len(), 6);
        assert_eq!(skinned_escort(&[]).len(), 2);
        let mix = skinned_escort(&[
            entry("archer", 0, 1500),
            entry("infantry", 1, 1500),
            entry("cavalry", 0, 1500),
        ]);
        assert_eq!(
            mix,
            vec![
                ("cavalry", 0),
                ("cavalry", 0),
                ("infantry", 1),
                ("infantry", 1),
                ("archer", 0),
                ("archer", 0)
            ]
        );
    }

    #[test]
    fn skinned_layout_lord_or_bearer() {
        let roster = [entry("infantry", 0, 500)];
        let lord = skinned_layout(&roster, 1.8);
        assert_eq!(lord.len(), 1 + 3);
        assert!((lord[0].slot.x - (1.5 + 2.6 * 0.8)).abs() < 1e-5);
        assert_eq!(lord[0].size, 1.8);
        let plain = skinned_layout(&roster, 1.0);
        assert_eq!(plain.len(), 2 + 3);
        assert_eq!(plain[1].kind, "infantry");
    }
}
