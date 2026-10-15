//! Battle sound decisions, pure logic (no Bevy ECS, no files): which sound plays where, derived
//! from the regiment states. Port of the decision half of game/scripts/audio/battle_audio.gd
//! (`_detect_events`, `_on_volley`, `_update_beds`, `_update_fronts`, `_is_duplicate_cry`) and of
//! the detection half of battle_voices.gd. [`super::battle`] plays what is decided here.
//!
//! Tunables come from `data/audio/sound_bank.json` (`tuning`, `battle`, `fronts`, `voices`).

use std::collections::{HashMap, HashSet};

use bevy::math::Vec3;
use serde_json::Value;

use super::playlist::XorShift;

/// Simplified regiment state (`UnitState` of the core).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Marching,
    Charging,
    Melee,
    Shooting,
    Routing,
    Rallied,
}

/// What the audio needs of a regiment, read from the simulation each frame.
#[derive(Debug, Clone)]
pub struct UnitView {
    pub id: u32,
    pub side: u8,
    pub phase: Phase,
    pub present: bool,
    pub soldiers: u32,
    pub ammo: u32,
    pub target: Option<u32>,
    /// World position, 1.5 m above the ground (`BattleAudio._pos`).
    pub position: Vec3,
    /// Unit vector of the facing, `(sin, 0, cos)`.
    pub forward: Vec3,
    pub depth: f32,
    pub mounted: bool,
    pub is_general: bool,
    pub siege: bool,
    pub unit_type: String,
    pub category: String,
}

/// A sound to play: bank event at a place, after `delay_s` of battle time.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub event: String,
    pub position: Vec3,
    pub gain_db: f32,
    pub delay_s: f32,
}

impl Cue {
    fn now(event: &str, position: Vec3, gain_db: f32) -> Self {
        Self {
            event: event.to_string(),
            position,
            gain_db,
            delay_s: 0.0,
        }
    }
}

/// A shouted line wanted for a regiment (`BattleVoices.play`).
#[derive(Debug, Clone, PartialEq)]
pub struct BarkRequest {
    pub situation: &'static str,
    pub unit: u32,
    pub position: Vec3,
}

/// Constants of the battle soundscape (`bank.tuning`, `bank.battle`, `bank.fronts`).
#[derive(Debug, Clone)]
pub struct Tuning {
    pub bed_radius: f32,
    pub melee_full: f32,
    pub march_full: f32,
    pub cavalry_full: f32,
    pub bed_smoothing: f32,
    pub death_groan_chance: f32,
    pub bell_period: f32,
    pub bell_until: f32,
    pub front_near_spread: f32,
    pub front_mid_spread: f32,
    pub dedup_radius: f32,
    pub near_distance: f32,
    pub missile_speed: HashMap<String, f32>,
    pub near_events: Vec<(String, u32)>,
    pub dedup_window: HashMap<String, f32>,
    pub max_emitters: usize,
    pub near_m: f32,
    pub mid_m: f32,
    pub engaged_full: f32,
    pub event_period: [f32; 2],
    pub front_beds: Vec<String>,
}

impl Default for Tuning {
    fn default() -> Self {
        Self::from_json(&Value::Null)
    }
}

