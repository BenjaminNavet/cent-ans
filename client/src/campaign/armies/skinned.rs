//! Animated skinned figures of the armies of the campaign map. Port of the troop part of
//! `game/scripts/map/army_figures.gd`: the lord (or leader and bearer) and the escort are drawn by
//! the instanced skinned pipeline of the battle (`battle::units::skinned`, bone texture baked
//! animations), one batch per army and figure family, in the faction livery. They walk (clip
//! `marching`, cadence set by the real ground speed of the troop) while the army slides to a new
//! point, and rest (`idle`) otherwise. Hover and selection brighten the figures. Camps and ships
//! stay glTF scenes (`figures.rs`). Rendering only: the layout comes from `figures.rs`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_campaign::ArmyId;

use super::figures::{RosterEntry, SKINNED_SCALE, SkinnedSlot, skinned_layout};
use super::markers::ArmyMarker;
use super::sea::Glides;
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::app::AppState;
use crate::battle::units::anim::{self, Mode};
use crate::battle::units::gaits::{self, GaitMemory};
use crate::battle::units::library::{FigureLibrary, FigureSpec};
use crate::battle::units::looks::{self, Look};
use crate::battle::units::regiments::trim_for;
use crate::battle::units::skinned::{RECORD_FLOATS, SkinParams, SkinnedBatch, push_record};
use crate::campaign::camera::zoom::ZoomView;

/// Height of the figures with the enlarged lord, in marker-local units (plate anchor, picking).
pub const SKINNED_HEIGHT: f32 = 8.0;
/// Camera distance (world units) below which the full mesh is drawn, and below which the medium
/// one (`ArmyFigures.LOD0_DISTANCE`, `LOD1_DISTANCE`).
pub const LOD0_DISTANCE: f32 = 90.0;
pub const LOD1_DISTANCE: f32 = 320.0;
/// Distance beyond which the figures fade into the banner colours (`far_start`).
pub const FAR_START: f32 = 260.0;
const FAR_SPAN: f32 = 180.0;
/// Strategic (parchment) weight above which the figures are not drawn.
pub const HIDE_STRATEGIC: f32 = 0.5;
/// Brightening of the figures (`army_marker.gd` HIGHLIGHT_*, rescaled to the battle shader).
pub const HIGHLIGHT_IDLE: f32 = 0.0;
pub const HIGHLIGHT_SELECTED: f32 = 1.0;
pub const HIGHLIGHT_HOVERED: f32 = 2.0;
const BLEND_TIME_S: f32 = 0.35;
const HYSTERESIS: f32 = 0.08;

/// Cadence settings of `data/fx/campaign_army_walk.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkSettings {
    pub enabled: bool,
    pub min_factor: f32,
    pub max_factor: f32,
    pub smoothing_per_s: f32,
    pub default_nominal_mps: f32,
}

impl Default for WalkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            min_factor: 0.35,
            max_factor: 1.8,
            smoothing_per_s: 5.0,
            default_nominal_mps: 1.35,
        }
    }
}

impl WalkSettings {
    pub fn load(data_dir: &std::path::Path) -> Self {
        let read = || -> Option<serde_json::Value> {
            let text = std::fs::read_to_string(data_dir.join("fx/campaign_army_walk.json")).ok()?;
            serde_json::from_str(&text).ok()
        };
        let mut settings = Self::default();
        if let Some(json) = read() {
            settings.enabled = json["enabled"].as_bool().unwrap_or(true);
            let cadence = &json["cadence"];
            let f = |key: &str, old: f32| cadence[key].as_f64().map_or(old, |v| v as f32);
            settings.min_factor = f("min_factor", settings.min_factor);
            settings.max_factor = f("max_factor", settings.max_factor);
            settings.smoothing_per_s = f("smoothing_per_s", settings.smoothing_per_s);
            settings.default_nominal_mps = f("default_nominal_mps", settings.default_nominal_mps);
        }
        settings
    }

