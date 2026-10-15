//! Siege battles: walls, gates, towers, engines, assault and wall-collapse effects, capture points.
//!
//! Ports game/scripts/battle/{battle_siege, siege_assault_fx, siege_engines_fx, wall_collapse_fx,
//! siege_fire_fx, siege_marks_fx, siege_health_bars, siege_capture_points}.gd. Owned by a single
//! lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Everything is drawn from the state of `sim-battle` and nothing here is a rule. One system
//! ([`read_frame`]) copies what the renderers need out of the simulation into [`SiegeFrame`]
//! after every step (pieces, engines, capture points, ladders, fires and the new `SiegeFx`
//! events); the other systems only read that resource.
//!
//! - `works` : the ring (wall pieces, gate, towers) and the houses of the town, per-piece damage
//!   tint, scars of the engine shots, collapse of a breached piece, ruins of burnt houses.
//! - `engines` : glTF models of the rams, siege towers and trebuchets / mangonels / bombards.
//! - `fx` : dust, debris, oil, fire and smoke particles, ladders of the climbing regiments.
//! - `overlay` : health bars of the engines and of the battered pieces, capture points.
//! - `historical` : scripted historical battles (`setup_historical`).
//!
//! Dump section `battle_siege`.
//!
//! Not ported: crew figures of the engines (`siege_crew_fx.gd`), the swing of the trebuchet arm
//! and recoil of the bombard, flying stones and bolts, ram beam swing, the kit buildings
//! (`BuildingKit`), the fire lights, the wall collapse's rubble physics (a timed sink and tilt
//! stands in), the capture stage cinematic (`battle_capture_stage.gd`).

pub mod animation;
pub mod crew;
pub mod engines;
pub mod fx;
pub mod historical;
pub mod kit_town;
pub mod overlay;
pub mod works;

use std::collections::BTreeMap;

use bevy::prelude::*;
use sim_battle::siege::House;
use sim_battle::{CapturePoint, FireState, SideId, SiegeEngineKind, SiegeFx, SiegeFxKind};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::{BattleSession, StepSet};

/// State of one wall piece.
#[derive(Debug, Clone, Copy, Default)]
pub struct PieceState {
    pub hp: f32,
    pub max_hp: f32,
    pub under_attack: bool,
    /// Top of the middle of the piece (health bar anchor).
    pub top: Vec3,
}

/// What kind of engine, and the model that draws it.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineKind {
    Ram,
    Tower,
    /// Static engine of a regiment (`trebuchet`, `mangonel`, `bombard`).
    Static(String),
}

impl EngineKind {
    pub fn model(&self) -> &str {
        match self {
            EngineKind::Ram => "ram",
            EngineKind::Tower => "siege_tower",
            EngineKind::Static(model) => model,
        }
    }
}

/// An engine unit on the field.
#[derive(Debug, Clone)]
pub struct EngineState {
    pub unit: u32,
    pub kind: EngineKind,
    pub side: SideId,
    pub pos: Vec3,
    /// Heading in the sim convention (`(cos, sin)` on x, z).
    pub facing: f32,
    pub hp: f32,
    pub max_hp: f32,
    /// Seconds before the next volley (core's `reload`) and the period of the regiment.
    pub reload: f32,
    pub reload_period: f32,
    /// The regiment is firing (`UnitState::Shooting`) and has ammunition.
    pub shooting: bool,
    pub ammo: u32,
}

/// One ladder of a climbing regiment.
#[derive(Debug, Clone, Copy)]
pub struct LadderState {
    pub unit: u32,
    pub index: usize,
    pub foot: Vec3,
    pub top: Vec3,
}

/// A burning or burnt house of the town.
#[derive(Debug, Clone, Copy)]
pub struct HouseFire {
    pub state: FireState,
    pub intensity: f32,
}

/// An event of the assault with its world position (ground height included).
#[derive(Debug, Clone)]
pub struct FxEvent {
    pub kind: SiegeFxKind,
    pub time: f64,
    /// Where it happens (impact, gate, oil target...).
    pub pos: Vec3,
}

