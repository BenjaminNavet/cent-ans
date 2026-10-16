//! FA7 grass atlas (port of the `fa_*` part of game/scripts/battle/battle_vegetation.gd and of
//! game/shaders/battle_grass.gdshader): tufts made of photographed blades
//! (`textures/battle/grass_tufts.png`, catalogue `data/art/battle_grass.json`). Each card of a
//! tuft draws its own cell of the atlas, by weight (short lawn in the gaps, tall grass at the
//! heart of the clumps), flipped one time in two; its height is the height of the cell.
//!
//! Godot picks the cell in the vertex shader of a camera-following MultiMesh; here the tufts are
//! scattered once, so the cell is picked on the CPU and baked into the UVs of the merged chunk.
//! The atlas is kept as is, which is what Godot does for the green group; the other groups
//! (cells replaced by the group's cards in `battle_grass_groups.gd`) keep their own cards.

use serde_json::Value;

/// Atlas texture, relative to the asset root.
pub const ATLAS_PATH: &str = "textures/battle/grass_tufts.png";
/// Catalogue under `data/`.
pub const CATALOGUE: &str = "art/battle_grass.json";

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub height_m: f32,
    pub weight_open: f32,
    pub weight_clump: f32,
}

/// Grass of the decor fields (`render.fields` of the catalogue; `fa_kind` 1 wheat, 2 stubble,
/// 3 seedlings in battle_grass.gdshader).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldCells {
    /// Cells of the wheat and of the stubble / seedlings.
    pub wheat: [usize; 2],
    pub stubble: [usize; 2],
    /// Height factor of wheat, fallow, stubble and seedlings.
    pub height: [f32; 4],
    pub wheat_gain: [f32; 3],
    pub wheat_foot_shade: f32,
    pub seedling_gain: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrassAtlas {
    pub columns: usize,
    pub rows: usize,
    /// Margin of a cell (fraction of the cell on each axis).
    pub pad: [f32; 2],
    /// Mean luminance of the atlas (`render.tex_lum`).
    pub tex_lum: f32,
    /// Width of a card at the top (m) and foot width / top width.
    pub width_m: f32,
    pub foot_ratio: f32,
    /// Size draw of a tuft (min, max).
    pub size: [f32; 2],
    /// Share kept of the patchy height variations.
    pub height_var: f32,
    /// Density kept between the clumps / in the gaps at ~25 m.
    pub gap_fill: f32,
    pub patch_fill: f32,
    /// Darkening at the foot of a tuft (1 = none).
    pub foot_shade: f32,
    pub variants: Vec<Variant>,
    /// Wheat, stubble and seedling fields (`render.fields`).
    pub fields: FieldCells,
    /// Tint gain of the atlas (`render.tint_gain`).
    pub tint_gain: [f32; 3],
    cum_open: Vec<f32>,
    cum_clump: Vec<f32>,
}

fn num(doc: &Value, path: &[&str], default: f32) -> f32 {
    path.iter()
        .try_fold(doc, |d, k| d.get(*k))
        .and_then(Value::as_f64)
        .map_or(default, |v| v as f32)
}

