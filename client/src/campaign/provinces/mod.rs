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
pub mod shields;

pub struct CampaignProvincesPlugin;

impl Plugin for CampaignProvincesPlugin {
    fn build(&self, app: &mut App) {
        picker::build(app);
        labels::build(app);
        overlay::build(app);
        shields::build(app);
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

/// Camera distances (map units) of the effective band width table.
pub const WIDTH_DISTANCES: [f32; 5] = [2600.0, 1200.0, 600.0, 200.0, 60.0];

/// Effective band widths in screen pixels at the reference distances, for the Godot comparison
/// (`docs/wip/bm-borders.md`), plus the live camera state.
fn border_widths_dump(world: &World, overlay: &overlay::OverlayState) -> Value {
    let Some(tuning) = world.get_resource::<modes::Tuning>() else {
        return Value::Null;
    };
    let t = &tuning.borders;
    let row = |distance: f32, focused: bool, mine: bool| {
        let w = borders::EffectiveWidths::new(t, distance, 0.0, focused, mine);
        json!({
            "realm_core_px": w.realm_core,
            "realm_outline_px": w.realm_outline,
            "realm_glow_px": w.realm_glow,
            "province_line_px": w.province_line,
            "occupation_band_px": w.occupation_band,
        })
    };
    let table = |focused: bool, mine: bool| -> Vec<Value> {
        WIDTH_DISTANCES
            .iter()
            .map(|&d| {
                let mut r = row(d, focused, mine);
                r["distance"] = json!(d);
                r
            })
            .collect()
    };
    json!({
        "unit": "screen px, both sides of the line for core / outline / province line",
        "focused": table(true, false),
        "rest": table(false, false),
        "player_focused": table(true, true),
        "view_scale": { "width": overlay.view_scale.width, "glow": overlay.view_scale.glow },
        "camera_distance": overlay.camera_distance,
    })
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
            "hatched": overlay.counts.hatched,
            "halo_edges": overlay.counts.halo,
            "halo_pulse": overlay.halo_pulse,
            "rebuilds": overlay.border_rebuilds,
            "alpha": overlay.border_alpha,
            "vertices": overlay.border_vertices,
        });
        section["border_widths"] = border_widths_dump(world, overlay);
        section["fill"] = json!({
            "texture": [overlay.fill_pixels.0, overlay.fill_pixels.1],
            "tinted_provinces": tinted,
            "rebuilds": overlay.fill_rebuilds,
            "alpha": overlay.fill_alpha,
            "hatch": overlay.hatch,
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
        if let Some(stats) = world.get_resource::<shields::ShieldStats>() {
            section["feudal_shields"] = json!({
                "spawned": stats.spawned, "visible": stats.visible, "rebuilds": stats.rebuilds,
                "hatch_layer": overlay.hatch_colors.iter().filter(|c| c.alpha > 0.0).count(),
                "doubles": overlay.doubles.len(),
            });
        }
        section["mode_palettes"] = mode_palettes(world, map, overlay);
        section["factions_seen"] = json!(modes::faction_count(&overlay.view));
        section["viewer"] = json!(overlay.view.viewer);
        section["turn"] = json!(overlay.view.turn);
    }
    (name, section)
}

/// Per map mode: tinted provinces, distinct colours and the five most used (`#rrggbb` + alpha),
/// computed from the live simulation with the ramp lenses on, so each mode can be checked by dump.
fn mode_palettes(world: &World, map: &data::ProvinceMap, overlay: &overlay::OverlayState) -> Value {
    use std::collections::BTreeMap;
    let (Some(sim), Some(tuning)) = (
        world.get_resource::<crate::sim::Sim>(),
        world.get_resource::<modes::Tuning>(),
    ) else {
        return Value::Null;
    };
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return Value::Null;
    };
    let view = modes::read_view(map, data, state, &sim.player, true);
    let mut out = serde_json::Map::new();
    let paris = map.index_of_id("prov_ile_de_france");
    for mode in MapMode::ALL {
        let colors =
            modes::province_colors(mode, map.max_index(), &view, &overlay.heraldic, tuning);
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for c in colors.iter().skip(1).filter(|c| c.alpha > 0.0) {
            let [r, g, b, a] = c.to_u8_array();
            *counts
                .entry(format!("#{r:02x}{g:02x}{b:02x}/{a}"))
                .or_default() += 1;
        }
        let mut top: Vec<(&String, &usize)> = counts.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let hatch = modes::province_hatch(mode, map.max_index(), &view, &overlay.heraldic);
        out.insert(
            mode.key().to_owned(),
            json!({
                "hatched": hatch.iter().filter(|c| c.alpha > 0.0).count(),
                "double_allegiances": if mode == MapMode::Feudal {
                    modes::double_allegiances(&view, map.max_index()).len()
                } else { 0 },
                "hover_paris": crate::campaign::hud::hover::mode_text(mode, &view, paris, None),
                "tinted": counts.values().sum::<usize>(),
                "distinct": counts.len(),
                "top": top.iter().take(5).map(|(k, n)| json!([k, n])).collect::<Vec<_>>(),
            }),
        );
    }
    Value::Object(out)
}