impl Tuning {
    pub fn from_json(root: &Value) -> Self {
        let tuning = |key: &str, default: f64| {
            root.pointer(&format!("/tuning/{key}"))
                .and_then(Value::as_f64)
                .unwrap_or(default) as f32
        };
        let fronts = |key: &str, default: f64| {
            root.pointer(&format!("/fronts/{key}"))
                .and_then(Value::as_f64)
                .unwrap_or(default) as f32
        };
        let names = |pointer: &str| -> Vec<String> {
            root.pointer(pointer)
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        };
        let map = |pointer: &str| -> HashMap<String, f32> {
            root.pointer(pointer)
                .and_then(Value::as_object)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|(key, value)| Some((key.clone(), value.as_f64()? as f32)))
                        .collect()
                })
                .unwrap_or_default()
        };
        let near_events: Vec<(String, u32)> = root
            .pointer("/battle/near_events")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|entry| {
                        Some((
                            entry.get("event")?.as_str()?.to_string(),
                            entry.get("weight")?.as_u64()? as u32,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let period = root
            .pointer("/fronts/event_period_s")
            .and_then(Value::as_array)
            .filter(|span| span.len() >= 2)
            .map(|span| {
                [
                    span[0].as_f64().unwrap_or(0.5) as f32,
                    span[1].as_f64().unwrap_or(1.6) as f32,
                ]
            })
            .unwrap_or([0.5, 1.6]);
        let mut front_beds = names("/fronts/beds");
        if front_beds.is_empty() {
            front_beds = vec!["melee_bed_1".into(), "melee_bed_2".into()];
        }
        Self {
            bed_radius: tuning("bed_radius_m", 260.0),
            melee_full: tuning("melee_full", 420.0),
            march_full: tuning("march_full", 500.0),
            cavalry_full: tuning("cavalry_full", 120.0),
            bed_smoothing: tuning("bed_smoothing", 2.5),
            death_groan_chance: tuning("death_groan_chance", 0.25),
            bell_period: tuning("bell_period_s", 22.0),
            bell_until: tuning("bell_until_s", 150.0),
            front_near_spread: tuning("front_near_spread_m", 6.0),
            front_mid_spread: tuning("front_mid_spread_m", 14.0),
            dedup_radius: tuning("dedup_radius_m", 40.0),
            near_distance: root
                .pointer("/voices/near_distance_m")
                .and_then(Value::as_f64)
                .unwrap_or(70.0) as f32,
            missile_speed: map("/battle/missile_speed_mps"),
            near_events,
            dedup_window: map("/battle/dedup_window_s"),
            max_emitters: fronts("max_emitters", 6.0) as usize,
            near_m: fronts("near_m", 40.0),
            mid_m: fronts("mid_m", 200.0),
            engaged_full: fronts("engaged_full", 250.0),
            event_period: period,
            front_beds,
        }
    }
}

/// Kind of missile loosed by a regiment (`BattleAudio.missile_kind`: the core keeps it private,
/// so the same special cases are recognised by unit type id).
pub fn missile_kind(unit_type: &str, siege: bool) -> &'static str {
    if siege {
        return if unit_type == "unit_bombard" {
            "ball"
        } else {
            "stone"
        };
    }
    match unit_type {
        "unit_culveriners" => "bullet",
        "unit_jinetes" | "unit_lithuanian_light_cavalry" => "javelin",
        t if t.contains("crossbow") => "bolt",
        _ => "arrow",
    }
}

/// Intensity 0..1 of a bed: square root, several regiments do not sound n times louder.
pub fn level(weight: f32, full: f32) -> f32 {
    (weight.max(0.0) / full).sqrt().clamp(0.0, 1.0)
}

/// Camera height (m) to 0 (ground) .. 1 (high view).
pub fn far01(camera_height: f32) -> f32 {
    ((camera_height - 25.0) / 260.0).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Default)]
struct Track {
    phase: Phase,
    ammo: u32,
    soldiers: u32,
    present: bool,
}

/// Target of one looping bed this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct BedTarget {
    pub name: String,
    pub level: f32,
    pub center: Option<Vec3>,
}

/// One melee front (a pair of regiments in contact).
#[derive(Debug, Clone)]
pub struct Front {
    pub key: String,
    pub position: Vec3,
    pub engaged: f32,
    pub losses: f32,
    pub distance: f32,
}

/// Per-front emitter state: smoothed level, shock timer, last shock type.
#[derive(Debug, Clone, Default)]
pub struct FrontEmitter {
    pub bed_name: String,
    pub level: f32,
    pub timer: f32,
    pub last_event: String,
}

/// What a front emitter does this frame.
#[derive(Debug, Clone)]
pub struct FrontDrive {
    pub key: String,
    pub bed_name: String,
    pub position: Vec3,
    /// Level of the mass bed (0 close up, where the individual shocks take over).
    pub bed_level: f32,
}

/// Decides every battle sound from the regiment states.
pub struct Detector {
    pub tuning: Tuning,
    track: HashMap<u32, Track>,
    side_cried: HashSet<String>,
    rng: XorShift,
    dedup_last: HashMap<String, (f32, Vec3)>,
    front_soldiers: HashMap<u32, f32>,
    pub emitters: HashMap<String, FrontEmitter>,
    victory_done: bool,
    pieces: HashMap<usize, (f32, bool)>,
    next_bell: f32,
}

/// A wall or gate piece of the siege works, as far as sound is concerned.
#[derive(Debug, Clone)]
pub struct PieceView {
    pub index: usize,
    pub hp: f32,
    pub gate: bool,
    /// Middle of the piece, 4 m above the ground.
    pub middle: Vec3,
}

impl Detector {
    pub fn new(tuning: Tuning, seed: u64) -> Self {
        Self {
            tuning,
            track: HashMap::new(),
            side_cried: HashSet::new(),
            rng: XorShift::new(seed),
            dedup_last: HashMap::new(),
            front_soldiers: HashMap::new(),
            emitters: HashMap::new(),
            victory_done: false,
            pieces: HashMap::new(),
            next_bell: 4.0,
        }
    }

