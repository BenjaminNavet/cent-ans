// Campaign sea plane at y = 0. Simplified port of water.gdshader: depth colour, swell normals,
// coast foam from the coast-distance raster, per-basin tint/swell/foam from a weight texture.
// Depth comes from the coast distance (no heightmap here: the terrain lot owns it).
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    shallow_color: vec4<f32>,
    mid_color: vec4<f32>,
    deep_color: vec4<f32>,
    foam_color: vec4<f32>,
    edge_haze_color: vec4<f32>,
    season_tint: vec4<f32>,
    basin_tint: array<vec4<f32>, 5>,
    basin_wave: array<vec4<f32>, 5>,
    basin_water: array<vec4<f32>, 5>,
    map_size: vec2<f32>,
    wave_scale: f32,
    wave_speed: f32,
    wave_strength: f32,
    foam_px: f32,
    coast_dist_scale: f32,
    depth_per_px_m: f32,
    max_depth_m: f32,
    depth_falloff_m: f32,
    mid_depth_m: f32,
    alpha_shallow: f32,
    alpha_deep: f32,
    edge_haze_px: f32,
    lake_pond_px: f32,
    basin_on: f32,
    sun_spec_power: f32,
    sun_spec_intensity: f32,
    life_a: vec4<f32>,
    shallows_a: vec4<f32>,
    shallows_b: vec4<f32>,
    crest_a: vec4<f32>,
    crest_b: vec4<f32>,
    crest_dir: vec4<f32>,
    tide_a: vec4<f32>,
    tide_mud: vec4<f32>,
    tide_wet: vec4<f32>,
    detail_sea: vec4<f32>,
    detail_sea_mean: vec4<f32>,
    detail_ocean: vec4<f32>,
    detail_ocean_mean: vec4<f32>,
    detail_depth: vec4<f32>,
    basin_detail: array<vec4<f32>, 5>,
    basin_detail_mean: array<vec4<f32>, 5>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var coast_dist: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var coast_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var basin_weights: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var basin_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var tide_zones: texture_2d<f32>;
