//! Life objects of the forest clearings (lot TS-T7c): a charcoal kiln, stacks of burners or a
//! timber yard stand in the heart of each clearing published by [`super::groves::ForestClearings`].
//!
//! The choice of the object is the data of the site (`ClearingSite::kind`, an index into
//! `clearings.kinds` of `data/art/campaign_forest_look.json`); the glb, real size, range and
//! budget per quality are the `clearings.life` block of that file. Existing decor models only
//! (`models/dn/buildings`, `models/dn/props`). Shown near the camera only, like the rest of the
//! decor: nearest sites first under the budget, shrinking out over `fade_from..max_distance`.

use std::collections::HashMap;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use super::VegWorld;
use super::groves::{ClearingSite, ForestClearings, hash01};
use super::look::{ClearingLife, ClearingLook, LifeObject};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::smoothstep;
use crate::campaign::terrain::surface::GroundHeight;
use crate::settings::RenderQuality;

/// Seconds between two streaming passes.
const PERIOD: f32 = 0.25;

#[derive(Component)]
pub struct ClearingObject {
    cell: (i64, i64),
    grounded_at: f32,
}

#[derive(Resource, Default)]
pub struct ClearingLifeView {
    live: HashMap<(i64, i64), Entity>,
    pub shown: usize,
}

/// Index of the quality level in `ClearingLife::budget`.
pub fn quality_index(level: &str) -> usize {
    match level {
        "low" => 0,
        "medium" => 1,
        "ultra" => 3,
        _ => 2,
    }
}

/// Screen-held size factor (1 = real size), the rule of the decor.
pub fn size_factor(life: &ClearingLife, object: &LifeObject, mpp: f32, distance: f32) -> f32 {
    let blend = smoothstep(
        life.real_below,
        life.full_from.max(life.real_below + 1e-3),
        distance,
    );
    let span = 2.0 * (life.fov_deg.to_radians() * 0.5).tan() * distance;
    let full = (object.fraction * span * mpp / object.size_m.max(0.1)).clamp(1.0, life.max_factor);
    1.0 + (full - 1.0) * blend
}

/// Shrink factor [0, 1] of the range fade.
pub fn range_fade(life: &ClearingLife, distance: f32) -> f32 {
    let from = life.fade_from.min(life.max_distance - 1e-3);
    ((life.max_distance - distance) / (life.max_distance - from).max(1e-3)).clamp(0.0, 1.0)
}

/// The object of a site: `kinds[kind]` looked up in `life.objects`.
pub fn object_of<'a>(look: &'a ClearingLook, site: &ClearingSite) -> Option<&'a LifeObject> {
    look.life.objects.get(look.kinds.get(site.kind)?)
}