    /// Siege sounds (`update_siege`): alarm bell, ram blows on the gate, stones on the walls,
    /// collapse of a piece. `stones` false when the siege renderer plays the stone impacts itself.
    pub fn siege(
        &mut self,
        center: Vec3,
        elapsed: f32,
        pieces: &[PieceView],
        stones: bool,
    ) -> Vec<Cue> {
        let mut cues = Vec::new();
        if elapsed < self.tuning.bell_until && elapsed >= self.next_bell {
            self.next_bell = elapsed + self.tuning.bell_period;
            cues.push(Cue::now(
                "bell_toll",
                center + Vec3::new(0.0, 25.0, 0.0),
                0.0,
            ));
        }
        for piece in pieces {
            let intact = piece.hp > 0.0;
            if let Some((hp, was_intact)) = self.pieces.get(&piece.index).copied() {
                if piece.hp < hp - 0.001 {
                    if piece.gate {
                        cues.push(Cue::now("ram_hit", piece.middle, 0.0));
                    } else if stones {
                        cues.push(Cue::now("stone_impact", piece.middle, 0.0));
                    }
                }
                if was_intact && !intact {
                    cues.push(Cue::now("wall_collapse", piece.middle, 0.0));
                }
            }
            self.pieces.insert(piece.index, (piece.hp, intact));
        }
        cues
    }

    pub fn unit_random(&mut self) -> f32 {
        self.rng.unit()
    }

    /// True if the same cry just played near `position` (`dedup_window_s` of the bank); else
    /// remembers this one. `now` is real time in seconds.
    pub fn is_duplicate(&mut self, event: &str, position: Vec3, now: f32) -> bool {
        let Some(window) = self.tuning.dedup_window.get(event).copied() else {
            return false;
        };
        if let Some((time, last)) = self.dedup_last.get(event)
            && now - time < window
            && position.distance(*last) <= self.tuning.dedup_radius
        {
            return true;
        }
        self.dedup_last.insert(event.to_string(), (now, position));
        false
    }

    /// Whether a side already shouted its war cry; marks it.
    pub fn side_cried(&self, side: u8) -> bool {
        self.side_cried.contains(&side.to_string())
    }

    /// Cues and barks from the state transitions since the previous call (battle clock only).
    pub fn detect(
        &mut self,
        units: &[UnitView],
        elapsed: f32,
        listener: Option<Vec3>,
    ) -> (Vec<Cue>, Vec<BarkRequest>) {
        let mut cues = Vec::new();
        let mut barks = Vec::new();
        let by_id: HashMap<u32, &UnitView> = units.iter().map(|unit| (unit.id, unit)).collect();
        for unit in units {
            let previous = self.track.insert(
                unit.id,
                Track {
                    phase: unit.phase,
                    ammo: unit.ammo,
                    soldiers: unit.soldiers,
                    present: unit.present,
                },
            );
            let Some(previous) = previous else { continue };
            let position = unit.position;
            if unit.is_general && previous.present && (!unit.present || unit.soldiers == 0) {
                cues.push(Cue::now("general_death", position, 0.0));
                if let Some(witness) = nearest_ally(units, unit) {
                    barks.push(BarkRequest {
                        situation: "general_down",
                        unit: witness.id,
                        position: witness.position,
                    });
                }
                continue;
            }
            if !unit.present {
                continue;
            }
            let side_key = unit.side.to_string();
            if unit.phase == Phase::Marching
                && previous.phase != Phase::Marching
                && self.side_cried.insert(format!("{side_key}:drum"))
            {
                cues.push(Cue::now("drum", position, 0.0));
            }
            if unit.phase == Phase::Charging && previous.phase != Phase::Charging {
                if self.side_cried.insert(side_key.clone()) {
                    cues.push(Cue::now("war_cry", position + unit.forward * 4.0, 0.0));
                    barks.push(BarkRequest {
                        situation: "war_cry",
                        unit: unit.id,
                        position,
                    });
                } else {
                    barks.push(BarkRequest {
                        situation: "charge",
                        unit: unit.id,
                        position,
                    });
                }
                if unit.mounted && self.side_cried.insert(format!("{side_key}:horn")) {
                    cues.push(Cue::now("horn", position, 0.0));
                }
                cues.push(Cue::now("charge_cry", position, 0.0));
                if unit.mounted {
                    cues.push(Cue::now("horse_neigh", position, 0.0));
                    cues.push(Cue::now("cavalry_charge_impact", position, 0.0));
                }
            }
            if unit.phase == Phase::Melee && previous.phase != Phase::Melee {
                let front = position + unit.forward * unit.depth * 0.5;
                let gain = if previous.phase == Phase::Charging {
                    2.0
                } else {
                    0.0
                };
                cues.push(Cue::now("contact", front, gain));
            }
            if unit.phase == Phase::Routing && previous.phase != Phase::Routing {
                cues.push(Cue::now("rout_cry", position, 0.0));
                barks.push(BarkRequest {
                    situation: "rout",
                    unit: unit.id,
                    position,
                });
            }
            if unit.phase == Phase::Rallied && previous.phase != Phase::Rallied {
                barks.push(BarkRequest {
                    situation: "rally",
                    unit: unit.id,
                    position,
                });
            }
            if unit.ammo < previous.ammo {
                self.volley(unit, &by_id, listener, &mut cues);
            }
            let lost = previous.soldiers.saturating_sub(unit.soldiers);
            if lost > 0
                && unit.phase == Phase::Melee
                && self.rng.unit() < self.tuning.death_groan_chance * (lost.min(3) as f32)
            {
                let jitter = Vec3::new(
                    (self.rng.unit() - 0.5) * 10.0,
                    0.0,
                    (self.rng.unit() - 0.5) * 10.0,
                );
                cues.push(Cue::now("death_groan", position + jitter, 0.0));
            }
        }
        let _ = elapsed;
        (cues, barks)
    }

