//! Soldier rendering: every living figure of the simulation, skinned and animated on the GPU,
//! positions and facings read from `sim_battle::BattleSim::soldier_poses` and blended between
//! the two latest fixed steps.
//!
//! Ports game/scripts/battle/battle_soldiers.gd (update loop, level of detail),
//! battle_skinned.gd (manifest, meshes, bone textures, state to clip tables),
//! battle_unit_looks.gd, battle_cavalry_gaits.gd, battle_impostors.gd (billboards beyond
//! 300 m, procedural silhouettes instead of a baked atlas), battle_banner_layer.gd and
//! battle_unit_markers.gd. Not ported: corpses and wounded (battle_soldiers.gd gore layers),
//! blood, severed limbs, dropped arms (battle_gore.gd, battle_dropped_arms.gd), battle_formation_outline.gd, battle_effects.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod anim;
pub mod arms;
pub mod banners;
pub mod corpses;
pub mod engage;
pub mod figures;
pub mod fine_maps;
pub mod follow;
pub mod gaits;
pub mod heraldry;
pub mod instancing;
pub mod library;
pub mod looks;
pub mod markers;
pub mod oriented;
pub mod regiments;
pub mod settings;
pub mod skinned;
pub mod smoke;
pub mod standards;
pub mod volleys;
pub mod wind;

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::json;
use sim_battle::BattleSim;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use banners::BattleBannersPlugin;
use instancing::InstancingPlugin;
use library::FigureLibrary;
use markers::BattleMarkersPlugin;
use regiments::{Lod, Regiments};
use skinned::SkinnedPlugin;

/// A figure that moved more than this between two steps is teleporting: no blend (m).
const TELEPORT_M: f32 = 8.0;

/// Counters of the soldier renderer (dump section `battle_units`, and `battle`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SoldierStats {
    /// Soldiers handed to the GPU this frame (all levels of detail and impostors).
    pub drawn: usize,
    /// `[attacker foot, attacker mounted, defender foot, defender mounted]`.
    pub per_batch: [usize; 4],
    /// Soldiers per level of detail: full, medium, far, impostor.
    pub lod: [usize; 4],
    /// Regiments drawn.
    pub regiments: usize,
}

/// One figure: `(x, y, z, facing)` as given by the simulation.
type Pose = [f32; 4];

/// Poses of the two latest simulation steps, by regiment id.
#[derive(Resource, Default)]
pub struct PoseFrames {
    ticks: Option<u64>,
    /// Figures drawn per simulated man, fixed when the battle starts (`unit_size` capped by
    /// `max_figures`, see [`figure_scale`]).
    pub scale: f64,
    prev: HashMap<u32, Vec<Pose>>,
    cur: HashMap<u32, Vec<Pose>>,
    followers: follow::Followers,
    /// Figures turned towards an enemy figure at the last step (melee, lot B2).
    pub engaged: usize,
}

impl PoseFrames {
    /// Reads the poses of the step `ticks`; the previous ones are kept for blending when the
    /// simulation advanced by a few steps only.
    fn refresh(
        &mut self,
        sim: &BattleSim,
        ticks: u64,
        unit_size: f32,
        max_figures: u32,
        engage_range_m: f32,
    ) {
        // A new battle restarts the clock: pick the scale again then, never mid-battle (the
        // head counts would change as men fall), as `battle_scene.gd` does once at the start.
        if self.scale <= 0.0 || self.ticks.is_some_and(|old| ticks < old) {
            let men: u32 = drawn_units(sim).map(|u| u.soldiers()).sum();
            self.scale = figure_scale(unit_size, men, max_figures);
        }
        let scale = self.scale;
        let advanced = self
            .ticks
            .is_some_and(|old| ticks > old && ticks - old <= 3);
        let old = std::mem::take(&mut self.cur);
        self.prev = if advanced { old } else { HashMap::new() };
        // A jump of the clock (new battle, replay seek, long stall) puts every figure on its place.
        let dt = match self.ticks {
            Some(old) if advanced => (ticks - old) as f32 * sim_battle::DT as f32,
            _ => 0.0,
        };
        let followers = &mut self.followers;
        let mut places: HashMap<u32, Vec<Pose>> = drawn_units(sim)
            .map(|unit| {
                let places: Vec<Pose> = sim
                    .soldier_poses(unit, scale)
                    .into_iter()
                    .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32])
                    .collect();
                (unit.id, places)
            })
            .collect();
        // Melee (B2): the men in contact face the nearest enemy figure.
        if engage_range_m > 0.0 {
            let pairs = engage::pairs(sim, engage_range_m);
            self.engaged = engage::face_nearest(&mut places, &pairs, engage_range_m);
        }
        self.cur = drawn_units(sim)
            .filter_map(|unit| {
                let places = places.remove(&unit.id)?;
                let rigid = unit.on_wall || unit.climbing.is_some();
                Some((unit.id, followers.follow(unit.id, &places, dt, rigid)))
            })
            .collect();
        self.ticks = Some(ticks);
    }
}

