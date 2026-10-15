//! Arms and shields left on the ground by routing regiments and by the wounded.
//! Port of game/scripts/battle/battle_dropped_arms.gd: one instanced draw per part of each kind
//! of object (swords, shields, polearms, bows, crossbows), ring buffers capped by
//! `dropped_arms` of `data/fx/battle_gore.json`, deterministic draws (same regiment and rank give
//! the same objects). Meshes are a few boxes. Not ported: the fade by distance (`far_m` hides the
//! whole layer when the camera is farther from every object instead).

use std::collections::HashMap;

use bevy::prelude::*;

use super::corpses::hash4;
use super::instancing::{InstancedBatch, MaterialParams};
use super::library::FigureLibrary;
use crate::app::AppState;
use crate::battle::scene::BattleSession;

const STEEL: [f32; 3] = [0.62, 0.63, 0.66];
const DARK_STEEL: [f32; 3] = [0.35, 0.36, 0.38];
const WOOD: [f32; 3] = [0.42, 0.29, 0.17];
const LEATHER: [f32; 3] = [0.3, 0.2, 0.12];

/// Objects dropped by an animation style: kind and share of the figures that leave one.
pub fn by_style(style: &str) -> &'static [(&'static str, f32)] {
    match style {
        "sword" => &[("sword", 0.8), ("shield", 0.5)],
        "militia" => &[("polearm", 0.9)],
        "pike" => &[("polearm", 0.95)],
        "bow" => &[("bow", 0.8)],
        "crossbow" => &[("crossbow", 0.85)],
        _ => &[],
    }
}

/// A request to leave arms at a place.
pub struct ArmDrop {
    pub style: String,
    pub at: Vec3,
    pub facing: f32,
    pub seed: u32,
}

/// A rout: figures of a regiment throw their arms.
pub struct RoutDrop {
    pub style: String,
    pub poses: Vec<[f32; 4]>,
    pub side: usize,
    pub regiment: u32,
}

/// One box of a mesh: centre, half sizes, yaw.
type BoxSpec = ([f32; 3], [f32; 3], f32);

/// Parts of a kind: colour and boxes (flat on the ground, length along +Z).
fn parts(kind: &str) -> Vec<([f32; 3], Vec<BoxSpec>)> {
    match kind {
        "sword" => vec![
            (STEEL, vec![([0.0, 0.008, 0.25], [0.022, 0.006, 0.4], 0.0)]),
            (
                DARK_STEEL,
                vec![
                    ([0.0, 0.012, -0.16], [0.11, 0.012, 0.014], 0.0),
                    ([0.0, 0.014, -0.32], [0.024, 0.02, 0.022], 0.0),
                ],
            ),
            (
                LEATHER,
                vec![([0.0, 0.012, -0.24], [0.016, 0.014, 0.07], 0.0)],
            ),
        ],
        // Painted face up: the face takes the livery colour (see `SHIELD_FACE`).
        "shield" => vec![(WOOD, vec![([0.0, 0.004, 0.02], [0.27, 0.006, 0.32], 0.0)])],
        "shield_face" => vec![(
            [1.0, 1.0, 1.0],
            vec![
                ([0.0, 0.02, -0.06], [0.25, 0.014, 0.2], 0.0),
                ([0.0, 0.02, 0.2], [0.17, 0.014, 0.08], 0.0),
                ([0.0, 0.02, 0.32], [0.07, 0.014, 0.05], 0.0),
            ],
        )],
        "polearm" => vec![
            (WOOD, vec![([0.0, 0.02, 0.0], [0.018, 0.018, 1.1], 0.0)]),
            (STEEL, vec![([0.0, 0.02, 1.22], [0.03, 0.01, 0.14], 0.0)]),
        ],
        "bow" => vec![(
            WOOD,
            vec![
                ([0.0, 0.012, 0.0], [0.018, 0.012, 0.3], 0.0),
                ([0.05, 0.012, 0.58], [0.014, 0.01, 0.3], 0.18),
                ([0.05, 0.012, -0.58], [0.014, 0.01, 0.3], -0.18),
            ],
        )],
        "crossbow" => vec![
            (WOOD, vec![([0.0, 0.03, 0.0], [0.03, 0.03, 0.4], 0.0)]),
            (
                DARK_STEEL,
                vec![
                    ([0.0, 0.03, 0.34], [0.36, 0.015, 0.02], 0.0),
                    ([0.0, 0.02, 0.44], [0.05, 0.01, 0.06], 0.0),
                ],
            ),
        ],
        _ => Vec::new(),
    }
}