    /// Volley: release sound, whistle in flight, impact when it lands, fly-by over the camera.
    fn volley(
        &self,
        unit: &UnitView,
        by_id: &HashMap<u32, &UnitView>,
        listener: Option<Vec3>,
        cues: &mut Vec<Cue>,
    ) {
        let position = unit.position;
        let kind = missile_kind(&unit.unit_type, unit.siege);
        match kind {
            "ball" => cues.push(Cue::now("bombard", position, 0.0)),
            "stone" => cues.push(Cue::now("trebuchet_release", position, 0.0)),
            "bolt" => cues.push(Cue::now("crossbow_release", position, 0.0)),
            // No dedicated sample: softened bombard (BattleVolleys.on_shot).
            "bullet" => cues.push(Cue::now("bombard", position, -14.0)),
            _ => cues.push(Cue::now("bow_release", position, 0.0)),
        }
        let Some(target) = unit.target.and_then(|id| by_id.get(&id)) else {
            return;
        };
        let aim = target.position;
        let speed = self
            .tuning
            .missile_speed
            .get(kind)
            .copied()
            .unwrap_or(48.0)
            .max(1.0);
        let flight = position.distance(aim) / speed;
        let delayed = |event: &str, at: Vec3, delay: f32| Cue {
            event: event.to_string(),
            position: at,
            gain_db: 0.0,
            delay_s: delay,
        };
        if kind == "arrow" || kind == "bolt" {
            cues.push(delayed(
                "arrow_whistle",
                position.lerp(aim, 0.55) + Vec3::new(0.0, 12.0, 0.0),
                (flight * 0.35).min(1.5),
            ));
            cues.push(delayed("arrow_impact", aim, flight));
            if let Some(listener) = listener
                && let Some((over, fraction)) = flyby_point(position, aim, listener)
            {
                cues.push(delayed("arrow_flyby", over, flight * fraction));
            }
        } else {
            cues.push(delayed("stone_impact", aim, flight));
        }
    }

    /// Looping bed targets: melee (3 layers), march, cavalry, fire, plus the 2D distant-battle
    /// level (second value) heard from a high camera.
    pub fn beds(
        &self,
        units: &[UnitView],
        focus: Vec3,
        far: f32,
        fires: &[(Vec3, f32)],
    ) -> (Vec<BedTarget>, f32) {
        #[derive(Default, Clone, Copy)]
        struct Sum {
            weight: f32,
            center: Vec3,
        }
        let mut melee = Sum::default();
        let mut march = Sum::default();
        let mut cavalry = Sum::default();
        let mut engaged_total = 0.0;
        for unit in units.iter().filter(|unit| unit.present) {
            let soldiers = unit.soldiers as f32;
            let slot = match unit.phase {
                Phase::Melee => {
                    engaged_total += soldiers;
                    &mut melee
                }
                Phase::Charging | Phase::Marching | Phase::Routing if unit.mounted => &mut cavalry,
                Phase::Charging | Phase::Marching | Phase::Routing => &mut march,
                _ => continue,
            };
            let flat = Vec3::new(unit.position.x - focus.x, 0.0, unit.position.z - focus.z);
            let distance = flat.length();
            if distance > self.tuning.bed_radius {
                continue;
            }
            let weight = soldiers * (1.0 - distance / self.tuning.bed_radius);
            slot.weight += weight;
            slot.center += unit.position * weight;
        }
        let center = |sum: Sum| (sum.weight > 0.0).then(|| sum.center / sum.weight);
        let melee_level = level(melee.weight, self.tuning.melee_full);
        let near = 1.0 - far * 0.7;
        let mut targets = vec![
            BedTarget {
                name: "melee_bed_1".into(),
                level: melee_level * near,
                center: center(melee),
            },
            BedTarget {
                name: "melee_bed_2".into(),
                level: (melee_level * 1.6 - 0.6).clamp(0.0, 1.0) * near,
                center: center(melee),
            },
            BedTarget {
                name: "clamor_bed".into(),
                level: melee_level * (0.6 + 0.4 * near),
                center: center(melee),
            },
            BedTarget {
                name: "march_bed".into(),
                level: level(march.weight, self.tuning.march_full) * near,
                center: center(march),
            },
            BedTarget {
                name: "cavalry_bed".into(),
                level: level(cavalry.weight, self.tuning.cavalry_full) * near,
                center: center(cavalry),
            },
        ];
        let nearest_fire = fires
            .iter()
            .min_by(|a, b| a.0.distance(focus).total_cmp(&b.0.distance(focus)));
        targets.push(BedTarget {
            name: "fire_bed".into(),
            level: if fires.is_empty() {
                0.0
            } else {
                (fires.len() as f32 / 4.0).clamp(0.0, 1.0)
            },
            center: nearest_fire.map(|fire| fire.0),
        });
        let distant =
            level(engaged_total, self.tuning.melee_full * 3.0) * (far * 1.4).clamp(0.0, 1.0);
        (targets, distant)
    }

