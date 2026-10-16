//! Servants of the siege engines (port of game/scripts/battle/siege_crew_fx.gd).
//!
//! The skinned `crew_0` (bare hands) and `crew_1` (with the rammer) figures of the battle kit
//! work the engines: winding the winch (`crank`), loading (`load` / `load_heavy`), swabbing,
//! pushing a ram or a tower. `engines.rs` decides each servant's role gesture from the reload the
//! core exposes and writes it to [`CrewFrame`]; [`draw_crew`] turns the entries into skinned
//! batches (one draw per figure and side, shader mode `Pick`: clip chosen per record, played
//! from the instant the servant began the gesture, looping). Rendering only, no rule.
//!
//! A gesture change fades from the previous clip over `role_blend_s` (`crew_fade.rs`, decoded in
//! `units/skinned.wesl`, mode 3).
//!
//! Not ported: the ammunition carriers (`haul`) and their piles.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bevy::prelude::*;
use sim_battle::SideId;

use super::animation::EngineSettings;
use super::crew_fade::{Gestures, role_blend_s};
use super::{SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::units::anim::{AnimConfig, Mode};
use crate::battle::units::library::{FigureLibrary, FigureSpec};
use crate::battle::units::regiments::trim_for;
use crate::battle::units::skinned::{RECORD_FLOATS, SkinParams, SkinnedBatch, push_record};

/// Settings of the engines (loaded once, shared by `engines` and `crew`).
#[derive(Resource, Clone)]
pub struct EngineSettingsRes(pub Arc<EngineSettings>);

/// One servant to draw this frame.
#[derive(Debug, Clone)]
pub struct CrewEntry {
    /// Stable from frame to frame (`e<unit>/<index>`).
    pub key: String,
    pub pos: Vec3,
    /// Heading about +Y (rad), the model's front is +Z.
    pub yaw: f32,
    /// Role gesture (`winch`, `loader`, `swab`, `pusher`, `pusher_still`, `idle`).
    pub act: String,
    /// Engine model (`trebuchet`, `ram`...), for the per-engine clip overrides.
    pub engine: String,
    pub index: usize,
    pub figure: u32,
    pub side: SideId,
}

/// Servants of the frame (filled by `engines::animate_engines`).
#[derive(Resource, Default)]
pub struct CrewFrame {
    pub entries: Vec<CrewEntry>,
}

struct CrewBatch {
    entity: Entity,
    spec: Arc<FigureSpec>,
    params: SkinParams,
    names: Vec<String>,
    params_version: u64,
    version: u64,
    lod: usize,
}

#[derive(Resource, Default)]
pub struct CrewState {
    batches: HashMap<(u32, usize), CrewBatch>,
    /// Gesture of each servant, with the previous one while it fades out (`crew_fade`).
    gestures: Gestures,
    /// `role_blend_s` of the animation data (0 = a gesture switches abruptly).
    blend_s: Option<f32>,
    slots: HashMap<String, usize>,
}

pub struct CrewPlugin;

impl Plugin for CrewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrewFrame>()
            .init_resource::<CrewState>()
            .add_systems(
                Update,
                draw_crew
                    .after(super::engines::animate_engines)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .run_if(resource_exists::<FigureLibrary>),
            )
            .add_systems(OnExit(AppState::Battle), |mut state: ResMut<CrewState>| {
                *state = CrewState::default();
            });
    }
}

fn srgb_to_linear(rgb: [f32; 3]) -> [f32; 3] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue]
}

fn side_index(side: SideId) -> usize {
    usize::from(side == SideId::Defender)
}

/// Parameters of a crew batch (livery of the side, rig tables).
fn build_params(spec: &FigureSpec, side_color: [f32; 3]) -> SkinParams {
    let mut params = SkinParams::default();
    params.set_livery(srgb_to_linear(side_color), 0.6);
    params.set_trim(srgb_to_linear(trim_for(side_color)));
    params.set_rig(spec.fps, spec.variants, 0.05, 6.0);
    params.set_clips(&spec.clip_table);
    params.set_ga3(spec.ga3_lum, spec.head_y, spec.albedo.is_some());
    params.set_wear(0.45, 0.85);
    params
}