// RC5 / TX detail textures (water_detail.gdshaderinc, water.gdshader): one repeating mip-mapped
// sampler for all of them.
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var detail_sea_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var detail_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) var detail_sea_albedo: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) var detail_ocean_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) var detail_ocean_albedo: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(11) var basin_detail_normal: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(12) var basin_detail_albedo: texture_2d_array<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world = get_world_from_local(in.instance_index) * vec4<f32>(in.position, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.world = world.xyz;
    return out;
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash12(i + vec2<f32>(0.0, 1.0)), hash12(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn swell(p: vec2<f32>, t: f32) -> f32 {
    var h = 0.0;
    let wp = vec2<f32>(value_noise(p * 0.21 + 3.0), value_noise(p * 0.17 + 11.0)) * 3.0;
    h = h + sin(dot(p, vec2<f32>(0.82, 0.57)) * 1.0 + t * 1.1 + wp.x) * 0.5;
    h = h + sin(dot(p, vec2<f32>(-0.35, 0.94)) * 1.7 - t * 1.4 + wp.y) * 0.3;
    h = h + sin(dot(p, vec2<f32>(0.99, -0.12)) * 2.9 + t * 2.1 + wp.x - wp.y) * 0.15;
    h = h + (value_noise(p * 1.3 + t * 0.3) - 0.5) * 0.6;
    return h;
}

// ME1 (sea_life.gdshaderinc). Saturation capped at `sat_max`, value kept.
fn limit_saturation(c: vec3<f32>, sat_max: f32) -> vec3<f32> {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let sat = (mx - mn) / max(mx, 1e-5);
    let k = min(1.0, sat_max / max(sat, 1e-5));
    return mix(vec3<f32>(mx), c, k);
}

// Light turquoise shallows near the coasts with animated caustics.
fn sea_shallows(col: vec3<f32>, p: vec2<f32>, depth: f32, footprint: f32) -> vec3<f32> {
    let amount = params.shallows_b.x;
    if amount <= 0.0 {
        return col;
    }
    let t = globals.time;
    let depth_m = params.shallows_a.w;
    let g = (1.0 - smoothstep(depth_m * 0.1, depth_m, depth)) * (1.0 - 0.5 * smoothstep(1.0, 4.0, footprint));
    let q = p * params.shallows_b.z;
    let ridge = 1.0 - abs(value_noise(q + vec2<f32>(t * 0.11, t * 0.07)) * 2.0 - 1.0);
    let ridge2 = 1.0 - abs(value_noise(q * 1.7 - vec2<f32>(t * 0.09, -t * 0.13) + 9.0) * 2.0 - 1.0);
    let caustic = pow(ridge * ridge2, 4.0) * (1.0 - smoothstep(0.5, 2.0, footprint));
    let base = mix(col, params.shallows_a.rgb * (0.55 + 0.45 * value_noise(p * 0.07)), g * amount);
    return base + vec3<f32>(0.6, 0.85, 0.8) * caustic * g * params.shallows_b.y;
}

// Offshore crest lines (0..1), off on shallow water and when zoomed out.
fn sea_crests(p: vec2<f32>, depth: f32, footprint: f32) -> f32 {
    let amount = params.crest_a.w;
    if amount <= 0.0 {
        return 0.0;
    }
    let deep = smoothstep(params.crest_b.x, params.crest_b.y, depth);
    let fade = 1.0 - smoothstep(0.8, 3.5, footprint);
    let phase = dot(p, normalize(params.crest_dir.xy)) * params.crest_b.z - globals.time * params.crest_b.w + value_noise(p * 0.3) * 6.0;
    let line = smoothstep(0.8, 1.0, sin(phase) * 0.5 + 0.5);
    let patch_mask = smoothstep(0.45, 0.8, value_noise(p * 0.55 + vec2<f32>(globals.time * 0.02, 0.0)));
    return line * patch_mask * deep * fade * amount;
}

// Tidal flats: x = exposed mud [0, 1], y = bright wet edge. `off` = distance to the coast (px).
fn sea_tide_flat(uv: vec2<f32>, p: vec2<f32>, off: f32) -> vec2<f32> {
    let zone = textureSampleLevel(tide_zones, basin_sampler, uv, 0.0).r;
    if zone <= 0.004 || off <= 0.0 {
        return vec2<f32>(0.0);
    }
    let tide = 0.5 + 0.5 * sin(globals.time * 6.2832 / params.tide_a.x);
    var exposed = zone * params.tide_a.y * (1.0 - tide * params.tide_a.z);
    exposed = exposed * (0.7 + 0.6 * value_noise(p * 0.55 + 3.0));
    let mud = 1.0 - smoothstep(exposed - 0.35, exposed, off);
    let edge = smoothstep(exposed - 0.9, exposed - 0.2, off) * (1.0 - smoothstep(exposed - 0.2, exposed + 0.1, off));
    return vec2<f32>(mud, edge);
}

const ROT_A: mat2x2<f32> = mat2x2<f32>(vec2<f32>(0.92, 0.39), vec2<f32>(-0.39, 0.92));
const ROT_B: mat2x2<f32> = mat2x2<f32>(vec2<f32>(0.6, -0.8), vec2<f32>(0.8, 0.6));
const ROT_OCEAN_A: mat2x2<f32> = mat2x2<f32>(vec2<f32>(0.87, 0.5), vec2<f32>(-0.5, 0.87));
const ROT_OCEAN_B: mat2x2<f32> = mat2x2<f32>(vec2<f32>(0.5, -0.87), vec2<f32>(0.87, 0.5));

// Slow sine warp that breaks the visible repetition of the detail tile.
fn detail_warp(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(sin(p.y * 0.071 + 1.3), cos(p.x * 0.057 + 0.4)) * 0.9;
}

// Tangent-ish offset (x, 0, z) to the world normal change (`water_detail_normal_ws` - up).
fn detail_ws(v: vec3<f32>) -> vec3<f32> {
    return normalize(vec3<f32>(v.x, 1.0, v.z)) - vec3<f32>(0.0, 1.0, 0.0);
}

// Slot (0 = default sea, 1-4 = basins) with the strongest weight (`basin_dominant`).
fn basin_dominant(w: vec4<f32>, w0: f32) -> i32 {
    var best = w0;
    var slot = 0;
    if w.x > best { best = w.x; slot = 1; }
    if w.y > best { best = w.y; slot = 2; }
    if w.z > best { best = w.z; slot = 3; }
    if w.w > best { slot = 4; }
    return slot;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world.xz;
    let uv = p / params.map_size;
    let footprint = max(length(dpdx(p)), length(dpdy(p)));
    let far_t = smoothstep(0.6, 2.5, footprint);
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));

    // Basin look: weights per basin (RGBA = basins 1-4), rest = default sea (slot 0).
    var w = vec4<f32>(0.0);
    if params.basin_on > 0.5 && inside {
        w = textureSampleLevel(basin_weights, basin_sampler, uv, 0.0);
    }
    let w0 = clamp(1.0 - (w.x + w.y + w.z + w.w), 0.0, 1.0);
    let wsum = max(w0 + w.x + w.y + w.z + w.w, 1e-4);
    let tint = (params.basin_tint[0].rgb * w0 + params.basin_tint[1].rgb * w.x + params.basin_tint[2].rgb * w.y + params.basin_tint[3].rgb * w.z + params.basin_tint[4].rgb * w.w) / wsum;
    let wave = (params.basin_wave[0] * w0 + params.basin_wave[1] * w.x + params.basin_wave[2] * w.y + params.basin_wave[3] * w.z + params.basin_wave[4] * w.w) / wsum;
    let water = (params.basin_water[0] * w0 + params.basin_water[1] * w.x + params.basin_water[2] * w.y + params.basin_water[3] * w.z + params.basin_water[4] * w.w) / wsum;

    // Signed distance to the coast in map pixels (negative at sea).
    var coast = -50.0;
    if inside {
        coast = (textureSampleLevel(coast_dist, coast_sampler, uv, 0.0).r * 255.0 - 128.0) / params.coast_dist_scale;
    }
    let off = max(-coast, 0.0);
    let depth = min(off * params.depth_per_px_m, params.max_depth_m);
    let clarity = max(water.x, 0.1);
    let seen_depth = select(depth + (1.0 / clarity - 1.0) * 20.0, depth / clarity, clarity >= 1.0);
    let mid = smoothstep(0.0, params.mid_depth_m, seen_depth);
    let abyss = smoothstep(params.mid_depth_m, params.depth_falloff_m, seen_depth);
    var col = mix(mix(params.shallow_color.rgb, params.mid_color.rgb, mid), params.deep_color.rgb, abyss);
    var alpha = mix(params.alpha_shallow, params.alpha_deep, smoothstep(0.0, 45.0, seen_depth));

    let t = globals.time * params.wave_speed;
    let q = p * params.wave_scale;
    let h0 = swell(q, t);
    let e = 0.15;
    let hx = swell(q + vec2<f32>(e, 0.0), t) - h0;
    let hz = swell(q + vec2<f32>(0.0, e), t) - h0;
    let ws = params.wave_strength * wave.x * (1.0 - smoothstep(0.15, 0.8, footprint));
    var n = normalize(vec3<f32>(-hx / e * ws, 1.0, -hz / e * ws));
    // RC5: coastal then open-ocean detail (normal added, colour tinted by the albedo relative to its
    // mean), per-basin generated surface when the slot has one; faded out when a tile gets small.
    let rc_ocean = smoothstep(params.detail_depth.x, params.detail_depth.y, depth);
    let warp = detail_warp(p);
    let slot = basin_dominant(w, w0);
    let bp = params.basin_detail[slot];
    let basin_on = select(0.0, 1.0, params.detail_depth.z > 0.5 && bp.x >= 0.0);
    let lay = i32(max(bp.x, 0.0));
    let sea_scale_here = mix(params.detail_sea.y, bp.z, basin_on);
    // Basin surface.
    let buv1 = ROT_A * p * bp.z + warp;
    let buv2 = ROT_B * p * bp.z * 0.37 + vec2<f32>(0.31, 0.57) - warp * 0.5;
    let bn1 = textureSample(basin_detail_normal, detail_sampler, buv1, lay).xyz * 2.0 - 1.0;
    let bn2 = textureSample(basin_detail_normal, detail_sampler, buv2, lay).xyz * 2.0 - 1.0;
    let basin_n = detail_ws(vec3<f32>(bn1.x + bn2.x, 0.0, -(bn1.y + bn2.y)) * 0.5 * bp.y);
    let basin_ratio = textureSample(basin_detail_albedo, detail_sampler, buv1, lay).rgb / params.basin_detail_mean[slot].rgb;
    let basin_col = col * mix(vec3<f32>(1.0), clamp(basin_ratio, vec3<f32>(0.5), vec3<f32>(1.6)), bp.y);
    // Coastal sea surface.
    let s_on = select(0.0, 1.0, params.detail_sea.x > 0.0);
    let suv1 = ROT_A * p * params.detail_sea.y + warp;
    let suv2 = ROT_B * p * params.detail_sea.y * 0.37 + vec2<f32>(0.31, 0.57) - warp * 0.5;
    let sn1 = textureSample(detail_sea_normal, detail_sampler, suv1).xyz * 2.0 - 1.0;
    let sn2 = textureSample(detail_sea_normal, detail_sampler, suv2).xyz * 2.0 - 1.0;
    let sea_n = detail_ws(vec3<f32>(sn1.x + sn2.x, 0.0, -(sn1.y + sn2.y)) * 0.5 * params.detail_sea.x * s_on);
    let sea_ratio = textureSample(detail_sea_albedo, detail_sampler, suv1).rgb / params.detail_sea_mean.rgb;
    let sea_col = col * mix(vec3<f32>(1.0), clamp(sea_ratio, vec3<f32>(0.5), vec3<f32>(1.6)), params.detail_sea.x * s_on);
    // Open ocean surface (long swell, two crossed scales, slow drift).
    let o_on = select(0.0, 1.0, params.detail_ocean.x > 0.0);
    let osc = params.detail_ocean.y;
    let ou = ROT_OCEAN_A * p * osc + (vec2<f32>(value_noise(p * 0.035), value_noise(p * 0.035 + 23.0)) - 0.5) * 2.5;
    let ou2 = ROT_OCEAN_B * p * osc * 0.43 + vec2<f32>(0.17, 0.63) - vec2<f32>(globals.time * 0.003, 0.0);
    let on = textureSample(detail_ocean_normal, detail_sampler, ou + vec2<f32>(globals.time * 0.004, 0.0)).xyz
        + textureSample(detail_ocean_normal, detail_sampler, ou2).xyz - 1.0;
    let ocean_n = vec3<f32>(on.x, 0.0, -on.y) * 0.6 * params.detail_ocean.x * o_on * (0.4 + 0.9 * value_noise(p * 0.025 + 11.0));
    let ocean_ratio = clamp(textureSample(detail_ocean_albedo, detail_sampler, ou).rgb / params.detail_ocean_mean.rgb, vec3<f32>(0.5), vec3<f32>(1.6));
    let ocean_col = col * mix(vec3<f32>(1.0), ocean_ratio, params.detail_ocean.x * o_on);
    let rc_near = mix(
        smoothstep(5.0, 18.0, 1.0 / max(footprint * sea_scale_here, 1e-4)),
        smoothstep(5.0, 18.0, 1.0 / max(footprint * max(params.detail_ocean.y, 1e-4), 1e-4)),
        rc_ocean,
    );
    let rc_sea_col = mix(sea_col, basin_col, basin_on);
    let rc_sea_n = mix(sea_n, basin_n, basin_on);
    col = mix(col, mix(rc_sea_col, ocean_col, rc_ocean), rc_near);
    n = normalize(n + mix(rc_sea_n, ocean_n, rc_ocean) * rc_near);
    col = col * (1.0 + h0 * 0.06 * wave.x * (1.0 - smoothstep(0.06, 0.4, footprint)));
    col = col * (1.0 + (n.x * 0.6 + n.z * 0.8) * 0.6 * (1.0 - smoothstep(0.1, 0.7, footprint)));
    let tint_seen = mix(sqrt(tint), tint, smoothstep(0.0, 40.0, depth));
    col = mix(col * tint_seen, vec3<f32>(0.4, 0.45, 0.5), clamp(0.0, 0.0, 1.0)) * params.season_tint.rgb;

    let life = params.life_a.x > 0.5;
    if life {
        col = sea_shallows(col, p, depth, footprint);
        col = mix(col, params.crest_a.rgb, sea_crests(p, depth, footprint));
        let cur = value_noise(p * 0.011 + 31.0) * 0.65 + value_noise(p * 0.027 + 7.0) * 0.35;
        col = col * params.life_a.y * (0.82 + 0.4 * cur) * mix(vec3<f32>(0.92, 1.04, 1.12), vec3<f32>(1.0, 1.1, 1.0), smoothstep(0.35, 0.65, cur));
        col = limit_saturation(col, params.life_a.z);
    }

    // Coast foam band (surf lines, broken by noise).
    var foam_w = max(params.foam_px, footprint * 1.2);
    if life && params.life_a.w > 0.0 {
        foam_w = max(params.foam_px, footprint * params.life_a.w);
    }
    let band = 1.0 - smoothstep(0.0, foam_w, off);
    let lines = smoothstep(0.55, 0.95, sin(off * 5.5 + globals.time * 1.6 + value_noise(p * 0.8) * 4.0) * 0.5 + 0.5);
    let broken = smoothstep(0.35, 0.75, value_noise(p * 1.6 + vec2<f32>(globals.time * 0.05, 0.0)));
    var foam = band * mix(0.35 + 0.65 * lines * broken, 0.6, far_t);
    foam = foam * (1.0 - smoothstep(foam_w * 0.2, foam_w, off) * 0.5);
    if life && params.life_a.w > 0.0 {
        foam = foam * (0.8 + 0.2 * sin(globals.time * 1.3 + value_noise(p * 0.15) * 9.0));
    }
    foam = clamp(foam * wave.z, 0.0, 1.6);
    col = mix(col, params.foam_color.rgb, min(foam * 0.55, 0.9));
    alpha = mix(alpha, 0.9, foam * 0.5);
    if life && inside {
        let flat_tide = sea_tide_flat(uv, p, off);
        let mud_var = 0.8 + 0.4 * value_noise(p * 2.3 + 5.0);
        let mud = mix(params.tide_mud.rgb, params.tide_wet.rgb, flat_tide.y * 0.8) * mud_var;
        col = mix(col, mud, flat_tide.x);
        alpha = mix(alpha, params.tide_a.w, flat_tide.x);
    }
    alpha = alpha * (1.0 - smoothstep(-0.2, 0.6, coast));

    // Narrow inland water (ponds): not sea, left to the lakes layer.
    if inside && coast < 0.5 && coast > -params.lake_pond_px {
        alpha = alpha * smoothstep(-params.lake_pond_px, -params.lake_pond_px * 0.5, coast);
    }

    // Fresnel and a soft sun glint.
    let to_eye = normalize(view.world_position - in.world);
    let fres = pow(1.0 - clamp(dot(n, to_eye), 0.0, 1.0), 5.0);
    col = col + vec3<f32>(0.02, 0.035, 0.045) * fres;
    let sun = normalize(vec3<f32>(-0.4, 0.8, -0.35));
    let h = normalize(to_eye + sun);
    let spec = pow(max(dot(n, h), 0.0), mix(params.sun_spec_power, params.sun_spec_power * 0.25, far_t)) * params.sun_spec_intensity * (1.0 - 0.7 * far_t);
    col = col + vec3<f32>(spec) * (1.0 - clamp(foam, 0.0, 1.0));

    // Map-edge haze.
    var edge_mist = 0.0;
    if params.edge_haze_px > 0.0 {
        let to_edge = min(p, params.map_size - p);
        edge_mist = 1.0 - smoothstep(0.0, params.edge_haze_px, min(to_edge.x, to_edge.y));
    }
    let rgb = mix(col, params.edge_haze_color.rgb, edge_mist);
    return vec4<f32>(rgb, mix(alpha, 1.0, edge_mist));
}