    /// Melee fronts: pairs of regiments in contact, nearest `max_emitters` to the listener.
    pub fn fronts(&mut self, units: &[UnitView], listener: Vec3) -> Vec<Front> {
        let by_id: HashMap<u32, &UnitView> = units.iter().map(|unit| (unit.id, unit)).collect();
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for unit in units {
            if !unit.present || unit.phase != Phase::Melee {
                continue;
            }
            let Some(target) = unit.target.and_then(|id| by_id.get(&id)) else {
                continue;
            };
            if !target.present || target.phase != Phase::Melee {
                continue;
            }
            let key = format!("{}:{}", unit.id.min(target.id), unit.id.max(target.id));
            if !seen.insert(key.clone()) {
                continue;
            }
            let previous_a = self
                .front_soldiers
                .get(&unit.id)
                .copied()
                .unwrap_or(unit.soldiers as f32);
            let previous_b = self
                .front_soldiers
                .get(&target.id)
                .copied()
                .unwrap_or(target.soldiers as f32);
            let losses = (previous_a - unit.soldiers as f32).max(0.0)
                + (previous_b - target.soldiers as f32).max(0.0);
            let position = (unit.position + target.position) * 0.5;
            candidates.push(Front {
                key,
                position,
                engaged: (unit.soldiers + target.soldiers) as f32,
                losses,
                distance: listener.distance(position),
            });
        }
        for unit in units.iter().filter(|unit| unit.present) {
            self.front_soldiers.insert(unit.id, unit.soldiers as f32);
        }
        candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        candidates.truncate(self.tuning.max_emitters);
        candidates
    }

    /// Advances the emitters of the fronts: smoothed levels, mass-bed gain, individual shocks
    /// (dense close up, sparse between `near_m` and `mid_m`). Fronts that dropped out of the
    /// list or are beyond `mid_m` lose their emitter (their key is in the second result).
    pub fn drive_fronts(
        &mut self,
        fronts: &[Front],
        dt: f32,
        real_dt: f32,
    ) -> (Vec<FrontDrive>, Vec<Cue>, Vec<String>) {
        let mut drives = Vec::new();
        let mut cues = Vec::new();
        let mut active = HashSet::new();
        let tuning = self.tuning.clone();
        for (rank, front) in fronts.iter().enumerate() {
            if front.distance >= tuning.mid_m {
                continue;
            }
            active.insert(front.key.clone());
            let target_level = level(front.engaged, tuning.engaged_full);
            let burst = (front.losses / 6.0).clamp(0.0, 1.0);
            let smoothing = (real_dt * tuning.bed_smoothing).clamp(0.0, 1.0);
            let bed_name = tuning.front_beds[rank % tuning.front_beds.len()].clone();
            let emitter = self
                .emitters
                .entry(front.key.clone())
                .or_insert_with(|| FrontEmitter {
                    bed_name,
                    ..Default::default()
                });
            emitter.level += (target_level - emitter.level) * smoothing;
            let mid_gain =
                ((front.distance - tuning.near_m) / tuning.near_m.max(1.0)).clamp(0.0, 1.0);
            let bed_name = emitter.bed_name.clone();
            let bed_level = emitter.level * mid_gain;
            emitter.timer -= dt;
            let fire_shock = emitter.timer <= 0.0 && target_level > 0.05 && dt > 0.0;
            if fire_shock {
                let near = front.distance < tuning.near_m;
                let sparse = if near { 1.0 } else { 2.6 };
                let density = (target_level * (1.0 + burst)).clamp(0.05, 2.0);
                let period = tuning.event_period[1]
                    + (tuning.event_period[0] - tuning.event_period[1]) * density.clamp(0.0, 1.0);
                emitter.timer = sparse * period;
                let spread = if near {
                    tuning.front_near_spread
                } else {
                    tuning.front_mid_spread
                };
                let count = if near && burst > 0.3 { 2 } else { 1 };
                for _ in 0..count {
                    let offset = Vec3::new(
                        (self.rng.unit() * 2.0 - 1.0) * spread,
                        0.0,
                        (self.rng.unit() * 2.0 - 1.0) * spread,
                    );
                    let last = self
                        .emitters
                        .get(&front.key)
                        .map(|emitter| emitter.last_event.clone())
                        .unwrap_or_default();
                    let event = self.pick_near_event(&last, burst);
                    if let Some(emitter) = self.emitters.get_mut(&front.key) {
                        emitter.last_event = event.clone();
                    }
                    cues.push(Cue::now(&event, front.position + offset, 0.0));
                }
            }
            drives.push(FrontDrive {
                key: front.key.clone(),
                bed_name,
                position: front.position,
                bed_level,
            });
        }
        let dropped: Vec<String> = self
            .emitters
            .keys()
            .filter(|key| !active.contains(*key))
            .cloned()
            .collect();
        for key in &dropped {
            self.emitters.remove(key);
        }
        (drives, cues, dropped)
    }

