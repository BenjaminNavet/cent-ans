//! Skinned, animated folk: the villagers, workers, carters, merchants, guards and riders of the
//! living map play the clips of the figures of the battle (`models/battle_skinned`, bone texture
//! `CAB1`), through the instanced skinned pipeline of `battle::units::skinned`.
//!
//! Port of `life_folk/folk_pool.gd` (`_figure_material`: livery 0.4 of the role, trim darkened by
//! 0.3, drab cloth colours, `FolkModels.activity_config`) and of the `folk` / `folk_carry` styles of
//! `battle_skinned.gd`. A figure is an anchor entity (`SkinFigure`, placed and scaled by the stream
//! like before); each frame the anchors of a (figure, role, activity) are written as the records of
//! one batch. The rest-pose mesh baked by `figures.rs` stays as a fallback until the batch takes
//! the figure over (library unreadable, rig without clips).
//!
//! Clips are the ones Godot picks: the `human` rig has no `scythe`, `carry` or `plough` clip yet, so
//! those activities fall back to the walk or the idle exactly like `state_config` does.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;

use super::data::LifeData;
use super::figures::RestPose;
use super::props::PropTable;
use super::stream::LifeView;
use crate::app::AppState;
use crate::battle::units::anim::{self, AnimConfig, JitterSettings, Mode, RigClips};
use crate::battle::units::library::{FigureLibrary, FigureSpec};
use crate::battle::units::looks;
use crate::battle::units::skinned::{RECORD_FLOATS, SkinParams, SkinnedBatch, push_record};
use crate::paths;

/// Livery share of the folk figures (`folk_pool.gd`).
const LIVERY_SHARE: f32 = 0.4;
/// Darkening of the trim relative to the livery.
const TRIM_DARKEN: f32 = 0.3;

/// An animated folk figure anchor: its transform places the figure, its scale is the world scale
/// (and the fade of the instance).
#[derive(Component, Debug, Clone)]
pub struct SkinFigure {
    pub role: String,
    pub kind: &'static str,
    pub variant: u32,
    pub activity: String,
    /// Fixed number in [0, 1[ of the figure (drives its variant, tint and clip phase).
    pub id: f32,
    /// The batch draws it: the rest-pose mesh is gone.
    pub live: bool,
}

/// Figure family names of the roles, as `&'static str` for the library.
pub fn static_kind(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "villager" => "villager",
        "crew" => "crew",
        "infantry" => "infantry",
        "archer" => "archer",
        "cavalry" => "cavalry",
        _ => return None,
    })
}

impl SkinFigure {
    pub fn new(
        role: &str,
        family: &str,
        variant: u32,
        activity: &str,
        h: f32,
        h2: f32,
    ) -> SkinFigure {
        SkinFigure {
            role: role.to_string(),
            kind: static_kind(family).unwrap_or("infantry"),
            variant,
            activity: activity.to_string(),
            id: ((h * 977.0 + h2 * 131.0) * 4096.0).floor() + 1.0,
            live: false,
        }
    }
}

/// Key of a batch: figure, role (livery) and activity (clip set).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BatchKey {
    pub kind: &'static str,
    pub variant: u32,
    pub role: String,
    pub activity: String,
}

struct Batch {
    entity: Entity,
    spec: Arc<FigureSpec>,
    version: u64,
    lod: usize,
    names: Vec<String>,
    speed: f32,
    count: u32,
}

#[derive(Resource, Default)]
pub struct FolkSkin {
    library: Option<FigureLibrary>,
    loaded: bool,
    batches: HashMap<BatchKey, Batch>,
    /// The library was readable: the CPU step bounce is not needed.
    pub active: bool,
    pub error: Option<String>,
}

