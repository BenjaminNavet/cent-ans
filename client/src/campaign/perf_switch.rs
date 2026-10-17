//! Dev-only GPU bisection switches: `CENT_ANS_PERF_OFF=shadows,post,veg,vegshadow,terrain,water,
//! provinces,towns,life,armies,markers,psea` (comma list) turns the named passes / material
//! families off so a bench shows what each one costs. Does nothing when the variable is unset.

use std::sync::OnceLock;

use bevy::camera::visibility::VisibilitySystems;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;

use super::{armies, life, markers, parchment, provinces, terrain, towns, water};

/// True when `name` is listed in `CENT_ANS_PERF_OFF`.
pub fn off(name: &str) -> bool {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        std::env::var("CENT_ANS_PERF_OFF")
            .map(|v| v.split(',').map(|s| s.trim().to_owned()).collect())
            .unwrap_or_default()
    })
    .iter()
    .any(|n| n == name)
}

/// Numeric tuning knob `CENT_ANS_<name>` (dev experiments).
pub fn value(name: &str) -> Option<f32> {
    static CACHE: OnceLock<std::collections::HashMap<String, f32>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            std::env::vars()
                .filter_map(|(k, v)| {
                    Some((k.strip_prefix("CENT_ANS_")?.to_owned(), v.parse().ok()?))
                })
                .collect()
        })
        .get(name)
        .copied()
}

pub struct PerfSwitchPlugin;

impl Plugin for PerfSwitchPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("CENT_ANS_SPAWN_LOG").is_some() {
            app.add_systems(
                PostUpdate,
                spawn_log.before(bevy::camera::visibility::VisibilitySystems::CalculateBounds),
            );
        }
        if std::env::var_os("CENT_ANS_PERF_OFF").is_none() {
            return;
        }
        let before = || VisibilitySystems::VisibilityPropagate;
        app.add_systems(
            PostUpdate,
            (
                sun,
                hide::<terrain::material::TerrainMaterial>("terrain"),
                hide::<water::materials::RiverMaterial>("water"),
                hide::<water::lanes::LaneMaterial>("water"),
                hide::<water::fresh::FreshMaterial>("water"),
                hide::<water::materials::LineMaterial>("water"),
                hide::<water::materials::RoadRibbonMaterial>("water"),
                hide::<water::materials::SeaMaterial>("water"),
                hide::<water::fine::material::FineRiverMaterial>("water"),
                hide::<water::fine::material::FineRoadMaterial>("water"),
            )
                .before(before()),
        )
        .add_systems(
            PostUpdate,
            (
                hide::<provinces::drape::ProvinceFillMaterial>("provinces"),
                hide::<provinces::drape::ProvinceBorderMaterial>("provinces"),
                hide::<towns::atlas::BuildingAtlasMaterial>("towns"),
                hide::<towns::far::TownFarMaterial>("towns"),
                hide::<markers::lod::KitMaterial>("markers"),
                hide::<markers::banner::BannerMaterial>("markers"),
                hide::<life::anim::AnimMaterial>("life"),
                hide::<armies::cloth::FlagMaterial>("armies"),
                hide::<parchment::sea::ParchmentSeaMaterial>("psea"),
            )
                .before(before()),
        );
    }
}

fn sun(mut suns: Query<&mut DirectionalLight>) {
    if off("shadows") {
        for mut sun in &mut suns {
            sun.shadow_maps_enabled = false;
        }
    }
}

fn hide<M: Material>(
    group: &'static str,
) -> impl FnMut(Query<&mut Visibility, With<MeshMaterial3d<M>>>) {
    move |mut q| {
        if off(group) {
            for mut v in &mut q {
                *v = Visibility::Hidden;
            }
        }
    }
}

/// Dev tool (`CENT_ANS_SPAWN_LOG=1`): logs the frames that add 100+ meshes at once, with the
/// nearest named ancestor of each, to find the spawn bursts that stall `calculate_bounds`.
fn spawn_log(
    added: Query<Entity, Added<Mesh3d>>,
    parents: Query<(Option<&Name>, Option<&ChildOf>)>,
    mut frame: Local<u64>,
) {
    *frame += 1;
    let count = added.iter().count();
    if count < 100 {
        return;
    }
    let mut by_name: std::collections::BTreeMap<String, usize> = Default::default();
    for entity in &added {
        let mut cursor = entity;
        let mut label = String::from("?");
        for _ in 0..8 {
            let Ok((name, parent)) = parents.get(cursor) else {
                break;
            };
            if let Some(name) = name {
                label = name.as_str().chars().take(24).collect();
                break;
            }
            match parent {
                Some(p) => cursor = p.parent(),
                None => break,
            }
        }
        *by_name.entry(label).or_default() += 1;
    }
    let mut top: Vec<_> = by_name.into_iter().collect();
    top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    top.truncate(6);
    info!(
        "SPAWN_BURST frame {} meshes {} top {:?}",
        *frame, count, top
    );
}