impl GrassAtlas {
    /// Parses the catalogue (`None` when it has no variants).
    pub fn from_json(doc: &Value) -> Option<GrassAtlas> {
        let variants: Vec<Variant> = doc["variants"]
            .as_array()?
            .iter()
            .filter_map(|v| {
                Some(Variant {
                    name: v["name"].as_str()?.to_owned(),
                    height_m: v["height_m"].as_f64()? as f32,
                    weight_open: v["weight_open"].as_f64()? as f32,
                    weight_clump: v["weight_clump"].as_f64()? as f32,
                })
            })
            .collect();
        if variants.is_empty() {
            return None;
        }
        let total = |f: fn(&Variant) -> f32| variants.iter().map(f).sum::<f32>().max(1e-6);
        let cumulate = |f: fn(&Variant) -> f32| {
            let sum = total(f);
            let mut acc = 0.0;
            variants
                .iter()
                .map(|v| {
                    acc += f(v) / sum;
                    acc
                })
                .collect::<Vec<f32>>()
        };
        let columns = num(doc, &["atlas", "columns"], 4.0) as usize;
        let rows = num(doc, &["atlas", "rows"], 3.0) as usize;
        let cell_w = num(doc, &["atlas", "cell_width"], 512.0);
        let cell_h = num(doc, &["atlas", "cell_height"], 256.0);
        let padding = num(doc, &["atlas", "padding"], 8.0);
        let cell = |key: &str, i: usize| {
            let name = doc["render"]["fields"][key][i].as_str().unwrap_or("");
            variants.iter().position(|v| v.name == name).unwrap_or(0)
        };
        let triple = |key: &str, default: f32| {
            let list = &doc["render"]["fields"][key];
            [0, 1, 2].map(|i| list[i].as_f64().map_or(default, |v| v as f32))
        };
        let fields = FieldCells {
            wheat: [cell("wheat_variants", 0), cell("wheat_variants", 1)],
            stubble: [cell("stubble_variants", 0), cell("stubble_variants", 1)],
            height: [
                num(doc, &["render", "fields", "wheat_height"], 1.0),
                num(doc, &["render", "fields", "fallow_height"], 1.2),
                num(doc, &["render", "fields", "stubble_height"], 1.0),
                num(doc, &["render", "fields", "seedling_height"], 0.6),
            ],
            wheat_gain: triple("wheat_gain", 1.0),
            wheat_foot_shade: num(doc, &["render", "fields", "wheat_foot_shade"], 0.6),
            seedling_gain: triple("seedling_gain", 1.0),
        };
        let tint_gain = {
            let list = &doc["render"]["tint_gain"];
            [0, 1, 2].map(|i| list[i].as_f64().map_or(1.0, |v| v as f32))
        };
        Some(GrassAtlas {
            fields,
            tint_gain,
            columns: columns.max(1),
            rows: rows.max(1),
            pad: [padding / cell_w, padding / cell_h],
            tex_lum: num(doc, &["render", "tex_lum"], 0.2),
            width_m: num(doc, &["render", "card", "width_m"], 0.8),
            foot_ratio: num(doc, &["render", "card", "foot_ratio"], 0.8),
            size: [
                doc["render"]["size"][0].as_f64().unwrap_or(0.8) as f32,
                doc["render"]["size"][1].as_f64().unwrap_or(1.25) as f32,
            ],
            height_var: num(doc, &["render", "height_var"], 0.5),
            gap_fill: num(doc, &["render", "gap_fill"], 0.9),
            patch_fill: num(doc, &["render", "patch_fill"], 0.5),
            foot_shade: num(doc, &["render", "foot_shade"], 0.85),
            cum_open: cumulate(|v| v.weight_open),
            cum_clump: cumulate(|v| v.weight_clump),
            variants,
        })
    }

    pub fn load(data_dir: &std::path::Path) -> Option<GrassAtlas> {
        let bytes = std::fs::read(data_dir.join(CATALOGUE)).ok()?;
        GrassAtlas::from_json(&serde_json::from_slice(&bytes).ok()?)
    }

    /// Cell drawn for `pick` in `[0, 1)` at `in_clump` in `[0, 1]` (0 in the gaps, 1 at the
    /// heart of a clump): the weights are mixed, then walked cumulatively (the last cell by
    /// default), like the shader.
    pub fn pick(&self, pick: f32, in_clump: f32) -> usize {
        for i in 0..self.variants.len() {
            let limit = self.cum_open[i] + (self.cum_clump[i] - self.cum_open[i]) * in_clump;
            if pick < limit {
                return i;
            }
        }
        self.variants.len() - 1
    }

    /// UV corners `(u0, v0, u1, v1)` of the cell `index` inside its margin, `v0` at the top.
    pub fn cell_uv(&self, index: usize) -> [f32; 4] {
        let col = (index % self.columns) as f32;
        let row = (index / self.columns) as f32;
        let (c, r) = (self.columns as f32, self.rows as f32);
        [
            (col + self.pad[0]) / c,
            (row + self.pad[1]) / r,
            (col + 1.0 - self.pad[0]) / c,
            (row + 1.0 - self.pad[1]) / r,
        ]
    }
}

