// Campaign terrain module `parcels` (VIS-L1, split of terrain.wesl): parcel grids: painted-field cells (VIS-R3b), fp parcels, Voronoi helpers.
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::ground_layers::{region_value, terroir_at};
import cent_ans_client::campaign::terrain::noise::{hash01i, luma, vnoise};
import cent_ans_client::campaign::terrain::seasons::{season_farm, season_field, season_field_w, season_grass, season_ratio, season_vine_leaf};
import cent_ans_client::campaign::terrain::terrain_params::{VINE_LEAF, ab_dist, fields, ground, ground_sampler, params, parcels_albedo, parcels_normal, season, splat, terroir_sampler};

const TINT_GRASS = vec3<f32>(0.105, 0.165, 0.045);

const TINT_FARM = vec3<f32>(0.21, 0.20, 0.075);

// --- Close patchwork (CV1 terroirs, V2b parcels, ZG5b fine parcels) ---------------------------
// Ports of campaign_life.gdshaderinc (season_field, terroir_*), terrain.gdshader (field_at and the
// V2b parcel colours, hedges, tracks) and fine_parcels.gdshaderinc (fp_splat, fp_parcels).

// V2b palette (terrain.gdshader uniforms, linear).
const CROP_WHEAT = vec3<f32>(0.36, 0.265, 0.075);

const CROP_GREEN = vec3<f32>(0.12, 0.175, 0.045);

const CROP_PLOUGHED = vec3<f32>(0.15, 0.10, 0.055);

const CROP_FALLOW = vec3<f32>(0.19, 0.175, 0.075);

const MEADOW_COLOR = vec3<f32>(0.095, 0.16, 0.042);

const HAY_COLOR = vec3<f32>(0.24, 0.215, 0.085);

const VINE_SOIL = vec3<f32>(0.19, 0.135, 0.075);

const DRY_GRASS = vec3<f32>(0.23, 0.19, 0.085);

const HEDGE_WIDTH = 0.16;

const TRACK_WIDTH = 0.10;

const FIELD_SCALE = 0.3;

// fine_parcels.gdshaderinc (fp_quality 2).
const FP_START = 0.05;

const FP_FULL = 0.015;

const FP_BLOCK = 8.0;

const FP_HEDGE = vec3<f32>(0.028, 0.045, 0.016);

const FP_HEADLAND = vec3<f32>(0.13, 0.17, 0.06);

const FP_WASTE = vec3<f32>(0.13, 0.13, 0.06);

const FP_ASSART = vec3<f32>(0.12, 0.12, 0.05);

const FP_TERRACE = vec3<f32>(0.16, 0.14, 0.11);

fn field_warp(q: vec2<f32>, ph: f32) -> vec2<f32> {
    let ox = 1.7 * sin(0.071 * q.y + 1.9 * sin(0.027 * q.x) + ph) + 0.55 * sin(0.23 * q.y + 1.3 * sin(0.061 * q.x));
    let oy = 1.7 * sin(0.063 * q.x + 1.7 * sin(0.023 * q.y) + ph * 1.3) + 0.55 * sin(0.19 * q.x + 1.1 * sin(0.047 * q.y));
    return vec2<f32>(ox, oy);
}

struct Field {
    cell_h: f32,
    strip_h: f32,
    du: f32,
    dv: f32,
    roll_u: f32,
    roll_v: f32,
    strip_d: f32,
    strip_dir: vec2<f32>,
    rows: f32,
};

// Parcel at p (layout `lay`): enclosure and strip draws, distances (parcel units) to the edges at
// constant u / of the row and the draws of those edges, distance to the strip edge, strip
// direction (map frame) and across coordinate (vine rows).
fn field_at(p: vec2<f32>, lay: i32) -> Field {
    var out: Field;
    let first = lay == 0;
    let k = select(-0.8, 0.35, first);
    let fu = select(4.6, 5.0, first);
    let fv = select(3.8, 4.2, first);
    let ph = select(2.1, 0.0, first);
    var q = p;
    for (var i = 0; i < 3; i = i + 1) {
        q = p - field_warp(q, ph);
    }
    let u = (q.x + k * q.y) / fu;
    let v = (q.y - k * q.x) / fv;
    let inv = inverseSqrt(1.0 + k * k);
    let cu = i32(floor(u));
    let vs = v + hash01i(cu * 5023 + 17 + lay * 101);
    let cv = i32(floor(vs));
    let fu_f = fract(u);
    let fv_f = fract(vs);
    out.du = min(fu_f, 1.0 - fu_f) * fu * inv;
    out.dv = min(fv_f, 1.0 - fv_f) * fv * inv;
    let uline = select(cu + 1, cu, fu_f < 0.5);
    let vline = select(cv + 1, cv, fv_f < 0.5);
    out.roll_u = hash01i(uline * 7919 + i32(floor(v)) * 104729 + lay * 7 + 1);
    out.roll_v = hash01i(vline * 7919 + cu * 104729 + lay * 7 + 31);
    let cell_key = cu * 7919 + cv * 104729 + lay * 7 + 3;
    out.cell_h = hash01i(cell_key);
    let h2 = hash01i(cell_key + 11);
    let h3 = hash01i(cell_key + 23);
    let n = 1.0 + floor(h2 * 4.0);
    let along_u = h3 < 0.5;
    var t = select(fv_f, fu_f, along_u);
    t = t + 0.035 * sin(select(u, v, along_u) * 9.0 + h2 * 6.0);
    let ts = clamp(t, 0.0, 0.999) * n;
    let strip = floor(ts);
    out.strip_h = hash01i(cell_key + i32(strip) * 13 + 5);
    let sf = fract(ts);
    out.strip_d = select(1e3, min(sf, 1.0 - sf) / n * select(fv, fu, along_u) * inv, n > 1.0);
    out.strip_dir = select(normalize(vec2<f32>(1.0, k)), normalize(vec2<f32>(-k, 1.0)), along_u);
    out.rows = ts;
    return out;
}

