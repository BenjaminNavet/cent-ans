//! Painted fields of the campaign map (HB / GR / ADR 0143, 0253, 0290, 0352): the per-biome and
//! per-landscape crop table of `data/art/ground_biome_mix.json` + `data/map/agri_landscapes.json`
//! as a uniform block, plus the `view` / `geo` tuning. Port of `HbGround.build_table` /
//! `apply_view` / `apply_geo` and `landscape_rows`; the Voronoi parcels, crop draw by weight,
//! relief and water rules and the far patchwork live in `terrain.wesl` (`fields_apply`).
//!
//! Differences from the Godot table: rows are indexed directly by biome (1..14) then by landscape
//! (15..22, mask value + 7); the season class of a crop is not stored (the seasonal grade is
//! applied to the whole ground by `season_ground`); crops of one row keep the file order only
//! when serde_json preserves it (cumulative draw: statistically identical either way).

use std::path::Path;

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use serde_json::Value;

/// Rows of the table: 0 unused, 1..14 biomes, 15..22 landscapes, 23 spare.
pub const ROWS: usize = 24;
/// Crop slots per row (`CROP_SLOTS`).
pub const SLOTS: usize = 12;
/// Row of a landscape mask value `raw` (Godot rows 8..15).
pub const LANDSCAPE_ROW_OFFSET: usize = 7;

/// Uniform block `FieldParams` of `terrain.wesl`.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct FieldParams {
    /// Per row: parcel size (m), strip stretch, hedge strength, farm presence.
    pub rows_a: [Vec4; ROWS],
    /// Per row: share of grassland turned to crops, share of vines on slopes, 0, 0.
    pub rows_b: [Vec4; ROWS],
    /// Per (row, slot): summer colour of the crop (rgb) and cumulative draw weight (a; 9 = empty).
    pub slots: [Vec4; ROWS * SLOTS],
    /// View tuning, `view` block: fade start, fade end, patch gain, patch start.
    pub v0: Vec4,
    /// patch end, cell jitter, cell hue, patch saturation.
    pub v1: Vec4,
    /// cell scale, sole scale, sole share, far gain.
    pub v2: Vec4,
    /// far start, far end, terroir scale, terroir start.
    pub v3: Vec4,
    /// terroir end, far edge, far hedge, open farm.
    pub v4: Vec4,
    /// strength, lum band, hue keep, hedge half width (m).
    pub v5: Vec4,
    /// region cells (map px), geo enabled, farm alt start, farm alt end.
    pub v6: Vec4,
    /// farm slope start / end, coteau slope min / max.
    pub v7: Vec4,
    /// coteau south, meadow px, meadow share, enabled.
    pub v8: Vec4,
    /// Vine colour (rgb) and presence (a: 1 when a vine material exists).
    pub vine: Vec4,
    /// Meadow colour (rgb) and presence.
    pub meadow: Vec4,
    /// Hedge colour (linear).
    pub hedge: Vec4,
}

impl Default for FieldParams {
    fn default() -> Self {
        let mut slots = [Vec4::new(0.19, 0.175, 0.075, 9.0); ROWS * SLOTS];
        slots[0].w = 9.0;
        FieldParams {
            rows_a: [Vec4::new(200.0, 1.0, 0.5, 0.0); ROWS],
            rows_b: [Vec4::ZERO; ROWS],
            slots,
            v0: Vec4::new(0.14, 0.38, 0.0, 0.05),
            v1: Vec4::new(0.16, 0.24, 0.3, 1.0),
            v2: Vec4::new(3.2, 0.0, 0.8, 0.0),
            v3: Vec4::new(1.5, 4.0, 0.4, 0.8),
            v4: Vec4::new(1.8, 0.35, 0.0, 0.0),
            v5: Vec4::new(0.92, 0.15, 0.15, 9.0),
            v6: Vec4::new(16.0, 0.0, 1100.0, 1700.0),
            v7: Vec4::new(0.2, 0.45, 0.05, 0.32),
            v8: Vec4::new(0.25, 2.5, 0.75, 0.0),
            vine: Vec4::ZERO,
            meadow: Vec4::ZERO,
            hedge: Vec4::new(0.03, 0.05, 0.02, 0.0),
        }
    }
}

fn f(value: &Value, default: f32) -> f32 {
    value.as_f64().map_or(default, |x| x as f32)
}