impl FolkSkin {
    /// Entries for the dump: (role, activity, kind, variant, figures, clips, clip speed).
    pub fn rows(&self) -> Vec<(String, String, String, u32, u32, Vec<String>, f32)> {
        let mut rows: Vec<_> = self
            .batches
            .iter()
            .filter(|(_, b)| b.count > 0)
            .map(|(k, b)| {
                (
                    k.role.clone(),
                    k.activity.clone(),
                    k.kind.to_string(),
                    k.variant,
                    b.count,
                    b.names.clone(),
                    b.speed,
                )
            })
            .collect();
        rows.sort_by(|a, b| (&a.0, &a.1, &a.2, a.3).cmp(&(&b.0, &b.1, &b.2, b.3)));
        rows
    }

    pub fn batches(&self) -> usize {
        self.batches.len()
    }
}

/// Animation of an activity for a figure (`FolkModels.activity_config`): villagers play the state
/// of their style (`folk`, `folk_carry`, else the battle style), the other figures the clip lists
/// of the activity (absent clips dropped, then the fallback, then the idle).
pub fn folk_config(
    table: &PropTable,
    rig: &RigClips,
    style: &str,
    kind: &str,
    activity: &str,
    jitter: JitterSettings,
) -> AnimConfig {
    let entry = table
        .activity_anims
        .get(activity)
        .or_else(|| table.activity_anims.get("idle"));
    let Some(entry) = entry else {
        return AnimConfig {
            key: format!("fk/{kind}/{activity}"),
            names: vec!["idle".into()],
            set: vec![rig.index("idle")],
            mode: Mode::Loop,
            speed: 1.0,
            cycle: 1.5,
            release: 1.0,
            jitter: jitter.idle,
        };
    };
    let locomotion = anim::is_locomotion(&entry.state);
    let rate = if locomotion { jitter.loco } else { jitter.idle };
    if kind == "villager" {
        if let Some(sets) = table.folk_styles.get(style) {
            let mut key = entry.state.as_str();
            if !sets.contains_key(key) {
                key = "idle";
            }
            let Some(mut chosen) = sets.get(key) else {
                return folk_config(table, rig, "", "", "idle", jitter);
            };
            let set: Vec<&str> = chosen.set.iter().map(String::as_str).collect();
            let mut names = rig.present(&set);
            if !chosen.fallback.is_empty()
                && names.first().map(String::as_str) != chosen.set.first().map(String::as_str)
            {
                names = chosen.fallback.clone();
            }
            if names.is_empty() {
                if let Some(idle) = sets.get("idle") {
                    chosen = idle;
                    let set: Vec<&str> = chosen.set.iter().map(String::as_str).collect();
                    names = rig.present(&set);
                }
            }
            if names.is_empty() {
                names = vec![chosen.set[0].clone()];
            }
            return AnimConfig {
                key: format!("fk/{style}/{}", entry.state),
                set: names.iter().map(|n| rig.index(n)).collect(),
                names,
                mode: Mode::Loop,
                speed: chosen.speed,
                cycle: 1.5,
                release: 1.0,
                jitter: rate,
            };
        }
        // Another style (rioters wear the militia style): the battle states.
        let mut config = anim::state_config(style, rig, &entry.state, false, jitter);
        config.key = format!("fk/{style}/{}", entry.state);
        return config;
    }
    let preferred: Vec<&str> = entry.clips.iter().map(String::as_str).collect();
    let mut names = rig.present(&preferred);
    if names.is_empty() {
        let fallback: Vec<&str> = entry.fallback.iter().map(String::as_str).collect();
        names = rig.present(&fallback);
    }
    if names.is_empty() {
        names = vec![if kind == "cavalry" { "c_idle" } else { "idle" }.to_string()];
    }
    AnimConfig {
        key: format!("fk/{kind}/{activity}"),
        set: names.iter().map(|n| rig.index(n)).collect(),
        names,
        mode: Mode::Loop,
        speed: 1.0,
        cycle: 1.5,
        release: 1.0,
        jitter: rate,
    }
}

fn srgb_to_linear(rgb: [f32; 3]) -> [f32; 3] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue]
}