/// Sites to show: those with an object, within `max_distance` of the focus, nearest first, capped.
pub fn pick_sites<'a>(
    look: &ClearingLook,
    sites: impl Iterator<Item = &'a ClearingSite>,
    focus: Vec2,
    budget: usize,
) -> Vec<&'a ClearingSite> {
    let reach = look.life.max_distance;
    let mut found: Vec<(f32, &ClearingSite)> = sites
        .filter(|s| object_of(look, s).is_some())
        .map(|s| (Vec2::new(s.x, s.z).distance(focus), s))
        .filter(|(d, _)| *d <= reach)
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.truncate(budget);
    found.into_iter().map(|f| f.1).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn stream_clearing_life(
    mut commands: Commands,
    world: Option<Res<VegWorld>>,
    clearings: Res<ForestClearings>,
    rig: Option<Res<CampaignRig>>,
    quality: Option<Res<RenderQuality>>,
    server: Res<AssetServer>,
    ground: GroundHeight,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut view: ResMut<ClearingLifeView>,
    mut objects: Query<(&mut ClearingObject, &mut Transform, &mut Visibility)>,
) {
    let (Some(world), Some(rig)) = (world, rig) else {
        return;
    };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = PERIOD;
    let look = &world.0.look.clearings;
    let life = &look.life;
    let mpp = world.0.meters_per_px;
    let d = rig.distance;
    let budget = life.budget[quality_index(quality.as_ref().map_or("high", |q| q.level.as_str()))];
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let wanted = if d < life.max_distance {
        pick_sites(look, clearings.sites.values(), focus, budget)
    } else {
        Vec::new()
    };
    let keep: std::collections::HashSet<(i64, i64)> = wanted.iter().map(|s| s.cell).collect();
    view.live.retain(|cell, entity| {
        let stay = keep.contains(cell);
        if !stay {
            commands.entity(*entity).despawn();
        }
        stay
    });
    for site in wanted {
        if view.live.contains_key(&site.cell) {
            continue;
        }
        let Some(object) = object_of(look, site) else {
            continue;
        };
        let path = format!("models/{}_lod{}.glb", object.model, life.lod);
        let handle: Handle<WorldAsset> = server.load(GltfAssetLabel::Scene(0).from_asset(path));
        let yaw = hash01(site.cell.0, site.cell.1, 21) * std::f32::consts::TAU;
        let entity = commands
            .spawn((
                Name::new(format!("clearing_life:{}", look.kinds[site.kind])),
                Transform::from_xyz(site.x, 0.0, site.z)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(1e-4)),
                Visibility::Hidden,
                ClearingObject {
                    cell: site.cell,
                    grounded_at: f32::NAN,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|p| {
                p.spawn(WorldAssetRoot(handle));
            })
            .id();
        view.live.insert(site.cell, entity);
    }
    // Size, ground (re-seated when the displayed relief moves) and range fade of the live ones.
    let fade = range_fade(life, d);
    let mut shown = 0;
    for (mut object, mut tr, mut vis) in &mut objects {
        let Some(site) = clearings.sites.get(&object.cell) else {
            continue;
        };
        let Some(spec) = object_of(look, site) else {
            continue;
        };
        if let Some(y) = ground.at(site.x, site.z)
            && (object.grounded_at.is_nan() || (object.grounded_at - y).abs() > 1e-3)
        {
            tr.translation.y = y;
            object.grounded_at = y;
        }
        let scale = size_factor(life, spec, mpp, d) / mpp * fade;
        tr.scale = Vec3::splat(scale.max(1e-5));
        let show = !object.grounded_at.is_nan() && fade > 0.01;
        shown += usize::from(show);
        let wanted = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != wanted {
            *vis = wanted;
        }
    }
    view.shown = shown;
}

pub fn reset(mut commands: Commands, mut view: ResMut<ClearingLifeView>) {
    for (_, entity) in view.live.drain() {
        commands.entity(entity).despawn();
    }
    view.shown = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> ClearingLook {
        let doc: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../data/art/campaign_forest_look.json"),
            )
            .unwrap(),
        )
        .unwrap();
        super::super::look::ForestLook::parse(&doc).clearings
    }

    fn site(x: f32, kind: usize, cell: i64) -> ClearingSite {
        ClearingSite::for_test(x, 0.0, kind, (cell, 0))
    }

    #[test]
    fn every_kind_has_an_object_and_the_budget_grows_with_quality() {
        let look = look();
        for k in &look.kinds {
            assert!(look.life.objects.contains_key(k), "no life object for {k}");
        }
        assert_eq!(look.life.budget[0], 0);
        assert!(look.life.budget.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn nearest_sites_first_under_budget_and_range() {
        let look = look();
        let far = look.life.max_distance + 10.0;
        let sites = [
            site(30.0, 0, 3),
            site(5.0, 1, 1),
            site(far, 2, 9),
            site(12.0, 2, 2),
        ];
        let picked = pick_sites(&look, sites.iter(), Vec2::ZERO, 2);
        let cells: Vec<_> = picked.iter().map(|s| s.cell.0).collect();
        assert_eq!(cells, vec![1, 2]);
        assert_eq!(pick_sites(&look, sites.iter(), Vec2::ZERO, 10).len(), 3);
        assert!(pick_sites(&look, sites.iter(), Vec2::ZERO, 0).is_empty());
    }

    #[test]
    fn size_is_real_when_close_and_fade_ends_at_the_range() {
        let look = look();
        let object = look.life.objects.values().next().unwrap();
        assert!((size_factor(&look.life, object, 100.0, 1.0) - 1.0).abs() < 1e-3);
        assert!(size_factor(&look.life, object, 100.0, look.life.full_from * 2.0) >= 1.0);
        assert_eq!(range_fade(&look.life, look.life.fade_from - 1.0), 1.0);
        assert_eq!(range_fade(&look.life, look.life.max_distance), 0.0);
    }
}