/// `BattleScene._figure_scale` / `capped_figure_scale`: the chosen « Taille des unités »
/// multiplier, lowered so that the total of figures for `men` simulated men fits in « Figurines
/// maximum » (0 = no cap); the core clamps it to 0.25..4.
pub fn figure_scale(unit_size: f32, men: u32, max_figures: u32) -> f64 {
    let size = f64::from(unit_size);
    let capped = if men == 0 || max_figures == 0 {
        size
    } else {
        size.min(f64::from(max_figures) / f64::from(men))
    };
    capped.clamp(0.25, 4.0)
}

/// Loading failure of the figure library (reported in the dump).
#[derive(Resource, Default)]
pub struct UnitsReport {
    pub library_error: Option<String>,
}

pub struct BattleUnitsPlugin;

impl Plugin for BattleUnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InstancingPlugin,
            SkinnedPlugin,
            BattleBannersPlugin,
            BattleMarkersPlugin,
            corpses::CorpsesPlugin,
            arms::ArmsPlugin,
            oriented::OrientedPlugin,
            volleys::VolleysPlugin,
            smoke::SmokePlugin,
            standards::StandardsPlugin,
            heraldry::HeraldryPlugin,
            fine_maps::FineMapsPlugin,
        ))
        .init_resource::<SoldierStats>()
        .init_resource::<PoseFrames>()
        .init_resource::<Regiments>()
        .init_resource::<UnitsReport>()
        .register_dump(dump_units)
        .add_systems(
            Update,
            (
                load_library.run_if(resource_added::<BattleSession>),
                wind::init_wind.run_if(resource_added::<BattleSession>),
                dev_view.run_if(resource_exists::<BattleSession>),
                dev_speed.run_if(resource_exists::<BattleSession>),
                regiments::update_regiments
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .run_if(resource_exists::<FigureLibrary>),
            )
                .chain()
                .after(crate::battle::scene::StepSet),
        )
        .add_systems(OnExit(AppState::Battle), clear_units);
    }
}

fn load_library(
    mut commands: Commands,
    mut report: ResMut<UnitsReport>,
    existing: Option<Res<FigureLibrary>>,
) {
    if existing.is_some() {
        return;
    }
    let loaded = crate::paths::find_root().and_then(|root| {
        FigureLibrary::load(
            crate::paths::asset_root(&root),
            &crate::paths::data_dir(&root),
        )
    });
    match loaded {
        Ok(library) => commands.insert_resource(library),
        Err(error) => {
            error!("battle units: {error}");
            report.library_error = Some(error);
        }
    }
}