fn crop_colour(crop_far: &Value, id: &str) -> Vec3 {
    let c = &crop_far[id]["color"];
    if c.is_null() {
        return Vec3::new(0.19, 0.175, 0.075);
    }
    Vec3::new(f(&c[0], 0.19), f(&c[1], 0.175), f(&c[2], 0.075))
}

/// Writes one table row from a landscape / biome entry.
fn set_row(out: &mut FieldParams, row: usize, entry: &Value, crop_far: &Value, fallbacks: &Value) {
    out.rows_a[row] = Vec4::new(
        f(&entry["cell_m"], 200.0),
        f(&entry["strip"], 1.0),
        f(&entry["hedge"], 0.5),
        f(&entry["farm"], 0.0),
    );
    out.rows_b[row] = Vec4::new(
        f(&entry["open_to_farm"], 0.0),
        f(&entry["coteau_vine"], 0.0),
        0.0,
        0.0,
    );
    let crops = entry["crops"].as_object();
    let total: f32 = crops
        .map(|c| c.values().map(|v| f(v, 0.0)).sum())
        .unwrap_or(0.0);
    let mut cumulative = 0.0;
    for slot in 0..SLOTS {
        out.slots[row * SLOTS + slot] = Vec4::new(0.19, 0.175, 0.075, 9.0);
    }
    if let Some(crops) = crops {
        for (slot, (id, weight)) in crops.iter().take(SLOTS).enumerate() {
            cumulative += f(weight, 0.0) / total.max(1e-6);
            let id = fallbacks[id.as_str()].as_str().unwrap_or(id);
            let colour = crop_colour(crop_far, id);
            out.slots[row * SLOTS + slot] = colour.extend(cumulative.min(1.0));
        }
    }
}

