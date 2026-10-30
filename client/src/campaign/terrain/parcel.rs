//! Typed parcel layout of the campaign ground (VIS-R3b, ADR XXXX): the data of the `landscapes`
//! block of `data/art/ground_biome_mix.json` and the pure CPU mirror of the `pc_*` functions of
//! `terrain.wesl` (hash, deformed strip grid, rings of soles around the finages, crop-class draw by
//! season). The mirror exists so that the rules are tested without a GPU; the WGSL functions use
//! the same integer hash and the same arithmetic.

use bevy::prelude::*;
use serde_json::Value;

/// Forms of the table: 0 default, 1 openfield, 2 bocage, 3 midi, 4 montagne.
pub const FORMS: usize = 5;
/// Names of the forms, in table order.
pub const FORM_NAMES: [&str; FORMS] = ["default", "openfield", "bocage", "midi", "montagne"];
/// Crop classes drawn from the shares (`season_field` classes 0 to 5).
pub const CLASSES: usize = 6;
/// Seasons, in the order of `SeasonUniform::weights` (spring, summer, autumn, winter).
pub const SEASONS: usize = 4;

/// Index of a form name (0 when unknown).
pub fn form_index(name: &str) -> usize {
    FORM_NAMES.iter().position(|n| *n == name).unwrap_or(0)
}

/// One form of parcel layout, as read from the data.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub aspect: [f32; 2],
    pub cell_scale: f32,
    pub warp: f32,
    pub brick: f32,
    pub len_var: f32,
    pub ring_strength: f32,
    pub ring_thresholds: [f32; 2],
    pub ring_min: f32,
    pub ring_centers: [f32; 3],
    pub ring_spread: f32,
    pub hedge_px: f32,
    pub hedge_range: f32,
    pub furrow_amp: f32,
    pub furrow_pitch_m: f32,
    pub furrow_range: f32,
    pub reassign: f32,
    /// Cumulative shares per season (spring, summer, autumn, winter) over the six classes.
    pub cumulative: [[f32; CLASSES]; SEASONS],
}

impl Default for Form {
    fn default() -> Self {
        Form {
            aspect: [0.0, 0.0],
            cell_scale: 1.0,
            warp: 1.0,
            brick: 0.0,
            len_var: 0.0,
            ring_strength: 0.0,
            ring_thresholds: [0.55, 0.2],
            ring_min: 0.03,
            ring_centers: [0.15, 0.45, 0.78],
            ring_spread: 0.3,
            hedge_px: 0.0,
            hedge_range: 0.0,
            furrow_amp: 0.0,
            furrow_pitch_m: 4.0,
            furrow_range: 0.0,
            reassign: 0.0,
            cumulative: [[1.0 / 6.0, 2.0 / 6.0, 3.0 / 6.0, 4.0 / 6.0, 5.0 / 6.0, 1.0]; SEASONS],
        }
    }
}

fn num(value: &Value, default: f32) -> f32 {
    value.as_f64().map_or(default, |x| x as f32)
}

/// Normalised cumulative weights of the six classes of a season block.
pub fn cumulative_shares(block: &Value) -> [f32; CLASSES] {
    const KEYS: [&str; CLASSES] = ["wheat", "green", "ploughed", "fallow", "meadow", "hay"];
    let weights: Vec<f32> = KEYS.iter().map(|k| num(&block[*k], 0.0).max(0.0)).collect();
    let total: f32 = weights.iter().sum();
    let mut out = [1.0; CLASSES];
    if total <= 1e-6 {
        return Form::default().cumulative[0];
    }
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        acc += w / total;
        out[i] = acc.min(1.0);
    }
    out[CLASSES - 1] = 1.0;
    out
}

impl Form {
    /// Reads one form of the `landscapes` block; missing keys keep the neutral defaults.
    pub fn from_value(v: &Value) -> Form {
        let d = Form::default();
        let pair = |key: &str, fallback: [f32; 2]| {
            [num(&v[key][0], fallback[0]), num(&v[key][1], fallback[1])]
        };
        let ring = &v["ring"];
        let furrow = &v["furrow"];
        let mut cumulative = d.cumulative;
        for (i, season) in ["spring", "summer", "autumn", "winter"].iter().enumerate() {
            if v["shares"][*season].is_object() {
                cumulative[i] = cumulative_shares(&v["shares"][*season]);
            }
        }
        Form {
            aspect: pair("aspect", d.aspect),
            cell_scale: num(&v["cell_scale"], d.cell_scale),
            warp: num(&v["warp"], d.warp),
            brick: num(&v["brick"], d.brick),
            len_var: num(&v["len_var"], d.len_var),
            ring_strength: num(&ring["strength"], d.ring_strength),
            ring_thresholds: [
                num(&ring["thresholds"][0], d.ring_thresholds[0]),
                num(&ring["thresholds"][1], d.ring_thresholds[1]),
            ],
            ring_min: num(&ring["min"], d.ring_min),
            ring_centers: [
                num(&ring["centers"][0], d.ring_centers[0]),
                num(&ring["centers"][1], d.ring_centers[1]),
                num(&ring["centers"][2], d.ring_centers[2]),
            ],
            ring_spread: num(&ring["spread"], d.ring_spread),
            hedge_px: num(&v["hedge_px"], d.hedge_px),
            hedge_range: num(&v["hedge_range"], d.hedge_range),
            furrow_amp: num(&furrow["amp"], d.furrow_amp),
            furrow_pitch_m: num(&furrow["pitch_m"], d.furrow_pitch_m),
            furrow_range: num(&furrow["range"], d.furrow_range),
            reassign: num(&v["reassign"], d.reassign),
            cumulative,
        }
    }

