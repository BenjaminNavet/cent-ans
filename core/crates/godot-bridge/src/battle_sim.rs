//! M7 battles on the GDExtension side (spec `docs/design/m7-battles.md` § 3):
//! the `BattleSim` class, its lifecycle and time methods. The rest of its
//! API lives in the `battle_sim_*` modules (orders, units, terrain, state);
//! the pending-battle methods of `CampaignSim` in `battle_sim_campaign.rs`.
//!
//! Setups, commands and outcomes cross the boundary as `Dictionary`s converted
//! through JSON with the serde types of `sim-battle`; positions go out as
//! packed float arrays.

use godot::classes::RefCounted;
use godot::prelude::*;
use sim_battle::{BattleSetup, SideId};

use crate::convert::from_dict;

pub(crate) fn parse_side(raw: &GString) -> Option<SideId> {
    SideId::parse(&raw.to_string())
}

/// Godot-facing handle on one battle (spec § 3).
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct BattleSim {
    pub(crate) sim: Option<sim_battle::BattleSim>,
    /// Visual unit-size multiplier (BV1, ADR 0016): figures drawn per
    /// simulated soldier. Rendering only.
    pub(crate) figure_scale: f64,
    /// Forced battle scale tier (EP1, `data/rules/battle_scale.json`);
    /// empty: by head count.
    scale_key: String,
    /// EP7: the historical map of the battle (menu or campaign site).
    pub(crate) historical: Option<sim_battle::HistoricalMap>,
    /// EP13: recording of the battle being fought (`battle_replay.rs`).
    pub(crate) recorder: Option<sim_battle::ReplayRecorder>,
    /// EP13: playback of a replay; the battle then takes no order.
    pub(crate) player: Option<sim_battle::ReplayPlayer>,
    /// PB3c: figure buffers kept between simulation steps (rendering only,
    /// `battle_sim_poses.rs`).
    pub(crate) poses: crate::battle_sim_poses::PoseCache,
    /// PB3c: `get_units` of the current step, with its key (see `PoseCache`).
    pub(crate) units_cache: Option<((u64, u64, u64), VarArray)>,
    /// RJ-b: regiment centres of the two latest steps (blended positions).
    pub(crate) unit_frames: crate::battle_sim_poses::UnitFrames,
    /// RJ-b: figures and regiments blended between steps (`set_pose_lerp`).
    pub(crate) pose_lerp: bool,
    /// RJ-b: PO4 loose ranks drawn by the core (`set_loose_ranks`).
    pub(crate) loose: Option<crate::battle_pose_lerp::LooseRanks>,
    /// PB3c: bumped by every change of the battle outside a simulation step
    /// (orders, deployment, new battle, replay jump): invalidates `poses`.
    pub(crate) pose_epoch: u64,
    /// PB3e (ADR 0090): the next step computed on a worker thread, when
    /// `set_step_thread(true)` (the battle scene; tests stay synchronous).
    step_thread: bool,
    steps: crate::battle_step_job::StepPipeline,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for BattleSim {
    fn init(base: Base<RefCounted>) -> Self {
        BattleSim {
            sim: None,
            figure_scale: 1.0,
            scale_key: String::new(),
            historical: None,
            recorder: None,
            player: None,
            poses: Default::default(),
            units_cache: None,
            unit_frames: Default::default(),
            pose_lerp: false,
            loose: None,
            pose_epoch: 0,
            step_thread: false,
            steps: Default::default(),
            base,
        }
    }
}

impl BattleSim {
    /// PB3c: the battle changed outside a simulation step: the cached figure
    /// poses must be rebuilt.
    pub(crate) fn touch_poses(&mut self) {
        self.pose_epoch = self.pose_epoch.wrapping_add(1);
        // PB3e: a step computed ahead from the state of before is useless.
        self.steps.clear();
    }
}

#[godot_api]
impl BattleSim {
    /// Builds the battle from a `CampaignSim.get_battle_setup` dictionary.
    #[func]
    fn setup(&mut self, setup: VarDictionary, seed: i64) -> bool {
        self.touch_poses();
        let forced = sim_battle::BattleScale::named(&self.scale_key);
        // EP7: a campaign battle on a historical site (`historical_site`,
        // the map's JSON text, added by `get_battle_setup`).
        let site = setup
            .get("historical_site")
            .and_then(|v| v.try_to::<GString>().ok())
            .and_then(
                |text| match sim_battle::HistoricalMap::from_json(&text.to_string()) {
                    Ok(map) => Some(map),
                    Err(error) => {
                        godot_warn!("BattleSim.setup: historical site: {error}");
                        None
                    }
                },
            );
        // EP13: one construction path, recorded for the replay.
        let start = from_dict::<BattleSetup>(&setup).map(|setup| match (&site, forced) {
            (Some(map), _) => sim_battle::ReplayStart::on_site(setup, seed as u64, map.clone()),
            (None, Some(scale)) => sim_battle::ReplayStart::scaled(setup, seed as u64, scale),
            (None, None) => sim_battle::ReplayStart::plain(setup, seed as u64),
        });
        let parsed = start.and_then(|start| start.build().map(|sim| (start, sim)));
        self.historical = site;
        self.player = None;
        self.recorder = None;
        match parsed {
            Ok((start, sim)) => {
                self.recorder = Some(sim_battle::ReplayRecorder::new(start, &sim));
                self.sim = Some(sim);
                // EP8: starting hour drawn by the campaign (`get_battle_setup`).
                if let Some(hour) = setup.get("hour").and_then(|v| v.try_to::<f64>().ok()) {
                    self.set_start_hour(hour);
                }
                true
            }
            Err(error) => {
                godot_error!("BattleSim.setup: {error}");
                self.sim = None;
                false
            }
        }
    }

    /// EP8: starts the battle at `hour` (0-24), e.g. a quick battle setting.
    #[func]
    fn set_start_hour(&mut self, hour: f64) {
        self.drive(sim_battle::ReplayAction::StartHour { hour });
    }

    /// EP8: starts the battle in phase `key` (`dawn`, `morning`, `midday`,
    /// `afternoon`, `dusk`); false if unknown.
    #[func]
    fn set_start_phase(&mut self, key: GString) -> bool {
        let rules = sim_battle::TimeOfDayRules::bundled();
        match rules.start_hour_of(&key.to_string()) {
            Some(hour) if self.sim.is_some() && self.player.is_none() => {
                self.drive(sim_battle::ReplayAction::StartHour { hour });
                true
            }
            _ => false,
        }
    }

    /// EP8: time of day now: `{hour, start_hour, key, label, visibility,
    /// range_factor, minutes_per_second}` (empty before `setup`).
    #[func]
    fn get_time_of_day(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let phase = sim.day_phase();
        vdict! {
            "hour" => sim.hour(),
            "start_hour" => sim.start_hour(),
            "key" => phase.key.as_str(),
            "label" => phase.label.as_str(),
            "visibility" => sim.visibility(),
            "range_factor" => sim.range_factor(),
            "minutes_per_second" => sim_battle::TimeOfDayRules::bundled().minutes_per_battle_second,
        }
    }

    /// Forces the scale tier of the next `setup` (EP1: `skirmish`, `large`,
    /// `epic`…; empty or unknown: by head count).
    #[func]
    fn set_scale_tier(&mut self, key: GString) {
        self.scale_key = key.to_string();
    }

    /// Scale of the battle (EP1): `{key, width, depth, attacker_line_z,
    /// defender_line_z, zone_depth, max_on_field}`.
    #[func]
    fn get_scale(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let scale = sim.scale();
        let size = sim.field().size;
        vdict! {
            "key" => scale.key.as_str(),
            "width" => size.width,
            "depth" => size.depth,
            "attacker_line_z" => size.attacker_line_z(),
            "defender_line_z" => size.defender_line_z(),
            "zone_depth" => size.zone_depth,
            "max_on_field" => scale.max_on_field as i64,
        }
    }

    /// Visual unit size (BV1, ADR 0016): figures drawn per simulated soldier
    /// (0.5 small, 1 normal, 1.5 large, 2.5 ultra). Changes only
    /// `get_soldier_buffer` and the `figures` key of `get_units`.
    #[func]
    fn set_figure_scale(&mut self, scale: f64) {
        self.figure_scale = scale.clamp(0.25, 4.0);
    }

    #[func]
    fn get_figure_scale(&self) -> f64 {
        self.figure_scale
    }

    /// Volleys resolved since the previous call (BV1): `[{time, shooter,
    /// target (-1: wall), from: Vector2, aim: Vector2, missiles, kills,
    /// kind: arrow|bolt|ball|stone|bullet|javelin (UR2), incendiary,
    /// cover: none|pavise|stakes|wall, indirect (R4: lobbed over a crest)}]`.
    #[func]
    fn get_shots(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_shots()
            .iter()
            .map(|shot| {
                vdict! {
                    "time" => shot.time,
                    "shooter" => i64::from(shot.shooter),
                    "target" => shot.target.map_or(-1, i64::from),
                    "from" => Vector2::new(shot.from.0 as f32, shot.from.1 as f32),
                    "aim" => Vector2::new(shot.aim.0 as f32, shot.aim.1 as f32),
                    "missiles" => i64::from(shot.missiles),
                    "kills" => shot.kills,
                    "kind" => shot.kind.key(),
                    "incendiary" => shot.incendiary,
                    "cover" => shot.cover.key(),
                    "indirect" => shot.indirect,
                }
                .to_variant()
            })
            .collect()
    }

    /// Advances the battle by `dt` seconds (fixed 0.1 s steps inside).
    #[func]
    fn tick(&mut self, dt: f64) {
        let Some(sim) = &mut self.sim else {
            return;
        };
        // EP13: a replay advances by its recorded inputs.
        if let Some(player) = &mut self.player {
            self.steps.clear();
            player.advance(sim, dt);
            return;
        }
        if self.step_thread {
            self.steps.tick(sim, dt, self.pose_epoch);
        } else {
            sim.tick(dt);
        }
        if let Some(recorder) = &mut self.recorder {
            recorder.observe(sim);
        }
    }

    /// PB3e (ADR 0090): computes the next fixed step on a worker thread
    /// while the frames show the current one (same battle, bit for bit, as
    /// the synchronous mode kept by default for tests and headless runs).
    #[func]
    fn set_step_thread(&mut self, enabled: bool) {
        self.step_thread = enabled;
        if !enabled {
            self.steps.clear();
        }
    }

    #[func]
    fn get_step_thread(&self) -> bool {
        self.step_thread
    }

    /// PB3e measures: `{adopted, in_place}` steps since the battle began
    /// (adopted: computed ahead on the worker thread).
    #[func]
    fn get_step_stats(&self) -> VarDictionary {
        vdict! {
            "adopted" => self.steps.adopted as i64,
            "in_place" => self.steps.in_place as i64,
        }
    }

    /// Charge impacts resolved since the previous call (BV2): `[{time,
    /// attacker, defender, kind: shock|pikes|stakes|broken, point: Vector2,
    /// heading, mass, knocked, unhorsed, depth, cohesion}]`. The renderer
    /// throws down `knocked` men of the defender, slows the horses over
    /// `depth` metres and unhorses `unhorsed` riders.
    #[func]
    fn get_impacts(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_impacts()
            .iter()
            .map(|hit| {
                vdict! {
                    "time" => hit.time,
                    "attacker" => i64::from(hit.attacker),
                    "defender" => i64::from(hit.defender),
                    "kind" => hit.kind.key(),
                    "point" => Vector2::new(hit.point.0 as f32, hit.point.1 as f32),
                    "heading" => hit.heading,
                    "mass" => hit.mass,
                    "knocked" => i64::from(hit.knocked),
                    "unhorsed" => i64::from(hit.unhorsed),
                    "depth" => hit.depth,
                    "cohesion" => hit.cohesion,
                }
                .to_variant()
            })
            .collect()
    }
}
