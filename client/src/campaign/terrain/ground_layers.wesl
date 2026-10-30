// Campaign terrain module `ground_layers` (VIS-L1, split of terrain.wesl): textured soil layers, terroir mask and the canopy field.
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::noise::{luma, vnoise, vnoised};
import cent_ans_client::campaign::terrain::terrain_params::{bg_albedo, bg_normal, ground, ground_sampler, params, splat, terroir_sampler, terroir_tex};

// Forest canopy carried by the terrain (VT3 of terrain.gdshader): four octaves of crowns (finest
// period 0.05 map units, then x4), each faded out when its period drops under 6 px.
// Returns (centred relief in [-1, 1], dimensionless slope).
fn canopy_field(p: vec2<f32>, footprint: f32) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    var period = 0.05;
    var amp = 1.0;
    var wsum = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let w = 1.0 - smoothstep(period / 12.0, period / 6.0, footprint);
        if (w > 0.0) {
            let nd = vnoised(p / period + vec2<f32>(f32(i) * 17.3, f32(i) * 5.1));
            acc = acc + w * amp * vec3<f32>(nd.x * 2.0 - 1.0, nd.y * 2.0, nd.z * 2.0);
        }
        wsum = wsum + amp;
        period = period * 4.0;
        amp = amp * 0.8;
    }
    return acc / max(wsum, 1e-3);
}

fn bg_layer(role: u32, biome: u32) -> u32 {
    let i = biome * 7u + role;
    return ground.layers[i / 4u][i % 4u];
}

struct LayerAcc {
    albedo: vec3<f32>,
    normal: vec2<f32>,
};

// One layer at two scales (anti-repetition): tinted albedo + normal, weighted by w. `detail` is the
// albedo detail strength and `fade` the normal one: when a term is 0 its taps would not contribute,
// so they are skipped (explicit gradients: legal in non-uniform control flow).
fn sample_layer(layer: u32, uv_a: vec2<f32>, uv_b: vec2<f32>, da: vec4<f32>, db: vec4<f32>, mix_b: f32,
        tint: vec3<f32>, detail: f32, fade: f32, w: f32, acc: LayerAcc) -> LayerAcc {
    let li = i32(layer);
    var out = acc;
    if (detail > 0.0) {
        let ta = textureSampleGrad(bg_albedo, ground_sampler, uv_a, li, da.xy, da.zw).rgb;
        let tb = textureSampleGrad(bg_albedo, ground_sampler, uv_b, li, db.xy, db.zw).rgb;
        let t = mix(ta, tb, mix_b);
        var ratio = t / max(ground.means[layer].rgb, vec3<f32>(0.01));
        // Detail mostly in luminance: no coloured reflections of the photos.
        ratio = mix(vec3<f32>(luma(ratio)), ratio, 0.35);
        let hue = mix(vec3<f32>(1.0), ground.hues[layer].rgb, ground.bg.w * clamp(detail, 0.0, 1.0));
        out.albedo = out.albedo + w * tint * hue * mix(vec3<f32>(1.0), ratio, detail);
    } else {
        out.albedo = out.albedo + w * tint;
    }
    if (fade > 0.0) {
        let na = textureSampleGrad(bg_normal, ground_sampler, uv_a, li, da.xy, da.zw).rg;
        let nb = textureSampleGrad(bg_normal, ground_sampler, uv_b, li, db.xy, db.zw).rg;
        let nm = mix(na, nb, mix_b);
        out.normal = out.normal + w * (nm * 2.0 - 1.0);
    }
    return out;
}

// One role of the background, blended between the pixel biome (a) and its neighbour (b).
fn sample_role(role: u32, ba: u32, bb: u32, wb: f32, uv_a: vec2<f32>, uv_b: vec2<f32>, da: vec4<f32>,
        db: vec4<f32>, mix_b: f32, tint: vec3<f32>, detail: f32, fade: f32, w: f32, acc: LayerAcc) -> LayerAcc {
    let la = bg_layer(role, ba);
    let lb = bg_layer(role, bb);
    if (la == lb || wb <= ground.bg.z) {
        return sample_layer(la, uv_a, uv_b, da, db, mix_b, tint, detail, fade, w, acc);
    }
    let first = sample_layer(la, uv_a, uv_b, da, db, mix_b, tint, detail, fade, w * (1.0 - wb), acc);
    return sample_layer(lb, uv_a, uv_b, da, db, mix_b, tint, detail, fade, w * wb, first);
}

// hb_clear_floor of hb_ground.gdshaderinc (DN-SOL clearings).
const HB_CLEAR_FLOOR = 0.3;

// Terroir mask (terroir.rs): R fields, G vine, B burnt, A pasture; square frame of the map.
fn terroir_at(p: vec2<f32>) -> vec4<f32> {
    if (ground.splat.w < 0.5) {
        return vec4<f32>(0.0);
    }
    return textureSampleLevel(terroir_tex, terroir_sampler, p / max(params.map.x, params.map.y), 0.0);
}

// Finage: meadows and waste near the settlements turn to fields, pastures grow in a ring.
fn terroir_splat(s_in: vec4<f32>, tr: vec4<f32>) -> vec4<f32> {
    var s = s_in;
    let to_farm = s.r * tr.r * 0.75 + s.a * tr.r * 0.4;
    s.r = s.r - s.r * tr.r * 0.75;
    s.a = s.a - s.a * tr.r * 0.4;
    s.g = s.g + to_farm;
    let to_grass = s.g * tr.a * 0.35 * (1.0 - tr.r);
    s.g = s.g - to_grass;
    s.r = s.r + to_grass;
    return s;
}

// Vines around the settlements of the wine regions (land use: R vine).
fn terroir_landuse(lu: vec3<f32>, tr: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(max(lu.r, tr.g * 1.6), lu.g, lu.b);
}

// DN-SOL clearings: the crops of the splat stay on the finages and the flat valley floors; the
// rest becomes meadow (75 %) and heath. `agri`: landscape mask value (0 outside).
fn hb_clearing_splat(s: vec4<f32>, terroir: vec4<f32>, p: vec2<f32>, slope: f32, agri: u32) -> vec4<f32> {
    let valley = 1.0 - smoothstep(0.03, 0.12, slope);
    let patch_n = smoothstep(0.38, 0.62, vnoise(p * 0.07 + vec2<f32>(11.0)));
    let clear = clamp(terroir.r * 1.25 + 0.4 * valley * patch_n, 0.0, 1.0);
    let floor_keep = select(HB_CLEAR_FLOOR, 0.55, agri > 0u);
    let keep = mix(floor_keep, 1.0, clear);
    let lost = s.g * (1.0 - keep);
    return vec4<f32>(s.r + lost * 0.75, s.g - lost, s.b, s.a + lost * 0.25);
}

fn region_value(p: vec2<f32>) -> f32 {
    return sin(0.0131 * p.x + 1.3 * sin(0.0093 * p.y)) + sin(0.0117 * p.y + 1.1 * sin(0.0171 * p.x)) - 0.25;
}