/// Parameters of a batch: livery of the role, drab cloth, rig tables and the activity's clips.
fn batch_params(
    spec: &FigureSpec,
    table: &PropTable,
    role: &str,
    config: &AnimConfig,
) -> SkinParams {
    let livery = table.livery_of(role);
    let trim = [
        livery[0] * (1.0 - TRIM_DARKEN),
        livery[1] * (1.0 - TRIM_DARKEN),
        livery[2] * (1.0 - TRIM_DARKEN),
    ];
    let mut params = SkinParams::default();
    params.set_livery(srgb_to_linear(livery), LIVERY_SHARE);
    params.set_trim(srgb_to_linear(trim));
    params.set_rig(spec.fps, spec.variants, 0.05, 6.0);
    params.set_clips(&spec.clip_table);
    params.set_ga3(spec.ga3_lum, spec.head_y, spec.albedo.is_some());
    params.set_wear(0.0, 0.0);
    if !table.drab.is_empty() {
        let drab: Vec<[f32; 3]> = table.drab.iter().map(|c| srgb_to_linear(*c)).collect();
        params.set_plain(&looks::padded(&drab), drab.len().min(looks::MAX_COLOURS));
    }
    params.set_config(config, 0.0, false);
    params
}

/// Heading (degrees) of a figure whose model looks along +Z, from the global rotation.
pub fn yaw_deg(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::Z;
    forward.x.atan2(forward.z).to_degrees()
}

/// Record of one figure: world position, yaw, scale, id and animation clock.
pub fn record_of(position: Vec3, yaw_deg: f32, scale: f32, id: f32, clock: f32) -> [f32; RECORD_FLOATS] {
    [
        position.x, position.y, position.z, yaw_deg, scale, id, clock, 0.0, -1000.0, 0.0, 0.0, 0.0,
    ]
}