/// Keys of the object under `"landscapes"` in file order (serde_json sorts keys; the mask values
/// of `agri_regions.png` follow the file order, as `HbGround.landscape_rows`).
pub fn landscape_order(text: &str) -> Vec<String> {
    let Some(start) = text.find("\"landscapes\"") else {
        return Vec::new();
    };
    let bytes = text.as_bytes();
    let mut i = start + "\"landscapes\"".len();
    while i < bytes.len() && bytes[i] != b'{' {
        i += 1;
    }
    let mut depth = 0usize;
    let mut keys = Vec::new();
    let mut in_string = false;
    let mut string_start = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                in_string = false;
                // A key: depth 1 and followed by a colon.
                let rest = text[i + 1..].trim_start();
                if depth == 1 && rest.starts_with(':') {
                    keys.push(text[string_start..i].to_string());
                }
            }
        } else {
            match c {
                b'"' => {
                    in_string = true;
                    string_start = i + 1;
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    keys
}

/// Builds the table from the data files under `root`.
pub fn load_fields(root: &Path) -> Result<FieldParams, String> {
    let read = |rel: &str| -> Result<Value, String> {
        let path = root.join(rel);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    };
    let mix = read("data/art/ground_biome_mix.json")?;
    let agri = read("data/map/agri_landscapes.json")?;
    let order = landscape_order(
        &std::fs::read_to_string(root.join("data/map/agri_landscapes.json"))
            .map_err(|e| e.to_string())?,
    );
    let mut out = FieldParams::default();
    let crop_far = &mix["crop_far"];
    let none = Value::Null;
    // Biomes, with the regional overrides of the TX pack (`with_tx_overrides`).
    if let Some(biomes) = mix["biomes"].as_object() {
        for (key, entry) in biomes {
            let Ok(row) = key.parse::<usize>() else {
                continue;
            };
            if !(1..=14).contains(&row) {
                continue;
            }
            let mut merged = entry.clone();
            if let Some(over) = mix["tx_overrides"][key.as_str()].as_object() {
                for (field, value) in over {
                    merged[field.as_str()] = value.clone();
                }
            }
            set_row(&mut out, row, &merged, crop_far, &none);
        }
    }
    // Landscapes: mask value = 8 + index in file order.
    let fallbacks = &agri["material_fallbacks"];
    for (index, id) in order.iter().enumerate() {
        let row = 8 + index + LANDSCAPE_ROW_OFFSET;
        if row < ROWS - 1 {
            set_row(
                &mut out,
                row,
                &agri["landscapes"][id.as_str()],
                crop_far,
                fallbacks,
            );
        }
    }
    let view = &mix["view"];
    let geo = &mix["geo"];
    let d = &out.clone();
    out.v0 = Vec4::new(
        f(&view["fade_start"], d.v0.x),
        f(&view["fade_end"], d.v0.y),
        f(&view["patch_gain"], d.v0.z),
        f(&view["patch_start"], d.v0.w),
    );
    out.v1 = Vec4::new(
        f(&view["patch_end"], d.v1.x),
        f(&view["cell_jitter"], d.v1.y),
        f(&view["cell_hue"], d.v1.z),
        f(&view["patch_sat"], d.v1.w),
    );
    out.v2 = Vec4::new(
        f(&view["cell_scale"], d.v2.x),
        f(&view["sole_scale"], d.v2.y),
        f(&view["sole_share"], d.v2.z),
        f(&view["far_gain"], d.v2.w),
    );
    out.v3 = Vec4::new(
        f(&view["far_start"], d.v3.x),
        f(&view["far_end"], d.v3.y),
        f(&view["terroir_scale"], d.v3.z),
        f(&view["terroir_start"], d.v3.w),
    );
    out.v4 = Vec4::new(
        f(&view["terroir_end"], d.v4.x),
        f(&view["far_edge"], d.v4.y),
        f(&view["far_hedge"], d.v4.z),
        f(&view["open_farm"], d.v4.w),
    );
    out.v5 = Vec4::new(
        d.v5.x,
        f(&view["lum_band"], d.v5.y),
        f(&view["hue_keep"], d.v5.z),
        d.v5.w,
    );
    let geo_on = geo["enabled"].as_bool().unwrap_or(false);
    out.v6 = Vec4::new(
        d.v6.x,
        if geo_on { 1.0 } else { 0.0 },
        f(&geo["farm_alt_start"], d.v6.z),
        f(&geo["farm_alt_end"], d.v6.w),
    );
    out.v7 = Vec4::new(
        f(&geo["farm_slope_start"], d.v7.x),
        f(&geo["farm_slope_end"], d.v7.y),
        f(&geo["coteau_slope_min"], d.v7.z),
        f(&geo["coteau_slope_max"], d.v7.w),
    );
    out.v8 = Vec4::new(
        f(&geo["coteau_south"], d.v8.x),
        f(&geo["meadow_px"], d.v8.y),
        f(&geo["meadow_share"], d.v8.z),
        1.0,
    );
    for (key, slot) in [
        ("vine_material", &mut out.vine),
        ("meadow_material", &mut out.meadow),
    ] {
        if let Some(id) = geo[key].as_str() {
            *slot = crop_colour(crop_far, id).extend(1.0);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landscape_order_follows_the_file() {
        let text = r#"{"description": "x", "landscapes": {"zeta": {"crops": {"a": 1}}, "alpha": {"label": "}\"q"}}, "regions": [{"landscape": "alpha"}]}"#;
        assert_eq!(landscape_order(text), vec!["zeta", "alpha"]);
        let real = include_str!("../../../../data/map/agri_landscapes.json");
        let order = landscape_order(real);
        assert_eq!(order.len(), 8);
        assert_eq!(order[0], "openfield_north");
        assert_eq!(order[1], "bocage_west");
    }

    #[test]
    fn table_carries_crops_landscapes_and_the_view_block() {
        let root = crate::paths::find_root().expect("root");
        let p = load_fields(&root).expect("fields");
        // Biome 2 (continental): wheat first colour, cumulative weights reach 1.
        let row = 2 * SLOTS;
        let last = p.slots[row..row + SLOTS]
            .iter()
            .filter(|s| s.w <= 1.0)
            .map(|s| s.w)
            .fold(0.0f32, f32::max);
        assert!((last - 1.0).abs() < 1e-4, "{last}");
        // Landscape 8 (first of agri_landscapes) -> row 15, openfield_north: cell 260 m, no hedge.
        assert_eq!(p.rows_a[15].x, 260.0);
        assert_eq!(p.rows_a[15].z, 0.0);
        // `view` block: patch gain 0.85, far gain 1.0.
        assert!((p.v0.z - 0.85).abs() < 1e-5);
        assert!((p.v2.w - 1.0).abs() < 1e-5);
        assert_eq!(p.v6.y, 1.0, "geo rules on");
        assert_eq!(p.vine.w, 1.0);
        assert_eq!(p.meadow.w, 1.0);
        // Regional biome 8 (desert) override: date palm oasis crops, not the olive of the base row.
        assert!(p.slots[8 * SLOTS].w <= 1.0, "desert row has crops");
    }
}
