//! Fallen soldiers: the dead lie where they fell (death clip played once then held), a share of
//! the casualties are wounded who crawl, sit or kneel for a few seconds then stay down. Rendering
//! only: how many men a regiment lost is decided by the simulation, this module spots the drop of
//! the drawn head count between two frames and lays that many bodies on the former slots.
//!
//! Port of `_spawn_corpses` / `_spawn_wounded` of game/scripts/battle/battle_soldiers.gd with the
//! shader mode `Pick` (clip chosen per record, started at the fall time). Not ported: blood
//! stains and sprays, severed limbs (battle_gore.gd), the thrown impulse of heavy hits, the
//! horse that flees from a fallen rider.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use super::anim::{AnimConfig, Mode};
use super::arms::{ArmDrop, DroppedArms};
use super::library::{FigureLibrary, FigureSpec};
use super::regiments::Lod;
use super::settings::FxSettings;
use super::skinned::{RECORD_FLOATS, SkinParams, SkinnedBatch, push_record};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// Clips of a fallen foot soldier, in the order of the clip pick index.
pub const DEATHS_FOOT: [&str; 4] = ["death", "death_m", "death_back", "death_knees"];
pub const DEATHS_CAVALRY: [&str; 3] = ["c_death", "c_death_m", "c_fall"];
pub const WOUNDED_FOOT: [&str; 3] = ["crawl", "wounded_sit", "wounded_kneel"];
/// Seconds after the latest fall during which the batch clock keeps running.
const ANIMATED_WINDOW_S: f32 = 12.0;
/// Hold time of a clip that never ends by itself (wounded loops): no limit for the dead.
const NO_LIMIT: f32 = 1.0e6;

/// splitmix-style hash giving four values in `[0, 1)` (`BattleDroppedArms.hash4`).
pub fn hash4(n: u64) -> [f32; 4] {
    let mut out = [0.0; 4];
    let mut state = n
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0x1234_5678);
    for slot in &mut out {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        *slot = (z >> 40) as f32 / (1u64 << 24) as f32;
    }
    out
}

/// A drop of the drawn head count of a regiment.
pub struct Loss<'a> {
    pub unit: u32,
    pub spec: &'a Arc<FigureSpec>,
    pub params: &'a SkinParams,
    /// Poses of the figures before the loss.
    pub previous: &'a [[f32; 4]],
    pub count: usize,
    pub cause: &'static str,
    /// Where the killers stand (xz), when known.
    pub killer: Option<Vec2>,
    /// Men the regiment still has (stable draw rank for replays).
    pub left: u32,
    pub fx: &'a FxSettings,
}

struct CorpseBatch {
    entity: Entity,
    spec: Arc<FigureSpec>,
    params: SkinParams,
    params_version: u64,
    /// Clip names of the set, in pick order.
    names: Vec<String>,
    records: Vec<u8>,
    count: usize,
    version: u64,
    centroid: Vec2,
    lod: Lod,
}

#[derive(Resource, Default)]
pub struct Corpses {
    batches: HashMap<u32, CorpseBatch>,
    /// Clock of the corpse batches (s of battle time).
    pub clock: f32,
    last_fall: f32,
    pub dead: usize,
    pub wounded: usize,
    pub by_cause: std::collections::BTreeMap<&'static str, usize>,
}

impl Corpses {
    pub fn total(&self) -> usize {
        self.dead + self.wounded
    }

