// Battlefield ground: port of game/shaders/battle_ground.gdshader and battle_common.gdshaderinc,
// lit by Bevy's PBR (sun, shadows, fog).
//
// The 13 layers of the biome pack (roles of data/fx/battle_ground_layers.json) are stacked by
// priority (snow, pebbles, mud, dirt, trodden grass, rock, forest, fresh plough, plough, stubble,
// flowering meadow, meadow, grass underneath) from the per-vertex splat, the slope, the R2 relief
// map, the EP6 decor fields, procedural fields outside the calm centre, the weather and the
// trample map. Albedo and tangent-space normals of each layer: two taps at different scales,
// flattened to the layer mean at mid distance; DA6 near detail, PO4 micro grain (micro_ground),
// R2 detail bumps.
//
// Texture reads below flow control use explicit gradients or levels (WGSL uniformity rules).
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::view,
};

struct GroundParams {
    layers: array<vec4<f32>, 13>,
    season: vec4<f32>,
    water: vec4<f32>,
    detail: vec4<f32>,
    rect: vec4<f32>,
    calm: vec4<f32>,
    micro: vec4<f32>,
    micro_layer: array<vec4<f32>, 4>,
    micro_tile: array<vec4<f32>, 4>,
    relief: vec4<f32>,
    relief_rect: vec4<f32>,
    weather: vec4<f32>,
    trample_rect: vec4<f32>,
    far: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> ground: GroundParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var albedo_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var micro_albedo: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var micro_albedo_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var micro_normal: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var micro_normal_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var decor_fields: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var decor_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var relief_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var relief_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var trample_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var trample_sampler: sampler;

const L_GRASS: i32 = 0;
const L_MEADOW: i32 = 1;
const L_FOREST: i32 = 2;
const L_DIRT: i32 = 3;
const L_MUD: i32 = 4;
const L_PEBBLES: i32 = 5;
const L_ROCK: i32 = 6;
const L_PLOUGH: i32 = 7;
const L_SNOW: i32 = 8;
const L_FLOWER: i32 = 9;
const L_TRODDEN: i32 = 10;
const L_STUBBLE: i32 = 11;
const L_FRESH: i32 = 12;
const LAYERS: i32 = 13;

// Noise scales of data/fx/battle_ground.json (cycles per metre).
const FREQ_MACRO: f32 = 0.0021;
const FREQ_MACRO2: f32 = 0.00043;
const FREQ_FINE: f32 = 0.017;
const FREQ_BLEND: f32 = 0.006;
const FREQ_PATCH: f32 = 0.0013;
const FREQ_PATCH2: f32 = 0.0047;
const FREQ_PUDDLE: f32 = 0.021;
const FREQ_MACRO_VAR: f32 = 0.0017;
const FINE_CONTRAST: f32 = 0.6;
const MACRO_TINT: f32 = 0.28;
const MACRO_WET: f32 = 0.3;
const MACRO_ALBEDO: f32 = 0.12;

// --- battle_common.gdshaderinc --------------------------------------------------------------

fn bt_hash21(q: vec2<f32>) -> f32 {
    var p = fract(q * vec2<f32>(123.34, 456.21));
    p = p + dot(p, p + 45.32);
    return fract(p.x * p.y);
}

fn bt_value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = bt_hash21(i);
    let b = bt_hash21(i + vec2<f32>(1.0, 0.0));
    let c = bt_hash21(i + vec2<f32>(0.0, 1.0));
    let d = bt_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Godot's `macro_noise` texture: 512 px seamless simplex fbm, 8 cells per repeat, 4 octaves.
// Read at `uv`, a smooth noise around 0.5.
fn macro_tex(uv: vec2<f32>) -> f32 {
    let q = uv * 8.0;
    let n = bt_value_noise(q) * 0.533 + bt_value_noise(q * 2.03 + 17.1) * 0.267
        + bt_value_noise(q * 4.01 + 41.7) * 0.133 + bt_value_noise(q * 7.97 + 9.3) * 0.067;
    return clamp(0.5 + (n - 0.5) * 1.6, 0.0, 1.0);
}

fn bt_mud_warp(p: vec2<f32>) -> vec2<f32> {
    let big = vec2<f32>(bt_value_noise(p * 0.02 + 3.1), bt_value_noise(p * 0.02 + 17.3)) - 0.5;
    let small = vec2<f32>(bt_value_noise(p * 0.08 + 9.7), bt_value_noise(p * 0.08 + 41.9)) - 0.5;
    return big * 40.0 + small * 10.0;
}

fn bt_edge_warp(p: vec2<f32>) -> vec2<f32> {
    let w = vec2<f32>(bt_value_noise(p * 0.035 + 11.3), bt_value_noise(p * 0.035 + 47.9)) - 0.5;
    let f = vec2<f32>(bt_value_noise(p * 0.21 + 5.1), bt_value_noise(p * 0.21 + 91.7)) - 0.5;
    return w * 10.0 + f * 2.4;
}

// Procedural field at `p0`: x = kind (0 none, 1 plough, 2 corn, 3 fallow), y = along the
// furrows, z = normalised distance to the edge (`bt_field_info_soft`).
fn bt_field_info_soft(p0: vec2<f32>) -> vec3<f32> {
    let p = p0 + bt_edge_warp(p0);
    let ca = cos(0.31);
    let sa = sin(0.31);
    // Godot mat2(vec2(c, s), vec2(-s, c)) * p.
    let q = vec2<f32>(ca * p.x - sa * p.y, sa * p.x + ca * p.y);
    let size = vec2<f32>(130.0, 46.0);
    let cell = floor(q / size);
    let h = bt_hash21(cell + 7.0);
    let local = fract(q / size);
    let edge = min(min(local.x, 1.0 - local.x) * size.x, min(local.y, 1.0 - local.y) * size.y);
    var kind = 0.0;
    if h < 0.22 {
        kind = 1.0;
    } else if h < 0.40 {
        kind = 2.0;
    } else if h < 0.52 {
        kind = 3.0;
    }
    let z = smoothstep(0.0, 9.0, edge);
    let cq = (cell + 0.5) * size;
    // transpose(rot) * cq: whole parcels only.
    let center = vec2<f32>(ca * cq.x + sa * cq.y, -sa * cq.x + ca * cq.y);
    if bt_value_noise(center * 0.0012 + 3.1) < 0.39 {
        kind = 0.0;
    }
    return vec3<f32>(kind, sa * p0.x + ca * p0.y, z);
}

fn bt_decor_field(df: vec4<f32>, p: vec2<f32>) -> vec3<f32> {
    let kind = floor(df.r * 8.0 + 0.5);
    let yaw = df.g * 3.14159265;
    return vec3<f32>(kind, dot(p, vec2<f32>(-sin(yaw), cos(yaw))), df.b);
}

fn bt_decor_edge_soft(z: f32, p: vec2<f32>) -> f32 {
    return clamp(z + (bt_value_noise(p * 0.3 + 3.7) - 0.5) * 0.35, 0.0, 1.0);
}

fn bt_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.3, 0.59, 0.11));
}