    /// Weighted draw of a close-range sound, never the same type twice in a row on a front;
    /// recent losses weigh more falls and groans (the last two of the list).
    pub fn pick_near_event(&mut self, last: &str, burst: f32) -> String {
        let near = &self.tuning.near_events;
        if near.is_empty() {
            return "sword_clash".into();
        }
        let mut weights: Vec<u32> = near.iter().map(|(_, weight)| *weight).collect();
        if burst > 0.4 && weights.len() >= 2 {
            let n = weights.len();
            weights[n - 2] += 3;
            weights[n - 1] += 3;
        }
        let total: u32 = weights.iter().sum();
        let pick = (self.rng.unit() * total.max(1) as f32) as u32;
        let mut acc = 0;
        let mut chosen = 0;
        for (index, weight) in weights.iter().enumerate() {
            acc += weight;
            if pick < acc {
                chosen = index;
                break;
            }
        }
        if near[chosen].0 == last && near.len() > 1 {
            chosen = (chosen + 1) % near.len();
        }
        near[chosen].0.clone()
    }

    /// Victory shout once, when the player's side wins (`BattleVoices` victory).
    pub fn victory_once(&mut self) -> bool {
        !std::mem::replace(&mut self.victory_done, true)
    }
}

/// Inverse-distance attenuation of a spatial sound (Godot `ATTENUATION_INVERSE_DISTANCE`):
/// full inside `unit_size`, `unit_size / distance` beyond, fading to silence over the last fifth
/// of `max_distance`, zero past it.
pub fn attenuation(distance: f32, unit_size: f32, max_distance: f32) -> f32 {
    if distance >= max_distance {
        return 0.0;
    }
    let base = (unit_size.max(0.1) / distance.max(unit_size.max(0.1))).min(1.0);
    let fade_start = max_distance * 0.8;
    if distance <= fade_start {
        base
    } else {
        base * (max_distance - distance) / (max_distance - fade_start)
    }
}

/// Spacing of the shouted lines (`BattleVoices.play`): one at a time, a higher priority cuts the
/// current one, a minimal gap between any two, cooldown per situation.
#[derive(Debug, Default)]
pub struct BarkGate {
    last_any: f32,
    current_priority: i32,
    current_until: f32,
    last_situation: HashMap<String, f32>,
    primed: bool,
}

impl BarkGate {
    /// Whether a line of `situation` may sound at `now` (real seconds); books it if so.
    pub fn admit(
        &mut self,
        situation: &str,
        priority: i32,
        cooldown: f32,
        global_gap: f32,
        now: f32,
        length: f32,
    ) -> bool {
        if !self.primed {
            self.primed = true;
            self.last_any = -1000.0;
            self.current_priority = -1;
        }
        let speaking = now < self.current_until;
        if speaking && priority <= self.current_priority {
            return false;
        }
        if !speaking && now - self.last_any < global_gap {
            return false;
        }
        let last = self
            .last_situation
            .get(situation)
            .copied()
            .unwrap_or(-1000.0);
        if now - last < cooldown {
            return false;
        }
        self.last_situation.insert(situation.to_string(), now);
        self.last_any = now;
        self.current_priority = priority;
        self.current_until = now + length;
        true
    }
}

fn nearest_ally<'a>(units: &'a [UnitView], fallen: &UnitView) -> Option<&'a UnitView> {
    units
        .iter()
        .filter(|unit| unit.id != fallen.id && unit.side == fallen.side && unit.present)
        .min_by(|a, b| {
            a.position
                .distance_squared(fallen.position)
                .total_cmp(&b.position.distance_squared(fallen.position))
        })
}