    /// Lays `loss.count` bodies on slots of `loss.previous`.
    pub fn spawn(
        &mut self,
        commands: &mut Commands,
        loss: Loss,
        arms: &mut DroppedArms,
        report_drop: bool,
    ) {
        let limits = &loss.fx.gore.corpses;
        let spec = loss.spec;
        let batch = self.batches.entry(loss.unit).or_insert_with(|| {
            let wanted: &[&str] = if spec.mounted {
                &DEATHS_CAVALRY
            } else {
                &DEATHS_FOOT
            };
            let mut names: Vec<String> = spec.clips.present(wanted);
            if !spec.mounted {
                names.extend(spec.clips.present(&WOUNDED_FOOT));
            }
            let config = AnimConfig {
                key: format!("{}/fallen", spec.key),
                set: names.iter().map(|n| spec.clips.index(n)).collect(),
                names: names.clone(),
                mode: Mode::Pick,
                speed: 1.0,
                cycle: 1.0,
                release: 1.0,
                jitter: 0.0,
            };
            let mut params = loss.params.clone();
            params.set_config(&config, 0.0, false);
            params.set_hit_set(&[], 0.6, 0.0);
            let entity = commands
                .spawn((
                    Name::new(format!("Fallen{}", loss.unit)),
                    SkinnedBatch {
                        mesh_handle: spec.lods[1].clone(),
                        albedo: spec.albedo.clone(),
                        bones: spec.bones.clone(),
                        params: Arc::new(params.data.clone()),
                        params_version: 1,
                        records: Arc::new(Vec::new()),
                        count: 0,
                        version: 0,
                    },
                    DespawnOnExit(AppState::Battle),
                ))
                .id();
            CorpseBatch {
                entity,
                spec: spec.clone(),
                params,
                params_version: 1,
                names,
                records: Vec::new(),
                count: 0,
                version: 0,
                centroid: Vec2::ZERO,
                lod: Lod::Medium,
            }
        });
        if batch.names.is_empty() {
            return;
        }
        let death = loss.fx.death(loss.cause);
        let clips: &[String] =
            death.map_or(&[], |d| if spec.mounted { &d.mounted } else { &d.foot });
        let impulse = death.map_or([0.0, 0.5], |d| {
            [
                d.impulse.first().copied().unwrap_or(0.0),
                d.impulse.get(1).copied().unwrap_or(0.5),
            ]
        });
        let wounded_share = if spec.mounted {
            0.0
        } else {
            loss.fx
                .gore
                .wounded
                .share
                .get(loss.cause)
                .copied()
                .unwrap_or(0.0)
        };
        let wounded_names: Vec<usize> = WOUNDED_FOOT
            .iter()
            .filter_map(|w| batch.names.iter().position(|n| n == w))
            .collect();
        let deaths_in_set = batch.names.len() - wounded_names.len();
        let prev_n = loss.previous.len();
        if prev_n == 0 {
            return;
        }
        let now = self.clock;
        for i in 0..loss.count.min(limits.spawn_per_update) {
            if self.dead + self.wounded >= limits.max_total || batch.count >= limits.max_per_layer {
                break;
            }
            let ordinal = u64::from(loss.left) * 512 + i as u64;
            let seed = u64::from(loss.unit) * 1_000_003 + ordinal;
            let h = hash4(seed);
            let pick_slot = |r: f32| ((r.powf(1.6) * prev_n as f32) as usize).min(prev_n - 1);
            let wounded = wounded_share > 0.0
                && !wounded_names.is_empty()
                && self.wounded < loss.fx.gore.wounded.max_total
                && h[0] < wounded_share;
            let slot = pick_slot(hash4(seed ^ 0xABCD)[0]);
            let pose = loss.previous[slot];
            let pos = Vec2::new(pose[0], pose[2]);
            let (index, yaw, max_t) = if wounded {
                let hw = hash4(seed * 13 + 5);
                let weights: Vec<f32> = wounded_names
                    .iter()
                    .map(|n| {
                        loss.fx
                            .gore
                            .wounded
                            .clips
                            .get(&batch.names[*n])
                            .copied()
                            .unwrap_or(1.0)
                    })
                    .collect();
                let total: f32 = weights.iter().sum();
                let mut r = hw[1] * total;
                let mut chosen = wounded_names.len() - 1;
                for (k, w) in weights.iter().enumerate() {
                    r -= w;
                    if r <= 0.0 {
                        chosen = k;
                        break;
                    }
                }
                let index = wounded_names[chosen];
                let mut yaw = pose[3];
                if batch.names[index] == "crawl" {
                    // The model crawls towards its front: back to the killer, else turned about.
                    let away = loss
                        .killer
                        .map(|k| pos - k)
                        .filter(|d| d.length() > 0.5)
                        .map_or(pose[3] + std::f32::consts::PI, |d| d.x.atan2(d.y));
                    yaw = away + (hw[2] - 0.5) * 0.9;
                }
                (index, yaw, loss.fx.gore.wounded.animated_seconds)
            } else {
                let mut index = clips
                    .get((h[1] * clips.len().max(1) as f32) as usize % clips.len().max(1))
                    .and_then(|c| batch.names.iter().position(|n| n == c))
                    .unwrap_or(usize::MAX);
                if index == usize::MAX {
                    index = ((h[2] * deaths_in_set as f32) as usize).min(deaths_in_set - 1);
                }
                let speed = impulse[0] + (impulse[1] - impulse[0]) * h[3];
                // Thrown by the blow: the figure faces the killer, the fall goes backwards.
                let yaw = match loss.killer {
                    Some(k) if speed > 0.3 && (pos - k).length() > 0.5 => {
                        let d = k - pos;
                        d.x.atan2(d.y)
                    }
                    _ => pose[3],
                };
                (index, yaw, NO_LIMIT)
            };
            let record: [f32; RECORD_FLOATS] = [
                pose[0],
                pose[1],
                pose[2],
                yaw.to_degrees(),
                1.0,
                (slot as u64 + u64::from(loss.unit) * 977 + batch.count as u64 * 31) as f32
                    % 4096.0,
                now,
                0.0,
                -1000.0,
                0.0,
                index as f32,
                max_t,
            ];
            push_record(&mut batch.records, &record);
            batch.count += 1;
            let n = batch.count as f32;
            batch.centroid += (pos - batch.centroid) / n;
            if wounded {
                self.wounded += 1;
                if report_drop {
                    arms.push(ArmDrop {
                        style: spec.style.clone(),
                        at: Vec3::new(pose[0], pose[1], pose[2]),
                        facing: pose[3],
                        seed: seed as u32,
                    });
                }
            } else {
                self.dead += 1;
            }
            *self.by_cause.entry(loss.cause).or_default() += 1;
            self.last_fall = now;
        }
        batch.version += 1;
    }
}

