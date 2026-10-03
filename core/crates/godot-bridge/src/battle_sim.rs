//! M7 battles on the GDExtension side (spec `docs/design/m7-battles.md` § 3):
//! the `BattleSim` class and the pending-battle methods of `CampaignSim`
//! (a secondary `#[godot_api]` block, so `campaign_sim.rs` stays untouched).
//!
//! Setups, commands and outcomes cross the boundary as `Dictionary`s converted
//! through JSON with the serde types of `sim-battle`; positions go out as
//! packed float arrays.

use data_model::{Ability, UnitCategory};
use godot::classes::RefCounted;
use godot::prelude::*;
use serde_json::Value;
use sim_battle::{BattleOutcome, BattleSetup, Command, SideId, Unit, UnitState};

use crate::battle_pose_lerp::push_pose;
use crate::battle_replay::{note, REPLAY_REFUSAL};
use crate::campaign_sim::{events_array, CampaignSim};
use crate::convert::variant_to_json;

/// Converts JSON into plain Godot values (integers stay integers).
pub(crate) fn json_to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_variant()
            } else if let Some(u) = n.as_u64() {
                (u as i64).to_variant()
            } else {
                n.as_f64().unwrap_or(0.0).to_variant()
            }
        }
        Value::String(s) => GString::from(s.as_str()).to_variant(),
        Value::Array(items) => items
            .iter()
            .map(json_to_variant)
            .collect::<VarArray>()
            .to_variant(),
        Value::Object(map) => {
            let mut dict = VarDictionary::new();
            for (key, item) in map {
                dict.set(key.as_str(), &json_to_variant(item));
            }
            dict.to_variant()
        }
    }
}

pub(crate) fn to_dict<T: serde::Serialize>(value: &T) -> VarDictionary {
    serde_json::to_value(value)
        .ok()
        .map(|json| json_to_variant(&json))
        .and_then(|variant| variant.try_to::<VarDictionary>().ok())
        .unwrap_or_default()
}

pub(crate) fn from_dict<T: serde::de::DeserializeOwned>(dict: &VarDictionary) -> Result<T, String> {
    variant_to_json(&dict.to_variant())
        .and_then(|json| serde_json::from_value::<T>(json).map_err(|e| e.to_string()))
}

pub(crate) fn result_dict(result: Result<(), String>) -> VarDictionary {
    match result {
        Ok(()) => vdict! { "ok" => true, "error" => "" },
        Err(error) => vdict! { "ok" => false, "error" => error.as_str() },
    }
}

fn parse_side(raw: &GString) -> Option<SideId> {
    SideId::parse(&raw.to_string())
}

/// Rendering family of a regiment: `infantry`, `archer`, `cavalry`, `siege`,
/// or `ram` / `tower` (siege battles: drawn as one machine, not soldiers).
/// BR3: a piece of street furniture for the renderer, `{kind, x, z, yaw,
/// length, depth, house}` (`house` = -1 on the market square; yaw of the
/// core: length along (cos, sin), front along (-sin, cos)).
fn prop_dict(prop: &sim_battle::Prop) -> VarDictionary {
    vdict! {
        "kind" => prop.kind.key(),
        "x" => prop.x,
        "z" => prop.z,
        "yaw" => prop.yaw,
        "length" => prop.length,
        "depth" => prop.depth,
        "house" => prop.house.map_or(-1, |h| h as i64),
    }
}

/// EP6: a decor prop `{kind, x, z, yaw, length, depth, count}`.
fn decor_prop_dict(prop: &sim_battle::DecorProp) -> VarDictionary {
    vdict! {
        "kind" => prop.kind.key(),
        "x" => prop.x,
        "z" => prop.z,
        "yaw" => prop.yaw,
        "length" => prop.length,
        "depth" => prop.depth,
        "count" => prop.count as i64,
    }
}

/// EP6: a decor area `{kind, x, z, length, width, yaw, state}` (`state`:
/// ploughed|sown|crop|stubble for ploughland, else "").
fn decor_area_dict(area: &sim_battle::Area) -> VarDictionary {
    vdict! {
        "kind" => area.kind.key(),
        "x" => area.x,
        "z" => area.z,
        "length" => area.length,
        "width" => area.width,
        "yaw" => area.yaw,
        "state" => area.state.map_or("", |s| s.key()),
    }
}

/// EP6: the decor of the field (see [`BattleSim::get_terrain`]).
fn decor_dict(decor: &sim_battle::Decor) -> VarDictionary {
    let buildings: VarArray = decor
        .buildings
        .iter()
        .map(|h| {
            vdict! {
                "x" => h.x, "z" => h.z, "length" => h.length, "width" => h.width,
                "yaw" => h.yaw, "kind" => h.kind.key(),
            }
            .to_variant()
        })
        .collect();
    let hamlets: VarArray = decor
        .hamlets
        .iter()
        .map(|h| {
            let buildings: PackedInt32Array = h.buildings.iter().map(|&i| i as i32).collect();
            vdict! {
                "layout" => h.layout.key(), "x" => h.x, "z" => h.z, "yaw" => h.yaw,
                "buildings" => &buildings,
            }
            .to_variant()
        })
        .collect();
    let areas: VarArray = decor
        .areas
        .iter()
        .map(|a| decor_area_dict(a).to_variant())
        .collect();
    let props: VarArray = decor
        .props
        .iter()
        .map(|p| decor_prop_dict(p).to_variant())
        .collect();
    let mounds: VarArray = decor
        .mounds
        .iter()
        .map(|m| {
            vdict! { "x" => m.x, "z" => m.z, "radius" => m.radius, "height" => m.height }
                .to_variant()
        })
        .collect();
    let moats: VarArray = decor
        .moats
        .iter()
        .map(|m| {
            vdict! {
                "x" => m.x, "z" => m.z, "length" => m.length, "width" => m.width,
                "yaw" => m.yaw, "ring" => m.ring,
            }
            .to_variant()
        })
        .collect();
    let camps: VarArray = decor
        .camps
        .iter()
        .map(|c| {
            let items: VarArray = c
                .items
                .iter()
                .map(|p| decor_prop_dict(p).to_variant())
                .collect();
            let convoy: VarArray = c
                .convoy
                .iter()
                .map(|p| decor_prop_dict(p).to_variant())
                .collect();
            vdict! {
                "side" => c.side.key(), "area" => &decor_area_dict(&c.area),
                "items" => &items, "convoy" => &convoy,
            }
            .to_variant()
        })
        .collect();
    vdict! {
        "profile" => decor.profile.as_str(),
        "vines_leafy" => decor.vines_leafy,
        "orchard_blossom" => decor.orchard_blossom,
        "buildings" => &buildings,
        "hamlets" => &hamlets,
        "areas" => &areas,
        "props" => &props,
        "mounds" => &mounds,
        "moats" => &moats,
        "camps" => &camps,
    }
}

fn render_key(unit: &Unit) -> &'static str {
    if unit.ram {
        return "ram";
    }
    if unit.siege_tower() {
        return "tower";
    }
    match unit.category {
        UnitCategory::Siege => "siege",
        _ if unit.mounted => "cavalry",
        UnitCategory::Ranged => "archer",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Infantry => "infantry",
    }
}

fn category_key(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "infantry",
        UnitCategory::Ranged => "ranged",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Siege => "siege",
    }
}