fn build_mesh(boxes: &[BoxSpec]) -> Mesh {
    let mut mesh: Option<Mesh> = None;
    for (center, half, yaw) in boxes {
        let part = Mesh::from(Cuboid::new(half[0] * 2.0, half[1] * 2.0, half[2] * 2.0))
            .rotated_by(Quat::from_rotation_y(*yaw))
            .translated_by(Vec3::new(center[0], center[1], center[2]));
        match mesh.as_mut() {
            Some(m) => m.merge(&part).expect("same layout"),
            None => mesh = Some(part),
        }
    }
    mesh.expect("a part has a box")
}

fn linear(rgb: [f32; 3]) -> [f32; 4] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue, 1.0]
}

struct Layer {
    /// Batches (one per part), all sharing the record ring buffer of the kind.
    entities: Vec<Entity>,
    records: Vec<u8>,
    next: usize,
    count: usize,
    version: u64,
    dirty: bool,
}

#[cfg(test)]
const KINDS: [&str; 6] = [
    "sword",
    "shield",
    "shield_face",
    "polearm",
    "bow",
    "crossbow",
];

#[derive(Resource, Default)]
pub struct DroppedArms {
    pending: Vec<ArmDrop>,
    routs: Vec<RoutDrop>,
    layers: HashMap<(&'static str, usize), Layer>,
    pub dropped: usize,
    colors: [[f32; 3]; 2],
}

impl DroppedArms {
    /// Arm left by a wounded man (first object of the style only).
    pub fn push(&mut self, drop: ArmDrop) {
        self.pending.push(drop);
    }

    pub fn push_rout(&mut self, rout: RoutDrop) {
        self.routs.push(rout);
    }

