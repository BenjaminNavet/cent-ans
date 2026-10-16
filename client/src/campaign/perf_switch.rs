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