fn state_label_fr(state: UnitState) -> &'static str {
    match state {
        UnitState::Idle => "au repos",
        UnitState::Marching => "en marche",
        UnitState::Charging => "charge",
        UnitState::Melee => "mêlée",
        UnitState::Shooting => "tir",
        UnitState::Routing => "déroute",
        UnitState::Rallied => "rallié",
        UnitState::Climbing => "escalade",
    }
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

    /// Width and depth of the field in metres (EP1), `(0, 0)` before `setup`.
    #[func]
    fn get_field_size(&self) -> Vector2 {
        self.sim.as_ref().map_or(Vector2::ZERO, |sim| {
            Vector2::new(sim.field().width as f32, sim.field().depth as f32)
        })
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

    /// `{type: "move"|"attack"|"halt"|"formation"|"fire_at_will"|"withdraw"
    /// |"target_wall"|"burn"|"leader_order", units: [ids], ...}` → `{ok, error}`.
    /// Siege fire (S2): `{type: "burn", units: [ids], house: i}` or
    /// `{type: "burn", units: [ids], gate: true}`.
    /// A leader's order: `{type: "leader_order", order: "order_war_cry",
    /// units: [ids] (selected-scope orders; empty = every eligible one)}`.
    #[func]
    fn issue_command(&mut self, command: VarDictionary) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return result_dict(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        let result = from_dict::<Command>(&command)
            .map_err(|e| crate::campaign_sim::invalid_order_message(&e))
            .and_then(|command| {
                note(
                    &mut self.recorder,
                    sim,
                    sim_battle::ReplayAction::Command {
                        command: command.clone(),
                    },
                );
                sim.issue_command(command).map_err(|e| e.to_string())
            });
        result_dict(result)
    }

    /// The leader's order bar of `side`: `[{id, kind, name, label,
    /// description, icon, available, reason, cooldown, cooldown_remaining,
    /// uses, uses_per_battle}]` by rank (`label` is the faction's wording,
    /// e.g. « Montjoie ! Saint-Denis ! »; `reason` is empty when available).
    #[func]
    fn get_leader_orders(&self, side: GString) -> VarArray {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return VarArray::new();
        };
        sim.leader_orders(side)
            .iter()
            .map(|view| {
                serde_json::to_value(view)
                    .map(|json| json_to_variant(&json))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// RS-F: the « Incendier » order of the battle bar for `units` of `side`
    /// (every regiment of the side when empty): `{siege: false}` outside a
    /// siege; else `{siege: true, available, reason, command}` where
    /// `command` is the `burn` order to give (`{type: "burn", units: [id],
    /// house: i}` or `gate: true`) and `target` names it (« la porte », « une
    /// maison du faubourg », « une maison »). The core decides; nothing drawn.
    #[func]
    fn get_burn_order(&self, side: GString, units: PackedInt32Array) -> VarDictionary {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return vdict! { "siege" => false };
        };
        if sim.fire_rules().is_none() || sim.siege().is_none() {
            return vdict! { "siege" => false };
        }
        let ids: Vec<u32> = units
            .as_slice()
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .collect();
        match sim.burn_choice(side, &ids) {
            Ok(choice) => {
                let ids: VarArray = [i64::from(choice.unit).to_variant()].into_iter().collect();
                let mut command = vdict! { "type" => "burn", "units" => &ids };
                let target = match choice.house {
                    Some(house) => {
                        command.set("house", house as i64);
                        if choice.suburb {
                            "une maison du faubourg"
                        } else {
                            "une maison"
                        }
                    }
                    None => {
                        command.set("gate", true);
                        "la porte"
                    }
                };
                vdict! {
                    "siege" => true,
                    "available" => true,
                    "reason" => "",
                    "command" => &command,
                    "target" => target,
                    "unit" => i64::from(choice.unit),
                    "distance_m" => choice.distance_m,
                }
            }
            Err(error) => vdict! {
                "siege" => true,
                "available" => false,
                "reason" => error.to_string(),
            },
        }
    }

    /// CB4: uses (or lifts, when all of them use it) ability `ability` on
    /// those of `units` that have it: `{ok, error}` like `issue_command`
    /// (recorded for the replay the same way).
    #[func]
    fn use_ability(&mut self, units: PackedInt32Array, ability: GString) -> VarDictionary {
        let ids: VarArray = units
            .as_slice()
            .iter()
            .map(|&id| i64::from(id).to_variant())
            .collect();
        let mut command = VarDictionary::new();
        command.set("type", "use_ability");
        command.set("units", &ids);
        command.set("ability", &ability);
        self.issue_command(command)
    }

    /// CB4: `{id: {id, kind, rank, name, description, icon}}` of the
    /// battle's ability catalogue (texts of the tooltips; the state of each
    /// regiment's abilities is in `get_units`).
    #[func]
    fn get_ability_catalog(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map(crate::battle_sim_abilities::catalog_dict)
            .unwrap_or_default()
    }

    /// Did `side` give the "no quarter" order?
    #[func]
    fn get_no_quarter(&self, side: GString) -> bool {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim.no_quarter(side),
            _ => false,
        }
    }

    /// Lets the AI command `side` (`"attacker"`/`"defender"`), e.g. for autoplay.
    #[func]
    fn set_ai(&mut self, side: GString, enabled: bool) {
        if let Some(side) = parse_side(&side) {
            self.drive(sim_battle::ReplayAction::SetAi { side, enabled });
        }
    }

    /// NT11: "hold ground" for `side` (`"attacker"`/`"defender"`): its
    /// regiments keep their place (no AI move, no skirmish step back), the
    /// rout under pressure aside (guided battle prologue).
    #[func]
    fn set_hold(&mut self, side: GString, enabled: bool) {
        if let Some(side) = parse_side(&side) {
            self.drive(sim_battle::ReplayAction::SetHold { side, enabled });
        }
    }

    /// NT11: does `side` hold its ground?
    #[func]
    fn get_hold(&self, side: GString) -> bool {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim.holds(side),
            _ => false,
        }
    }

    /// F5a: opens the deployment phase (call right after `setup`, before
    /// any `tick`). `false` once the battle has started.
    #[func]
    fn begin_deployment(&mut self) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        let Some(sim) = &mut self.sim else {
            return false;
        };
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::BeginDeployment,
        );
        sim.begin_deployment()
    }

    /// `true` during the deployment phase (ticks do nothing).
    #[func]
    fn is_deploying(&self) -> bool {
        self.sim.as_ref().is_some_and(|sim| sim.is_deploying())
    }

    /// `{x0, z0, x1, z1}` rectangle of `side` (field metres); empty if unknown
    /// or when the side has no deployment phase (CV3-2: caught in column,
    /// forced march). An ambusher may have a second zone: see
    /// [`Self::get_deployment_zones`].
    #[func]
    fn get_deployment_zone(&self, side: GString) -> VarDictionary {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) if sim.can_deploy(side) => to_dict(&sim.deployment_zone(side)),
            _ => VarDictionary::new(),
        }
    }

    /// CV3-2: every `{x0, z0, x1, z1}` deployment zone of `side` (the one or
    /// two flanks of an ambusher; none for a side that cannot deploy).
    #[func]
    fn get_deployment_zones(&self, side: GString) -> VarArray {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim
                .deployment_zones(side)
                .iter()
                .map(|z| to_dict(z).to_variant())
                .collect(),
            _ => VarArray::new(),
        }
    }

    /// CV3-2: how the battle opened. `{kind: standard|ambush, victim:
    /// attacker|defender|"", on_road, path: PackedVector2Array (column centre
    /// line, head last), attacker_forced_march, defender_forced_march,
    /// attacker_entrenched, defender_entrenched, attacker_can_deploy,
    /// defender_can_deploy}`; empty before `setup`.
    #[func]
    fn get_opening(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let setup = sim.setup();
        let layout = sim.ambush_layout();
        let path: PackedVector2Array = layout
            .map(|l| {
                l.path
                    .iter()
                    .map(|p| Vector2::new(p.0 as f32, p.1 as f32))
                    .collect()
            })
            .unwrap_or_default();
        let mut dict = vdict! {
            "kind" => if setup.opening.is_standard() { "standard" } else { "ambush" },
            "victim" => setup.opening.ambush_victim().map_or("", |s| s.key()),
            "on_road" => layout.is_some_and(|l| l.on_road),
            "path" => &path,
        };
        for side in sim_battle::SideId::BOTH {
            let key = side.key();
            let s = setup.side(side);
            dict.set(format!("{key}_forced_march").as_str(), s.forced_march);
            dict.set(format!("{key}_entrenched").as_str(), s.entrenched);
            dict.set(format!("{key}_can_deploy").as_str(), sim.can_deploy(side));
        }
        dict
    }

    /// Places regiment `id` at (x, z); `facing` in radians, NaN keeps the
    /// current facing; CB1 `width` (metres, optional, ≤ 0 for none): the
    /// frontage of a right-drag, the regiment forming a Line that wide.
    /// → `{ok, error}` (French error).
    #[func]
    fn deploy_unit(
        &mut self,
        id: i64,
        x: f64,
        z: f64,
        facing: f64,
        #[opt(default = 0.0)] width: f64,
    ) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return result_dict(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        let facing = facing.is_finite().then_some(facing);
        let width = (width.is_finite() && width > 0.0).then_some(width);
        let result = u32::try_from(id)
            .map_err(|_| format!("unité inconnue : {id}"))
            .and_then(|id| {
                note(
                    &mut self.recorder,
                    sim,
                    sim_battle::ReplayAction::DeployUnit {
                        unit: id,
                        x,
                        z,
                        facing,
                        width,
                    },
                );
                sim.deploy_unit_width(id, x, z, facing, width)
                    .map_err(|e| sim.error_text(&e))
            });
        result_dict(result)
    }

    /// Ends the deployment phase → `{ok, error}`.
    #[func]
    fn start_battle(&mut self) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return result_dict(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::StartBattle,
        );
        result_dict(sim.start_battle().map_err(|e| e.to_string()))
    }

    /// One dictionary per regiment (spec § 3), plus rendering helpers.
    /// PB3c: rebuilt only after a simulation step or a change of the battle
    /// (same key as the figure poses); in between the same array comes back —
    /// read-only for the caller.
    #[func]
    /// RJ-b: `x`, `y`, `z`, `facing` blended between the two latest steps
    /// with `set_pose_lerp` (the dictionaries change in place each frame),
    /// `ground_speed` = m/s of the centre over the latest step.
    fn get_units(&mut self) -> VarArray {
        self.units_now()
    }

    /// The dictionaries of [`Self::get_units`].
    pub(crate) fn build_units(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim.units()
            .iter()
            .map(|unit| {
                let (width, depth) = unit.extent();
                let mut dict = vdict! {
                    "id" => i64::from(unit.id),
                    "side" => unit.side.key(),
                    "type" => unit.unit_type.as_str(),
                    "name" => unit.name.as_str(),
                    "category" => category_key(unit.category),
                    "render" => render_key(unit),
                    "soldiers" => i64::from(unit.soldiers()),
                    "figures" => i64::from(unit.figure_count(self.figure_scale)),
                    "max_soldiers" => i64::from(unit.max_soldiers),
                    "initial_soldiers" => i64::from(unit.initial_soldiers),
                    "kills" => unit.kills.round() as i64,
                    "morale" => unit.morale,
                    "fatigue" => unit.fatigue,
                    "ammo" => i64::from(unit.ammo),
                    "max_ammo" => i64::from(if unit.can_shoot() { unit.stats.ammo } else { 0 }),
                    // SG2: seconds before the next shot and the full reload (engines
                    // are wound back over it).
                    "reload" => unit.reload.max(0.0),
                    "reload_period" => unit.reload_period(),
                    "state" => unit.state.key(),
                    "state_label" => state_label_fr(unit.state),
                    "formation" => unit.formation.key(),
                    "x" => unit.x,
                    "z" => unit.z,
                    "y" => sim.standing_height(unit, unit.x, unit.z),
                    "facing" => unit.facing,
                    "width" => width,
                    "depth" => depth,
                    "is_general" => unit.is_general,
                    // CB3: fog of war for the tactical view (Tab) — purely derived, not part of
                    // the core state (`state_digest`/replay): own side always spotted; no player
                    // side set (spectated battle) shows everyone, as the normal view already does.
                    "spotted" => sim
                        .setup()
                        .player_side
                        .is_none_or(|side| sim.spotted_by(unit, side)),
                    "present" => unit.present(),
                    "reserve" => unit.reserve,
                    "left_field" => unit.left_field,
                    // Q2: end-of-battle fate (destroyed, routed, withdrawn, reserve, held).
                    "fate" => unit.fate().key(),
                    "fire_at_will" => unit.fire_at_will,
                    "can_shoot" => unit.can_shoot(),
                    "running" => unit.running,
                    "withdrawing" => unit.withdrawing,
                    "stakes" => unit.stakes_planted,
                    "target" => unit.target.map_or(-1, i64::from),
                    "on_wall" => unit.on_wall,
                    "climbing" => unit.climbing.map_or(-1, |p| p as i64),
                    "climb_progress" => unit.climb_progress,
                    "ladders" => sim.on_ladders(unit),
                    "synthetic" => unit.synthetic,
                    "ram" => unit.ram,
                    "siege_tower" => unit.siege_tower(),
                    "wall_breaker" => unit.wall_breaker(),
                    "pavise" => unit.pavise.is_some(),
                    "pavise_cover" => unit.pavise.is_some()
                        || (unit.has(Ability::Pavise) && unit.state != UnitState::Marching),
                    "dismounted" => unit.dismounted,
                    "order_morale" => unit.order_morale,
                    // BV2: cause of the latest deaths (chooses the death drawn).
                    "loss_cause" => unit.loss_cause.key(),
                    "loss_by" => unit.loss_by.map_or(-1, i64::from),
                    "knocked" => if unit.knocked_timer > 0.0 { unit.knocked } else { 0.0 },
                    // CB-M4: range drawn on the ground (0 without missiles) and the
                    // half-angle of the drawn sector (radians, `battle_hover.json`).
                    "effective_range" => sim.ground_range(unit),
                    "fire_arc" => sim_battle::HoverRules::bundled().range_arc.fire_half_angle(),
                };
                if let Some((x, z)) = unit.destination {
                    dict.set("destination", Vector2::new(x as f32, z as f32));
                }
                // CB1: files of a dragged Line (-1: default depth), and the
                // tag of the grouped order it walks under (-1: none).
                dict.set("line_files", unit.line_files.map_or(-1, i64::from));
                dict.set("group_tag", unit.group_tag.map_or(-1, i64::from));
                dict.set("match_speed", unit.match_speed);
                // CB-M3: orders waiting behind the current one.
                dict.set("queue", &crate::battle_sim_queue::queue_array(sim, unit));
                // CB2: modes on, modes available, states for the badges.
                crate::battle_sim_modes::add_mode_fields(sim, unit, &mut dict);
                // CB4: the regiment's abilities (buttons of its card).
                crate::battle_sim_abilities::add_ability_fields(sim, unit, &mut dict);
                // EP11: push of the lines in melee (m/s, > 0 driving the enemy
                // back, < 0 giving ground), compression (0-1), ground given (m).
                dict.set("push_speed", unit.push.speed);
                dict.set("compression", unit.push.compression);
                dict.set("ground_lost", unit.push.ground_lost);
                // EP5: the regiment's standard (`carried`, `fallen`, `captured`,
                // `lost`), where it lies on the ground, the regiment that took it,
                // and the figures of the buffer that carry it.
                let bearers = sim.standard_bearers(unit.id as usize);
                if bearers > 0 {
                    dict.set("standard", unit.standard.key());
                    if unit.is_general {
                        let sovereign = sim
                            .setup()
                            .side(unit.side)
                            .general
                            .as_ref()
                            .is_some_and(|g| g.sovereign);
                        dict.set("sovereign", sovereign);
                    }
                    if let Some((x, z)) = unit.standard.ground() {
                        dict.set("standard_x", x);
                        dict.set("standard_z", z);
                        dict.set("standard_y", sim.field().height(x, z));
                    }
                    if let sim_battle::unit::StandardState::Fallen { timer, .. } = unit.standard {
                        dict.set("standard_timer", timer);
                    }
                    if let sim_battle::unit::StandardState::Captured { by } = unit.standard {
                        dict.set(
                            "standard_by",
                            if by == u32::MAX { -1 } else { i64::from(by) },
                        );
                    }
                    let slots: PackedInt32Array = unit
                        .standard_slots(self.figure_scale, bearers)
                        .iter()
                        .map(|&s| s as i32)
                        .collect();
                    dict.set("bearer_slots", &slots);
                }
                // SG1: the first `climbers_shown` soldiers of the buffer are on
                // the ladders / bridge; `ladder_lines` = [foot, top] per ladder
                // as Vector3 pairs (ground and crenel heights).
                if unit.climbing.is_some() {
                    dict.set(
                        "climbers_shown",
                        sim.climbers_shown(unit, self.figure_scale) as i64,
                    );
                    let lines: PackedVector3Array = sim
                        .ladders(unit)
                        .iter()
                        .flat_map(|l| {
                            [
                                Vector3::new(l.foot.0 as f32, l.foot_y as f32, l.foot.1 as f32),
                                Vector3::new(l.top.0 as f32, l.top_y as f32, l.top.1 as f32),
                            ]
                        })
                        .collect();
                    dict.set("ladder_lines", &lines);
                }
                dict.to_variant()
            })
            .collect()
    }

    /// `(x, y, z, angle)` per living soldier of `side`.
    #[func]
    fn get_soldier_transforms(&self, side: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        sim.soldier_transforms(side, None)
            .iter()
            .flat_map(|t| t.map(|v| v as f32))
            .collect()
    }

    /// `MultiMesh.buffer` (12 floats per instance, rotation about Y) of the
    /// living soldiers of `side` whose regiment renders as `render`
    /// (`infantry`, `archer`, `cavalry`, `siege`). Uncached; the renderer uses
    /// [`Self::get_soldier_buffers`].
    #[func]
    fn get_soldier_buffer(&self, side: GString, render: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        let render = render.to_string();
        let mut buffer: Vec<f32> = Vec::new();
        for unit in sim
            .units()
            .iter()
            .filter(|u| u.side == side && render_key(u) == render)
        {
            // SG1: climbers drawn on their ladders / the tower bridge.
            for pose in sim.soldier_poses(unit, self.figure_scale) {
                push_pose(&mut buffer, pose);
            }
        }
        PackedFloat32Array::from(buffer.as_slice())
    }

    /// `{width, depth, resolution, nx, nz, heights, forests[{x, z, radius}],
    /// mud[..], river?{points: PackedVector2Array, width, fords[{x, z, half_width}]},
    /// siege?{...}}` (siege geometry: see [`Self::get_siege`]). B5 (campaign site):
    /// `terrain` (province terrain key), `season`, `ground` (`dry|muddy|snowy`),
    /// `ground_label`, `site_label` (B6), `woodland` (0-1), `pools[{x, z, radius}]`,
    /// `obstacles[{a: Vector2, b: Vector2, kind: hedge|fence|ditch|palisade}]`
    /// (CV3-2: `palisade` = entrenched camp),
    /// `coast?{flank: west|east, shore_x, beach}`,
    /// `village?{x, z, radius, farm, houses[{x, z, length, width, yaw, kind}],
    /// props[{kind, x, z, yaw, length, depth, house}]}` (BR3).
    /// R2: `forests` and `mud` are overlapping discs (anchors first, then
    /// lobes and copses).
    /// EP3: `river` also carries `widths` (water width at each point),
    /// `flow` (+1 when the water runs towards +x), `banks[{x0, x1, north,
    /// kind: steep|marsh}]`; `bridges[{x, z, yaw, length, width, span,
    /// deck, stone, arches, stream}]` (`stream` = -1 on the river),
    /// `streams[{kind: tributary|brook, points, width}]`, `roads[{kind:
    /// main|track, points, width}]`; oxbows are appended to `pools`.
    /// EP6: `decor{profile, vines_leafy, orchard_blossom, buildings[{x, z,
    /// length, width, yaw, kind}], hamlets[{layout, x, z, yaw, buildings}],
    /// areas[{kind, x, z, length, width, yaw, state}], props[{kind, x, z,
    /// yaw, length, depth, count}], mounds[{x, z, radius, height}],
    /// moats[{x, z, length, width, yaw, ring}], camps[{side, area, items,
    /// convoy}]}` (windmill mounds are already in `heights`).
    #[func]
    fn get_terrain(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let field = sim.field();
        let zones = |zones: &[sim_battle::Zone]| -> VarArray {
            zones
                .iter()
                .map(|z| vdict! { "x" => z.x, "z" => z.z, "radius" => z.radius }.to_variant())
                .collect()
        };
        let heights: PackedFloat32Array = field.heights.iter().map(|h| *h as f32).collect();
        let mut dict = vdict! {
            "width" => field.width,
            "depth" => field.depth,
            // EP1: battle lines of the field (250 / 550 on the standard one).
            "attacker_line_z" => field.attacker_line_z(),
            "defender_line_z" => field.defender_line_z(),
            "resolution" => field.resolution,
            "nx" => field.nx as i64,
            "nz" => field.nz as i64,
            "heights" => &heights,
            // R2: a wood (a patch of mud) is its anchor disc plus its lobes
            // and copses; the discs overlap.
            "forests" => &zones(&[field.forests.as_slice(), &field.forest_parts].concat()),
            "mud" => &zones(&[field.mud.as_slice(), &field.mud_parts].concat()),
            // B5: campaign site.
            "terrain" => field.terrain.key(),
            "season" => season_key(field.season),
            "ground" => field.ground.key(),
            "ground_label" => field.ground.label_fr(),
            // B6: the site in one compact line (pre-battle dialog, HUD).
            "site_label" => field.site_label_fr(),
            "woodland" => field.woodland,
            "pools" => &zones(&[field.pools.as_slice(), &field.oxbows].concat()),
            "obstacles" => &field
                .obstacles
                .iter()
                .map(|o| {
                    vdict! {
                        "a" => Vector2::new(o.a.0 as f32, o.a.1 as f32),
                        "b" => Vector2::new(o.b.0 as f32, o.b.1 as f32),
                        "kind" => o.kind.key(),
                    }
                    .to_variant()
                })
                .collect::<VarArray>(),
        };
        if let Some(coast) = &field.coast {
            let flank = match coast.flank {
                sim_battle::Flank::West => "west",
                sim_battle::Flank::East => "east",
            };
            dict.set(
                "coast",
                &vdict! { "flank" => flank, "shore_x" => coast.shore_x, "beach" => coast.beach },
            );
        }
        if let Some(village) = &field.village {
            let houses: VarArray = village
                .houses
                .iter()
                .map(|h| {
                    vdict! {
                        "x" => h.x, "z" => h.z, "length" => h.length, "width" => h.width,
                        "yaw" => h.yaw, "kind" => h.kind.key(),
                    }
                    .to_variant()
                })
                .collect();
            // BR3: props laid by the core (solid for the figures).
            let props: VarArray = sim
                .village_props()
                .iter()
                .map(|p| prop_dict(p).to_variant())
                .collect();
            dict.set(
                "village",
                &vdict! {
                    "x" => village.zone.x, "z" => village.zone.z, "radius" => village.zone.radius,
                    "farm" => village.farm, "houses" => &houses, "props" => &props,
                },
            );
        }
        if let Some(river) = &field.river {
            let points: PackedVector2Array = river
                .polyline(10.0, field.width)
                .iter()
                .map(|(x, z)| Vector2::new(*x as f32, *z as f32))
                .collect();
            let fords: VarArray = river
                .fords
                .iter()
                .map(|f| {
                    vdict! { "x" => f.x, "z" => river.center_z(f.x), "half_width" => f.half_width }
                        .to_variant()
                })
                .collect();
            let widths: PackedFloat32Array = river
                .polyline(10.0, field.width)
                .iter()
                .map(|(x, _)| river.width_at(*x) as f32)
                .collect();
            let banks: VarArray = river
                .banks
                .iter()
                .map(|b| {
                    vdict! { "x0" => b.x0, "x1" => b.x1, "north" => b.north, "kind" => b.kind.key() }
                        .to_variant()
                })
                .collect();
            // The water runs towards the lower end of the field.
            let flow = if field.height(0.0, river.center_z(0.0))
                >= field.height(field.width, river.center_z(field.width))
            {
                1
            } else {
                -1
            };
            dict.set(
                "river",
                &vdict! {
                    "points" => &points, "width" => river.width, "fords" => &fords,
                    "widths" => &widths, "banks" => &banks, "flow" => flow,
                },
            );
        }
        let polyline = |points: &[(f64, f64)]| -> PackedVector2Array {
            points
                .iter()
                .map(|(x, z)| Vector2::new(*x as f32, *z as f32))
                .collect()
        };
        let bridges: VarArray = field
            .bridges
            .iter()
            .map(|b| {
                vdict! {
                    "x" => b.x, "z" => b.z, "yaw" => b.yaw(), "length" => b.length,
                    "width" => b.width, "span" => b.span, "deck" => b.deck, "stone" => b.stone,
                    "arches" => b.arches as i64,
                    "stream" => b.stream.map_or(-1, |s| s as i64),
                }
                .to_variant()
            })
            .collect();
        dict.set("bridges", &bridges);
        let streams: VarArray = field
            .streams
            .iter()
            .map(|s| {
                vdict! { "kind" => s.kind.key(), "points" => &polyline(&s.points), "width" => s.width }
                    .to_variant()
            })
            .collect();
        dict.set("streams", &streams);
        let roads: VarArray = field
            .roads
            .iter()
            .map(|r| {
                vdict! { "kind" => r.kind.key(), "points" => &polyline(&r.points), "width" => r.width }
                    .to_variant()
            })
            .collect();
        dict.set("roads", &roads);
        dict.set("decor", &decor_dict(&field.decor));
        if sim.siege().is_some() {
            dict.set("siege", &self.get_siege());
        }
        dict
    }

    /// Siege battle walls (empty dictionary in a field battle):
    /// `{fortification, center: Vector2, square_radius, thickness, wall_height,
    /// gate, hold_time, hold_to_win, integrity, pieces[{index, kind: "wall"|"gate",
    /// a: Vector2, b: Vector2, hp, max_hp, intact, docked_tower, under_attack (SB)}],
    /// towers[{x, z, radius, height}], houses[{x, z, radius, suburb, fire: {state:
    /// "intact"|"burning"|"burnt", intensity}, length, depth, yaw, rows, church}],
    /// props[{kind, x, z, yaw, length, depth, house}] (BR3), engines[{unit, kind:
    /// "ram"|"tower", side, x, z, hp, max_hp}] (SB), points[{kind: "square"|"gate",
    /// x, z, radius, progress, hold_s, share, status: "held"|"contested"|"capturing"|
    /// "taken", attackers, defenders}] (T4, ADR 0108), gate_fire: {state, intensity},
    /// wind: Vector2 (direction × strength 0-1), houses_burning, houses_burnt,
    /// sortie, ram_period, oil_period}`. Pieces lose HP and houses burn during the battle (S2): call it
    /// again to show the damage.
    #[func]
    fn get_siege(&self) -> VarDictionary {
        let Some(works) = self.sim.as_ref().and_then(|s| s.siege()) else {
            return VarDictionary::new();
        };
        let v2 = |p: (f64, f64)| Vector2::new(p.0 as f32, p.1 as f32);
        let pieces: VarArray = works
            .pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| {
                vdict! {
                    "index" => index as i64,
                    "kind" => match piece.kind {
                        sim_battle::PieceKind::Wall => "wall",
                        sim_battle::PieceKind::Gate => "gate",
                    },
                    "a" => v2(piece.a),
                    "b" => v2(piece.b),
                    "hp" => piece.hp,
                    "max_hp" => piece.max_hp,
                    "intact" => piece.intact(),
                    "docked_tower" => piece.docked_tower.map_or(-1, i64::from),
                    // SB (ADR 0107): battered, shot at or burning just now.
                    "under_attack" => piece.under_attack(),
                }
                .to_variant()
            })
            .collect();
        let towers: VarArray = works
            .towers
            .iter()
            .map(|t| {
                vdict! { "x" => t.x, "z" => t.z, "radius" => t.radius, "height" => t.height }
                    .to_variant()
            })
            .collect();
        // F5a: house blocks (obstacles of the siege pathing), `{x, z, radius}`;
        // S2: their fire (a burnt house no longer blocks) and the suburbs.
        let blaze = |b: &sim_battle::Blaze| {
            vdict! { "state" => b.state.key(), "intensity" => b.intensity }
        };
        let houses: VarArray = works
            .houses
            .iter()
            .map(|h| {
                vdict! {
                    "x" => h.x,
                    "z" => h.z,
                    "radius" => h.radius,
                    "suburb" => h.suburb,
                    "fire" => &blaze(&h.fire),
                    // BR3: the block's rectangle (length along (cos, sin)
                    // of yaw, row 0 facing (-sin, cos)), rows, church.
                    "length" => h.length,
                    "depth" => h.depth,
                    "yaw" => h.yaw,
                    "rows" => i64::from(h.rows),
                    "church" => h.church,
                    // NT1: a castle keep (drawn as a great tower).
                    "keep" => h.keep,
                    // NT8: the keep's height (0 for other buildings).
                    "height" => h.height,
                    // NT11: a keep with a crenellated terrace roof.
                    "terrace" => h.terrace,
                }
                .to_variant()
            })
            .collect();
        let props: VarArray = works
            .props
            .iter()
            .map(|p| prop_dict(p).to_variant())
            .collect();
        // SB (ADR 0107): rams and siege towers with their strength, for the
        // health bars.
        let engines: VarArray = self
            .sim
            .as_ref()
            .map(|s| s.siege_engines())
            .unwrap_or_default()
            .iter()
            .map(|e| {
                vdict! {
                    "unit" => i64::from(e.unit),
                    "kind" => e.kind.key(),
                    "side" => e.side.key(),
                    "x" => e.x,
                    "z" => e.z,
                    "hp" => e.hp,
                    "max_hp" => e.max_hp,
                }
                .to_variant()
            })
            .collect();
        // T4 (ADR 0108): capture points (market square, gate) with their
        // progress, for the flags and the capture bars.
        let points: VarArray = works
            .capture_points()
            .iter()
            .map(|p| {
                vdict! {
                    "kind" => p.kind.key(),
                    "x" => p.x,
                    "z" => p.z,
                    "radius" => p.radius,
                    "progress" => p.progress,
                    "hold_s" => p.hold_s,
                    "share" => p.share(),
                    "status" => p.status.key(),
                    "attackers" => p.attackers,
                    "defenders" => p.defenders,
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "fortification" => i64::from(works.fortification),
            "center" => v2(works.center),
            "square_radius" => works.square_radius,
            "thickness" => works.thickness,
            "wall_height" => works.wall_height,
            "gate" => works.gate as i64,
            "hold_time" => works.hold_time,
            "hold_to_win" => sim_battle::CaptureRules::bundled().square.hold_s,
            "points" => &points,
            "integrity" => works.integrity(),
            "pieces" => &pieces,
            "towers" => &towers,
            // NT1 (ADR 0126): "city" | "borough" | "castle".
            "place" => works.place.key(),
            "houses" => &houses,
            "props" => &props,
            "engines" => &engines,
            "sortie" => works.sortie,
            "gate_fire" => &blaze(&works.gate_fire),
            "wind" => v2(works.wind),
            "houses_burning" => works.burning_houses() as i64,
            "houses_burnt" => works.burnt_houses() as i64,
            // SG1: rhythm of the ram and of the oil pots (seconds).
            "ram_period" => sim_battle::siege_fx::RAM_PERIOD,
            "oil_period" => sim_battle::siege_fx::OIL_PERIOD,
        }
    }

    /// L3 (ADR 0026): the besieged town drawn from a landmark plan, `{id,
    /// name, gate_name, gatehouses: [{name, at: Vector2}], streets:
    /// [PackedVector2Array], quay: [piece index], plan_scale}`; empty for the
    /// generic town or a field battle.
    #[func]
    fn get_siege_landmark(&self) -> VarDictionary {
        let Some(landmark) = self
            .sim
            .as_ref()
            .and_then(|s| s.siege())
            .and_then(|w| w.landmark.as_ref())
        else {
            return VarDictionary::new();
        };
        let v2 = |p: (f64, f64)| Vector2::new(p.0 as f32, p.1 as f32);
        let gatehouses: VarArray = landmark
            .gatehouses
            .iter()
            .map(|(name, x, z)| {
                vdict! { "name" => name.as_str(), "at" => v2((*x, *z)) }.to_variant()
            })
            .collect();
        let streets: VarArray = landmark
            .streets
            .iter()
            .map(|s| {
                s.iter()
                    .map(|&p| v2(p))
                    .collect::<PackedVector2Array>()
                    .to_variant()
            })
            .collect();
        let quay: VarArray = landmark
            .quay
            .iter()
            .map(|&i| (i as i64).to_variant())
            .collect();
        vdict! {
            "id" => landmark.id.as_str(),
            "name" => landmark.name.as_str(),
            "gate_name" => landmark.gate_name.as_str(),
            "gatehouses" => &gatehouses,
            "streets" => &streets,
            "quay" => &quay,
            "plan_scale" => landmark.plan_scale,
        }
    }

    /// Debug (tests, captures, S2): sets house `house` on fire, or the gate
    /// when `house` < 0; `false` when it already burns or is not a siege.
    #[func]
    fn debug_ignite(&mut self, house: i64) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        let Some(sim) = &mut self.sim else {
            return false;
        };
        let target = usize::try_from(house).ok();
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::Ignite { house: target },
        );
        if house < 0 {
            sim.ignite_gate()
        } else {
            sim.ignite_house(house as usize)
        }
    }

    /// Debug (captures, SG1): sets the HP of wall piece `index` (clamped to
    /// its maximum; 0 opens it). `false` outside a siege or for a bad index.
    #[func]
    fn debug_set_piece_hp(&mut self, index: i64, hp: f64) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        if let (Some(sim), Ok(piece)) = (&self.sim, usize::try_from(index)) {
            note(
                &mut self.recorder,
                sim,
                sim_battle::ReplayAction::PieceHp { piece, hp },
            );
        }
        let Some(works) = self.sim.as_mut().and_then(|s| s.siege_mut()) else {
            return false;
        };
        let Some(piece) = usize::try_from(index)
            .ok()
            .and_then(|i| works.pieces.get_mut(i))
        else {
            return false;
        };
        piece.hp = hp.clamp(0.0, piece.max_hp);
        true
    }

    /// Ground height at (x, z).
    #[func]
    fn get_height(&self, x: f64, z: f64) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.field().height(x, z))
    }

    /// EP3: height one walks at (x, z): a bridge deck, else the ground.
    #[func]
    fn get_walk_height(&self, x: f64, z: f64) -> f64 {
        self.sim
            .as_ref()
            .map_or(0.0, |s| s.field().walk_height(x, z))
    }

    /// EP3 (for EP6, a water mill): a spot on a bank near (x, z), `setback`
    /// metres from the water, clear of fords, bridges and roads:
    /// `{x, z, yaw (facing the water, from +x towards +z), stream (-1: the
    /// river)}`, or an empty dictionary without water.
    #[func]
    fn get_waterside_spot(&self, x: f64, z: f64, setback: f64) -> VarDictionary {
        let Some(spot) = self
            .sim
            .as_ref()
            .and_then(|s| s.field().waterside_spot((x, z), setback))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "x" => spot.x, "z" => spot.z,
            "yaw" => spot.towards_water.1.atan2(spot.towards_water.0),
            "stream" => spot.stream.map_or(-1, |s| s as i64),
        }
    }

    /// B6: the battle site in one compact French line, e.g. « Terre gelée ·
    /// hiver · village · haies · côte ouest » (empty without a battle).
    #[func]
    fn get_site_label(&self) -> GString {
        self.sim
            .as_ref()
            .map(|sim| GString::from(sim.field().site_label_fr().as_str()))
            .unwrap_or_default()
    }

    /// `{key: "clear"|"rain"|"fog"|"snow", label}`.
    #[func]
    fn get_weather(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let weather = sim.weather();
        vdict! { "key" => weather.key(), "label" => weather.label_fr() }
    }

    /// The setup the battle was built from (names, factions, general).
    #[func]
    fn get_setup(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map(|s| to_dict(s.setup()))
            .unwrap_or_default()
    }

    /// Head count still able to fight on `side`.
    #[func]
    fn get_strength(&self, side: GString) -> i64 {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => i64::from(sim.strength(side)),
            _ => 0,
        }
    }

    /// Simulated seconds since the start.
    #[func]
    fn get_elapsed(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.elapsed())
    }

    #[func]
    fn get_ticks(&self) -> i64 {
        self.sim.as_ref().map_or(0, |s| s.ticks() as i64)
    }

    #[func]
    fn is_finished(&self) -> bool {
        self.sim.as_ref().is_some_and(|s| s.is_finished())
    }

    /// Result in the format accepted by `CampaignSim.resolve_battle` (empty
    /// while the battle goes on).
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .and_then(|s| s.outcome())
            .map(|o| to_dict(&o))
            .unwrap_or_default()
    }

    /// SG1: siege assault events for the 3D view, added since the last call:
    /// `[{time, kind, ...}]` with `kind` among `engine_shot {unit, piece, x, z,
    /// height (0-1), breached}`, `ram_strike {unit, piece, breached}`,
    /// `tower_volley {tower, target}`, `ladders_raised {unit, piece}`,
    /// `tower_docked {unit, piece}`, `tower_undocked {unit, piece}`,
    /// `on_wall {unit, piece}`, `boiling_oil {piece, x, z, targets: [ids]}`,
    /// `gate_broken {piece}`, `wall_breached {piece}`, `defenders_fall_back`.
    /// Rendering only: the rules are already applied.
    #[func]
    fn get_siege_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_siege_fx()
            .iter()
            .map(|fx| {
                serde_json::to_value(fx)
                    .map(|json| json_to_variant(&json))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Journal entries `[{time, text_fr, side}]` added since the last call.
    #[func]
    fn get_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_events()
            .iter()
            .map(|event| {
                vdict! {
                    "time" => event.time,
                    "text_fr" => event.text_fr.as_str(),
                    "side" => event.side.map_or("", |s| s.key()),
                }
                .to_variant()
            })
            .collect()
    }

    /// CB5: typed alerts `[{kind, time, x, z, side, unit}]` added since the
    /// last call. `kind` is one of `rout`, `general_down`, `flanked`,
    /// `reinforcements`, `ammo_out`, `wall_breached`, `gate_destroyed`, `square_threatened` (T4).
    /// `side` is `""` and `unit` is `-1` for a wall/gate piece. Output only:
    /// reading it never changes the simulation.
    #[func]
    fn get_alerts(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_alerts()
            .iter()
            .map(|alert| {
                vdict! {
                    "kind" => alert.kind.key(),
                    "time" => alert.time,
                    "x" => alert.x,
                    "z" => alert.z,
                    "side" => alert.side.map_or("", |s| s.key()),
                    "unit" => alert.unit.map_or(-1, |u| u as i64),
                }
                .to_variant()
            })
            .collect()
    }
}

