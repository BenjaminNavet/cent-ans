//! Province layer of the campaign map: data, projection, picking, borders, map modes, labels.
//!
//! Skeleton. Ports game/scripts/map/{province_picker, faction_borders, map_mode_controller,
//! stance_fill, region_labels, label_placer}.gd. Owned by a single lot of agent (ADR 0357).

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;

pub use labels::{LabelPlacer, RegionLabelObstacles};
pub use modes::{MapMode, MapModeState, SetMapMode};
pub use overlay::ProvinceOverlayConfig;
pub use picker::{
    HoveredProvince, ProvinceClickGuard, ProvinceClicked, ProvincePickSet, SelectProvince,
    SelectedProvince, SurfaceHeight,
};

pub mod borders;
pub mod data;
pub mod drape;
pub mod labels;
pub mod modes;
pub mod overlay;
pub mod picker;
pub mod projection;

pub struct CampaignProvincesPlugin;

impl Plugin for CampaignProvincesPlugin {
    fn build(&self, app: &mut App) {
        picker::build(app);
        labels::build(app);
        overlay::build(app);
        drape::register_shaders(app);
        if app.world().contains_resource::<Assets<Mesh>>() {
            app.add_plugins((
                MaterialPlugin::<drape::ProvinceFillMaterial>::default(),
                MaterialPlugin::<drape::ProvinceBorderMaterial>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    crate::campaign::water::drape::sync_drape::<drape::ProvinceFillMaterial>,
                    crate::campaign::water::drape::sync_drape::<drape::ProvinceBorderMaterial>,
                ),
            );
        }
        app.register_dump(dump_provinces);
    }
}

/// `--dump-state` section `campaign_provinces`.
fn dump_provinces(world: &World) -> (String, Value) {
    let name = "campaign_provinces".to_owned();
    let Some(map) = world.get_resource::<data::ProvinceMap>() else {
        return (name, json!({ "loaded": false }));
    };
    let hovered = world.resource::<HoveredProvince>();
    let selected = world.resource::<SelectedProvince>();
    let mode = world.resource::<MapModeState>();
    let labels = world.resource::<labels::RegionLabelStats>();
    let mut section = json!({
        "loaded": true,
        "provinces": map.provinces().len(),
        "raster": [map.width, map.height],
        "map_mode": mode.mode.key(),
        "focus_faction": mode.focus_faction,
        "hovered": { "index": hovered.index, "id": hovered.id },
        "selected": { "index": selected.index, "id": selected.id },
        "label_count": labels.names.len(),
        "label_names": labels.names,
        "label_weight": labels.weight,
    });
    if let Some(overlay) = world.get_resource::<overlay::OverlayState>() {
        let tinted = overlay.colors.iter().filter(|c| c.alpha > 0.0).count();
        section["border_segments"] = json!({
            "edges": overlay.edges.len(),
            "realm": overlay.counts.realm,
            "province_lines": overlay.counts.province_lines,
            "rebuilds": overlay.border_rebuilds,
            "alpha": overlay.border_alpha,
            "vertices": overlay.border_vertices,
        });
        section["fill"] = json!({
            "texture": [overlay.fill_pixels.0, overlay.fill_pixels.1],
            "tinted_provinces": tinted,
            "rebuilds": overlay.fill_rebuilds,
            "alpha": overlay.fill_alpha,
        });
        let bound = |scale: Option<f32>| scale.is_some_and(|w| w > 0.5);
        let fill_bound = world
            .get_resource::<Assets<drape::ProvinceFillMaterial>>()
            .and_then(|a| a.iter().next().map(|(_, m)| m.relief.scale.w));
        let border_bound = world
            .get_resource::<Assets<drape::ProvinceBorderMaterial>>()
            .and_then(|a| a.iter().next().map(|(_, m)| m.relief.scale.w));
        let seen = |world: &World| {
            world
                .try_query::<(&Name, &ViewVisibility, &InheritedVisibility)>()
                .map(|mut q| {
                    q.iter(world)
                        .filter(|(n, _, _)| n.starts_with("Province"))
                        .map(|(n, v, i)| json!([n.as_str(), v.get(), i.get()]))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        section["overlay_visibility"] = json!(seen(world));
        section["drape"] = json!({
            "fill_relief_bound": bound(fill_bound),
            "border_relief_bound": bound(border_bound),
            "fill_grid_step": overlay::FILL_GRID_STEP,
            "relief_scale": world.get_resource::<Assets<drape::ProvinceBorderMaterial>>()
                .and_then(|a| a.iter().next().map(|(_, m)| [m.relief.scale.to_array(), m.relief.info.to_array()])),
            "border_lift": world.get_resource::<Assets<drape::ProvinceBorderMaterial>>()
                .and_then(|a| a.iter().next().map(|(_, m)| m.params.lift_base)),
        });
        section["factions_seen"] = json!(modes::faction_count(&overlay.view));
        section["viewer"] = json!(overlay.view.viewer);
        section["turn"] = json!(overlay.view.turn);
    }
    (name, section)
}