    /// Cell stretch (length / width) of a region, from its hash draw `h` in [0, 1).
    /// 0 when the form leaves the row's `strip` in charge.
    pub fn aspect_of(&self, h: f32) -> f32 {
        if self.aspect[1] <= 0.0 {
            0.0
        } else {
            self.aspect[0] + (self.aspect[1] - self.aspect[0]) * h
        }
    }
}

/// Reads every form of the block, in table order.
pub fn load_forms(block: &Value) -> [Form; FORMS] {
    std::array::from_fn(|i| Form::from_value(&block[FORM_NAMES[i]]))
}

// --- Mirror of the `pc_*` functions of terrain.wesl --------------------------------------------

/// `pc_hash`: integer hash of a 2D point to [0, 1) (bit pattern of the floats, so it is exact on
/// both sides).
pub fn pc_hash(x: f32, y: f32) -> f32 {
    let mut h = x.to_bits() ^ y.to_bits().wrapping_mul(0x9E37_79B1);
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb_352d);
    h = (h ^ (h >> 15)).wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    (h >> 8) as f32 / 16_777_216.0
}

/// A cell of the deformed strip grid, in the stretched space of the parcel frame (`uu`): id, site,
/// and the distance to its nearest edge in metres per `cell_px` (the along-strip distance is
/// scaled by the stretch).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub id: Vec2,
    pub site: Vec2,
    pub edge: f32,
}

/// `pc_cell`: rows of height 1, each shifted by a hash (`brick`) and cut into cells of a length of
/// its own (`len_var`). `strip` is the stretch already applied to `u.x` by the caller.
pub fn pc_cell(u: Vec2, strip: f32, brick: f32, len_var: f32) -> Cell {
    let row = u.y.floor();
    let off = pc_hash(row, 91.0) * brick * 7.0;
    let mult = 1.0 / (1.0 + len_var * (pc_hash(row, 13.0) - 0.5));
    let xs = (u.x + off) * mult;
    let col = xs.floor();
    let fx = xs - col;
    let fy = u.y - row;
    let across = fy.min(1.0 - fy);
    let along = fx.min(1.0 - fx) / mult * strip;
    Cell {
        id: Vec2::new(col, row),
        site: Vec2::new((col + 0.5) / mult - off, row + 0.5),
        edge: across.min(along),
    }
}

/// `pc_ring_h`: the crop draw `h` of a cell, pulled to the centre of the sole of its ring. `r` is
/// the finage mask (terroir R) at the cell: 0 outside any finage, which keeps the free draw.
pub fn pc_ring_h(form: &Form, r: f32, h: f32, h_dom: f32) -> f32 {
    if form.ring_strength <= 0.0 || r < form.ring_min {
        return h;
    }
    let centre = if r > form.ring_thresholds[0] {
        form.ring_centers[0]
    } else if r > form.ring_thresholds[1] {
        form.ring_centers[1]
    } else {
        form.ring_centers[2]
    };
    let zoned = (centre + (h_dom - 0.5) * 0.2 + (h - 0.5) * form.ring_spread).clamp(0.0, 1.0);
    h + (zoned - h) * form.ring_strength
}