/// CV3: the last battle classification as a dictionary (see
/// `CampaignSim.get_last_battle_outcome`).
fn battle_outcome_dict(state: &sim_campaign::CampaignState) -> VarDictionary {
    let Some(report) = &state.last_battle_outcome else {
        return VarDictionary::new();
    };
    let mut dict = vdict! {
        "turn" => i64::from(report.turn),
        "province" => report.province.as_str(),
        "attacker_faction" => report.attacker_faction.as_str(),
        "defender_faction" => report.defender_faction.as_str(),
        "attacker_class" => report.attacker.key.as_str(),
        "attacker_label" => report.attacker.label.as_str(),
        "defender_class" => report.defender.key.as_str(),
        "defender_label" => report.defender.label.as_str(),
    };
    if let Some(view) = report.class_of(&state.player_faction) {
        dict.set("player_class", view.key.as_str());
        dict.set("player_label", view.label.as_str());
    }
    dict
}

#[godot_api(secondary)]
impl CampaignSim {
    /// `[{index, attacker, defender, province, province_name, attacker_name,
    /// defender_name, player_side, attacker_strength, defender_strength, seed}]`.
    #[func]
    fn get_pending_battles(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        state
            .pending_battle_views(data)
            .iter()
            .map(|view| {
                let strength = |id: &sim_campaign::ArmyId| {
                    state.army(id).map_or(0, |a| i64::from(a.total_strength()))
                };
                let settlement_state = state.settlement_state(&view.location);
                let defender_strength = if view.siege {
                    settlement_state.map_or(0, |s| {
                        s.garrison.iter().map(|u| i64::from(u.strength)).sum()
                    })
                } else {
                    strength(&view.defender)
                };
                let breach = settlement_state
                    .and_then(|s| s.siege.as_ref())
                    .map_or(0, |s| i64::from(s.breach));
                let settlement_name = data
                    .settlements
                    .get(&view.location)
                    .map_or_else(|| view.location.to_string(), |s| s.name.display.clone());
                let province_name = data
                    .provinces
                    .get(&view.province)
                    .map_or_else(|| view.province.to_string(), |p| p.name.display.clone());
                let seed = state.seed
                    ^ (u64::from(state.turn()) << 20)
                    ^ ((view.index as u64) << 8)
                    ^ 0xBA77;
                vdict! {
                    "index" => view.index as i64,
                    "attacker" => view.attacker.as_str(),
                    "defender" => view.defender.as_str(),
                    "province" => view.province.as_str(),
                    "province_name" => province_name.as_str(),
                    "attacker_name" => view.attacker_name.as_str(),
                    "defender_name" => view.defender_name.as_str(),
                    "player_side" => view.player_side.map_or("", |s| s.key()),
                    "attacker_strength" => strength(&view.attacker),
                    "defender_strength" => defender_strength,
                    "siege" => view.siege,
                    "fortification" => i64::from(state.fortification_level(data, &view.location)),
                    "location" => view.location.as_str(),
                    "settlement_name" => settlement_name.as_str(),
                    "settlement_kind" => state.settlement_kind(&view.location).key(),
                    "breach" => breach,
                    "seed" => (seed & 0x7FFF_FFFF_FFFF) as i64,
                }
                .to_variant()
            })
            .collect()
    }