/// Moves the corpse clock, keeps the batches' level of detail and uploads what changed.
pub fn update_corpses(
    time: Res<Time>,
    session: Res<BattleSession>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut corpses: ResMut<Corpses>,
    mut batches: Query<&mut SkinnedBatch>,
    library: Option<Res<FigureLibrary>>,
) {
    if !session.paused {
        corpses.clock += time.delta_secs() * session.speed as f32;
    }
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::new(0.0, 100.0, 0.0), |t| t.translation());
    let far = library.map_or(380.0, |l| l.fx.gore.corpses.far_m);
    let animating = corpses.clock - corpses.last_fall < ANIMATED_WINDOW_S;
    let clock = corpses.clock;
    for batch in corpses.batches.values_mut() {
        let Ok(mut skinned) = batches.get_mut(batch.entity) else {
            continue;
        };
        let distance = camera_position.distance(Vec3::new(
            batch.centroid.x,
            camera_position.y.min(20.0),
            batch.centroid.y,
        ));
        let lod = if distance < 60.0 {
            Lod::Medium
        } else {
            Lod::Far
        };
        let wanted = &batch.spec.lods[lod.index().min(2)];
        if skinned.mesh_handle != *wanted {
            skinned.mesh_handle = wanted.clone();
        }
        batch.lod = lod;
        let shown = if distance > far + 150.0 {
            0
        } else {
            batch.count
        };
        if skinned.count != shown as u32 || skinned.version != batch.version {
            skinned.count = shown as u32;
            skinned.records = Arc::new(batch.records.clone());
            skinned.version = batch.version;
        }
        if animating || batch.params_version == 1 {
            batch.params.set_now(clock);
            batch.params_version += 1;
            skinned.params = Arc::new(batch.params.data.clone());
            skinned.params_version = batch.params_version;
        }
    }
}

impl Corpses {
    /// `(regiment, soldiers lying)` for the dump.
    pub fn summaries(&self) -> Vec<(u32, usize)> {
        let mut all: Vec<_> = self.batches.iter().map(|(id, b)| (*id, b.count)).collect();
        all.sort_unstable();
        all
    }
}

fn clear_corpses(mut corpses: ResMut<Corpses>) {
    *corpses = Corpses::default();
}

fn dump_fallen(world: &World) -> (String, serde_json::Value) {
    let Some(c) = world.get_resource::<Corpses>() else {
        return ("battle_fallen".into(), serde_json::Value::Null);
    };
    let arms = world.get_resource::<DroppedArms>();
    (
        "battle_fallen".into(),
        serde_json::json!({
            "dead": c.dead,
            "wounded": c.wounded,
            "by_cause": c.by_cause,
            "regiments_with_fallen": c.summaries().len(),
            "clock": c.clock,
            "dropped_arms": arms.map_or(0, |a| a.dropped),
            "arms_on_ground": arms.map_or(0, DroppedArms::shown),
        }),
    )
}

pub struct CorpsesPlugin;

impl Plugin for CorpsesPlugin {
    fn build(&self, app: &mut App) {
        use crate::agent::dump::DumpAppExt;
        app.init_resource::<Corpses>()
            .register_dump(dump_fallen)
            .add_systems(OnExit(AppState::Battle), clear_corpses)
            .add_systems(
                Update,
                update_corpses
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(super::regiments::update_regiments),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_in_range() {
        let a = hash4(42);
        assert_eq!(a, hash4(42));
        assert_ne!(a, hash4(43));
        assert!(a.iter().all(|v| (0.0..1.0).contains(v)));
    }
}