    /// Clock factor of a group walking at `ground_speed` (world units per second) whose clip
    /// advances at `nominal_mps` (m/s at speed 1) at the world scale `world_scale`
    /// (`ArmyFigures.cadence_factor`).
    pub fn cadence_factor(&self, ground_speed: f32, nominal_mps: f32, world_scale: f32) -> f32 {
        let nominal = (nominal_mps * world_scale).max(0.001);
        (ground_speed / nominal).clamp(self.min_factor, self.max_factor)
    }
}

/// Enlargement of the lord (`map.army_figure_scale` of `data/ui/campaign_map.json`, at least 1).
pub fn load_lord_scale(data_dir: &std::path::Path) -> f32 {
    std::fs::read_to_string(data_dir.join("ui/campaign_map.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json["map"]["army_figure_scale"].as_f64())
        .map_or(1.0, |v| (v as f32).max(1.0))
}

/// Level of detail for a camera distance, with a margin around each edge when `current` is known.
pub fn lod_for(distance: f32, current: Option<usize>) -> usize {
    const EDGES: [f32; 2] = [LOD0_DISTANCE, LOD1_DISTANCE];
    let plain = EDGES.iter().position(|e| distance < *e).unwrap_or(2);
    let Some(at) = current else {
        return plain;
    };
    match plain.cmp(&at) {
        std::cmp::Ordering::Equal => at,
        std::cmp::Ordering::Greater if distance < EDGES[at] * (1.0 + HYSTERESIS) => at,
        std::cmp::Ordering::Less if distance > EDGES[at - 1] * (1.0 - HYSTERESIS) => at,
        _ => plain,
    }
}

/// Highlight of an army's figures.
pub fn highlight_for(selected: bool, hovered: bool) -> f32 {
    if hovered {
        HIGHLIGHT_HOVERED
    } else if selected {
        HIGHLIGHT_SELECTED
    } else {
        HIGHLIGHT_IDLE
    }
}

type GroupKey = (&'static str, u32);

struct Group {
    entity: Entity,
    /// Unit type whose outfit look the batch wears (the strongest regiment of the family).
    look_type: Option<String>,
    /// Gait memory of a mounted group (walk / trot hysteresis).
    gait: GaitMemory,
    spec: Arc<FigureSpec>,
    params: SkinParams,
    params_version: u64,
    config_key: String,
    nominal_mps: f32,
    clock: f32,
    factor: f32,
    version: u64,
    last_records: Arc<Vec<u8>>,
    count: u32,
}

struct Rig {
    roster: Vec<RosterEntry>,
    slots: Vec<SkinnedSlot>,
    groups: HashMap<GroupKey, Group>,
    lod: Option<usize>,
}

/// Counts of the last update (dump).
#[derive(Debug, Default, Clone)]
pub struct SkinnedReport {
    pub armies: usize,
    pub batches: usize,
    pub figures: usize,
    pub walking_groups: usize,
    pub trotting_groups: usize,
    pub looked_groups: usize,
    pub idle_groups: usize,
    pub highlighted: usize,
    pub clips: Vec<String>,
    pub max_cadence: f32,
    pub lods: [usize; 3],
    pub record_uploads: usize,
}

/// Skinned figures of the armies (library, batches per army).
#[derive(Resource, Default)]
pub struct SkinnedArmies {
    /// The skinned figures are available: the glTF troop is replaced.
    pub enabled: bool,
    pub lord_scale: f32,
    pub walk: WalkSettings,
    pub error: Option<String>,
    library: Option<FigureLibrary>,
    rigs: HashMap<ArmyId, Rig>,
    pub report: SkinnedReport,
}

impl SkinnedArmies {
    /// Clock factor of the figure family `key` of an army (1 when absent): the pace of the
    /// bearer's steps (`ArmyFigures.group_factor`).
    pub fn group_factor(&self, id: &ArmyId, key: (&'static str, u32)) -> f32 {
        self.rigs
            .get(id)
            .and_then(|rig| rig.groups.get(&key))
            .map_or(1.0, |group| group.factor)
    }
}

/// Loads the figure library (once) when the campaign opens.
pub fn load_skinned(mut skinned: ResMut<SkinnedArmies>) {
    if skinned.library.is_some() {
        skinned.enabled = true;
        return;
    }
    let loaded = crate::paths::find_root().and_then(|root| {
        let data = crate::paths::data_dir(&root);
        let library = FigureLibrary::load(crate::paths::asset_root(&root), &data)?;
        Ok((library, WalkSettings::load(&data), load_lord_scale(&data)))
    });
    match loaded {
        Ok((library, walk, lord)) => {
            let usable = library.manifest.figure("cavalry", 0).is_some()
                && library.manifest.figure("infantry", 0).is_some();
            skinned.enabled = usable;
            skinned.library = usable.then_some(library);
            skinned.walk = walk;
            skinned.lord_scale = lord;
            if !usable {
                skinned.error = Some("manifest without cavalry_0 / infantry_0".into());
            }
        }
        Err(error) => {
            warn!("army figures: {error}");
            skinned.error = Some(error);
        }
    }
}

/// Forgets the batches (they are despawned with the campaign state).
pub fn reset_skinned(mut skinned: ResMut<SkinnedArmies>) {
    skinned.rigs.clear();
    skinned.report = SkinnedReport::default();
}

fn srgb_to_linear(rgb: [f32; 3]) -> [f32; 3] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue]
}

/// Unit type of the strongest regiment of the figure family `key` (its outfit look).
pub fn look_type(roster: &[RosterEntry], key: GroupKey) -> Option<String> {
    roster
        .iter()
        .filter(|r| (r.kind, r.variant) == key && !r.unit_type.is_empty())
        .max_by_key(|r| r.strength)
        .map(|r| r.unit_type.clone())
}

/// Livery share of a group: the lord and his household wear the livery (0.95), the escort half
/// of it (0.7) unless its unit type declares another share (`unit_looks.json`).
pub fn livery_share(variant: u32, look: Option<&Look>) -> f32 {
    if variant == 0 {
        0.95
    } else {
        look.and_then(|l| l.livery_share).unwrap_or(0.7)
    }
}

fn army_params(spec: &FigureSpec, color: [f32; 3], look: Option<&Look>) -> SkinParams {
    let mut params = SkinParams::default();
    params.set_livery(srgb_to_linear(color), livery_share(spec.variant, look));
    params.set_trim(srgb_to_linear(trim_for(color)));
    params.set_rig(
        spec.fps,
        spec.variants,
        if spec.mounted { 0.0 } else { 0.05 },
        6.0,
    );
    params.set_blend_time(BLEND_TIME_S);
    params.set_clips(&spec.clip_table);
    params.set_ga3(spec.ga3_lum, spec.head_y, spec.albedo.is_some());
    params.set_wear(0.0, 0.0);
    if let Some(look) = look {
        // Undyed cloth and horse coats of the unit type (same tables as the battle).
        let cloth = look.cloth_colours();
        if !cloth.is_empty() {
            params.set_plain(&looks::padded(&cloth), cloth.len());
        }
        let coats = look.coat_colours();
        if !coats.is_empty() {
            params.set_coats(&looks::padded(&coats), coats.len());
        }
    }
    params
}

fn seed_clock(id: &ArmyId) -> f32 {
    let hash = id
        .as_str()
        .bytes()
        .fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    (hash % 1000) as f32 * 0.013
}

/// Nominal speed (m/s at clip speed 1) of the locomotion clip of `config`.
fn nominal_of(library: &FigureLibrary, walk: &WalkSettings, config: &anim::AnimConfig) -> f32 {
    config
        .names
        .first()
        .and_then(|n| library.settings.cadence.get(n))
        .map_or(walk.default_nominal_mps, |base| *base * config.speed)
}

/// Pose records of the figures of `key` for a marker (world position, yaw, scale, clock).
#[allow(clippy::too_many_arguments)]
fn records_of(
    slots: &[SkinnedSlot],
    key: GroupKey,
    marker: &ArmyMarker,
    transform: &Transform,
    id_offset: f32,
    clock: f32,
    far_blend: f32,
    highlight: f32,
) -> (Vec<u8>, u32) {
    let frame = Transform {
        translation: transform.translation,
        rotation: Quat::from_rotation_y(marker.heading),
        scale: transform.scale,
    };
    let mut bytes = Vec::new();
    let mut count = 0u32;
    for (index, slot) in slots.iter().enumerate() {
        if (slot.kind, slot.variant) != key {
            continue;
        }
        let at = frame.transform_point(Vec3::new(slot.slot.x, 0.0, slot.slot.y));
        // The figures look at +Z; the troop frame looks at +X.
        let yaw = 90.0 + marker.heading.to_degrees() + slot.yaw.to_degrees();
        let record: [f32; RECORD_FLOATS] = [
            at.x,
            at.y,
            at.z,
            yaw,
            SKINNED_SCALE * slot.size * transform.scale.x,
            index as f32 + id_offset,
            clock,
            far_blend,
            -1000.0,
            highlight,
            0.0,
            0.0,
        ];
        push_record(&mut bytes, &record);
        count += 1;
    }
    (bytes, count)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_skinned_armies(
    mut commands: Commands,
    mut skinned: ResMut<SkinnedArmies>,
    views: Res<ArmyViews>,
    glides: Res<Glides>,
    selected: Res<SelectedArmy>,
    hovered: Res<HoveredArmy>,
    zoom: Option<Res<ZoomView>>,
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    mut batches: Query<&mut SkinnedBatch>,
) {
    if !skinned.enabled {
        return;
    }
    let skinned = &mut *skinned;
    let Some(library) = skinned.library.as_mut() else {
        return;
    };
    let Some(camera) = camera.iter().next() else {
        return;
    };
    let dt = time.delta_secs();
    let hidden_by_zoom = zoom.is_some_and(|z| z.strategic > HIDE_STRATEGIC);
    let mut report = SkinnedReport::default();
    let mut live: Vec<&ArmyId> = Vec::new();
    for (marker, transform) in &markers {
        let Some(view) = views.get(&marker.id) else {
            continue;
        };
        live.push(&marker.id);
        let at_sea = glides.active.contains_key(&marker.id);
        let rig = skinned
            .rigs
            .entry(marker.id.clone())
            .or_insert_with(|| Rig {
                roster: Vec::new(),
                slots: Vec::new(),
                groups: HashMap::new(),
                lod: None,
            });
        if rig.groups.is_empty() || rig.roster != view.roster {
            for group in rig.groups.drain().map(|(_, g)| g) {
                commands.entity(group.entity).despawn();
            }
            rig.roster = view.roster.clone();
            rig.slots = skinned_layout(&view.roster, skinned.lord_scale);
            let mut keys: Vec<GroupKey> = Vec::new();
            for slot in &rig.slots {
                if !keys.contains(&(slot.kind, slot.variant)) {
                    keys.push((slot.kind, slot.variant));
                }
            }
            for key in keys {
                let Some(spec) = library.spec(key.0, key.1, &mut meshes, &mut images, &assets)
                else {
                    continue;
                };
                let look_type = look_type(&rig.roster, key);
                let params = army_params(
                    &spec,
                    view.color,
                    look_type.as_deref().and_then(|t| library.looks.get(t)),
                );
                let entity = commands
                    .spawn((
                        Name::new(format!(
                            "ArmyFigures_{}_{}_{}",
                            marker.id.as_str(),
                            key.0,
                            key.1
                        )),
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
                        DespawnOnExit(AppState::Campaign),
                    ))
                    .id();
                rig.groups.insert(
                    key,
                    Group {
                        entity,
                        look_type,
                        gait: GaitMemory::default(),
                        spec,
                        params,
                        params_version: 1,
                        config_key: String::new(),
                        nominal_mps: skinned.walk.default_nominal_mps,
                        clock: seed_clock(&marker.id),
                        factor: 1.0,
                        version: 0,
                        last_records: Arc::new(Vec::new()),
                        count: 0,
                    },
                );
            }
        }
        let distance = camera.translation().distance(transform.translation);
        let lod = lod_for(distance, rig.lod);
        rig.lod = Some(lod);
        let far_blend = ((distance - FAR_START) / FAR_SPAN).clamp(0.0, 1.0);
        let highlight = highlight_for(
            selected.0.as_ref() == Some(&marker.id),
            hovered.0.as_ref() == Some(&marker.id),
        );
        let shown = !at_sea && !hidden_by_zoom;
        report.armies += 1;
        if shown && highlight > 0.0 {
            report.highlighted += 1;
        }
        for (key, group) in &mut rig.groups {
            let walking = marker.walking && skinned.walk.enabled;
            // Riders walk or trot by their speed on screen (`BattleCavalryGaits`, no gallop and no
            // turn clips on the map); everyone else marches.
            let world_scale = SKINNED_SCALE * transform.scale.x;
            let state = if !walking {
                group.gait = GaitMemory::default();
                "idle"
            } else if group.spec.mounted && library.settings.gaits.enabled {
                gaits::pick(
                    &library.settings.gaits,
                    &mut group.gait,
                    marker.ground_speed / world_scale.max(0.001),
                    false,
                    0.0,
                )
                .key()
            } else {
                "marching"
            };
            let config = anim::state_config(
                &group.spec.style,
                &group.spec.clips,
                state,
                false,
                library.settings.jitter,
            );
            if config.key != group.config_key {
                let fade = !group.config_key.is_empty();
                group.params.set_config(&config, group.clock, fade);
                group.config_key = config.key.clone();
                group.params_version += 1;
                group.nominal_mps = nominal_of(library, &skinned.walk, &config);
            }
            // Walking advances the clip at the pace of the ground (no sliding feet); at rest
            // the clock runs normally.
            let target = if walking && config.mode == Mode::Loop {
                skinned
                    .walk
                    .cadence_factor(marker.ground_speed, group.nominal_mps, world_scale)
            } else {
                1.0
            };
            group.factor += (target - group.factor) * (1.0 - (-8.0 * dt).exp());
            group.clock += dt * group.factor;
            if shown {
                if walking {
                    report.walking_groups += 1;
                    if state == "trotting" {
                        report.trotting_groups += 1;
                    }
                } else {
                    report.idle_groups += 1;
                }
                if group.look_type.is_some() {
                    report.looked_groups += 1;
                }
                if !report.clips.contains(&config.key) {
                    report.clips.push(config.key.clone());
                }
                report.max_cadence = report.max_cadence.max(group.factor);
            }
            let id_offset = (seed_clock(&marker.id) * 100.0).floor() % 13.0 * 500.0;
            let (bytes, count) = if shown {
                records_of(
                    &rig.slots,
                    *key,
                    marker,
                    transform,
                    id_offset,
                    group.clock,
                    far_blend,
                    highlight,
                )
            } else {
                (Vec::new(), 0)
            };
            let Ok(mut batch) = batches.get_mut(group.entity) else {
                continue;
            };
            let wanted = &group.spec.lods[lod.min(2)];
            if batch.mesh_handle != *wanted {
                batch.mesh_handle = wanted.clone();
            }
            if bytes != *group.last_records {
                group.last_records = Arc::new(bytes);
                group.version += 1;
                batch.records = group.last_records.clone();
                batch.version = group.version;
                report.record_uploads += 1;
            }
            group.count = count;
            if batch.count != count {
                batch.count = count;
            }
            if batch.params_version != group.params_version {
                batch.params = Arc::new(group.params.data.clone());
                batch.params_version = group.params_version;
            }
            if shown {
                report.batches += 1;
                report.figures += count as usize;
                report.lods[lod.min(2)] += 1;
            }
        }
    }
    // Armies that left the list: their batches go.
    skinned.rigs.retain(|id, rig| {
        let keep = live.contains(&id);
        if !keep {
            for group in rig.groups.values() {
                commands.entity(group.entity).despawn();
            }
        }
        keep
    });
    skinned.report = report;
}

/// Dump section fields of the skinned figures.
pub fn dump_fields(skinned: &SkinnedArmies) -> serde_json::Value {
    let r = &skinned.report;
    serde_json::json!({
        "skinned_enabled": skinned.enabled,
        "skinned_error": skinned.error,
        "skinned_lord_scale": skinned.lord_scale,
        "skinned_armies": r.armies,
        "skinned_batches": r.batches,
        "skinned_figures": r.figures,
        "skinned_walking_groups": r.walking_groups,
        "skinned_trotting_groups": r.trotting_groups,
        "skinned_looked_groups": r.looked_groups,
        "skinned_idle_groups": r.idle_groups,
        "skinned_highlighted_armies": r.highlighted,
        "skinned_clips": r.clips,
        "skinned_max_cadence": r.max_cadence,
        "skinned_lods": r.lods,
        "skinned_record_uploads": r.record_uploads,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_follows_the_ground_speed_within_bounds() {
        let walk = WalkSettings::default();
        // 1.35 m/s at world scale 2.3 is 3.1 units per second.
        let nominal = 1.35 * 2.3;
        assert!((walk.cadence_factor(nominal, 1.35, 2.3) - 1.0).abs() < 1e-5);
        assert_eq!(walk.cadence_factor(0.0, 1.35, 2.3), 0.35);
        assert_eq!(walk.cadence_factor(100.0, 1.35, 2.3), 1.8);
    }

    #[test]
    fn lod_has_hysteresis() {
        assert_eq!(lod_for(50.0, None), 0);
        assert_eq!(lod_for(200.0, None), 1);
        assert_eq!(lod_for(500.0, None), 2);
        // Just past the edge, a figure already at LOD 0 stays there.
        assert_eq!(lod_for(95.0, Some(0)), 0);
        assert_eq!(lod_for(120.0, Some(0)), 1);
        assert_eq!(lod_for(85.0, Some(1)), 1);
        assert_eq!(lod_for(70.0, Some(1)), 0);
    }

    #[test]
    fn look_type_is_the_strongest_regiment_of_the_family() {
        let entry = |unit_type: &str, strength| RosterEntry {
            kind: "infantry",
            variant: 1,
            strength,
            unit_type: unit_type.into(),
        };
        let roster = vec![entry("unit_a", 100), entry("unit_b", 300), entry("", 900)];
        assert_eq!(
            look_type(&roster, ("infantry", 1)).as_deref(),
            Some("unit_b")
        );
        assert_eq!(look_type(&roster, ("archer", 0)), None);
    }

    #[test]
    fn livery_share_keeps_the_household_and_lets_the_escort_look_decide() {
        let look = Look {
            livery_share: Some(0.3),
            ..Default::default()
        };
        assert_eq!(livery_share(0, Some(&look)), 0.95);
        assert_eq!(livery_share(1, Some(&look)), 0.3);
        assert_eq!(livery_share(1, None), 0.7);
    }

    #[test]
    fn highlight_priority() {
        assert_eq!(highlight_for(false, false), HIGHLIGHT_IDLE);
        assert_eq!(highlight_for(true, false), HIGHLIGHT_SELECTED);
        assert_eq!(highlight_for(true, true), HIGHLIGHT_HOVERED);
    }

    #[test]
    fn settings_read_the_bundled_data() {
        let root = crate::paths::find_root().expect("root");
        let data = crate::paths::data_dir(&root);
        assert!(WalkSettings::load(&data).enabled);
        assert!((load_lord_scale(&data) - 1.8).abs() < 1e-5);
    }
}