/// `pc_class`: the class of a parcel in a season. `base` is the class of its crop; the draw `h`
/// replaces it for the classes 0 to 5 when a second draw `hr` falls under `reassign`.
pub fn pc_class(form: &Form, season: usize, base: i32, h: f32, hr: f32) -> i32 {
    if base > 5 || hr >= form.reassign {
        return base;
    }
    let cum = &form.cumulative[season];
    cum.iter().position(|c| h < *c).unwrap_or(CLASSES - 1) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_forms() -> [Form; FORMS] {
        let text = include_str!("../../../../data/art/ground_biome_mix.json");
        let value: Value = serde_json::from_str(text).expect("json");
        load_forms(&value["landscapes"])
    }

    #[test]
    fn hash_is_stable_and_uniform() {
        assert_eq!(pc_hash(3.0, 4.0), pc_hash(3.0, 4.0));
        assert_ne!(pc_hash(3.0, 4.0), pc_hash(4.0, 3.0));
        let n = 20_000;
        let mean: f32 = (0..n)
            .map(|i| pc_hash((i % 141) as f32, (i / 141) as f32))
            .sum::<f32>()
            / n as f32;
        assert!((mean - 0.5).abs() < 0.01, "{mean}");
    }

    #[test]
    fn data_block_has_the_five_forms_with_sane_values() {
        let forms = real_forms();
        let open = &forms[form_index("openfield")];
        assert!(
            open.aspect[0] >= 4.0 && open.aspect[1] <= 10.0,
            "1:4 to 1:10"
        );
        assert!(open.ring_strength > 0.0);
        assert!(forms[form_index("bocage")].hedge_px >= 1.0);
        for f in &forms {
            for season in &f.cumulative {
                assert!((season[CLASSES - 1] - 1.0).abs() < 1e-6);
                assert!(season.windows(2).all(|w| w[0] <= w[1]));
            }
        }
    }

    /// Over the same ground distance, the strips are crossed far less often along their axis than
    /// across it (the ratio is the stretch), and a cell is a pure function of its position.
    #[test]
    fn strips_are_elongated_along_their_axis() {
        let strip = 6.0;
        let count = |along: bool| {
            let mut ids = std::collections::HashSet::new();
            for i in 0..2000 {
                let metres = i as f32 * 0.05; // 100 cell widths of ground
                let u = if along {
                    Vec2::new(metres / strip, 0.37)
                } else {
                    Vec2::new(0.37, metres)
                };
                let c = pc_cell(u, strip, 0.0, 0.0);
                ids.insert((c.id.x as i32, c.id.y as i32));
            }
            ids.len()
        };
        let (along, across) = (count(true), count(false));
        assert!(across >= 90, "{across}");
        assert!(
            across as f32 / along as f32 >= strip * 0.7,
            "ratio {across}/{along}"
        );
        let a = pc_cell(Vec2::new(2.3, 5.6), strip, 0.0, 0.9);
        assert_eq!(a, pc_cell(Vec2::new(2.3, 5.6), strip, 0.0, 0.9));
        assert!((0.0..=0.5).contains(&a.edge));
    }

    #[test]
    fn rows_are_aligned_without_brick_and_shifted_with_it() {
        let a = pc_cell(Vec2::new(0.1, 0.5), 1.0, 0.0, 0.0);
        let b = pc_cell(Vec2::new(0.1, 1.5), 1.0, 0.0, 0.0);
        assert_eq!(a.site.x, b.site.x);
        let c = pc_cell(Vec2::new(0.1, 0.5), 1.0, 1.0, 0.0);
        let d = pc_cell(Vec2::new(0.1, 1.5), 1.0, 1.0, 0.0);
        assert_ne!(c.site.x, d.site.x);
    }

    #[test]
    fn rings_order_the_crops_from_the_village_outwards() {
        let forms = real_forms();
        let open = &forms[form_index("openfield")];
        let near = pc_ring_h(open, 0.9, 0.5, 0.5);
        let mid = pc_ring_h(open, 0.35, 0.5, 0.5);
        let far = pc_ring_h(open, 0.1, 0.5, 0.5);
        assert!(near < mid && mid < far, "{near} {mid} {far}");
        // Outside any finage the free draw is kept.
        assert_eq!(pc_ring_h(open, 0.0, 0.37, 0.5), 0.37);
        // A form without rings never moves the draw.
        assert_eq!(pc_ring_h(&forms[0], 0.9, 0.37, 0.5), 0.37);
    }

    /// Share of each class within 5 points of the data over 10 000 draws, every season.
    #[test]
    fn class_shares_follow_the_data() {
        let text = include_str!("../../../../data/art/ground_biome_mix.json");
        let value: Value = serde_json::from_str(text).expect("json");
        let forms = real_forms();
        for (fi, name) in FORM_NAMES.iter().enumerate() {
            let form = &forms[fi];
            if form.reassign <= 0.0 {
                continue;
            }
            for (si, season) in ["spring", "summer", "autumn", "winter"].iter().enumerate() {
                let block = &value["landscapes"][*name]["shares"][*season];
                let cum = cumulative_shares(block);
                let mut expected = [0.0f32; CLASSES];
                let mut prev = 0.0;
                for i in 0..CLASSES {
                    expected[i] = cum[i] - prev;
                    prev = cum[i];
                }
                let n = 10_000;
                let mut counts = [0usize; CLASSES];
                for i in 0..n {
                    // Every draw is reassigned: hr = 0.
                    let h = pc_hash(i as f32, 7.0 + si as f32);
                    counts[pc_class(form, si, 0, h, 0.0) as usize] += 1;
                }
                for c in 0..CLASSES {
                    let got = counts[c] as f32 / n as f32;
                    assert!(
                        (got - expected[c]).abs() < 0.05,
                        "{name} {season} class {c}: {got} vs {}",
                        expected[c]
                    );
                }
            }
        }
    }

    #[test]
    fn perennial_classes_are_never_reassigned() {
        let forms = real_forms();
        let open = &forms[form_index("openfield")];
        assert_eq!(pc_class(open, 1, 6, 0.1, 0.0), 6);
        // `hr` above `reassign` keeps the crop's class.
        assert_eq!(pc_class(open, 1, 2, 0.1, 0.99), 2);
    }
}