struct V2b {
    grass_tint: vec3<f32>,
    farm_tint: vec3<f32>,
    hedge: f32,
    track: f32,
    balk: f32,
    vine_rows: f32,
    // TB1 season shifts of the meadows and the crops (season / summer, summer gold included).
    k_grass: vec3<f32>,
    k_farm: vec3<f32>,
    grass_summer: vec3<f32>,
    farm_summer: vec3<f32>,
};

// Lot V2b of terrain.gdshader: enclosures, crop strips, meadows, vines, hedges and dirt tracks;
// the tints replace the grass / farmland tints of the textured soil. Call in uniform control
// flow (fwidth of the vine rows).
fn v2b_parcels(p: vec2<f32>, footprint: f32, s: vec4<f32>, lu: vec3<f32>) -> V2b {
    var out: V2b;
    let dry = lu.g;
    let open_land = clamp(s.g + 0.6 * s.r, 0.0, 1.0);
    let grass_share = s.r / max(s.r + s.g, 1e-3);
    let grass_summer = mix(TINT_GRASS, DRY_GRASS, dry * 0.75);
    let farm_summer = mix(TINT_FARM, mix(TINT_FARM, CROP_WHEAT, 0.35) * vec3<f32>(1.05, 0.98, 0.9), dry * 0.6);
    let grass_base = season_grass(grass_summer);
    let farm_base = season_farm(farm_summer);
    out.grass_summer = grass_summer;
    out.farm_summer = farm_summer;
    out.k_grass = season_ratio(grass_base, grass_summer);
    out.k_farm = season_ratio(farm_base * mix(vec3<f32>(1.0), season.summer_gold.rgb, season.weights.y), farm_summer);
    out.grass_tint = grass_base;
    out.farm_tint = farm_base;
    let field_fp = footprint / FIELD_SCALE;
    let cell_px = 4.4 / max(field_fp, 1e-4);
    let field_vis = smoothstep(5.0, 14.0, cell_px);
    let rv = region_value(p);
    let lay = select(0, 1, rv > 0.0);
    let f = field_at(p / FIELD_SCALE, lay);
    // Derivatives first (uniform control flow).
    let rw = fwidth(f.rows * 5.0);
    if (field_vis <= 0.0 || (s.g + s.r) <= 0.05) {
        return out;
    }
    let strip_vis = smoothstep(3.0, 8.0, cell_px * 0.4);
    let h = select(f.cell_h, f.strip_h, strip_vis > 0.5);
    let h_b = hash01i(i32(h * 99991.0) + 7);
    let p_meadow = mix(0.15, 0.85, grass_share);
    var crop: vec3<f32>;
    let vine = h_b < lu.r * 0.32 && grass_share < 0.8;
    if (vine) {
        crop = season_field(VINE_SOIL, 6, h_b);
    } else if (h < p_meadow) {
        let m = h / max(p_meadow, 1e-3);
        if (m < 0.6) {
            crop = season_field(MEADOW_COLOR * (0.85 + 0.35 * h_b), 4, h_b);
        } else if (m < 0.85) {
            crop = season_field(mix(MEADOW_COLOR, HAY_COLOR, 0.55), 5, h_b);
        } else {
            crop = season_field(HAY_COLOR, 5, h_b);
        }
        crop = mix(crop, DRY_GRASS, dry * 0.7);
    } else {
        let c = (h - p_meadow) / max(1.0 - p_meadow, 1e-3);
        if (c < 0.36) {
            crop = season_field(CROP_WHEAT, 0, h_b);
        } else if (c < 0.6) {
            crop = season_field(CROP_GREEN, 1, h_b);
        } else if (c < 0.76) {
            crop = season_field(CROP_PLOUGHED, 2, h_b);
        } else {
            crop = season_field(CROP_FALLOW, 3, h_b);
        }
        crop = crop * (0.9 + 0.2 * h_b);
        crop = mix(crop, crop * vec3<f32>(1.08, 0.96, 0.82), dry);
    }
    // Parcel colours later than their outline: no noisy checkerboard at mid zoom.
    let crop_fade = smoothstep(6.0, 24.0, cell_px);
    out.grass_tint = mix(grass_base, crop, crop_fade * 0.8);
    out.farm_tint = mix(farm_base, crop, crop_fade * 0.85);
    if (vine) {
        let r = abs(fract(f.rows * 5.0) - 0.5);
        let fine = 1.0 - smoothstep(0.25, 0.6, rw);
        out.vine_rows = mix(0.45, 1.0 - smoothstep(0.22 - rw, 0.22 + rw, r), fine) * field_vis;
    }
    // Edges: hedge (draw < probability), dirt track (other draw), else a grassy balk.
    let p_hedge = open_land * mix(0.1, 0.75, lu.b);
    let aa = field_fp * 0.75;
    let hedge_u = step(f.roll_u, p_hedge) * (1.0 - smoothstep(HEDGE_WIDTH - aa, HEDGE_WIDTH + aa, f.du));
    let hedge_v = step(f.roll_v, p_hedge) * (1.0 - smoothstep(HEDGE_WIDTH - aa, HEDGE_WIDTH + aa, f.dv));
    let cover = clamp(2.0 * HEDGE_WIDTH / max(field_fp, 1e-4), 0.0, 1.0);
    out.hedge = max(hedge_u, hedge_v) * mix(cover, 1.0, cover) * field_vis;
    let tr_u = step(fract(f.roll_u * 7.31), 0.07) * (1.0 - smoothstep(TRACK_WIDTH - aa, TRACK_WIDTH + aa, f.du));
    let tr_v = step(fract(f.roll_v * 7.31), 0.07) * (1.0 - smoothstep(TRACK_WIDTH - aa, TRACK_WIDTH + aa, f.dv));
    let tcover = clamp(2.0 * TRACK_WIDTH / max(field_fp, 1e-4), 0.0, 1.0);
    var tr_r = 0.0;
    if (tcover > 0.15 && abs(rv) < 0.05) {
        // Border between the two layouts: a continuous track.
        let rgrad = length(vec2<f32>(region_value(p + vec2<f32>(1.0, 0.0)) - rv, region_value(p + vec2<f32>(0.0, 1.0)) - rv));
        let rd = abs(rv) / max(rgrad, 1e-4);
        tr_r = 1.0 - smoothstep(TRACK_WIDTH * 1.3 - aa, TRACK_WIDTH * 1.3 + aa, rd);
    }
    out.track = max(max(tr_u, tr_v), tr_r) * tcover * field_vis;
    let edge = min(min(f.du, f.dv), f.strip_d);
    out.balk = (1.0 - smoothstep(0.03, 0.03 + aa * 2.0, edge)) * smoothstep(8.0, 20.0, cell_px) * 0.5;
    return out;
}

