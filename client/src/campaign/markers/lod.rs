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

use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfMaterialName;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use super::places::Places;
use super::{DnBanner, Maquette};
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::towns::atlas::BuildingAtlasMaterial;

/// Name of the banner surface of the kit models.
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

/// True when a kind casts shadows at rig distance `distance`.
pub fn casts_shadow(distance: f32, model_shadow_distance: f32, kind_range: f32) -> bool {
    distance < model_shadow_distance.min(kind_range)
}

/// Controller colours and the banner materials of the kit maquettes.
#[derive(Resource, Default)]
pub struct KitBanners {
    pub colours: HashMap<String, Srgba>,
    materials: HashMap<[u8; 3], Handle<StandardMaterial>>,
}

impl KitBanners {
    pub fn clear(&mut self) {
        self.colours.clear();
        self.materials.clear();
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

    pub fn count(&self) -> usize {
        self.materials.len()
    }
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
    standard: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
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
        commands
            .entity(entity)
            .try_insert(VisibilityRange::abrupt(0.0, f32::MAX));
        stats.range_meshes += 1;
        let is_banner = names
            .get(entity)
            .is_ok_and(|n| n.0.split('.').next() == Some(KIT_BANNER_MATERIAL));
        if is_banner && standard.contains(entity) {
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
    mut stats: ResMut<LodStats>,
) {
    if !places.is_changed() {
        return;
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

/// Dithered fade of every maquette mesh at the end of its kind's range, on the rig distance.
#[allow(clippy::too_many_arguments)]
pub(super) fn fade_maquettes(
    view: Res<ZoomView>,
    places: Res<Places>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    maquettes: Query<&Maquette>,
    mut ranges: Query<(&GlobalTransform, &mut VisibilityRange)>,
    banners: Query<(Entity, &DnBanner)>,
    mut stats: ResMut<LodStats>,
) {
    let Some((_, camera)) = cameras.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let eye = camera.translation();
    let margin = places.maquettes.fade_margin();
    let rig = view.distance;
    let mut fading = 0;
    let mut apply = |entity: Entity, limit: f32| {
        let Ok((t, mut range)) = ranges.get_mut(entity) else {
            return;
        };
        let offset = (eye - t.translation()).length() - rig;
        let want = end_margin(limit, margin, offset);
        if (range.end_margin.start - want.start).abs() > 0.25
            || (range.end_margin.end - want.end).abs() > 0.25
        {
            range.end_margin = want;
        }
        fading += usize::from(rig > limit - margin && rig < limit);
    };
    for m in &maquettes {
        for &entity in &m.meshes {
            apply(entity, m.range);
        }
    }
    for (entity, b) in &banners {
        if let Some(p) = places.places.get(b.place) {
            apply(entity, places.maquettes.range(p.shown_kind));
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
            let Ok((mesh, standard, atlas)) = pieces.get(entity) else {
                continue;
            };
            let material = match (standard, atlas) {
                (Some(s), _) => s.0.id().untyped(),
                (None, Some(a)) => a.0.id().untyped(),
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
}
