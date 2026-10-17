//! Level of detail of the maquettes (port of `TownMaquetteLayer._update_fades`, lot towns4):
//!
//! - range fade: the maquettes of a kind fade out over `fade_margin` before their `visibility`
//!   limit (rig distance) with Bevy's dithered [`VisibilityRange`] (Godot: `visibility_range_end`
//!   with `FADE_SELF` plus `transparency = 1 - alpha`). The range is measured from the eye, the
//!   limit from the rig: every mesh carries `limit + (eye distance - rig distance)`.
//! - shadow range: a kind casts no shadow beyond `shadow_range` (small places) nor beyond the
//!   `model_shadow_distance` of the quality preset.
//! - kit banner faces: the `Banner` surface of the kit maquettes takes the controller's colour
//!   (`maquette_kit.gdshader`: vertex alpha 0 -> `INSTANCE_CUSTOM.rgb`).
//! - instancing: Bevy batches the meshes sharing a mesh and a material (the glb of a place is
//!   one scene per place, as the glb are shared between places the batches are the
//!   MultiMesh-equivalent); `batch_stats` counts them.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use bevy::asset::embedded_asset;
use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfMaterialName;
use bevy::light::NotShadowCaster;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::world_serialization::WorldInstanceReady;

use super::places::Places;
use super::{DnBanner, Maquette};
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::towns::atlas::BuildingAtlasMaterial;

/// Name of the single surface of the six family kits (banner faces: vertex alpha 0).
pub const KIT_MATERIAL: &str = "Kit";
/// Name of the banner surface of the older multi-surface kit models.
pub const KIT_BANNER_MATERIAL: &str = "Banner";
/// Colour of a banner whose controller has no heraldic colour.
pub const DEFAULT_BANNER: Srgba = Srgba::new(0.55, 0.1, 0.08, 1.0);

/// Opacity of a maquette at rig distance `distance` (`_update_fades`): 1 up to `limit - margin`,
/// 0 from `limit`.
pub fn fade_alpha(limit: f32, margin: f32, distance: f32) -> f32 {
    ((limit - distance) / margin.max(0.001)).clamp(0.0, 1.0)
}

/// End margin of the [`VisibilityRange`] of a mesh whose eye distance exceeds the rig distance by
/// `offset`: fully visible up to `limit - margin + offset`, gone at `limit + offset`.
pub fn end_margin(limit: f32, margin: f32, offset: f32) -> Range<f32> {
    (limit - margin + offset)..(limit + offset)
}

/// Offset step of the quantised fade range. Bevy never frees a `VisibilityRange` slot (one per
/// distinct value ever seen, 65535 max), so per-frame continuous offsets used to overflow it and
/// white-out the ground; a coarse step keeps the set of distinct values small and bounded.
pub fn offset_step(margin: f32) -> f32 {
    (margin / 8.0).max(2.0)
}

/// [`end_margin`] with the offset snapped to [`offset_step`] and clamped to +-`OFFSET_CLAMP`.
pub fn quantised_end_margin(limit: f32, margin: f32, offset: f32) -> Range<f32> {
    let step = offset_step(margin);
    let snapped = (offset.clamp(-OFFSET_CLAMP, OFFSET_CLAMP) / step).round() * step;
    end_margin(limit, margin, snapped)
}

/// Largest eye-to-rig distance difference taken into account by the fade ranges.
const OFFSET_CLAMP: f32 = 4096.0;

/// True when a kind casts shadows at rig distance `distance`.
pub fn casts_shadow(distance: f32, model_shadow_distance: f32, kind_range: f32) -> bool {
    distance < model_shadow_distance.min(kind_range)
}

/// Extension of the kit material: colour of the banner faces (linear rgb).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct KitExt {
    #[uniform(100)]
    pub tint: Vec4,
}

impl MaterialExtension for KitExt {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/markers/kit_banner.wesl".into()
    }
}

pub type KitMaterial = ExtendedMaterial<StandardMaterial, KitExt>;

pub fn add(app: &mut App) {
    embedded_asset!(app, "kit_banner.wesl");
    app.add_plugins(MaterialPlugin::<KitMaterial>::default());
}

/// Controller colours and the banner materials of the kit maquettes.
#[derive(Resource, Default)]
pub struct KitBanners {
    pub colours: HashMap<String, Srgba>,
    materials: HashMap<[u8; 3], Handle<StandardMaterial>>,
    faces: HashMap<([u8; 3], bevy::asset::AssetId<StandardMaterial>), Handle<KitMaterial>>,
}