/// Copy of the simulation state the renderers need, refreshed after every step.
#[derive(Resource, Default)]
pub struct SiegeFrame {
    /// A siege battle is running.
    pub active: bool,
    pub elapsed: f32,
    /// Animation clock: `elapsed` plus the part of the next step already run (smooth poses).
    pub clock: f32,
    pub pieces: Vec<PieceState>,
    pub gate: usize,
    pub gate_fire: Option<HouseFire>,
    pub houses: Vec<HouseFire>,
    pub engines: Vec<EngineState>,
    pub points: Vec<CapturePoint>,
    pub ladders: Vec<LadderState>,
    /// Events since the previous refresh (taken by the fx systems).
    pub events: Vec<FxEvent>,
    /// Ground height samples for the renderers (the field copied once: width, depth, cell, heights).
    pub ground: Option<Ground>,
    /// Number of `SiegeFx` already read from the simulation.
    read: usize,
    /// `unit_type` -> model of the static engines (`data/fx/siege_engines.json`).
    pub engine_models: BTreeMap<String, String>,
}

/// Height grid of the field.
#[derive(Debug, Clone)]
pub struct Ground {
    pub width: f64,
    pub depth: f64,
    pub cell: f64,
    pub nx: usize,
    pub nz: usize,
    pub heights: Vec<f64>,
}

impl Ground {
    /// Bilinear height at `(x, z)`.
    pub fn at(&self, x: f64, z: f64) -> f32 {
        let fx = (x / self.cell).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.cell).clamp(0.0, (self.nz - 1) as f64);
        let (i, j) = (fx.floor() as usize, fz.floor() as usize);
        let (i1, j1) = ((i + 1).min(self.nx - 1), (j + 1).min(self.nz - 1));
        let (tx, tz) = (fx - i as f64, fz - j as f64);
        let h = |i: usize, j: usize| self.heights[j * self.nx + i];
        let top = h(i, j) * (1.0 - tx) + h(i1, j) * tx;
        let bottom = h(i, j1) * (1.0 - tx) + h(i1, j1) * tx;
        (top * (1.0 - tz) + bottom * tz) as f32
    }
}

/// Counters for the dump.
#[derive(Resource, Default, Debug, Clone)]
pub struct SiegeStats {
    pub pieces: usize,
    pub towers: usize,
    pub houses: usize,
    pub breaches: usize,
    pub collapses_played: usize,
    pub scars: usize,
    pub engine_models: usize,
    pub engine_models_loaded: usize,
    pub particles_spawned: usize,
    pub events_seen: usize,
    pub events_by_kind: BTreeMap<String, usize>,
    pub bars: usize,
    /// Kit buildings and furniture of the town, distinct kit models, ruins laid by the fire.
    pub kit_pieces: usize,
    pub kit_models: usize,
    pub kit_ruins: usize,
    /// Static engines posed, swings started (and how many were predicted), servants drawn.
    pub swings: usize,
    pub predicted_swings: usize,
    pub crew_shown: usize,
    pub crew_batches: usize,
    pub crew_by_clip: BTreeMap<String, usize>,
    /// Latest pose of each static engine (dump).
    pub engine_poses: Vec<serde_json::Value>,
}

pub struct BattleSiegePlugin;