// ga_macro.gdshaderinc `ga_macro_variation(h, 0.5, 1.0, 0.0)`: luminance and a slight warm drift.
fn ga_macro_variation(h: f32) -> vec3<f32> {
    let c = (h - 0.5) * 2.0;
    return vec3<f32>(1.0 + c * 0.09, 1.0 + c * 0.07, 1.0 + c * 0.03);
}

// R2 detail height (m): clods and ruts (3 m), bumps (9 m), swells (26 m).
fn detail_height(p: vec2<f32>, rough: f32) -> f32 {
    var h = (macro_tex(p / 26.0 + vec2<f32>(0.13, 0.57)) - 0.5) * 0.55;
    h = h + (macro_tex(p / 9.0 + vec2<f32>(0.71, 0.29)) - 0.5) * 0.22;
    h = h + (macro_tex(p / 3.1 + vec2<f32>(0.43, 0.91)) - 0.5) * mix(0.05, 0.14, rough);
    return h;
}

// --- Layers ---------------------------------------------------------------------------------

struct Grads {
    dx: vec2<f32>,
    dy: vec2<f32>,
};

// Godot mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * p.
fn rot_a(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(0.8 * p.x - 0.6 * p.y, 0.6 * p.x + 0.8 * p.y);
}

// Godot mat2(vec2(0.6, -0.8), vec2(0.8, 0.6)) * p.
fn rot_b(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(0.6 * p.x + 0.8 * p.y, -0.8 * p.x + 0.6 * p.y);
}