/// Smoothstep (GLSL).
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in `[0, 1]` of a position (`bt_value_noise`), hashed on the lattice.
pub fn value_noise(x: f32, z: f32, salt: f32) -> f32 {
    let (ix, iz) = (x.floor(), z.floor());
    let (fx, fz) = (x - ix, z - iz);
    let h = |dx: f32, dz: f32| super::hash01(ix + dx, iz + dz, salt);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let a = h(0.0, 0.0) + (h(1.0, 0.0) - h(0.0, 0.0)) * sx;
    let b = h(0.0, 1.0) + (h(1.0, 1.0) - h(0.0, 1.0)) * sx;
    a + (b - a) * sz
}

/// Density kept at `(x, z)`: the continuous sheet of FA7, patchy at ~25 m and in ~2 m clumps
/// (`density` of the vertex shader), and the clump factor (`in_clump`) that drives the cells.
pub fn density_at(atlas: &GrassAtlas, x: f32, z: f32) -> (f32, f32) {
    let patches = value_noise(x * 0.04 + 5.3, z * 0.04 + 5.3, 11.0) * 0.7
        + value_noise(x * 0.11 + 1.9, z * 0.11 + 1.9, 12.0) * 0.3;
    let clump = value_noise(x * 0.55 + 7.1, z * 0.55 + 7.1, 13.0);
    let in_clump = smoothstep(0.3, 0.72, clump);
    let density = (atlas.patch_fill + (1.0 - atlas.patch_fill) * smoothstep(0.08, 0.38, patches))
        * (atlas.gap_fill + (1.0 - atlas.gap_fill) * in_clump);
    (density, in_clump)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> GrassAtlas {
        let root = crate::paths::find_root().expect("root");
        GrassAtlas::load(&crate::paths::data_dir(&root)).expect("catalogue")
    }

    #[test]
    fn the_catalogue_describes_a_twelve_cell_atlas() {
        let a = atlas();
        assert_eq!((a.columns, a.rows), (4, 3));
        assert_eq!(a.variants.len(), 12);
        assert!((a.tex_lum - 0.2).abs() < 1e-6);
        assert!((a.size[0] - 0.8).abs() < 1e-6 && (a.size[1] - 1.25).abs() < 1e-6);
        assert!((a.pad[0] - 8.0 / 512.0).abs() < 1e-6);
    }

    #[test]
    fn short_grass_fills_the_gaps_and_tall_grass_the_clumps() {
        let a = atlas();
        let short = |i: usize| a.variants[i].height_m < 0.3;
        // Gaps: the low cells dominate the draw; clump hearts: the tall ones take more.
        let mean = |in_clump: f32| {
            (0..1000)
                .map(|k| a.variants[a.pick(k as f32 / 1000.0, in_clump)].height_m)
                .sum::<f32>()
                / 1000.0
        };
        assert!(mean(1.0) > mean(0.0) + 0.05, "{} {}", mean(1.0), mean(0.0));
        assert!(short(a.pick(0.0, 0.0)));
        // Every draw lands in the atlas.
        assert!(a.pick(0.999999, 0.5) < a.variants.len());
    }

    #[test]
    fn the_catalogue_names_the_field_cells() {
        let a = atlas();
        assert_eq!(a.variants[a.fields.wheat[0]].name, "wheat_a");
        assert_eq!(a.variants[a.fields.wheat[1]].name, "wheat_b");
        assert_eq!(a.variants[a.fields.stubble[0]].name, "short_a");
        assert!((a.fields.height[3] - 0.6).abs() < 1e-6);
        assert!((a.fields.wheat_gain[2] - 0.56).abs() < 1e-6);
        assert!((a.tint_gain[0] - 1.25).abs() < 1e-6);
    }

    #[test]
    fn cells_are_inset_by_their_margin() {
        let a = atlas();
        let uv = a.cell_uv(5); // column 1, row 1
        assert!(uv[0] > 0.25 && uv[2] < 0.5);
        assert!(uv[1] > 1.0 / 3.0 && uv[3] < 2.0 / 3.0);
    }

    #[test]
    fn the_sheet_is_dense_but_not_uniform() {
        let a = atlas();
        let samples: Vec<f32> = (0..400)
            .map(|k| density_at(&a, k as f32 * 3.7, k as f32 * 1.3).0)
            .collect();
        assert!(samples.iter().all(|d| (0.0..=1.0 + 1e-5).contains(d)));
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(mean > 0.6, "{mean}");
        assert!(samples.iter().any(|d| *d < 0.95));
    }
}