impl KitBanners {
    pub fn clear(&mut self) {
        self.colours.clear();
        self.materials.clear();
        self.faces.clear();
    }

    pub fn colour_of(&self, controller: &str) -> Srgba {
        self.colours
            .get(controller)
            .copied()
            .unwrap_or(DEFAULT_BANNER)
    }

    /// Shared flat material of a colour (rough 0.8, two-sided like the glb's `Banner`).
    pub fn material(
        &mut self,
        assets: &mut Assets<StandardMaterial>,
        colour: Srgba,
    ) -> Handle<StandardMaterial> {
        let key =
            [colour.red, colour.green, colour.blue].map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
        self.materials
            .entry(key)
            .or_insert_with(|| {
                assets.add(StandardMaterial {
                    base_color: colour.into(),
                    perceptual_roughness: 0.8,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            })
            .clone()
    }

    /// Kit material of `source` with banner faces of `colour` (shared by colour and source).
    pub fn face_material(
        &mut self,
        assets: &mut Assets<KitMaterial>,
        source: &StandardMaterial,
        source_id: bevy::asset::AssetId<StandardMaterial>,
        colour: Srgba,
    ) -> Handle<KitMaterial> {
        let key =
            [colour.red, colour.green, colour.blue].map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
        self.faces
            .entry((key, source_id))
            .or_insert_with(|| {
                let linear = LinearRgba::from(colour);
                assets.add(KitMaterial {
                    base: source.clone(),
                    extension: KitExt {
                        tint: Vec4::new(linear.red, linear.green, linear.blue, 1.0),
                    },
                })
            })
            .clone()
    }

    pub fn count(&self) -> usize {
        self.materials.len() + self.faces.len()
    }
}

/// A `Kit` surface whose banner faces follow the controller of its place.
#[derive(Component)]
pub struct KitFaces {
    pub place: usize,
    pub controller: String,
    source: Handle<StandardMaterial>,
}

/// A banner surface of a kit maquette, repainted when its place changes hands.
#[derive(Component)]
pub struct KitBanner {
    pub place: usize,
    pub controller: String,
}

/// Counters of the dump.
#[derive(Resource, Default, Debug)]
pub struct LodStats {
    pub range_meshes: usize,
    pub kit_banners: usize,
    /// `Kit` surfaces whose banner faces follow the controller.
    pub kit_faces: usize,
    pub kit_banners_recoloured: u64,
    /// Maquettes without shadows (beyond their shadow range).
    pub shadowless: usize,
    pub scene_meshes: usize,
    /// Distinct (mesh, material) pairs among the scene meshes: the draw batches.
    pub batches: usize,
    /// Meshes currently in their fade band (partially dithered).
    pub fading: usize,
}

/// Once a maquette scene is spawned: lists its meshes, gives each its visibility range and
/// repaints the banner surface of the kit models.
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_maquette(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&GltfMaterialName>,
    meshes: Query<(), With<Mesh3d>>,
    standard: Query<&MeshMaterial3d<StandardMaterial>>,
    mut face_assets: ResMut<Assets<KitMaterial>>,
    mut maquettes: Query<&mut Maquette>,
    places: Res<Places>,
    mut kit: ResMut<KitBanners>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<LodStats>,
) {
    let Ok(mut maquette) = maquettes.get_mut(ready.entity) else {
        return;
    };
    let controller = places
        .places
        .get(maquette.place)
        .map(|p| p.controller.clone())
        .unwrap_or_default();
    for entity in children.iter_descendants(ready.entity) {
        if !meshes.contains(entity) {
            continue;
        }
        maquette.meshes.push(entity);
        commands.entity(entity).try_insert(solid_range());
        stats.range_meshes += 1;
        let is_banner = names
            .get(entity)
            .is_ok_and(|n| n.0.split('.').next() == Some(KIT_BANNER_MATERIAL));
        let is_kit = names
            .get(entity)
            .is_ok_and(|n| n.0.split('.').next() == Some(KIT_MATERIAL));
        if is_kit && let Ok(source) = standard.get(entity) {
            let colour = kit.colour_of(&controller);
            let fallback = StandardMaterial {
                perceptual_roughness: 0.9,
                double_sided: true,
                cull_mode: None,
                ..default()
            };
            let handle = kit.face_material(
                &mut face_assets,
                materials.get(&source.0).unwrap_or(&fallback),
                source.0.id(),
                colour,
            );
            commands
                .entity(entity)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert((
                    MeshMaterial3d(handle),
                    KitFaces {
                        place: maquette.place,
                        controller: controller.clone(),
                        source: source.0.clone(),
                    },
                ));
            stats.kit_faces += 1;
        } else if is_banner && standard.contains(entity) {
            let colour = kit.colour_of(&controller);
            let handle = kit.material(&mut materials, colour);
            commands.entity(entity).try_insert((
                MeshMaterial3d(handle),
                KitBanner {
                    place: maquette.place,
                    controller: controller.clone(),
                },
            ));
            stats.kit_banners += 1;
        }
    }
}

/// Repaints the kit banners whose place changed hands.
pub fn recolour_kit_banners(
    places: Res<Places>,
    mut kit: ResMut<KitBanners>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut banners: Query<(&mut KitBanner, &mut MeshMaterial3d<StandardMaterial>)>,
    mut faces: Query<(&mut KitFaces, &mut MeshMaterial3d<KitMaterial>)>,
    mut face_assets: ResMut<Assets<KitMaterial>>,
    mut stats: ResMut<LodStats>,
) {
    if !places.is_changed() {
        return;
    }
    for (mut face, mut material) in &mut faces {
        let Some(p) = places.places.get(face.place) else {
            continue;
        };
        if p.controller == face.controller {
            continue;
        }
        face.controller = p.controller.clone();
        let colour = kit.colour_of(&p.controller);
        let fallback = StandardMaterial {
            perceptual_roughness: 0.9,
            double_sided: true,
            cull_mode: None,
            ..default()
        };
        let source = materials.get(&face.source).unwrap_or(&fallback).clone();
        material.0 = kit.face_material(&mut face_assets, &source, face.source.id(), colour);
        stats.kit_banners_recoloured += 1;
    }
    for (mut banner, mut material) in &mut banners {
        let Some(p) = places.places.get(banner.place) else {
            continue;
        };
        if p.controller == banner.controller {
            continue;
        }
        banner.controller = p.controller.clone();
        let colour = kit.colour_of(&p.controller);
        material.0 = kit.material(&mut materials, colour);
        stats.kit_banners_recoloured += 1;
    }
}

/// Quantum (world units) of the per-mesh range offsets. Bevy keeps every distinct
/// [`VisibilityRange`] ever seen in a table that is never purged (`RenderVisibilityRanges::
/// range_to_index`, 65 535 slots, then every new range silently shares slot 0), so the ranges
/// written each frame must come from a small set.
pub const RANGE_QUANTUM: f32 = 1.0;

/// Rounds an eye/rig distance offset to [`RANGE_QUANTUM`].
pub fn quantise_offset(offset: f32) -> f32 {
    (offset / RANGE_QUANTUM).round() * RANGE_QUANTUM
}

/// True when the maquettes of a kind (limit `limit`, fade `margin`) need a per-mesh dithered
/// range at rig distance `rig`: only inside the fade band. Outside it the fade is constant
/// (fully visible below, root hidden by `update_range` above) and the meshes keep the shared
/// abrupt range, which avoids rewriting 1000+ ranges (and re-extracting as many meshes) every
/// frame and keeps them off the dither pipeline variant.
pub fn in_fade_band(limit: f32, margin: f32, rig: f32) -> bool {
    rig > limit - margin && rig < limit
}

/// The constant range of a mesh outside the fade band.
pub fn solid_range() -> VisibilityRange {
    if crate::campaign::perf_switch::off("fadesoftsolid") {
        // Dev experiment: a constant but crossfading range (dither pipeline variant kept).
        VisibilityRange {
            start_margin: 0.0..0.0,
            end_margin: 1.0e8..1.0e8 + 1.0e4,
            use_aabb: false,
        }
    } else {
        VisibilityRange::abrupt(0.0, f32::MAX)
    }
}

/// Dithered fade of every maquette mesh at the end of its kind's range, on the rig distance.
#[allow(clippy::too_many_arguments)]
pub(super) fn fade_maquettes(
    view: Res<ZoomView>,
    places: Res<Places>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut maquettes: Query<&mut Maquette>,
    mut ranges: Query<(&GlobalTransform, &mut VisibilityRange)>,
    banners: Query<(Entity, &DnBanner)>,
    mut banded_banners: Local<HashSet<Entity>>,
    mut stats: ResMut<LodStats>,
) {
    let Some((_, camera)) = cameras.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let eye = camera.translation();
    let margin = places.maquettes.fade_margin();
    let rig = view.distance;
    let legacy = crate::campaign::perf_switch::off("fadeband");
    let mut fading = 0;
    let mut apply = |entity: Entity, limit: f32, banded: &mut bool| {
        let band = legacy || in_fade_band(limit, margin, rig);
        if !band {
            if *banded {
                if let Ok((_, mut range)) = ranges.get_mut(entity) {
                    *range = solid_range();
                }
            }
            return;
        }
        let Ok((t, mut range)) = ranges.get_mut(entity) else {
            return;
        };
        let raw = (eye - t.translation()).length() - rig;
        let want = if legacy {
            end_margin(limit, margin, raw)
        } else {
            quantised_end_margin(limit, margin, raw)
        };
        if range.end_margin != want {
            range.end_margin = want;
        }
        fading += usize::from(rig > limit - margin && rig < limit);
    };
    for mut m in &mut maquettes {
        let band = legacy || in_fade_band(m.range, margin, rig);
        // A maquette outside the band whose meshes are already solid costs nothing.
        if !band && !m.banded {
            continue;
        }
        let range = m.range;
        let mut banded = m.banded;
        for &entity in &m.meshes {
            apply(entity, range, &mut banded);
        }
        m.banded = band;
    }
    for (entity, b) in &banners {
        if let Some(p) = places.places.get(b.place) {
            let limit = places.maquettes.range(p.shown_kind);
            let band = legacy || in_fade_band(limit, margin, rig);
            if !band && !banded_banners.contains(&entity) {
                continue;
            }
            let mut banded = banded_banners.contains(&entity);
            apply(entity, limit, &mut banded);
            if band {
                banded_banners.insert(entity);
            } else {
                banded_banners.remove(&entity);
            }
        }
    }
    stats.fading = fading;
}

/// Shadow range per kind: the meshes of a maquette cast no shadow beyond `shadow_range` or the
/// `model_shadow_distance` of the quality preset.
pub(super) fn update_shadows(
    mut commands: Commands,
    view: Res<ZoomView>,
    quality: Option<Res<crate::settings::quality::RenderQuality>>,
    mut maquettes: Query<&mut Maquette>,
    mut stats: ResMut<LodStats>,
) {
    let limit = quality.map_or(f32::INFINITY, |q| q.model_shadow_distance);
    let mut shadowless = 0;
    for mut m in &mut maquettes {
        let want = casts_shadow(view.distance, limit, m.shadow_range);
        shadowless += usize::from(!want);
        if m.meshes.is_empty() || m.casting == Some(want) {
            continue;
        }
        m.casting = Some(want);
        for &entity in &m.meshes {
            if want {
                commands.entity(entity).try_remove::<NotShadowCaster>();
            } else {
                commands.entity(entity).try_insert(NotShadowCaster);
            }
        }
    }
    stats.shadowless = shadowless;
}

/// Counts the draw batches of the maquette meshes twice a second.
pub(super) fn batch_stats(
    time: Res<Time>,
    mut timer: Local<f32>,
    maquettes: Query<&Maquette>,
    pieces: Query<(
        &Mesh3d,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<BuildingAtlasMaterial>>,
        Option<&MeshMaterial3d<KitMaterial>>,
    )>,
    mut stats: ResMut<LodStats>,
) {
    *timer += time.delta_secs();
    if *timer < 0.5 {
        return;
    }
    *timer = 0.0;
    let mut pairs: HashSet<(AssetId<Mesh>, bevy::asset::UntypedAssetId)> = HashSet::new();
    let mut count = 0;
    for m in &maquettes {
        for &entity in &m.meshes {
            let Ok((mesh, standard, atlas, kit)) = pieces.get(entity) else {
                continue;
            };
            let material = match (standard, atlas, kit) {
                (Some(s), _, _) => s.0.id().untyped(),
                (None, Some(a), _) => a.0.id().untyped(),
                (None, None, Some(k)) => k.0.id().untyped(),
                _ => continue,
            };
            count += 1;
            pairs.insert((mesh.0.id(), material));
        }
    }
    stats.scene_meshes = count;
    stats.batches = pairs.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantised_ranges_stay_few_over_a_long_session() {
        // Regression (white ground): Bevy never frees VisibilityRange slots (65535 max).
        let mut seen = std::collections::HashSet::new();
        for limit in [300.0_f32, 600.0, 900.0, 1500.0, 2400.0, 4000.0] {
            for i in 0..200_000 {
                let offset = (i as f32) * 0.0137 - 1300.0;
                let r = quantised_end_margin(limit, 60.0, offset);
                seen.insert((r.start.to_bits(), r.end.to_bits()));
            }
        }
        assert!(seen.len() < 5000, "{} distinct ranges", seen.len());
    }

    #[test]
    fn alpha_is_one_below_the_band_and_zero_at_the_limit() {
        assert_eq!(fade_alpha(900.0, 60.0, 800.0), 1.0);
        assert!((fade_alpha(900.0, 60.0, 870.0) - 0.5).abs() < 1e-6);
        assert_eq!(fade_alpha(900.0, 60.0, 900.0), 0.0);
        assert_eq!(fade_alpha(900.0, 60.0, 2000.0), 0.0);
        // A zero margin is a hard cut, not a division by zero.
        assert_eq!(fade_alpha(900.0, 0.0, 899.0), 1.0);
        assert_eq!(fade_alpha(900.0, 0.0, 901.0), 0.0);
    }

    #[test]
    fn band_is_open_below_the_limit_only() {
        assert!(!in_fade_band(900.0, 60.0, 840.0));
        assert!(in_fade_band(900.0, 60.0, 841.0));
        assert!(in_fade_band(900.0, 60.0, 899.9));
        assert!(!in_fade_band(900.0, 60.0, 900.0));
    }

    #[test]
    fn offsets_are_quantised_to_a_small_set() {
        use std::collections::HashSet;
        let distinct: HashSet<i32> = (0..10_000)
            .map(|i| quantise_offset(i as f32 * 0.037 - 100.0) as i32)
            .collect();
        // 370 units of span -> at most ~371 distinct ranges instead of 10 000.
        assert!(distinct.len() <= 372, "{}", distinct.len());
    }

    #[test]
    fn end_margin_follows_the_eye_offset() {
        let r = end_margin(900.0, 60.0, 0.0);
        assert_eq!((r.start, r.end), (840.0, 900.0));
        let r = end_margin(900.0, 60.0, 35.0);
        assert_eq!((r.start, r.end), (875.0, 935.0));
    }

    #[test]
    fn shadows_stop_at_the_nearest_limit() {
        assert!(casts_shadow(50.0, 250.0, 100.0));
        assert!(!casts_shadow(150.0, 250.0, 100.0));
        assert!(!casts_shadow(300.0, 250.0, f32::INFINITY));
        assert!(casts_shadow(300.0, f32::INFINITY, f32::INFINITY));
    }

    #[test]
    fn kit_banner_materials_are_shared_by_colour() {
        let mut assets = Assets::<StandardMaterial>::default();
        let mut kit = KitBanners::default();
        let a = kit.material(&mut assets, Srgba::new(0.1, 0.2, 0.9, 1.0));
        let b = kit.material(&mut assets, Srgba::new(0.1, 0.2, 0.9, 1.0));
        let c = kit.material(&mut assets, Srgba::new(0.9, 0.2, 0.1, 1.0));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(kit.count(), 2);
        assert_eq!(kit.colour_of("none"), DEFAULT_BANNER);
    }

    #[test]
    fn kit_faces_are_shared_by_colour_and_source() {
        let mut std_assets = Assets::<StandardMaterial>::default();
        let source = std_assets.add(StandardMaterial::default());
        let base = StandardMaterial::default();
        let mut faces = Assets::<KitMaterial>::default();
        let mut kit = KitBanners::default();
        let red = Srgba::new(0.8, 0.1, 0.1, 1.0);
        let a = kit.face_material(&mut faces, &base, source.id(), red);
        let b = kit.face_material(&mut faces, &base, source.id(), red);
        let c = kit.face_material(
            &mut faces,
            &base,
            source.id(),
            Srgba::new(0.1, 0.1, 0.8, 1.0),
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        // The tint is linear.
        let tint = faces.get(&a).unwrap().extension.tint;
        assert!((tint.x - LinearRgba::from(red).red).abs() < 1e-6);
    }
}