    pub fn shown(&self) -> usize {
        self.layers.values().map(|l| l.count).sum()
    }
}

fn place(
    items: &[(&'static str, f32)],
    at: Vec3,
    facing: f32,
    seed: u64,
    scatter: f32,
    out: &mut Vec<(&'static str, [f32; 6])>,
) {
    for (k, (kind, share)) in items.iter().enumerate() {
        let h = hash4(seed * 31 + k as u64);
        if h[0] >= *share {
            continue;
        }
        // Thrown in front of the feet, a little aside, flat in any direction.
        let offset = Vec3::new((h[1] - 0.5) * scatter, 0.03, (h[2] - 0.5) * scatter);
        let yaw = facing + h[3] * std::f32::consts::TAU;
        let at = at + offset;
        let name: &'static str = if *kind == "shield" {
            // Face up (painted, on the board) or turned over (board only).
            if h[2] > 0.5 {
                out.push((
                    "shield_face",
                    [at.x, at.y, at.z, yaw.to_degrees(), 1.0, 0.0],
                ));
            }
            "shield"
        } else {
            kind
        };
        out.push((name, [at.x, at.y, at.z, yaw.to_degrees(), 1.0, 0.0]));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn flush_arms(
    mut commands: Commands,
    mut arms: ResMut<DroppedArms>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<&mut InstancedBatch>,
    library: Option<Res<FigureLibrary>>,
    session: Option<Res<BattleSession>>,
) {
    let Some(library) = library else {
        return;
    };
    let cfg = library.fx.gore.dropped_arms.clone();
    if let Some(session) = session {
        arms.colors = session.colors;
    }
    let drops = std::mem::take(&mut arms.pending);
    let routs = std::mem::take(&mut arms.routs);
    if drops.is_empty() && routs.is_empty() && !arms.layers.values().any(|l| l.dirty) {
        return;
    }
    let mut placed: Vec<(&'static str, usize, [f32; 6])> = Vec::new();
    let mut scratch = Vec::new();
    for drop in drops {
        let items = by_style(&drop.style);
        if let Some(first) = items.first() {
            scratch.clear();
            place(
                &[*first],
                drop.at,
                drop.facing,
                u64::from(drop.seed),
                cfg.scatter_m,
                &mut scratch,
            );
            placed.extend(scratch.iter().map(|(k, r)| (*k, 0usize, *r)));
        }
    }
    for rout in routs {
        let items = by_style(&rout.style);
        let n = rout.poses.len();
        if items.is_empty() || n == 0 {
            continue;
        }
        let cap = n.min(cfg.max_per_rout);
        let step = n as f32 / cap as f32;
        for j in 0..cap {
            let slot = ((j as f32 * step) as usize).min(n - 1);
            let pose = rout.poses[slot];
            scratch.clear();
            place(
                items,
                Vec3::new(pose[0], pose[1], pose[2]),
                pose[3],
                u64::from(rout.regiment) * 7919 + slot as u64,
                cfg.scatter_m,
                &mut scratch,
            );
            placed.extend(scratch.iter().map(|(k, r)| (*k, rout.side, *r)));
        }
    }
    let colors = arms.colors;
    for (kind, side, record) in placed {
        let side = if kind == "shield_face" { side } else { 0 };
        let layer = arms.layers.entry((kind, side)).or_insert_with(|| {
            let entities = parts(kind)
                .into_iter()
                .map(|(rgb, boxes)| {
                    let color = if kind == "shield_face" {
                        colors[side]
                    } else {
                        rgb
                    };
                    let metallic = if rgb == STEEL || rgb == DARK_STEEL {
                        0.4
                    } else {
                        0.0
                    };
                    commands
                        .spawn((
                            Name::new(format!("Dropped_{kind}_{side}")),
                            InstancedBatch {
                                mesh_handle: meshes.add(build_mesh(&boxes)),
                                records: std::sync::Arc::new(Vec::new()),
                                count: 0,
                                texture_handle: None,
                                params: MaterialParams {
                                    base_color: linear(color),
                                    alpha_cutoff: 0.0,
                                    roughness: 0.72,
                                    metallic,
                                    double_sided: false,
                                },
                                version: 0,
                            },
                            DespawnOnExit(AppState::Battle),
                        ))
                        .id()
                })
                .collect();
            Layer {
                entities,
                records: Vec::new(),
                next: 0,
                count: 0,
                version: 0,
                dirty: false,
            }
        });
        let cap = cfg.max_per_kind.max(1);
        let bytes: Vec<u8> = record.iter().flat_map(|v| v.to_le_bytes()).collect();
        if layer.count < cap {
            layer.records.extend_from_slice(&bytes);
            layer.count += 1;
        } else {
            layer.records[layer.next * 24..layer.next * 24 + 24].copy_from_slice(&bytes);
        }
        layer.next = (layer.next + 1) % cap;
        layer.dirty = true;
        arms.dropped += 1;
    }
    for layer in arms.layers.values_mut() {
        if !layer.dirty {
            continue;
        }
        layer.dirty = false;
        layer.version += 1;
        let shared = std::sync::Arc::new(layer.records.clone());
        for entity in &layer.entities {
            if let Ok(mut batch) = batches.get_mut(*entity) {
                batch.records = shared.clone();
                batch.count = layer.count as u32;
                batch.version = layer.version;
            }
        }
    }
}

pub fn clear_arms(mut arms: ResMut<DroppedArms>) {
    *arms = DroppedArms::default();
}

pub struct ArmsPlugin;

impl Plugin for ArmsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DroppedArms>()
            .add_systems(
                Update,
                flush_arms
                    .run_if(in_state(AppState::Battle))
                    .after(super::regiments::update_regiments),
            )
            .add_systems(OnExit(AppState::Battle), clear_arms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_parts_and_a_mesh() {
        for kind in KINDS {
            let p = parts(kind);
            assert!(!p.is_empty(), "{kind}");
            for (_, boxes) in p {
                assert!(build_mesh(&boxes).count_vertices() > 0);
            }
        }
    }

    #[test]
    fn placement_is_deterministic() {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        place(by_style("sword"), Vec3::ZERO, 0.0, 12, 0.9, &mut a);
        place(by_style("sword"), Vec3::ZERO, 0.0, 12, 0.9, &mut b);
        assert_eq!(a, b);
    }
}