impl Plugin for BattleSiegePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SiegeFrame>()
            .init_resource::<SiegeStats>()
            .register_dump(dump_siege)
            .add_plugins((
                works::WorksPlugin,
                engines::EnginesPlugin,
                crew::CrewPlugin,
                fx::FxPlugin,
                overlay::OverlayPlugin,
            ))
            .add_systems(
                Update,
                kit_town::update_ruins
                    .after(read_frame)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                read_frame
                    .after(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(Update, dev_speed.run_if(resource_added::<BattleSession>))
            .add_systems(
                Update,
                dev_damage
                    .before(read_frame)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(OnExit(AppState::Battle), reset);
    }
}

/// Dev hook for the proofs: `CENT_ANS_BATTLE_SPEED=<factor>` sets the simulation speed at the
/// opening of a battle (a siege needs minutes of simulated time to show a breach).
fn dev_speed(mut session: ResMut<BattleSession>) {
    if let Some(speed) = std::env::var("CENT_ANS_BATTLE_SPEED")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
    {
        session.speed = speed.clamp(0.1, 32.0);
    }
}

/// Dev hooks for the proofs, fired once 5 s of battle have passed:
/// `CENT_ANS_SIEGE_BREAK=<piece index>` brings that wall piece down (hp 0, as a breach would),
/// `CENT_ANS_SIEGE_IGNITE=1` sets the gate on fire (`BattleSim::ignite_gate`).
fn dev_damage(mut session: ResMut<BattleSession>, mut done: Local<bool>) {
    if *done || session.snapshot.elapsed_s < 5.0 {
        return;
    }
    let break_piece = std::env::var("CENT_ANS_SIEGE_BREAK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    let ignite = std::env::var("CENT_ANS_SIEGE_IGNITE").is_ok();
    if break_piece.is_none() && !ignite {
        *done = true;
        return;
    }
    let sim = session.sim_mut();
    if let Some(index) = break_piece
        && let Some(works) = sim.siege_mut()
        && let Some(piece) = works.pieces.get_mut(index)
    {
        piece.hp = 0.0;
    }
    if ignite {
        sim.ignite_gate();
    }
    *done = true;
    info!("siege dev hooks fired: break {break_piece:?}, ignite {ignite}");
}

fn reset(mut frame: ResMut<SiegeFrame>, mut stats: ResMut<SiegeStats>) {
    *frame = SiegeFrame::default();
    *stats = SiegeStats::default();
}

/// Name of an event kind (dump).
fn kind_key(kind: &SiegeFxKind) -> &'static str {
    match kind {
        SiegeFxKind::EngineShot { .. } => "engine_shot",
        SiegeFxKind::RamStrike { .. } => "ram_strike",
        SiegeFxKind::TowerVolley { .. } => "tower_volley",
        SiegeFxKind::LaddersRaised { .. } => "ladders_raised",
        SiegeFxKind::TowerDocked { .. } => "tower_docked",
        SiegeFxKind::TowerUndocked { .. } => "tower_undocked",
        SiegeFxKind::OnWall { .. } => "on_wall",
        SiegeFxKind::BoilingOil { .. } => "boiling_oil",
        SiegeFxKind::GateBroken { .. } => "gate_broken",
        SiegeFxKind::WallBreached { .. } => "wall_breached",
        SiegeFxKind::DefendersFallBack => "defenders_fall_back",
    }
}

fn fire_of(house: &House) -> HouseFire {
    HouseFire {
        state: house.fire.state,
        intensity: house.fire.intensity as f32,
    }
}

/// Engine models by unit type, from `data/fx/siege_engines.json` (`engines`).
fn load_engine_models() -> BTreeMap<String, String> {
    let Some(dir) = crate::menu::battles::data_dir() else {
        return BTreeMap::new();
    };
    std::fs::read_to_string(dir.join("fx/siege_engines.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| {
            v["engines"].as_object().map(|map| {
                map.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Copies what the renderers need out of the simulation (every frame; cheap: a few dozen items).
fn read_frame(
    mut session: ResMut<BattleSession>,
    mut frame: ResMut<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
) {
    if frame.ground.is_none() {
        frame.engine_models = load_engine_models();
    }
    let fraction = session.step_fraction();
    let sim = session.sim_mut();
    let field = sim.field();
    if frame.ground.is_none() {
        frame.ground = Some(Ground {
            width: field.width,
            depth: field.depth,
            cell: field.resolution,
            nx: field.nx,
            nz: field.nz,
            heights: field.heights.clone(),
        });
    }
    frame.elapsed = sim.elapsed() as f32;
    frame.clock = frame.elapsed + fraction * sim_battle::DT as f32;
    let Some(works) = sim.siege() else {
        frame.active = false;
        return;
    };
    frame.active = true;
    frame.gate = works.gate;
    frame.pieces = works
        .pieces
        .iter()
        .map(|p| {
            let (mx, mz) = p.midpoint();
            PieceState {
                hp: p.hp as f32,
                max_hp: p.max_hp as f32,
                under_attack: p.under_attack(),
                top: Vec3::new(
                    mx as f32,
                    field.height(mx, mz) as f32 + works.wall_height as f32 + 3.0,
                    mz as f32,
                ),
            }
        })
        .collect();
    frame.gate_fire = Some(fire_of_blaze(&works.gate_fire));
    frame.houses = works.houses.iter().map(fire_of).collect();
    frame.points = works.capture_points();
    // New events, with a position.
    let ground = |x: f64, z: f64| field.height(x, z) as f32;
    let new: Vec<SiegeFx> = sim.siege_fx()[frame.read.min(sim.siege_fx().len())..].to_vec();
    frame.read = sim.siege_fx().len();
    let works = sim.siege().expect("checked above");
    let mut events = Vec::with_capacity(new.len());
    for fx in new {
        let piece_mid = |piece: usize| {
            let p = &works.pieces[piece.min(works.pieces.len() - 1)];
            let (x, z) = p.midpoint();
            Vec3::new(x as f32, ground(x, z), z as f32)
        };
        let pos = match &fx.kind {
            SiegeFxKind::EngineShot {
                x,
                z,
                height,
                piece,
                ..
            } => {
                let base = ground(*x, *z);
                let wall = works.wall_height as f32;
                let _ = piece;
                Vec3::new(*x as f32, base + wall * (*height as f32), *z as f32)
            }
            SiegeFxKind::BoilingOil { x, z, .. } => Vec3::new(*x as f32, ground(*x, *z), *z as f32),
            SiegeFxKind::RamStrike { piece, .. }
            | SiegeFxKind::LaddersRaised { piece, .. }
            | SiegeFxKind::TowerDocked { piece, .. }
            | SiegeFxKind::TowerUndocked { piece, .. }
            | SiegeFxKind::OnWall { piece, .. }
            | SiegeFxKind::GateBroken { piece }
            | SiegeFxKind::WallBreached { piece } => piece_mid(*piece),
            SiegeFxKind::TowerVolley { tower, .. } => {
                let t = &works.towers[(*tower).min(works.towers.len().saturating_sub(1))];
                Vec3::new(t.x as f32, ground(t.x, t.z) + t.height as f32, t.z as f32)
            }
            SiegeFxKind::DefendersFallBack => Vec3::new(
                works.center.0 as f32,
                ground(works.center.0, works.center.1),
                works.center.1 as f32,
            ),
        };
        *stats
            .events_by_kind
            .entry(kind_key(&fx.kind).to_owned())
            .or_default() += 1;
        stats.events_seen += 1;
        events.push(FxEvent {
            kind: fx.kind,
            time: fx.time,
            pos,
        });
    }
    frame.events = events;
    // Engines and ladders.
    let models = frame.engine_models.clone();
    let mut engines = Vec::new();
    let mut ladders = Vec::new();
    for unit in sim.units().iter().filter(|u| u.present()) {
        let kind = if unit.ram {
            Some(EngineKind::Ram)
        } else if unit.siege_tower() {
            Some(EngineKind::Tower)
        } else {
            models
                .get(&*unit.unit_type)
                .map(|m| EngineKind::Static(m.clone()))
        };
        if let Some(kind) = kind {
            let _ = SiegeEngineKind::Ram;
            engines.push(EngineState {
                unit: unit.id,
                kind,
                side: unit.side,
                pos: Vec3::new(unit.x as f32, ground(unit.x, unit.z), unit.z as f32),
                facing: unit.facing as f32,
                hp: unit.hp.max(0.0) as f32,
                max_hp: f64::from(unit.initial_soldiers.max(1)) as f32,
                reload: unit.reload as f32,
                reload_period: unit.reload_period() as f32,
                shooting: unit.state == sim_battle::UnitState::Shooting && unit.ammo > 0,
                ammo: unit.ammo,
            });
        }
        if unit.climbing.is_some() {
            for (index, ladder) in sim.ladders(unit).into_iter().enumerate() {
                ladders.push(LadderState {
                    unit: unit.id,
                    index,
                    foot: Vec3::new(
                        ladder.foot.0 as f32,
                        ladder.foot_y as f32,
                        ladder.foot.1 as f32,
                    ),
                    top: Vec3::new(
                        ladder.top.0 as f32,
                        ladder.top_y as f32,
                        ladder.top.1 as f32,
                    ),
                });
            }
        }
    }
    frame.engines = engines;
    frame.ladders = ladders;
}

fn fire_of_blaze(blaze: &sim_battle::Blaze) -> HouseFire {
    HouseFire {
        state: blaze.state,
        intensity: blaze.intensity as f32,
    }
}

fn dump_siege(world: &World) -> (String, serde_json::Value) {
    let frame = world.resource::<SiegeFrame>();
    let stats = world.resource::<SiegeStats>();
    if !frame.active {
        return (
            "battle_siege".into(),
            serde_json::json!({ "active": false }),
        );
    }
    let broken = frame.pieces.iter().filter(|p| p.hp <= 0.0).count();
    let engines = |wanted: &dyn Fn(&EngineKind) -> bool| {
        frame.engines.iter().filter(|e| wanted(&e.kind)).count()
    };
    (
        "battle_siege".into(),
        serde_json::json!({
            "active": true,
            "pieces": frame.pieces.len(),
            "pieces_broken": broken,
            "pieces_hp": frame.pieces.iter().map(|p| p.hp.round()).collect::<Vec<_>>(),
            "gate": frame.gate,
            "gate_fire": frame.gate_fire.map(|f| format!("{:?}", f.state)),
            "houses": frame.houses.len(),
            "houses_burning": frame.houses.iter().filter(|h| h.state == FireState::Burning).count(),
            "engines": {
                "rams": engines(&|k| *k == EngineKind::Ram),
                "towers": engines(&|k| *k == EngineKind::Tower),
                "static": engines(&|k| matches!(k, EngineKind::Static(_))),
            },
            "ladders": frame.ladders.len(),
            "capture_points": frame.points.iter().map(|p| serde_json::json!({
                "kind": p.kind.key(), "status": p.status.key(), "share": p.share(),
                "attackers": p.attackers, "defenders": p.defenders,
            })).collect::<Vec<_>>(),
            "render": {
                "structure_pieces": stats.pieces, "towers": stats.towers, "houses": stats.houses,
                "breaches_played": stats.collapses_played, "scars": stats.scars,
                "engine_models": stats.engine_models, "engine_models_loaded": stats.engine_models_loaded,
                "particles_spawned": stats.particles_spawned, "bars": stats.bars,
                "kit_pieces": stats.kit_pieces, "kit_models": stats.kit_models, "kit_ruins": stats.kit_ruins,
                "swings": stats.swings, "predicted_swings": stats.predicted_swings,
                "crew_shown": stats.crew_shown, "crew_batches": stats.crew_batches, "crew_by_clip": stats.crew_by_clip,
                "engine_poses": stats.engine_poses,
            },
            "events_seen": stats.events_seen,
            "events_by_kind": stats.events_by_kind,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::scene::setup;
    use crate::paths;
    use data_model::GameData;
    use sim_battle::custom::CustomEngines;

    #[test]
    fn ground_height_interpolates_and_clamps() {
        let ground = Ground {
            width: 20.0,
            depth: 20.0,
            cell: 10.0,
            nx: 3,
            nz: 3,
            heights: vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0],
        };
        assert!((ground.at(5.0, 5.0) - 5.0).abs() < 1e-4);
        assert!((ground.at(100.0, 0.0) - 20.0).abs() < 1e-4);
        assert!(ground.at(-5.0, -5.0).abs() < 1e-4);
    }

    #[test]
    fn a_siege_battle_gives_the_renderers_works_engines_and_points() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).expect("game data");
        let mut custom = setup::default_custom_battle(&game);
        custom.siege = true;
        custom.fortification = Some(sim_battle::CustomBattleRules::bundled().default_fortification);
        custom.engines = Some(CustomEngines {
            ladders: true,
            ram: true,
            towers: 1,
        });
        let mut sim = setup::build_sim(&game, &dir, &custom, 11).expect("siege builds");
        let works = sim.siege().expect("siege works");
        assert!(works.pieces.len() >= 8 && !works.towers.is_empty());
        assert!(works.pieces[works.gate].kind == sim_battle::PieceKind::Gate);
        assert_eq!(works.capture_points().len(), 2);
        assert!(!sim.siege_engines().is_empty(), "ram and tower present");
        for _ in 0..600 {
            sim.step();
        }
        assert!(
            sim.siege_fx()
                .iter()
                .any(|fx| matches!(fx.kind, SiegeFxKind::TowerVolley { .. }))
                || !sim.siege_fx().is_empty()
        );
        let models = load_engine_models();
        assert_eq!(
            models.get("unit_trebuchet").map(String::as_str),
            Some("trebuchet")
        );
    }
}