// --- Fine parcels (ZG5b, fine_parcels.gdshaderinc) ---
fn fp_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn fp_hash2(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn fp_mod(x: vec2<f32>, y: f32) -> vec2<f32> {
    return x - y * floor(x / y);
}

struct FpVor {
    id: vec2<f32>,
    center: vec2<f32>,
    border: f32,
};

// Voronoi on a jittered grid of `per` cells per block: global id of the cell, centre (m, local
// frame), approximate distance to the edge (m).
fn fp_voronoi(q: vec2<f32>, blk: vec2<f32>, per: f32, jitter: f32, mpp: f32) -> FpVor {
    let size = FP_BLOCK * mpp / per;
    let g = floor(q / size);
    let f = q / size - g;
    var d1 = 1e9;
    var d2 = 1e9;
    var best = vec2<f32>(0.0);
    var best_c = vec2<f32>(0.0);
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let o = vec2<f32>(f32(i), f32(j));
            let gid = g + o;
            let id = (blk + floor(gid / per)) * per + fp_mod(gid, per);
            let c = o + 0.5 + (fp_hash2(id) - 0.5) * jitter;
            let d = length(c - f);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                best = id;
                best_c = (g + c) * size;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return FpVor(best, best_c, (d2 - d1) * 0.5 * size);
}

// Hook 1: splat texel read in warped coordinates close up (edges of the 719 m squares broken).
fn fp_splat(s: vec4<f32>, p: vec2<f32>, footprint: f32) -> vec4<f32> {
    let vis = 1.0 - smoothstep(0.03, 0.12, footprint);
    if (vis <= 0.0) {
        return s;
    }
    var w = vec2<f32>(vnoise(p * 0.9), vnoise(p * 0.9 + vec2<f32>(31.7, 11.3))) - 0.5;
    w = w + (vec2<f32>(vnoise(p * 3.7 + 5.1), vnoise(p * 3.7 + 17.9)) - 0.5) * 0.45;
    w = w + (vec2<f32>(vnoise(p * 14.0 + 2.3), vnoise(p * 14.0 + 8.1)) - 0.5) * 0.18;
    let warped = textureSampleLevel(splat, terroir_sampler, (p + w * 1.1) / params.map.xy, 0.0);
    return mix(s, warped, vis);
}

struct FpOut {
    color: vec3<f32>,
    dn: vec2<f32>,
};

// Hook 3: parcels at their real scale, in metres (openfield strips and three-field soles,
// bocage enclosures with hedges, vines on south slopes, assarts by the woods, waste far from the
// villages). Not ported: valley hay meadows (relief occlusion, wetlands), 1:1 town finages.
// `grad` in m per map px, `open_w` / `forest_w` normalised shares.
fn fp_parcels(p: vec2<f32>, footprint: f32, grad: vec2<f32>, slope: f32, hc: f32, s: vec4<f32>,
        lu: vec3<f32>, terroir: vec4<f32>, open_w: f32, forest_w: f32, col: vec3<f32>) -> FpOut {
    var out = FpOut(col, vec2<f32>(0.0));
    let mpp = max(ground.micro.w, 1.0);
    let blk = floor(p / FP_BLOCK);
    let q = (p - blk * FP_BLOCK) * mpp;
    let g = grad / mpp;
    let g_len = length(g);
    let downhill = select(vec2<f32>(0.0, 1.0), -g / max(g_len, 1e-6), g_len > 1e-4);
    // Derivatives first (uniform control flow): vine rows along the contour lines.
    let row = dot(q, downhill) / 1.8;
    let rw = fwidth(row);
    let vis = 1.0 - smoothstep(FP_FULL, FP_START, footprint);
    if (vis <= 0.0 || open_w + forest_w < 0.05 || hc < 0.25) {
        return out;
    }
    let px_m = footprint * mpp;
    let aa = max(px_m, 0.05);
    let bocage = smoothstep(0.25, 0.6, lu.b + (vnoise(p * 0.25) - 0.5) * 0.25);
    let village = clamp(terroir.r * 1.3 + terroir.a * 0.6, 0.0, 1.0);
    let grass_share = s.r / max(s.r + s.g, 1e-3);
    let south = select(0.0, clamp(-g.y / max(g_len, 1e-6), -1.0, 1.0), g_len > 1e-4);
    let vine_site = smoothstep(0.04, 0.10, slope) * (1.0 - smoothstep(0.45, 0.7, slope)) * smoothstep(0.1, 0.6, south);
    let vine_w = vine_site * clamp(lu.r * 2.2 + 0.12, 0.0, 1.0) * (1.0 - bocage * 0.6);
    let assart = smoothstep(0.08, 0.3, forest_w) * (1.0 - smoothstep(0.5, 0.75, forest_w));
    let waste = (1.0 - smoothstep(0.05, 0.35, village)) * smoothstep(0.3, 0.8, grass_share + slope * 2.0);
    var hedge = 0.0;
    var furrow_on = 0.0;
    // Openfield: quarters of strips.
    let fv = fp_voronoi(q, blk, 14.0, 0.8, mpp);
    let fh = fp_hash(fv.id + 3.1);
    let ang = fh * 3.14159;
    let dir = vec2<f32>(cos(ang), sin(ang));
    let across = vec2<f32>(-dir.y, dir.x);
    let strip_w = mix(15.0, 35.0, fp_hash(fv.id + 7.7));
    let t = dot(q - fv.center, across) / strip_w;
    let strip = floor(t);
    let sh = fp_hash(fv.id * 1.37 + vec2<f32>(strip, strip * 0.37));
    // Soles of the three-field rotation (about 1.5 km) shared by neighbouring quarters.
    let sole = floor(vnoise(p * 0.45 + 3.3) * 2.999 + fh * 0.4);
    let c_pick = fract(sole / 3.0 + select(sh, 0.0, sh < 0.75));
    var crop: vec3<f32>;
    if (c_pick < 0.34) {
        crop = season_field(CROP_WHEAT, 0, sh);
    } else if (c_pick < 0.67) {
        crop = season_field(mix(CROP_GREEN, CROP_WHEAT, sh * 0.4), 1, sh);
    } else {
        crop = season_field(CROP_FALLOW, 3, sh);
    }
    crop = crop * (0.8 + 0.4 * fp_hash(vec2<f32>(strip, fh * 97.0)));
    let balk_d = (0.5 - abs(fract(t) - 0.5)) * strip_w;
    let balk = (1.0 - smoothstep(0.6, 0.6 + aa, balk_d)) * (1.0 - smoothstep(1.5, 6.0, px_m));
    let headland = (1.0 - smoothstep(5.0, 5.0 + aa * 1.5, fv.border)) * (0.6 + 0.4 * vnoise(q * 0.08));
    let openfield = mix(crop, FP_HEADLAND, max(balk * 0.45, headland * 0.45));
    var furrow = dir;
    furrow_on = (1.0 - headland) * select(0.3, 1.0, c_pick < 0.67);
    // Bocage: irregular enclosures and tree hedges.
    var bocage_col = openfield;
    if (bocage > 0.01) {
        let warp = vec2<f32>(vnoise(p * 6.0), vnoise(p * 6.0 + 4.0)) * 55.0
            + vec2<f32>(vnoise(p * 25.0), vnoise(p * 25.0 + 2.0)) * 14.0;
        let bv = fp_voronoi(q + warp, blk, 38.0, 0.95, mpp);
        let bh = fp_hash(bv.id + 11.3);
        var field: vec3<f32>;
        if (bh < 0.55) {
            field = season_field(MEADOW_COLOR * (0.85 + 0.3 * bh), 4, bh);
        } else if (bh < 0.8) {
            field = season_field(CROP_WHEAT, 0, bh);
        } else {
            field = season_field(CROP_PLOUGHED, 2, bh);
        }
        let crowns = vnoise(q * 0.18) * 0.6 + vnoise(q * 0.55) * 0.4;
        let hw = 4.0 + 5.0 * crowns;
        hedge = (1.0 - smoothstep(hw - aa, hw + aa, bv.border)) * step(0.1, fp_hash(bv.id + 5.5));
        let core = 1.0 - smoothstep(0.0, hw, bv.border);
        bocage_col = mix(field, FP_HEDGE * (0.6 + 0.7 * crowns) * mix(1.15, 0.7, core), hedge);
    }
    var pc = mix(openfield, bocage_col, bocage);
    furrow_on = furrow_on * (1.0 - bocage);
    // Vines on the south slopes (rows along the contour lines), terraces on steep ones.
    if (vine_w > 0.01) {
        let contour = select(dir, vec2<f32>(-g.y, g.x) / max(g_len, 1e-6), g_len > 1e-4);
        let r = abs(fract(row) - 0.5);
        let fine = 1.0 - smoothstep(0.25, 0.6, rw);
        let leaf = mix(0.45, 1.0 - smoothstep(0.2 - rw, 0.2 + rw, r), fine);
        var vine = mix(season_field(VINE_SOIL, 6, fh), season_vine_leaf(VINE_LEAF), leaf);
        let terrace = smoothstep(0.2, 0.35, slope)
            * (1.0 - smoothstep(0.08, 0.08 + aa / 10.0, abs(fract(dot(q, downhill) / 10.0) - 0.5) - 0.42));
        vine = mix(vine, FP_TERRACE, terrace * 0.7);
        pc = mix(pc, vine, vine_w);
        furrow = contour;
        furrow_on = furrow_on * (1.0 - vine_w);
    }
    // Assarts by the woods and waste far from the villages.
    if (assart > 0.01) {
        let stumps = smoothstep(0.78, 0.82, vnoise(q * 0.4)) * (1.0 - smoothstep(0.3, 1.0, px_m));
        let cleared = mix(FP_ASSART * (0.85 + 0.3 * vnoise(q * 0.05)), FP_HEDGE, stumps);
        pc = mix(pc, cleared, assart * 0.7);
    }
    if (waste > 0.01) {
        let bush = smoothstep(0.7, 0.76, vnoise(q * 0.12) * 0.7 + vnoise(q * 0.5) * 0.3);
        let rough = mix(season_field(FP_WASTE, 4, fh), FP_HEDGE * 1.3, bush * 0.8);
        pc = mix(pc, rough, waste * 0.75);
    }
    // High-frequency detail (tufts, clods), close up only.
    if (px_m < 3.0) {
        let hf = vnoise(q * 0.9) * 0.5 + vnoise(q * 3.1) * 0.3 + vnoise(q * 9.7) * 0.2;
        pc = pc * (0.9 + 0.2 * mix(0.5, hf, 1.0 - smoothstep(0.4, 3.0, px_m)));
    }
    let w = vis * clamp(open_w + assart * forest_w, 0.0, 1.0);
    out.color = mix(col, pc, w * 0.92);
    // Furrows across the strips, hedges in relief (map-frame normal perturbation, z flipped as in
    // the fragment: n.z - dn.y).
    if (furrow_on > 0.01) {
        let kk = dot(q, vec2<f32>(-furrow.y, furrow.x)) / 0.9;
        let fade = 1.0 - smoothstep(0.2, 0.8, px_m);
        let bump = vec2<f32>(-furrow.y, furrow.x) * cos(kk * 6.2832) * 0.12 * fade * furrow_on * w;
        out.dn = out.dn + vec2<f32>(bump.x, -bump.y);
    }
    if (hedge > 0.001) {
        let hb = vec2<f32>(vnoise(q * 0.5) - 0.5, vnoise(q * 0.5 + 9.0) - 0.5) * hedge * 0.6 * w;
        out.dn = out.dn + vec2<f32>(hb.x, -hb.y);
    }
    return out;
}

// --- Painted fields (hb_ground.gdshaderinc: hb_apply, farmed parcels and the far patchwork) ---
fn hb_hash(q: vec2<f32>) -> f32 {
    return fract(sin(dot(q, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn hb_rot(a: f32) -> mat2x2<f32> {
    return mat2x2<f32>(vec2<f32>(cos(a), sin(a)), vec2<f32>(-sin(a), cos(a)));
}

struct Vor {
    id: vec2<f32>,
    site: vec2<f32>,
    edge: f32,
};

// Additively weighted Voronoi cells in parcel units: id, site, approximate distance to the edge.
fn hb_voronoi(u: vec2<f32>) -> Vor {
    let ic = floor(u);
    var d1 = 1e9;
    var d2 = 1e9;
    var id1 = ic;
    var s1 = ic;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let id = ic + vec2<f32>(f32(i), f32(j));
            let st = id + 0.1 + 0.8 * vec2<f32>(hb_hash(id + 3.1), hb_hash(id + 17.7));
            let d = length(u - st) - (hb_hash(id + 41.3) - 0.5) * 0.7;
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id1 = id;
                s1 = st;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return Vor(id1, s1, (d2 - d1) * 0.5);
}

// --- VIS-R3b typed parcel layout (mirror and tests: parcel.rs) -----------------------------
fn pc_hash(x: f32, y: f32) -> f32 {
    var h = bitcast<u32>(x) ^ (bitcast<u32>(y) * 0x9E3779B1u);
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    h = h ^ (h >> 16u);
    return f32(h >> 8u) / 16777216.0;
}

// Deformed strip grid: rows of height 1, each shifted (`brick`) and cut in cells of its own
// length (`len_var`); `strip` is the stretch already applied to u.x, so the edge distance along
// the strip is scaled back to the units of the rows.
fn pc_cell(u: vec2<f32>, strip: f32, brick: f32, len_var: f32) -> Vor {
    let row = floor(u.y);
    let off = pc_hash(row, 91.0) * brick * 7.0;
    let mult = 1.0 / (1.0 + len_var * (pc_hash(row, 13.0) - 0.5));
    let xs = (u.x + off) * mult;
    let col = floor(xs);
    let fx = xs - col;
    let fy = u.y - row;
    let across = min(fy, 1.0 - fy);
    let along = min(fx, 1.0 - fx) / mult * strip;
    return Vor(vec2<f32>(col, row), vec2<f32>((col + 0.5) / mult - off, row + 0.5), min(across, along));
}

// Crop draw of a cell pulled to the centre of the sole of its ring around the finage (r: terroir R).
fn pc_ring_h(form: u32, r: f32, h: f32, h_dom: f32) -> f32 {
    let b = fields.forms_b[form];
    let d = fields.forms_d[form];
    let e = fields.forms_e[form];
    if (b.z <= 0.0 || r < d.z) {
        return h;
    }
    var centre = e.z;
    if (r > d.x) {
        centre = e.x;
    } else if (r > d.y) {
        centre = e.y;
    }
    let zoned = clamp(centre + (h_dom - 0.5) * 0.2 + (h - 0.5) * d.w, 0.0, 1.0);
    return h + (zoned - h) * b.z;
}

// Class of a parcel in a season: the crop's own, or a draw by the shares of the form.
fn pc_class(form: u32, s: u32, base: i32, h: f32, hr: f32) -> i32 {
    if (base > 5 || hr >= fields.forms_b[form].w) {
        return base;
    }
    let a = fields.shares[(form * 4u + s) * 2u];
    let b = fields.shares[(form * 4u + s) * 2u + 1u];
    if (h < a.x) {
        return 0;
    }
    if (h < a.y) {
        return 1;
    }
    if (h < a.z) {
        return 2;
    }
    if (h < a.w) {
        return 3;
    }
    if (h < b.x) {
        return 4;
    }
    return 5;
}

fn pc_class_summer(cls: i32) -> vec3<f32> {
    if (cls == 0) {
        return CROP_WHEAT;
    } else if (cls == 1) {
        return CROP_GREEN;
    } else if (cls == 2) {
        return CROP_PLOUGHED;
    } else if (cls == 3) {
        return CROP_FALLOW;
    } else if (cls == 4) {
        return MEADOW_COLOR;
    }
    return HAY_COLOR;
}

// Season colour of a parcel whose class is drawn by season from the shares of its form
// (no filter: each season has its own classes, the colours are those of `season_field_w`).
fn season_field_form(summer: vec3<f32>, cls: i32, h: f32, hq: f32, hr: f32, form: u32) -> vec3<f32> {
    let w = season.weights;
    if (fields.forms_b[form].w <= 0.0 || cls > 5) {
        return season_field_w(summer, cls, h, w);
    }
    var acc = vec3<f32>(0.0);
    var used = 0.0;
    for (var s = 0u; s < 4u; s = s + 1u) {
        let ws = w[s];
        if (ws > 0.001) {
            let c = pc_class(form, s, cls, hq, hr);
            var sm = summer;
            if (c != cls) {
                sm = pc_class_summer(c);
            }
            let one = vec4<f32>(f32(s == 0u), f32(s == 1u), f32(s == 2u), f32(s == 3u));
            acc = acc + ws * season_field_w(sm, c, h, one);
            used = used + ws;
        }
    }
    return acc / max(used, 1e-3);
}

// Parcel frame (region rotation, stretch, cell size): world position of a cell site.
struct PcFrame {
    rot_t: mat2x2<f32>,
    strip: f32,
    cell_px: f32,
    origin: vec2<f32>,
    rid37: vec2<f32>,
    form: u32,
};

fn pc_site_world(site_uu: vec2<f32>, fr: PcFrame) -> vec2<f32> {
    return fr.rot_t * (vec2<f32>((site_uu.x - fr.rid37.x) * fr.strip, site_uu.y - fr.rid37.y) * fr.cell_px) + fr.origin;
}

// Finage mask (terroir R) at a cell site, -1 for the forms without rings (no texture read).
fn pc_ring_r(site_uu: vec2<f32>, fr: PcFrame) -> f32 {
    if (fields.forms_b[fr.form].z <= 0.0) {
        return -1.0;
    }
    return terroir_at(pc_site_world(site_uu, fr)).r;
}

fn pc_cell_of(u: vec2<f32>, form: u32, strip: f32) -> Vor {
    if (fields.pc0.y > 0.5) {
        return pc_cell(u, strip, fields.forms_b[form].x, fields.forms_b[form].y);
    }
    return hb_voronoi(u);
}

// Biome A, landscape mask value and river distance (px, signed) at a map uv.
struct MapCell {
    biome: u32,
    agri: u32,
    river_px: f32,
};

fn map_cell(uv: vec2<f32>) -> MapCell {
    let dims = vec2<f32>(textureDimensions(ab_dist));
    let t = textureLoad(ab_dist, vec2<i32>(clamp(uv * dims, vec2<f32>(0.0), dims - 1.0)), 0);
    let packed = u32(t.r * 255.0 + 0.5);
    let river = (textureSampleLevel(ab_dist, terroir_sampler, uv, 0.0).a * 255.0 - 128.0) / 16.0;
    return MapCell(clamp(packed % 16u, 1u, 14u), u32(t.b * 255.0 + 0.5), river);
}

fn field_row(cell: MapCell) -> u32 {
    if (cell.agri > 0u) {
        return min(cell.agri + 7u, 23u);
    }
    return cell.biome;
}

fn hb_slot(row: u32, h: f32) -> i32 {
    for (var i = 0u; i < 12u; i = i + 1u) {
        let slot = fields.slots[row * 12u + i];
        if (slot.w > 1.5) {
            return -1;
        }
        if (h <= slot.w) {
            return i32(i);
        }
    }
    return -1;
}

// Crop of a cell: 70 % follow the dominant crop of their region; -2 meadow, -3 vine, -1 none.
fn hb_maille(id: vec2<f32>, row: u32, h_dom: f32, vine_k: f32, meadow_k: f32, ring_r: f32, form: u32) -> i32 {
    if (fields.meadow.w > 0.5 && hb_hash(id + 23.3) < meadow_k) {
        return -2;
    }
    if (fields.vine.w > 0.5 && hb_hash(id + 19.1) < vine_k) {
        return -3;
    }
    var h = hb_hash(id + 27.7);
    if (hb_hash(id + 5.9) < 0.7) {
        h = clamp(h_dom + (h - 0.5) * 0.5, 0.0, 1.0);
    }
    if (ring_r >= 0.0) {
        h = pc_ring_h(form, ring_r, h, h_dom);
    }
    return hb_slot(row, h);
}

fn hb_maille_far(m: i32, row: u32) -> vec4<f32> {
    if (m >= 0) {
        let i = row * 12u + u32(m);
        return vec4<f32>(fields.slots[i].rgb, fields.slot_tex[i].z);
    }
    if (m == -3) {
        return vec4<f32>(fields.vine.rgb, fields.tex_flags.x);
    }
    return vec4<f32>(fields.meadow.rgb, fields.tex_flags.y);
}

// Far colour of a mesh in its season (`hb_patch_color`): luminance jitter and saturation.
fn hb_patch_color(id: vec2<f32>, far: vec4<f32>, form: u32) -> vec3<f32> {
    let lumv = 1.0 + 2.0 * fields.v1.y * (hb_hash(id + 3.3) - 0.5);
    let c = season_field_form(far.rgb, i32(far.a + 0.5), hb_hash(id + 61.3), pc_hash(id.x, id.y + 211.0),
        pc_hash(id.x + 5.0, id.y + 97.0), form) * lumv;
    return max(mix(vec3<f32>(luma(c)), c, fields.v1.w), vec3<f32>(0.0));
}

// The luminance of `c` stays within `lum_band` (relative) of the local ground colour.
fn hb_relative(base: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let lb = max(luma(base), 1e-3);
    let lc = max(luma(c), 1e-3);
    return c * (clamp(lc, lb * (1.0 - fields.v5.y), lb * (1.0 + fields.v5.y)) / lc);
}

// VIS-L1: the patchwork of the far parcels, drawn at the finest of three cell sizes whose strips
// span a few screen pixels (`view.strip_px`): a strip narrower than the pixel is not point-sampled
// any more (the "combed" ground), it hands over to the cells 1 / z and 1 / z^2 larger, and last to
// the colour map (weight `a` of the result < 1). `view.strip_px` 0 keeps the former two-level fade
// by footprint. Returns the colour (rgb) and the weight of the patchwork over the ground (a).
fn hb_far_patch(uu: vec2<f32>, footprint: f32, px_u: f32, row: u32, h_dom: f32, vine_k: f32, meadow_k: f32,
        fr: PcFrame) -> vec4<f32> {
    let sole_scale = fields.v2.y;
    let form = fr.form;
    var c = vec3<f32>(0.0);
    var e = 0.0;
    if (fields.pc0.w > 0.0) {
        let lo = fields.pc0.z;
        let hi = max(fields.pc0.w, lo + 0.01);
        let z = clamp(fields.v3.z, 0.1, 0.9);
        // Short side of a cell of each level, in screen pixels.
        let w_a = 1.0 / max(px_u * sole_scale, 1e-6);
        var rest = 1.0;
        var res = 0.0;
        let t_a = smoothstep(lo, hi, w_a);
        if (t_a > 0.001) {
            let v = pc_cell_of(uu * sole_scale, form, fr.strip);
            let ring_r = pc_ring_r(v.site / max(sole_scale, 1e-3), fr);
            c = hb_patch_color(v.id, hb_maille_far(hb_maille(v.id, row, h_dom, vine_k, meadow_k, ring_r, form), row), form) * t_a;
            e = v.edge / max(sole_scale, 1e-3) * t_a;
            res = t_a;
            rest = 1.0 - t_a;
        }
        let t_b = smoothstep(lo, hi, w_a / z) * rest;
        if (t_b > 0.001) {
            let v = pc_cell_of(uu * sole_scale * z + 53.0, form, fr.strip);
            let id = v.id + 0.5;
            c = c + hb_patch_color(id, hb_maille_far(hb_maille(id, row, h_dom, vine_k, meadow_k, -1.0, form), row), form) * t_b;
            e = e + v.edge / max(sole_scale * z, 1e-3) * t_b;
            res = res + t_b;
            rest = rest - t_b;
        }
        let t_c = smoothstep(lo, hi, w_a / (z * z)) * rest;
        if (t_c > 0.001) {
            let v = pc_cell_of(uu * sole_scale * z * z + 97.0, form, fr.strip);
            let id = v.id + 0.25;
            c = c + hb_patch_color(id, hb_maille_far(hb_maille(id, row, h_dom, vine_k, meadow_k, -1.0, form), row), form) * t_c;
            e = e + v.edge / max(sole_scale * z * z, 1e-3) * t_c;
            res = res + t_c;
        }
        let norm = 1.0 / max(res, 1e-3);
        let line = 1.0 - smoothstep(0.0, 1.2 * px_u, e * norm);
        return vec4<f32>(c * norm * (1.0 - fields.v4.y * line), res);
    }
    let k = smoothstep(fields.v3.w, fields.v4.x, footprint);
    if (k < 0.999) {
        let v = pc_cell_of(uu * sole_scale, form, fr.strip);
        let ring_r = pc_ring_r(v.site / max(sole_scale, 1e-3), fr);
        c = hb_patch_color(v.id, hb_maille_far(hb_maille(v.id, row, h_dom, vine_k, meadow_k, ring_r, form), row), form) * (1.0 - k);
        e = v.edge / max(sole_scale, 1e-3) * (1.0 - k);
    }
    if (k > 0.001) {
        let v = pc_cell_of(uu * sole_scale * fields.v3.z + 53.0, form, fr.strip);
        let id = v.id + 0.5;
        c = c + hb_patch_color(id, hb_maille_far(hb_maille(id, row, h_dom, vine_k, meadow_k, -1.0, form), row), form) * k;
        e = e + v.edge / max(sole_scale * fields.v3.z, 1e-3) * k;
    }
    let line = 1.0 - smoothstep(0.0, 1.2 * px_u, e);
    return vec4<f32>(c * (1.0 - fields.v4.y * line), 1.0);
}

// One parcels material (`hb_sample`): layer, tile (m), rotated by `ang` around the map origin.
fn hb_sample(layer: f32, tile_m: f32, p: vec2<f32>, dpx: vec2<f32>, dpy: vec2<f32>, ang: f32) -> vec3<f32> {
    let r = hb_rot(ang);
    let k = max(ground.micro.w, 1.0) / max(tile_m, 1.0);
    return textureSampleGrad(parcels_albedo, ground_sampler, r * p * k, i32(layer + 0.5), r * dpx * k, r * dpy * k).rgb;
}

// Normal of the same material, back in the map frame (x, z).
fn hb_sample_normal(layer: f32, tile_m: f32, p: vec2<f32>, dpx: vec2<f32>, dpy: vec2<f32>, ang: f32) -> vec2<f32> {
    let r = hb_rot(ang);
    let k = max(ground.micro.w, 1.0) / max(tile_m, 1.0);
    let t = textureSampleGrad(parcels_normal, ground_sampler, r * p * k, i32(layer + 0.5), r * dpx * k, r * dpy * k).rg * 2.0 - 1.0;
    return transpose(r) * t;
}

fn tex_mean(layer: f32) -> vec3<f32> {
    return fields.tex_means[min(u32(max(layer, 0.0) + 0.5), 47u)].rgb;
}