/// Stable small hash of a key (the servant's variant: head, cap).
fn key_hash(key: &str) -> f32 {
    let mut h: u32 = 2166136261;
    for b in key.bytes() {
        h = (h ^ u32::from(b)).wrapping_mul(16777619);
    }
    (h % 4096) as f32
}

#[allow(clippy::too_many_arguments)]
pub fn draw_crew(
    mut commands: Commands,
    mut state: ResMut<CrewState>,
    crew: Res<CrewFrame>,
    siege: Res<SiegeFrame>,
    settings: Option<Res<EngineSettingsRes>>,
    mut library: ResMut<FigureLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    session: Res<BattleSession>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut batches: Query<&mut SkinnedBatch>,
    mut stats: ResMut<SiegeStats>,
) {
    let Some(settings) = settings else { return };
    let cfg = &settings.0.crew;
    let now = siege.clock;
    let camera_position = camera.iter().next().map(GlobalTransform::translation);
    // Group the visible servants by (figure, side).
    let mut groups: HashMap<(u32, usize), Vec<(&CrewEntry, f32)>> = HashMap::new();
    for entry in &crew.entries {
        let distance = camera_position.map_or(0.0, |c| c.distance(entry.pos));
        if distance > cfg.hide_beyond_m {
            continue;
        }
        groups
            .entry((entry.figure, side_index(entry.side)))
            .or_default()
            .push((entry, distance));
    }
    let mut shown = 0usize;
    let mut by_clip: BTreeMap<String, usize> = BTreeMap::new();
    let mut fades = 0usize;
    for (key, entries) in &groups {
        if !state.batches.contains_key(key) {
            let Some(spec) = library.spec("crew", key.0, &mut meshes, &mut images, &assets) else {
                continue;
            };
            let wanted = cfg.all_clips();
            let wanted_refs: Vec<&str> = wanted.iter().map(String::as_str).collect();
            let names = spec.clips.present(&wanted_refs);
            if names.is_empty() {
                continue;
            }
            let mut params = build_params(&spec, session.colors[key.1]);
            let config = AnimConfig {
                key: format!("{}/crew", spec.key),
                set: names.iter().map(|n| spec.clips.index(n)).collect(),
                names: names.clone(),
                mode: Mode::Pick,
                speed: 1.0,
                cycle: 1.0,
                release: 1.0,
                jitter: 0.0,
            };
            params.set_config(&config, 0.0, false);
            let entity = commands
                .spawn((
                    Name::new(format!("Crew{}_{}", key.0, key.1)),
                    SkinnedBatch {
                        mesh_handle: spec.lods[0].clone(),
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
            state.batches.insert(
                *key,
                CrewBatch {
                    entity,
                    spec,
                    params,
                    names,
                    params_version: 1,
                    version: 0,
                    lod: 0,
                },
            );
        }
        let Some(names) = state.batches.get(key).map(|b| b.names.clone()) else {
            continue;
        };
        let nearest = entries
            .iter()
            .map(|(_, d)| *d)
            .fold(f32::INFINITY, f32::min);
        let lod = if nearest < cfg.detail_m {
            0
        } else if nearest < cfg.medium_m {
            1
        } else {
            2
        };
        let rig_has = |c: &str| names.iter().any(|n| n == c);
        let mut records: Vec<u8> = Vec::with_capacity(entries.len() * RECORD_FLOATS * 4);
        let mut gestures = Vec::new();
        for (entry, _) in entries {
            let clip = cfg.clip_of(&entry.act, &entry.engine, entry.index, rig_has);
            gestures.push((entry.key.clone(), clip));
        }
        let mut count = 0u32;
        for ((entry, _), (_, clip)) in entries.iter().zip(&gestures) {
            let Some(pick) = names.iter().position(|n| n == clip) else {
                continue;
            };
            // A new gesture restarts its clip (a slight offset between servants) and the
            // previous one keeps playing for the fade (NT10).
            let slot = {
                let next = state.slots.len();
                *state.slots.entry(entry.key.clone()).or_insert(next)
            };
            let blend_s = *state.blend_s.get_or_insert_with(|| {
                crate::paths::find_root()
                    .map(|root| role_blend_s(&crate::paths::data_dir(&root)))
                    .unwrap_or(0.0)
            });
            let stagger_s = 0.13 * (slot % 5) as f32;
            let gesture = state
                .gestures
                .update(&entry.key, clip, now, stagger_s, blend_s)
                .clone();
            let fade = gesture
                .fade(now, blend_s, |c| names.iter().position(|n| n == c))
                .unwrap_or(-1000.0);
            let since = gesture.since;
            fades += usize::from(fade > 0.0);
            let record: [f32; RECORD_FLOATS] = [
                entry.pos.x,
                entry.pos.y,
                entry.pos.z,
                entry.yaw.to_degrees(),
                1.0,
                key_hash(&entry.key),
                since,
                0.0,
                fade,
                0.0,
                pick as f32,
                1.0e6,
            ];
            push_record(&mut records, &record);
            count += 1;
            *by_clip.entry(clip.clone()).or_default() += 1;
        }
        shown += count as usize;
        let blend_s = state.blend_s.unwrap_or(0.0);
        let Some(batch) = state.batches.get_mut(key) else {
            continue;
        };
        batch.params.set_now(now);
        batch.params.set_role_blend(blend_s);
        batch.params_version += 1;
        batch.version += 1;
        let mesh = batch.spec.lods[lod].clone();
        if let Ok(mut component) = batches.get_mut(batch.entity) {
            component.records = Arc::new(records);
            component.count = count;
            component.version = batch.version;
            component.params = Arc::new(batch.params.data.clone());
            component.params_version = batch.params_version;
            if batch.lod != lod {
                batch.lod = lod;
                component.mesh_handle = mesh;
            }
        }
    }
    // Batches with nobody to draw this frame.
    for (key, batch) in &mut state.batches {
        if groups.contains_key(key) {
            continue;
        }
        if let Ok(mut component) = batches.get_mut(batch.entity)
            && component.count != 0
        {
            batch.version += 1;
            component.records = Arc::new(Vec::new());
            component.count = 0;
            component.version = batch.version;
        }
    }
    stats.crew_listed = crew.entries.len();
    stats.crew_shown = shown;
    stats.crew_batches = state.batches.len();
    stats.crew_by_clip = by_clip;
    stats.crew_fading = fades;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_hash_is_stable_and_small() {
        assert_eq!(key_hash("e1/0/0"), key_hash("e1/0/0"));
        assert_ne!(key_hash("e1/0/0"), key_hash("e1/0/1"));
        assert!(key_hash("anything") < 4096.0);
    }

    #[test]
    fn the_crew_figures_exist_in_the_kit_with_the_gestures_the_roles_need() {
        let root = crate::paths::find_root().expect("root");
        let manifest =
            crate::battle::units::figures::Manifest::load(&crate::paths::asset_root(&root))
                .expect("manifest");
        let settings = EngineSettings::load(&crate::paths::data_dir(&root)).expect("settings");
        for variant in [0, 1] {
            let (_, figure) = manifest
                .figure_or_default("crew", variant)
                .expect("crew figure");
            let rig = manifest.rig(&figure.rig).expect("rig");
            let clips = rig.clips();
            for clip in settings.crew.all_clips() {
                assert!(
                    clips.get(&clip).is_some(),
                    "{clip} missing in {}",
                    figure.rig
                );
            }
            // The pick set fits the shader's slots.
            let wanted = settings.crew.all_clips();
            let refs: Vec<&str> = wanted.iter().map(String::as_str).collect();
            assert!(clips.present(&refs).len() <= crate::battle::units::anim::MAX_SET);
        }
    }
}