fn layer_albedo(k: i32, uv: vec2<f32>, g: Grads, scale: f32) -> vec3<f32> {
    return textureSampleGrad(albedo_array, albedo_sampler, uv, k, g.dx * scale, g.dy * scale).rgb;
}

fn layer_normal_xy(k: i32, uv: vec2<f32>, g: Grads, scale: f32) -> vec2<f32> {
    if ground.detail.z < 0.5 {
        return vec2<f32>(0.0);
    }
    return textureSampleGrad(normal_array, normal_sampler, uv, k, g.dx * scale, g.dy * scale).xy * 2.0 - 1.0;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xz;
    let world_y = in.world_position.y;
    let view_dist = distance(in.world_position.xyz, view.world_position);
    let world_normal = normalize(in.world_normal);
    let g = Grads(dpdx(p), dpdy(p));

    let macro_n = macro_tex(p * FREQ_MACRO);
    let macro2 = macro_tex(p * FREQ_MACRO2 + vec2<f32>(0.3, 0.7));
    let fine = 0.5 + (macro_tex(p * FREQ_FINE) - 0.5) * FINE_CONTRAST;
    let blend = smoothstep(0.35, 0.65, macro_tex(p * FREQ_BLEND + 0.5));
    let patch_n = macro_tex(p * FREQ_PATCH + vec2<f32>(0.61, 0.13));
    let patch2 = macro_tex(p * FREQ_PATCH2 + vec2<f32>(0.27, 0.83));

    // Per-vertex splat (10 m cells): grass, forest, dirt, mud | stone (ford, banks), plough.
    let forest_s = in.color.g;
    let dirt_s = in.color.b;
    let stone_s = in.uv_b.x;
    let land = in.color.r + in.color.g + in.color.b + in.color.a + in.uv_b.x + in.uv_b.y;
    // Mud discs broken into ragged patches (Godot reads the mud channel at `bt_mud_warp`).
    let mud_n = bt_value_noise((p + bt_mud_warp(p)) * 0.05);
    let mud_s = in.color.a * (0.75 + 0.5 * mud_n);

    // EP6 decor fields and clear areas.
    let suv = (p - ground.rect.xy) / ground.rect.zw;
    let inside = all(suv >= vec2<f32>(0.0)) && all(suv <= vec2<f32>(1.0));
    let duv = (p + bt_edge_warp(p) - ground.rect.xy) / ground.rect.zw;
    let df = textureSampleLevel(decor_fields, decor_sampler, duv, 0.0);
    let clear_zone = select(0.0, step(0.25, df.a), inside);

    // Procedural fields outside the calm centre.
    let calm_d = max(ground.calm.xy - p, p - ground.calm.zw);
    let outside_calm = smoothstep(0.0, 60.0, max(calm_d.x, calm_d.y));
    var field = bt_field_info_soft(p);
    var field_w = select(0.0, 1.0, field.x > 0.5) * outside_calm * (1.0 - clear_zone) * field.z;
    var decor_kind = 0.0;
    var furrow_period = 1.3;
    var plough_vertex = in.uv_b.y;
    if ground.weather.z > 0.5 && inside {
        // The decor map replaces the vertex plough weight (same areas, sharper edges).
        plough_vertex = 0.0;
        if df.a > 0.75 {
            field = bt_decor_field(df, p);
            decor_kind = field.x;
            if decor_kind == 4.0 || decor_kind == 5.0 {
                field.x = 1.0;
                furrow_period = select(0.9, 2.2, decor_kind == 5.0);
            } else if decor_kind == 6.0 {
                field.x = 2.0;
            }
            field_w = bt_decor_edge_soft(field.z, p);
        }
    }
    if plough_vertex > 0.0 && field.x != 1.0 {
        field = vec3<f32>(1.0, field.y, 1.0);
        field_w = plough_vertex;
    }
    let furrow_px = fwidth(field.y / furrow_period);

    var w_forest = smoothstep(0.15, 0.6, forest_s + (fine - 0.5) * 0.4);
    var w_dirt = smoothstep(0.2, 0.6, dirt_s + (fine - 0.5) * 0.3);
    var w_mud = smoothstep(0.15, 0.55, mud_s + (macro_n - 0.5) * 0.4);
    var w_pebbles = smoothstep(0.25, 0.7, stone_s + (fine - 0.5) * 0.3);
    let slope = 1.0 - clamp(world_normal.y, 0.0, 1.0);
    var w_rock = smoothstep(0.16, 0.3, slope + (macro_n - 0.5) * 0.12);

    // R2 relief: slope, hollows and crests.
    var relief = vec4<f32>(0.0, 0.5, 0.5, 1.0);
    if ground.relief.z > 0.0 {
        let ruv = (p - ground.relief_rect.xy) / ground.relief_rect.zw;
        let r = textureSampleLevel(relief_map, relief_sampler, ruv, 0.0);
        if all(ruv >= vec2<f32>(0.0)) && all(ruv <= vec2<f32>(1.0)) {
            relief = r;
        }
    }
    let grade = relief.r * 0.5;
    let hollow = clamp((relief.g - 0.5) * 2.0 + (relief.b - 0.5) * 1.2, -1.0, 1.0);
    let crest = smoothstep(0.05, 0.5, -hollow);
    let dip = smoothstep(0.05, 0.5, hollow);
    let outcrops = ground.relief.x;
    let outcrop = outcrops * smoothstep(0.5, 0.95, grade * 1.9 + crest * 0.4 + (fine - 0.5) * 0.5 + (macro_n - 0.5) * 0.35);
    w_rock = max(w_rock, outcrop);
    let thin_soil = smoothstep(0.3, 0.8, -hollow) * smoothstep(0.55, 0.85, fine + macro_n * 0.3) * mix(0.3, 0.8, outcrops);
    w_dirt = max(w_dirt, thin_soil * 0.5);
    w_pebbles = max(w_pebbles, thin_soil * outcrops * 0.5);
    w_rock = w_rock * (1.0 - dip * 0.8);
    // River banks: wet grass rather than bare rock.
    let bank = stone_s;
    w_rock = w_rock * (1.0 - smoothstep(0.1, 0.5, bank) * 0.9);

    let decor_labour = decor_kind == 1.0;
    let w_plough = select(0.0, field_w, field.x == 1.0 && !decor_labour);
    let w_fresh = select(0.0, field_w, decor_labour);
    let w_stubble = select(0.0, field_w, decor_kind == 6.0);
    var w_meadow = smoothstep(0.25, 0.55, macro2 + (fine - 0.5) * 0.2);
    if field.x == 3.0 {
        w_meadow = max(w_meadow, field_w);
    }
    let flower_share = smoothstep(0.45, 0.7, (1.0 - patch_n) * 0.75 + patch2 * 0.35);
    let w_flower = w_meadow * flower_share;
    w_meadow = w_meadow - w_flower;
    // Trodden grass: halo of the roads and of the yards (Godot `splat_b.g`).
    let w_trodden = clamp(clear_zone * 0.5 + dirt_s * 0.8 - w_dirt, 0.0, 1.0) * (1.0 - w_forest) * 0.6;
    let snow = ground.season.a;
    var w_snow = snow * smoothstep(0.35, 0.75, world_normal.y * 0.6 + macro_n * 0.5 + fine * 0.2);
    w_snow = max(w_snow, smoothstep(ground.relief.w, ground.relief.w + 60.0, world_y + (macro_n - 0.5) * 50.0) * smoothstep(0.25, 0.6, world_normal.y));

    // B7 / B8: trodden snow and churned mud, read on the flattened-grass map of `flatten.rs`.
    var trampled = 0.0;
    if ground.weather.y > 0.0 && ground.trample_rect.z > 0.0 {
        let tuv = (p - ground.trample_rect.xy) / ground.trample_rect.zw;
        let tm = textureSampleLevel(trample_map, trample_sampler, tuv, 0.0).r;
        if all(tuv >= vec2<f32>(0.0)) && all(tuv <= vec2<f32>(1.0)) {
            trampled = smoothstep(0.08, 0.85, tm + (fine - 0.5) * 0.35 * tm);
        }
        w_snow = w_snow * (1.0 - smoothstep(0.7, 1.0, trampled) * smoothstep(0.6, 0.9, fine + macro_n * 0.3) * 0.25);
        w_mud = max(w_mud, smoothstep(0.3, 0.9, trampled) * (1.0 - snow) * mix(0.45, 0.8, fine));
    }

    // Priority stack: each layer covers the ones below, grass takes the rest.
    var w: array<f32, 13>;
    for (var i = 0; i < LAYERS; i++) {
        w[i] = 0.0;
    }
    var order = array<i32, 12>(L_SNOW, L_PEBBLES, L_MUD, L_DIRT, L_TRODDEN, L_ROCK, L_FOREST, L_FRESH, L_PLOUGH, L_STUBBLE, L_FLOWER, L_MEADOW);
    var ws = array<f32, 12>(
        w_snow, w_pebbles, w_mud * (1.0 - w_snow * 0.5), w_dirt * (1.0 - w_snow * 0.6),
        w_trodden, w_rock, w_forest, w_fresh, w_plough, w_stubble, w_flower, w_meadow,
    );
    var rest = 1.0;
    for (var i = 0; i < 12; i++) {
        let wi = clamp(ws[i], 0.0, 1.0) * rest;
        w[order[i]] = wi;
        rest = rest - wi;
    }
    w[L_GRASS] = rest;

    // Layers: two taps (scales and orientations), flattened to their mean at mid distance.
    let far_k = smoothstep(ground.far.y, ground.far.z, view_dist) * ground.far.x;
    let g2 = Grads(rot_a(g.dx), rot_a(g.dy));
    var albedo = vec3<f32>(0.0);
    var nxy = vec2<f32>(0.0);
    var mean_mix = vec3<f32>(0.0);
    var dom = 0;
    var wmax = -1.0;
    for (var i = 0; i < LAYERS; i++) {
        let wi = w[i];
        mean_mix = mean_mix + ground.layers[i].rgb * wi;
        if wi > wmax {
            wmax = wi;
            dom = i;
        }
        if wi < 0.004 {
            continue;
        }
        let s = ground.layers[i].a;
        let uv1 = p / s;
        let uv2 = rot_a(p) / (s * 2.3) + vec2<f32>(0.37, 0.71);
        var a = mix(layer_albedo(i, uv1, g, 1.0 / s), layer_albedo(i, uv2, g2, 1.0 / (s * 2.3)), blend);
        var n = mix(layer_normal_xy(i, uv1, g, 1.0 / s), layer_normal_xy(i, uv2, g2, 1.0 / (s * 2.3)), blend);
        let mean = ground.layers[i].rgb;
        a = mix(a, mean, far_k);
        n = n * (1.0 - far_k);
        if i == L_SNOW {
            // The generated snow layer has strong stripes: keep its mean and a little grain.
            a = mix(a, mean, 0.88);
            n = n * 0.2;
        }
        albedo = albedo + a * wi;
        nxy = nxy + n * wi;
    }
    // Beyond the texture fade distance, only the layer means (Bevy look, `texture_fade_m`).
    albedo = mix(albedo, mean_mix, smoothstep(ground.water.a * 0.35, ground.water.a * 1.6, view_dist));

    // Grass: tint, corn, dry / lush / rusty patches, hollows and crests, macro variation.
    let grassy = w[L_GRASS] + w[L_MEADOW] + w[L_FLOWER] + w[L_TRODDEN];
    var tint = ground.season.rgb * mix(vec3<f32>(0.92, 1.0, 0.86), vec3<f32>(1.08, 1.02, 0.86), macro2);
    if field.x == 2.0 {
        tint = mix(tint, select(vec3<f32>(1.45, 1.18, 0.62), vec3<f32>(1.3, 1.1, 0.7), decor_kind == 6.0), field_w);
    }
    let dry = smoothstep(0.42, 0.66, patch_n * 0.7 + patch2 * 0.45);
    let lush = smoothstep(0.45, 0.7, (1.0 - patch_n) * 0.75 + patch2 * 0.35);
    let rust = smoothstep(0.68, 0.8, patch2) * (1.0 - dry);
    tint = mix(tint, tint * vec3<f32>(1.35, 1.12, 0.62), dry * 0.85);
    tint = mix(tint, tint * vec3<f32>(0.7, 0.86, 0.66), lush * 0.8);
    tint = mix(tint, tint * vec3<f32>(1.15, 0.92, 0.72), rust * 0.45);
    let tx_ground = ground.micro.w;
    tint = mix(tint, mix(vec3<f32>(1.0), tint, 0.6), tx_ground);
    tint = mix(tint, tint * vec3<f32>(0.78, 0.92, 0.72), dip * 0.7);
    tint = mix(tint, tint * vec3<f32>(1.14, 1.06, 0.8), crest * 0.55);
    let mv = macro_tex(p * FREQ_MACRO_VAR + vec2<f32>(0.47, 0.19)) * 0.65
        + macro_tex(p * FREQ_MACRO_VAR * 0.43 + vec2<f32>(0.81, 0.33)) * 0.35;
    let mv_c = (mv - 0.5) * 2.0;
    tint = tint * mix(vec3<f32>(1.0), vec3<f32>(1.0 + 0.5 * mv_c, 1.0 + 0.12 * mv_c, 1.0 - 0.45 * mv_c), MACRO_TINT);
    albedo = mix(albedo, albedo * tint, grassy);
    albedo = albedo * (1.0 + mv_c * MACRO_ALBEDO);
    albedo = mix(albedo, albedo * vec3<f32>(0.78, 0.8, 0.74), clamp(-mv_c, 0.0, 1.0) * MACRO_WET);
    // Straw-coloured ground of the corn and stubble fields.
    if field.x == 2.0 {
        let rows = 0.5 + 0.5 * sin(field.y * 6.2832 / 1.1);
        var straw = select(vec3<f32>(0.56, 0.44, 0.2), vec3<f32>(0.5, 0.42, 0.26), decor_kind == 6.0);
        straw = straw * mix(0.86, 1.08, macro_n) * mix(0.9, 1.06, fine) * mix(0.93, 1.03, rows);
        albedo = mix(albedo, straw, field_w * select(0.6, 0.8, decor_kind > 0.0) * (1.0 - w[L_SNOW]));
    }
    albedo = albedo * mix(0.84, 1.1, macro_n) * mix(0.93, 1.05, fine);

    // DA6 near detail: the dominant layer read at ~1/4 of its scale; its luminance modulates.
    let near_k = 1.0 - smoothstep(ground.far.w, ground.far.w * 2.6, view_dist);
    if near_k > 0.01 {
        let s = ground.layers[dom].a * 0.27;
        let uvn = rot_b(p) / s + vec2<f32>(0.19, 0.83);
        let gn = Grads(rot_b(g.dx), rot_b(g.dy));
        let dcol = layer_albedo(dom, uvn, gn, 1.0 / s);
        let dl = bt_luma(dcol) / max(bt_luma(ground.layers[dom].rgb), 0.02);
        albedo = albedo * mix(1.0, clamp(dl, 0.55, 1.5), near_k * 0.5);
        nxy = nxy + layer_normal_xy(dom, uvn, gn, 1.0 / s) * near_k * 0.6;
    }
    // PO4 micro grain on grass, meadows, dirt and mud (not rock nor snow).
    var nd_k = 1.0 - smoothstep(ground.micro.z * 0.45, ground.micro.z, view_dist);
    nd_k = nd_k * clamp(1.0 - w[L_ROCK] - w[L_SNOW] - w[L_PEBBLES] * 0.5, 0.0, 1.0);
    if nd_k > 0.01 && ground.micro.x > 0.5 {
        let ml = i32(ground.micro_layer[dom / 4][dom % 4] + 0.5);
        let mt = max(ground.micro_tile[dom / 4][dom % 4], 0.05);
        let muv = rot_a(p) / mt + vec2<f32>(0.37, 0.11);
        let mdx = rot_a(g.dx) / mt;
        let mdy = rot_a(g.dy) / mt;
        let mcol = textureSampleGrad(micro_albedo, micro_albedo_sampler, muv, ml, mdx, mdy).rgb;
        let mref = textureSampleLevel(micro_albedo, micro_albedo_sampler, muv, ml, 12.0).rgb;
        let mlum = bt_luma(mcol) / max(bt_luma(mref), 0.02);
        albedo = albedo * mix(1.0, clamp(mlum, 0.5, 1.6), nd_k * ground.micro.y);
        let mn = textureSampleGrad(micro_normal, micro_normal_sampler, muv, ml, mdx, mdy).xy * 2.0 - 1.0;
        nxy = nxy + mn * nd_k * 0.8;
    }
    // Furrows, faded where their period goes under a few pixels and at mid distance.
    if w_plough > 0.0 || w_fresh > 0.0 {
        var furrow = mix(0.5, 0.5 + 0.5 * sin(field.y * 6.2832 / furrow_period), 1.0 - smoothstep(0.06, 0.2, furrow_px));
        furrow = mix(furrow, 0.5, smoothstep(25.0, 90.0, view_dist) * 0.9);
        albedo = albedo * mix(1.0, mix(0.72, 1.05, furrow), w[L_PLOUGH]);
        if decor_kind == 4.0 {
            albedo = mix(albedo, albedo * vec3<f32>(0.75, 1.25, 0.55), w[L_PLOUGH] * (0.35 + 0.4 * furrow));
        }
    }
    // B7: drifts stretched by the wind, bluish lee slopes, whiter crests; trodden snow grey.
    let snowy_w = w[L_SNOW];
    if snowy_w > 0.01 {
        let wind_p = vec2<f32>(0.94 * p.x - 0.34 * p.y, 0.34 * p.x + 0.94 * p.y);
        let drift = macro_tex(wind_p * vec2<f32>(0.0014, 0.0026));
        let drift2 = macro_tex(wind_p * vec2<f32>(0.0034, 0.0052) + vec2<f32>(0.17, 0.53));
        let lee = smoothstep(0.02, 0.22, slope);
        let s_crest = smoothstep(0.55, 0.8, drift * 0.7 + drift2 * 0.4);
        var snow_col = vec3<f32>(mix(0.9, 1.04, drift) * mix(0.97, 1.02, drift2));
        snow_col = mix(snow_col, snow_col * vec3<f32>(0.8, 0.87, 1.0), lee * 0.8 + (1.0 - s_crest) * 0.12);
        snow_col = snow_col * mix(0.97, 1.03, fine);
        albedo = mix(albedo, albedo * snow_col * 1.9, snowy_w);
        albedo = mix(albedo, albedo * vec3<f32>(0.8, 0.79, 0.77), trampled * snowy_w);
    }
    let mud_trample = trampled * (1.0 - snowy_w);
    albedo = mix(albedo, albedo * vec3<f32>(0.62, 0.54, 0.46), mud_trample * 0.7);
    // Wet banks and rain-soaked ground; water stays in the hollows.
    let wetness = ground.weather.x;
    var wet = clamp(max(wetness * (0.55 + 0.45 * macro_n), bank * mix(0.4, 0.85, max(w_mud, w_pebbles))), 0.0, 1.0) * (1.0 - w[L_SNOW]);
    wet = clamp(wet + dip * (0.12 + wetness * 0.45) * (1.0 - w[L_SNOW]), 0.0, 1.0);
    albedo = albedo * mix(1.0, 0.62, wet);
    let puddle = smoothstep(0.62, 0.8, macro_tex(p * FREQ_PUDDLE + vec2<f32>(0.53, 0.29)));
    albedo = mix(albedo, vec3<f32>(0.17, 0.14, 0.1) * (0.75 + 0.5 * fine) * mix(1.0, 0.6, puddle), w_mud * mix(0.7, 0.2, tx_ground) * (1.0 - w[L_SNOW]));
    albedo = mix(albedo, albedo * vec3<f32>(0.8, 0.66, 0.48), w_dirt * mix(0.8, 0.0, tx_ground) * (1.0 - w[L_SNOW]));
    let bank_wet = smoothstep(0.3, 0.8, bank) * max(w_mud, w_pebbles) * (1.0 - w[L_SNOW]);
    albedo = mix(albedo, albedo * vec3<f32>(0.66, 0.72, 0.58), bank_wet * 0.6);
    albedo = albedo * (1.0 - dip * 0.08);
    // GA2 macro variation (50 to 200 m).
    let ga2_h = macro_tex(p / 50.0 + vec2<f32>(0.11, 0.37)) * 0.4 + macro_tex(p / 90.0 + vec2<f32>(0.53, 0.02)) * 0.3
        + macro_tex(p / 140.0 + vec2<f32>(0.29, 0.71)) * 0.2 + macro_tex(p / 200.0 + vec2<f32>(0.83, 0.19)) * 0.1;
    albedo = albedo * ga_macro_variation(ga2_h);
    // Large lighter and darker grass patches (Bevy look, `macro_contrast`).
    let patches = bt_value_noise(p / 140.0) * 0.55 + bt_value_noise(p / 23.0) * 0.45;
    albedo = albedo * (1.0 + (patches - 0.5) * ground.detail.w * grassy);
    // Muted decor (DA bible 3.3).
    albedo = mix(vec3<f32>(bt_luma(albedo)), albedo, ground.weather.w);
    // Deep water where no splat is left.
    let water_w = 1.0 - clamp(land * 4.0, 0.0, 1.0);
    albedo = mix(albedo, ground.water.rgb, water_w);
    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);

    // Normals: layer maps (tangent +x, bitangent cross(up, +x)) faded with the distance, plus the
    // R2 detail bumps (stronger on stony ground), faded far away and under snow.
    let strength = ground.detail.x * (1.0 - smoothstep(ground.detail.y * 0.4, ground.detail.y, view_dist));
    let up = normalize(pbr_input.world_normal);
    let tangent = normalize(vec3<f32>(1.0, 0.0, 0.0) - up * up.x);
    let bitangent = cross(up, tangent);
    var n_world = up + (tangent * nxy.x + bitangent * nxy.y) * strength;
    let bump_fade = (1.0 - smoothstep(120.0, 420.0, view_dist)) * ground.relief.y * (1.0 - snowy_w * 0.7);
    if bump_fade > 0.01 {
        let rough = clamp(outcrops + w[L_ROCK] + w[L_PEBBLES], 0.0, 1.0);
        let e = 0.6;
        let h0 = detail_height(p, rough);
        let grad = vec2<f32>(detail_height(p + vec2<f32>(e, 0.0), rough) - h0, detail_height(p + vec2<f32>(0.0, e), rough) - h0) / e;
        n_world = n_world - vec3<f32>(grad.x, 0.0, grad.y) * 2.2 * bump_fade;
    }
    pbr_input.N = normalize(n_world);
    // Churned mud and soaked ground shine (capped far away, where it mirrored the sky).
    let mud_gloss = clamp(wet + mud_trample * 0.7, 0.0, 1.0);
    let gloss_far = mix(0.38, 0.62, smoothstep(80.0, 300.0, view_dist));
    pbr_input.material.perceptual_roughness = mix(0.95, gloss_far, mud_gloss * (0.5 + 0.5 * w[L_MUD] + 0.3 * w[L_DIRT]));

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