/// Draws every `SkinFigure` through the batches, one per (figure, role, activity).
pub fn update_skin(
    mut commands: Commands,
    mut skin: ResMut<FolkSkin>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    mut figures: Query<(&mut SkinFigure, &GlobalTransform, &InheritedVisibility, &Children)>,
    rest: Query<(), With<RestPose>>,
    mut batches: Query<&mut SkinnedBatch>,
) {
    if !skin.loaded {
        skin.loaded = true;
        match paths::find_root().and_then(|root| {
            FigureLibrary::load(paths::asset_root(&root), &paths::data_dir(&root))
        }) {
            Ok(library) => {
                skin.library = Some(library);
                skin.active = true;
            }
            Err(e) => {
                warn!("folk skin: {e}");
                skin.error = Some(e);
            }
        }
    }
    let skin = &mut *skin;
    let Some(library) = skin.library.as_mut() else {
        return;
    };
    let clock = view.time;
    let mut groups: HashMap<BatchKey, Vec<u8>> = HashMap::new();
    let mut counts: HashMap<BatchKey, u32> = HashMap::new();
    for (mut figure, global, visible, children) in &mut figures {
        let key = BatchKey {
            kind: figure.kind,
            variant: figure.variant,
            role: figure.role.clone(),
            activity: figure.activity.clone(),
        };
        // Batch on demand.
        if !skin.batches.contains_key(&key) {
            let Some(spec) = library.spec(key.kind, key.variant, &mut meshes, &mut images, &assets)
            else {
                continue;
            };
            let style = library
                .manifest
                .figure(key.kind, spec.variant)
                .and_then(|f| f.style.clone())
                .unwrap_or_default();
            let config = folk_config(
                &data.table,
                &spec.clips,
                &style,
                key.kind,
                &key.activity,
                library.settings.jitter,
            );
            let params = batch_params(&spec, &data.table, &key.role, &config);
            let entity = commands
                .spawn((
                    Name::new(format!(
                        "life:skin:{}_{}:{}:{}",
                        key.kind, key.variant, key.role, key.activity
                    )),
                    SkinnedBatch {
                        mesh_handle: spec.lods[2].clone(),
                        albedo: spec.albedo.clone(),
                        bones: spec.bones.clone(),
                        params: Arc::new(params.data.clone()),
                        params_version: 1,
                        records: Arc::new(Vec::new()),
                        count: 0,
                        version: 0,
                    },
                    DespawnOnExit(AppState::Campaign),
                ))
                .id();
            skin.batches.insert(
                key.clone(),
                Batch {
                    entity,
                    spec,
                    version: 0,
                    lod: 2,
                    names: config.names.clone(),
                    speed: config.speed,
                    count: 0,
                },
            );
        }
        if !figure.live {
            figure.live = true;
            for child in children.iter() {
                if rest.contains(child) {
                    commands.entity(child).try_despawn();
                }
            }
        }
        if !visible.get() {
            continue;
        }
        let (scale, rotation, position) = global.to_scale_rotation_translation();
        let record = record_of(position, yaw_deg(rotation), scale.x, figure.id, clock);
        push_record(groups.entry(key.clone()).or_default(), &record);
        *counts.entry(key).or_default() += 1;
    }
    let want_fine = view.distance < data.props.folk_detail_distance;
    for (key, batch) in &mut skin.batches {
        let records = groups.remove(key).unwrap_or_default();
        let count = counts.get(key).copied().unwrap_or(0);
        let lod = if want_fine { 1 } else { 2 };
        let Ok(mut component) = batches.get_mut(batch.entity) else {
            continue;
        };
        if count == 0 && batch.count == 0 {
            continue;
        }
        batch.count = count;
        batch.version += 1;
        component.records = Arc::new(records);
        component.count = count;
        component.version = batch.version;
        if batch.lod != lod {
            batch.lod = lod;
            component.mesh_handle = batch.spec.lods[lod].clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::units::anim::ClipInfo;
    use std::collections::BTreeMap;

    fn rig(names: &[&str]) -> RigClips {
        let clips: BTreeMap<String, ClipInfo> = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    (*n).to_string(),
                    ClipInfo {
                        start: i as u32 * 50,
                        frames: 24,
                        looped: true,
                    },
                )
            })
            .collect();
        RigClips::new(24.0, &clips)
    }

    fn table() -> PropTable {
        let root = paths::find_root().expect("root");
        PropTable::load(&paths::data_dir(&root))
    }

    #[test]
    fn data_carries_the_activity_and_style_tables() {
        let t = table();
        assert_eq!(t.activity_anims.len(), t.activities.len());
        for name in t.activities.keys() {
            assert!(t.activity_anims.contains_key(name), "{name}");
        }
        assert!(t.folk_styles["folk"].contains_key("plough"));
        assert!(t.folk_styles["folk_carry"].contains_key("carry"));
    }

    #[test]
    fn villagers_play_their_state_and_fall_back_without_the_work_clips() {
        let t = table();
        let j = JitterSettings::default();
        // A kit with the work clips.
        let full = rig(&["idle", "idle_look", "walk", "scythe", "carry", "plough"]);
        let c = folk_config(&t, &full, "folk", "villager", "scythe", j);
        assert_eq!(c.names, ["scythe"]);
        let c = folk_config(&t, &full, "folk", "villager", "plough", j);
        assert_eq!(c.names, ["plough"]);
        assert!((c.speed - 0.7).abs() < 1e-6, "slower plough clip");
        // The human rig of this repository has none of them: idle / walk.
        let coarse = rig(&["idle", "idle_look", "walk", "run", "flee", "flee_m"]);
        assert_eq!(
            folk_config(&t, &coarse, "folk", "villager", "scythe", j).names,
            ["idle"]
        );
        assert_eq!(
            folk_config(&t, &coarse, "folk", "villager", "plough", j).names,
            ["walk"]
        );
        let walk = folk_config(&t, &coarse, "folk", "villager", "walk", j);
        assert_eq!(walk.names, ["walk"]);
        assert_eq!(walk.jitter, j.loco, "walking uses the locomotion jitter");
        let idle = folk_config(&t, &coarse, "folk", "villager", "idle", j);
        assert_eq!(idle.names, ["idle", "idle", "idle_look"]);
        assert_eq!(idle.jitter, j.idle);
        // Porters carry the bag: the marching state is `carry`, else the walk.
        let carry = folk_config(&t, &coarse, "folk_carry", "villager", "walk", j);
        assert_eq!(carry.names, ["walk"]);
    }

    #[test]
    fn other_figures_take_the_clip_lists_of_the_activity() {
        let t = table();
        let j = JitterSettings::default();
        let human = rig(&["idle", "walk", "pike_walk", "guard"]);
        assert_eq!(
            folk_config(&t, &human, "crew", "infantry", "guard_walk", j).names,
            ["pike_walk", "walk"]
        );
        assert_eq!(
            folk_config(&t, &human, "militia", "infantry", "drill", j).names,
            ["guard", "idle"]
        );
        let horse = rig(&["c_idle", "c_walk"]);
        assert_eq!(
            folk_config(&t, &horse, "lance", "cavalry", "ride", j).names,
            ["c_walk"]
        );
        let none = rig(&["slash"]);
        assert_eq!(
            folk_config(&t, &none, "crew", "infantry", "walk", j).names,
            ["idle"],
            "no clip of the lists: the idle name"
        );
        // Unknown activity falls to the idle entry.
        assert_eq!(
            folk_config(&t, &human, "crew", "infantry", "no_such", j).names,
            ["idle", "guard"]
        );
    }

    #[test]
    fn rioters_use_the_battle_militia_states() {
        let t = table();
        let j = JitterSettings::default();
        let human = rig(&["pike_idle", "pike_look", "pike_walk", "idle", "walk"]);
        let c = folk_config(&t, &human, "militia", "villager", "walk", j);
        assert_eq!(c.names, ["pike_walk"]);
    }

    #[test]
    fn yaw_follows_the_heading() {
        // Facing +X: forward (1,0,0) gives 90 degrees.
        assert!((yaw_deg(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)) - 90.0).abs() < 1e-3);
        assert!(yaw_deg(Quat::IDENTITY).abs() < 1e-4);
        let r = record_of(Vec3::new(1.0, 2.0, 3.0), 45.0, 0.5, 7.0, 1.5);
        assert_eq!(&r[..7], &[1.0, 2.0, 3.0, 45.0, 0.5, 7.0, 1.5]);
        assert_eq!(r[8], -1000.0);
    }

    #[test]
    fn the_real_rig_gives_walk_and_idle_clips_to_villagers() {
        let root = paths::find_root().expect("root");
        let Ok(lib) = FigureLibrary::load(paths::asset_root(&root), &paths::data_dir(&root)) else {
            return;
        };
        let t = table();
        let (_, entry) = lib.manifest.figure_or_default("villager", 0).expect("villager_0");
        let rig = lib.manifest.rig(&entry.rig).expect("rig").clips();
        let style = entry.style.clone().unwrap_or_default();
        let walk = folk_config(&t, &rig, &style, "villager", "walk", lib.settings.jitter);
        assert_eq!(walk.names, ["walk"]);
        let idle = folk_config(&t, &rig, &style, "villager", "idle", lib.settings.jitter);
        assert!(idle.names.iter().all(|n| rig.get(n).is_some()), "{:?}", idle.names);
        // Every activity resolves to clips that exist in the rig, for every folk figure.
        for (kind, variant) in [("villager", 0), ("villager", 1), ("villager", 2), ("villager", 3), ("crew", 0), ("infantry", 2)] {
            let Some((_, f)) = lib.manifest.figure_or_default(kind, variant) else { continue };
            let rig = lib.manifest.rig(&f.rig).expect("rig").clips();
            let style = f.style.clone().unwrap_or_default();
            for activity in t.activity_anims.keys() {
                let c = folk_config(&t, &rig, &style, kind, activity, lib.settings.jitter);
                assert!(!c.set.is_empty());
                assert!(
                    c.names.iter().all(|n| rig.get(n).is_some()),
                    "{kind}_{variant} {activity}: {:?}",
                    c.names
                );
            }
        }
    }
}