/// Dev hook: `CENT_ANS_UNITS_VIEW=<regiment id>,<distance m>[,<yaw rad>]` frames a regiment from
/// the given distance once the battle opens (captures and dumps of a close view).
fn dev_view(
    mut done: Local<bool>,
    mut session: ResMut<BattleSession>,
    state: Option<ResMut<crate::battle::scene::camera::BattleCameraState>>,
) {
    if *done {
        return;
    }
    let Ok(spec) = std::env::var("CENT_ANS_UNITS_VIEW") else {
        *done = true;
        return;
    };
    let Some(mut state) = state else {
        return;
    };
    let mut parts = spec.split(',');
    let id: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let distance: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(40.0);
    let sim = &*session.sim_mut();
    let Some(unit) = sim.units().iter().find(|u| u.id == id) else {
        return;
    };
    state.target = Vec3::new(unit.x as f32, 0.0, unit.z as f32);
    state.distance = distance;
    state.target_distance = distance;
    if let Some(yaw) = parts.next().and_then(|v| v.parse().ok()) {
        state.yaw = yaw;
    }
    *done = true;
}

/// Dev hook: `CENT_ANS_UNITS_SPEED=<factor>[,<ai 0|1>]` runs the battle faster (and, with `1`,
/// turns on the battle AI of both sides) so that a dump taken a few hundred frames later shows
/// contact: bodies, wounded, dropped arms, volleys, smoke.
fn dev_speed(mut done: Local<bool>, mut session: ResMut<BattleSession>) {
    if *done {
        return;
    }
    *done = true;
    let Ok(spec) = std::env::var("CENT_ANS_UNITS_SPEED") else {
        return;
    };
    let mut parts = spec.split(',');
    if let Some(speed) = parts.next().and_then(|v| v.parse::<f64>().ok()) {
        session.speed = speed;
    }
    if parts.next() == Some("1") {
        let sim = session.sim_mut();
        sim.set_ai(sim_battle::SideId::Attacker, true);
        sim.set_ai(sim_battle::SideId::Defender, true);
    }
}

fn clear_units(
    mut stats: ResMut<SoldierStats>,
    mut frames: ResMut<PoseFrames>,
    mut regiments: ResMut<Regiments>,
) {
    *stats = SoldierStats::default();
    *frames = PoseFrames::default();
    *regiments = Regiments::default();
}

fn dump_units(world: &World) -> (String, serde_json::Value) {
    let stats = world
        .get_resource::<SoldierStats>()
        .cloned()
        .unwrap_or_default();
    let regiments = world.get_resource::<Regiments>();
    let mut by_figure: std::collections::BTreeMap<String, usize> = Default::default();
    let mut by_state: std::collections::BTreeMap<String, usize> = Default::default();
    let mut by_lod = [0usize; 4];
    let mut samples = Vec::new();
    if let Some(regiments) = regiments {
        let mut all: Vec<_> = regiments.summaries().collect();
        all.sort_by_key(|r| r.0);
        for (id, figure, state, lod, drawn) in all {
            *by_figure.entry(figure.to_owned()).or_default() += drawn;
            *by_state.entry(state.to_owned()).or_default() += drawn;
            by_lod[lod.index()] += 1;
            if samples.len() < 8 {
                samples.push(json!({ "id": id, "figure": figure, "state": state, "lod": format!("{lod:?}"), "soldiers": drawn }));
            }
        }
    }
    let lod_names = [Lod::Full, Lod::Medium, Lod::Far, Lod::Impostor];
    let named = |counts: [usize; 4]| {
        lod_names
            .iter()
            .zip(counts)
            .map(|(l, n)| (format!("{l:?}").to_lowercase(), json!(n)))
            .collect::<serde_json::Map<_, _>>()
    };
    let library = world.get_resource::<FigureLibrary>();
    (
        "battle_units".into(),
        json!({
            "library": library.map(|l| json!({
                "figures_in_manifest": l.manifest.figures.len(),
                "unit_looks": l.looks.len(),
                "load_failures": l.failures,
            })),
            "library_error": world.get_resource::<UnitsReport>().and_then(|r| r.library_error.clone()),
            "regiments": stats.regiments,
            "regiments_with_lance": regiments.map(|r| r.lance_stats().0),
            "steel_occlusion_min": regiments.map(|r| r.lance_stats().1),
            "soldiers_drawn": stats.drawn,
            "soldiers_by_lod": named(stats.lod),
            "regiments_by_lod": named(by_lod),
            "soldiers_by_figure": by_figure,
            "soldiers_by_state": by_state,
            "per_batch": stats.per_batch,
            "samples": samples,
        }),
    )
}