    /// Setup of pending battle `index` for `BattleSim.setup` (empty if unknown).
    #[func]
    fn get_battle_setup(&self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match state.battle_setup(data, index.max(0) as usize) {
            Ok(setup) => {
                let mut dict = to_dict(&setup);
                // EP7: in the province and years of a historical map, the
                // battle is fought on the real site.
                if let Some(dir) = crate::campaign_sim::loaded_data_dir() {
                    if let Some((map, text)) =
                        crate::historical_battles::campaign_site(&dir, &setup, state.year())
                    {
                        dict.set("historical_site", text.as_str());
                        dict.set("historical_site_id", map.id.as_str());
                        dict.set("historical_horizon", map.horizon_key().as_str());
                    }
                }
                // EP8: hour of the day drawn from the battle (turn, index,
                // province), no random stream consumed.
                let key = sim_battle::time_of_day::campaign_battle_key(
                    state.turn(),
                    index.max(0) as usize,
                    &setup.province,
                );
                dict.set(
                    "hour",
                    sim_battle::TimeOfDayRules::bundled().campaign_hour(key),
                );
                dict
            }
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_setup({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: estimated balance of pending battle `index` for the pre-battle
    /// screen (`battle_forecast.rs`): `{attacker_power, defender_power,
    /// attacker_share, attacker_win_chance, attacker_soldiers,
    /// defender_soldiers, attacker_reinforcements, defender_reinforcements,
    /// modifiers, can_withdraw, siege}`; empty if unknown.
    #[func]
    fn get_battle_forecast(&self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match state.battle_forecast(data, index.max(0) as usize) {
            Ok(forecast) => to_dict(&forecast),
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_forecast({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: the player calls off pending battle `index` (attacker only; an
    /// assault is postponed, the siege goes on) → `{ok, error, events}`.
    #[func]
    fn withdraw_pending_battle(&mut self, index: i64) -> VarDictionary {
        if self.refuse_while_turn_pending("withdraw_pending_battle") {
            return result_dict(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        match state.withdraw_pending_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error.to_string())),
        }
    }

    /// Applies a `BattleSim.get_outcome()` dictionary → `{ok, error, events}`.
    #[func]
    fn resolve_battle(&mut self, index: i64, outcome: VarDictionary) -> VarDictionary {
        if self.refuse_while_turn_pending("resolve_battle") {
            return result_dict(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        let before = state.last_battle_outcome.clone();
        let result = from_dict::<BattleOutcome>(&outcome)
            .map_err(|e| format!("résultat invalide : {e}"))
            .and_then(|outcome| {
                state
                    .resolve_pending_battle(data, index.max(0) as usize, &outcome)
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                // CV3: class of the result (heroic, disaster...).
                if state.last_battle_outcome != before {
                    dict.set("outcome", &battle_outcome_dict(state));
                }
                dict
            }
            Err(error) => result_dict(Err(error)),
        }
    }

    /// CV3: class of the last field battle (auto-resolved or 3D) →
    /// `{turn, province, attacker_faction, defender_faction, attacker_class,
    /// attacker_label, defender_class, defender_label[, player_class,
    /// player_label]}`; classes: `heroic`, `decisive`, `pyrrhic`, `victory`,
    /// `honourable_defeat`, `disaster`, `defeat`. Empty before any battle.
    #[func]
    fn get_last_battle_outcome(&self) -> VarDictionary {
        self.state
            .as_ref()
            .map_or_else(VarDictionary::new, battle_outcome_dict)
    }

    /// Auto-resolves pending battle `index` now; returns its events.
    #[func]
    fn auto_resolve_battle(&mut self, index: i64) -> VarArray {
        if self.refuse_while_turn_pending("auto_resolve_battle") {
            return VarArray::new();
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return VarArray::new();
        };
        match state.auto_resolve_pending(data, index.max(0) as usize) {
            Ok(events) => events_array(&events),
            Err(error) => {
                godot_warn!("CampaignSim.auto_resolve_battle({index}): {error}");
                VarArray::new()
            }
        }
    }

    /// Player setting: fight own battles in 3D (`true`, default) or always
    /// auto-resolve them.
    #[func]
    fn set_interactive_battles(&mut self, enabled: bool) {
        if self.refuse_while_turn_pending("set_interactive_battles") {
            return;
        }
        if let Some(state) = &mut self.state {
            state.interactive_battles = enabled;
        }
    }

    #[func]
    fn get_interactive_battles(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.interactive_battles)
    }

    /// Debug (smoke test, screenshots): puts `army` in siege of `province`
    /// (garrisoned if empty) and records a pending siege battle; returns its
    /// index or -1.
    #[func]
    fn debug_stage_siege(&mut self, army: GString, province: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_siege") {
            return -1;
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        let Ok(province) = data_model::ProvinceId::new(province.to_string().as_str()) else {
            return -1;
        };
        match state.debug_stage_siege(data, &army, &province) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_siege: {error}");
                -1
            }
        }
    }

    /// NT1 demo (captures): `army` besieges the first settlement laid out as
    /// a place of `kind` (`city`, `borough`, `castle`). Returns the battle
    /// index or -1.
    #[func]
    fn debug_stage_place_siege(&mut self, army: GString, kind: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_place_siege") {
            return -1;
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        match state.debug_stage_place_siege(data, &army, &kind.to_string()) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_place_siege: {error}");
                -1
            }
        }
    }

    /// SG2 demo: `army` besieges the town drawn from landmark plan
    /// `landmark` (`data/landmarks/<id>.json`, e.g. `avignon`, `bruges`),
    /// at war with its holder if needed. Returns the battle index or -1.
    #[func]
    fn debug_stage_landmark_siege(&mut self, army: GString, landmark: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_landmark_siege") {
            return -1;
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        match state.debug_stage_landmark_siege(data, &army, &landmark.to_string()) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_landmark_siege: {error}");
                -1
            }
        }
    }

    /// Debug (smoke test, screenshots): brings `defender` to `attacker` and
    /// records a pending battle; returns its index or -1.
    #[func]
    fn debug_stage_battle(&mut self, attacker: GString, defender: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_battle") {
            return -1;
        }
        let Some(state) = &mut self.state else {
            return -1;
        };
        let (Some(a), Some(d)) = (
            sim_campaign::ArmyId::parse(&attacker.to_string()),
            sim_campaign::ArmyId::parse(&defender.to_string()),
        ) else {
            return -1;
        };
        match state.debug_stage_battle(&a, &d) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_battle: {error}");
                -1
            }
        }
    }
}

/// `snake_case` key of a battle season (B5, `get_terrain`).
#[godot_api(secondary)]
impl BattleSim {
    /// EP6: state of each side's camp, `[{side, progress (0-1), looted,
    /// alarmed, looters, guards}]` (empty without camps).
    #[func]
    fn get_camps(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim_battle::SideId::BOTH
            .iter()
            .filter_map(|&side| {
                let state = sim.camp_state(side)?;
                Some(
                    vdict! {
                        "side" => side.key(),
                        "progress" => state.progress,
                        "looted" => state.looted,
                        "alarmed" => state.alarmed,
                        "looters" => state.looters as i64,
                        "guards" => state.guards as i64,
                    }
                    .to_variant(),
                )
            })
            .collect()
    }
}

fn season_key(season: sim_battle::BattleSeason) -> &'static str {
    match season {
        sim_battle::BattleSeason::Spring => "spring",
        sim_battle::BattleSeason::Summer => "summer",
        sim_battle::BattleSeason::Autumn => "autumn",
        sim_battle::BattleSeason::Winter => "winter",
    }
}