/// Where (and at which fraction of the flight) a volley passes within 55 m of the listener, for
/// the extra fly-by whistle right above the camera.
pub fn flyby_point(from: Vec3, to: Vec3, listener: Vec3) -> Option<(Vec3, f32)> {
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    let (from, to, ear) = (flat(from), flat(to), flat(listener));
    let segment = to - from;
    let length_sq = segment.length_squared();
    if length_sq < 1.0 {
        return None;
    }
    let t = ((ear - from).dot(segment) / length_sq).clamp(0.0, 1.0);
    let closest = from.lerp(to, t);
    if closest.distance(ear) > 55.0 {
        return None;
    }
    Some((Vec3::new(closest.x, listener.y + 10.0, closest.z), t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, side: u8, phase: Phase) -> UnitView {
        UnitView {
            id,
            side,
            phase,
            present: true,
            soldiers: 100,
            ammo: 10,
            target: None,
            position: Vec3::new(id as f32 * 10.0, 1.5, 0.0),
            forward: Vec3::Z,
            depth: 6.0,
            mounted: false,
            is_general: false,
            siege: false,
            unit_type: "unit_men_at_arms".into(),
            category: "infantry".into(),
        }
    }

    fn detector() -> Detector {
        let bank: Value =
            serde_json::from_str(include_str!("../../../data/audio/sound_bank.json")).unwrap();
        Detector::new(Tuning::from_json(&bank), 7)
    }

    #[test]
    fn tuning_reads_the_bank() {
        let tuning = detector().tuning;
        assert_eq!(tuning.max_emitters, 6);
        assert_eq!(tuning.near_events.len(), 6);
        assert_eq!(tuning.missile_speed["arrow"], 48.0);
        assert_eq!(tuning.dedup_window["rout_cry"], 2.0);
        assert_eq!(tuning.front_beds.len(), 3);
    }

    #[test]
    fn first_charge_of_a_side_is_a_war_cry_then_barks() {
        let mut d = detector();
        let idle = [unit(1, 0, Phase::Idle), unit(2, 0, Phase::Idle)];
        assert!(d.detect(&idle, 0.0, None).0.is_empty());
        let charging = [unit(1, 0, Phase::Charging), unit(2, 0, Phase::Idle)];
        let (cues, barks) = d.detect(&charging, 1.0, None);
        assert!(cues.iter().any(|cue| cue.event == "war_cry"));
        assert!(cues.iter().any(|cue| cue.event == "charge_cry"));
        assert_eq!(barks.len(), 1);
        assert_eq!(barks[0].situation, "war_cry");
        let second = [unit(1, 0, Phase::Charging), unit(2, 0, Phase::Charging)];
        let (cues, barks) = d.detect(&second, 2.0, None);
        assert!(!cues.iter().any(|cue| cue.event == "war_cry"));
        assert_eq!(barks[0].situation, "charge");
    }

    #[test]
    fn contact_after_a_charge_is_louder() {
        let mut d = detector();
        d.detect(&[unit(1, 0, Phase::Charging)], 0.0, None);
        let (cues, _) = d.detect(&[unit(1, 0, Phase::Melee)], 1.0, None);
        let contact = cues.iter().find(|cue| cue.event == "contact").unwrap();
        assert_eq!(contact.gain_db, 2.0);
        assert!(contact.position.z > 2.0);
    }

    #[test]
    fn volley_schedules_whistle_and_impact() {
        let mut d = detector();
        let mut archer = unit(1, 0, Phase::Shooting);
        archer.unit_type = "unit_longbowmen".into();
        archer.target = Some(2);
        let mut enemy = unit(2, 1, Phase::Idle);
        enemy.position = Vec3::new(100.0, 1.5, 0.0);
        d.detect(&[archer.clone(), enemy.clone()], 0.0, None);
        archer.ammo = 9;
        let (cues, _) = d.detect(&[archer, enemy], 1.0, Some(Vec3::new(50.0, 30.0, 10.0)));
        let names: Vec<&str> = cues.iter().map(|cue| cue.event.as_str()).collect();
        assert!(names.contains(&"bow_release"));
        assert!(names.contains(&"arrow_whistle"));
        assert!(names.contains(&"arrow_impact"));
        assert!(names.contains(&"arrow_flyby"));
        let impact = cues.iter().find(|cue| cue.event == "arrow_impact").unwrap();
        assert!((impact.delay_s - 90.0 / 48.0).abs() < 0.01);
    }

    #[test]
    fn general_death_cues_and_witness_bark() {
        let mut d = detector();
        let mut general = unit(1, 0, Phase::Melee);
        general.is_general = true;
        let ally = unit(2, 0, Phase::Idle);
        d.detect(&[general.clone(), ally.clone()], 0.0, None);
        general.present = false;
        let (cues, barks) = d.detect(&[general, ally], 1.0, None);
        assert!(cues.iter().any(|cue| cue.event == "general_death"));
        assert_eq!((barks[0].situation, barks[0].unit), ("general_down", 2));
    }

    #[test]
    fn duplicate_cries_are_dropped_inside_the_window() {
        let mut d = detector();
        let place = Vec3::new(10.0, 0.0, 10.0);
        assert!(!d.is_duplicate("rout_cry", place, 0.0));
        assert!(d.is_duplicate("rout_cry", place + Vec3::X * 5.0, 1.0));
        assert!(!d.is_duplicate("rout_cry", place, 3.0));
        assert!(!d.is_duplicate("rout_cry", place + Vec3::X * 500.0, 3.5));
        assert!(!d.is_duplicate("sword_clash", place, 0.0));
    }

    #[test]
    fn beds_follow_engaged_soldiers_and_zoom() {
        let d = detector();
        let melee: Vec<UnitView> = (1..4).map(|id| unit(id, 0, Phase::Melee)).collect();
        let (low, distant_low) = d.beds(&melee, Vec3::ZERO, 0.0, &[]);
        let (high, distant_high) = d.beds(&melee, Vec3::ZERO, 1.0, &[]);
        let bed = |list: &[BedTarget], name: &str| {
            list.iter().find(|bed| bed.name == name).unwrap().level
        };
        assert!(bed(&low, "melee_bed_1") > bed(&high, "melee_bed_1"));
        assert_eq!(distant_low, 0.0);
        assert!(distant_high > 0.0);
        assert_eq!(bed(&low, "march_bed"), 0.0);
        let fire = d.beds(&[], Vec3::ZERO, 0.0, &[(Vec3::X, 1.0)]).0;
        assert_eq!(bed(&fire, "fire_bed"), 0.25);
    }

    #[test]
    fn fronts_pair_regiments_and_emit_shocks() {
        let mut d = detector();
        let mut a = unit(1, 0, Phase::Melee);
        let mut b = unit(2, 1, Phase::Melee);
        a.target = Some(2);
        b.target = Some(1);
        let units = [a, b];
        let fronts = d.fronts(&units, Vec3::new(0.0, 20.0, 0.0));
        assert_eq!(fronts.len(), 1);
        assert_eq!(fronts[0].key, "1:2");
        let (drives, cues, dropped) = d.drive_fronts(&fronts, 0.1, 0.1);
        assert_eq!(drives.len(), 1);
        assert!(!cues.is_empty());
        assert!(dropped.is_empty());
        let (_, _, dropped) = d.drive_fronts(&[], 0.1, 0.1);
        assert_eq!(dropped, vec!["1:2".to_string()]);
    }

    #[test]
    fn near_event_never_repeats() {
        let mut d = detector();
        for _ in 0..50 {
            assert_ne!(d.pick_near_event("sword_clash", 0.0), "sword_clash");
        }
    }

    #[test]
    fn siege_pieces_ring_the_bell_and_collapse() {
        let mut d = detector();
        let piece = |hp: f32, gate: bool| PieceView {
            index: usize::from(gate),
            hp,
            gate,
            middle: Vec3::new(0.0, 4.0, 0.0),
        };
        let first = d.siege(
            Vec3::ZERO,
            5.0,
            &[piece(100.0, true), piece(50.0, false)],
            true,
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].event, "bell_toll");
        let second = d.siege(
            Vec3::ZERO,
            6.0,
            &[piece(90.0, true), piece(40.0, false)],
            true,
        );
        let names: Vec<&str> = second.iter().map(|cue| cue.event.as_str()).collect();
        assert_eq!(names, ["ram_hit", "stone_impact"]);
        let third = d.siege(
            Vec3::ZERO,
            7.0,
            &[piece(90.0, true), piece(0.0, false)],
            false,
        );
        assert_eq!(third[0].event, "wall_collapse");
        assert!(d.siege(Vec3::ZERO, 200.0, &[], true).is_empty());
    }

    #[test]
    fn attenuation_curve() {
        assert_eq!(attenuation(5.0, 10.0, 100.0), 1.0);
        assert!((attenuation(40.0, 10.0, 100.0) - 0.25).abs() < 1e-6);
        assert!(attenuation(90.0, 10.0, 100.0) < attenuation(80.0, 10.0, 100.0));
        assert_eq!(attenuation(100.0, 10.0, 100.0), 0.0);
    }

    #[test]
    fn bark_gate_rules() {
        let mut gate = BarkGate::default();
        assert!(gate.admit("select", 1, 2.5, 1.0, 10.0, 1.0));
        // Same moment, same priority: refused while speaking.
        assert!(!gate.admit("move", 1, 2.0, 1.0, 10.5, 1.0));
        // Higher priority cuts in.
        assert!(gate.admit("charge", 4, 4.0, 1.0, 10.6, 1.0));
        // After the line, the global gap applies.
        assert!(!gate.admit("rout", 5, 5.0, 2.0, 11.8, 1.0));
        // Cooldown of the situation.
        assert!(!gate.admit("charge", 4, 4.0, 1.0, 13.0, 1.0));
        assert!(gate.admit("charge", 4, 4.0, 1.0, 15.0, 1.0));
    }

    #[test]
    fn missile_kinds() {
        assert_eq!(missile_kind("unit_bombard", true), "ball");
        assert_eq!(missile_kind("unit_trebuchet", true), "stone");
        assert_eq!(missile_kind("unit_genoese_crossbowmen", false), "bolt");
        assert_eq!(missile_kind("unit_longbowmen", false), "arrow");
    }
}