/// Foot soldier: a 1.7 m capsule (rigid fallback, kept for other lots' tests).
pub fn foot_mesh() -> Mesh {
    Mesh::from(Capsule3d::new(0.22, 1.26)).translated_by(Vec3::Y * 0.85)
}

/// Horseman: body, legs and a rider (rigid fallback).
pub fn mounted_mesh() -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(0.55, 0.9, 1.9)).translated_by(Vec3::Y * 0.85);
    let legs = Mesh::from(Cuboid::new(0.4, 0.45, 1.4)).translated_by(Vec3::Y * 0.22);
    let rider = Mesh::from(Capsule3d::new(0.22, 0.9)).translated_by(Vec3::Y * 1.97);
    mesh.merge(&legs).expect("same layout");
    mesh.merge(&rider).expect("same layout");
    mesh
}

/// Regiments drawn as figures: field regiments with soldiers (not engines, not off-field reserve).
pub fn drawn_units(sim: &BattleSim) -> impl Iterator<Item = &sim_battle::Unit> {
    sim.units()
        .iter()
        .filter(|u| !u.synthetic && !u.reserve && u.category != data_model::UnitCategory::Siege)
}

/// Blends `prev` towards `cur` by `t`; a figure that jumped, or a regiment whose head count
/// changed, shows its current pose.
pub fn blend_poses(prev: Option<&[Pose]>, cur: &[Pose], t: f32) -> Vec<Pose> {
    let Some(prev) = prev.filter(|p| p.len() == cur.len() && t < 1.0) else {
        return cur.to_vec();
    };
    cur.iter()
        .zip(prev)
        .map(|(c, p)| {
            let jump = Vec3::new(c[0] - p[0], 0.0, c[2] - p[2]).length();
            if jump > TELEPORT_M {
                return *c;
            }
            let mut turn = c[3] - p[3];
            turn = (turn + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            [
                p[0] + (c[0] - p[0]) * t,
                p[1] + (c[1] - p[1]) * t,
                p[2] + (c[2] - p[2]) * t,
                p[3] + turn * t,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_interpolates_and_skips_jumps() {
        let prev = [[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]];
        let cur = [[1.0, 0.0, 2.0, 0.5], [50.0, 0.0, 0.0, 0.0]];
        let out = blend_poses(Some(&prev), &cur, 0.5);
        assert_eq!(out[0], [0.5, 0.0, 1.0, 0.25]);
        assert_eq!(out[1], cur[1], "a jump is not blended");
        assert_eq!(blend_poses(Some(&prev[..1]), &cur, 0.5), cur.to_vec());
        assert_eq!(blend_poses(None, &cur, 0.5), cur.to_vec());
    }

    #[test]
    fn unit_size_is_capped_by_the_figure_budget() {
        assert_eq!(figure_scale(1.5, 4000, 0), 1.5, "no cap");
        assert_eq!(figure_scale(1.5, 4000, 15_000), 1.5, "fits");
        assert!((figure_scale(2.5, 8000, 10_000) - 1.25).abs() < 1e-9);
        assert_eq!(figure_scale(2.5, 0, 100), 2.5);
        assert_eq!(figure_scale(1.0, 100_000, 1000), 0.25, "core floor");
    }

    #[test]
    fn blend_turns_the_short_way() {
        let prev = [[0.0, 0.0, 0.0, 3.0]];
        let cur = [[0.0, 0.0, 0.0, -3.0]];
        let out = blend_poses(Some(&prev), &cur, 0.5);
        assert!(out[0][3].abs() > 3.0, "{}", out[0][3]);
    }
}
