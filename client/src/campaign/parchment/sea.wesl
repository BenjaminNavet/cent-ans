// Portulan sea of the strategic parchment view (port of parchment_sea.gdshaderinc and the sea part
// of parchment_common.gdshaderinc, lot CM2 + FA6): pale blue-green wash, water lines along the
// coasts, rhumb lines radiating from the wind roses (main winds in ink, half winds in green,
// quarter winds in red), painted wind roses of the 1375 Catalan Atlas, ink coastline, double frame
// on the map edges and burnt paper margins outside the map. A plane just above the sea (y = 0),
// blended over it with the parchment weight; land hides it by depth. Colours are linear.
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    roses: array<vec4<f32>, 4>,
    // x, y = map size (px), z = parchment weight, w = rose count.
    map: vec4<f32>,
    // x = coast scale, y = rhumb alpha, z = painted rose (1/0), w = has coast raster (1/0).
    misc: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var coast_dist: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var coast_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var rose_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var rose_sampler: sampler;

const PI = 3.14159265;
const PM_PAPER = vec3<f32>(0.80, 0.67, 0.43);
const PM_PAPER_DARK = vec3<f32>(0.52, 0.36, 0.17);
const PM_INK = vec3<f32>(0.035, 0.020, 0.010);
const PM_RED_INK = vec3<f32>(0.42, 0.045, 0.025);
const PM_GREEN_INK = vec3<f32>(0.07, 0.17, 0.05);
const PM_SEA_WASH = vec3<f32>(0.40, 0.56, 0.54);
// Painted rose: half side of the texture in rose radii; its circle fills 0.9 of the half side.
const PM_ROSE_EXTENT = 2.0;
const PM_ROSE_CIRCLE = 0.9;

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

fn pm_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn pm_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(pm_hash(i), pm_hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(pm_hash(i + vec2<f32>(0.0, 1.0)), pm_hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn pm_fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * pm_noise(p);
        p = p * 2.07 + vec2<f32>(13.7, 5.3);
        a = a * 0.5;
    }
    return v;
}

// Aged paper: stains, fibres, burnt edges (map uv; outside the map = the parchment's margin).
fn pm_paper_at(p: vec2<f32>, uv: vec2<f32>, footprint: f32) -> vec3<f32> {
    let stain = pm_fbm(p * 0.0045 + vec2<f32>(3.1, 1.7));
    let blot = smoothstep(0.62, 0.8, pm_fbm(p * 0.011 + vec2<f32>(7.3, 2.9)));
    let fibre = pm_noise(p * vec2<f32>(0.9, 0.22) / max(footprint, 0.5)) - 0.5;
    var col = PM_PAPER * (0.88 + 0.2 * stain) * (1.0 + 0.035 * fibre);
    col = mix(col, PM_PAPER_DARK, blot * 0.22);
    let e = min(uv, 1.0 - uv);
    let edge = min(e.x, e.y);
    let burn = 1.0 - smoothstep(-0.06, 0.07, edge + (pm_noise(p * 0.01) - 0.5) * 0.03);
    col = mix(col, PM_PAPER_DARK * 0.75, burn * 0.7);
    return col;
}

// Double rule around the map (edges of the heightmap).
fn pm_frame(uv: vec2<f32>, footprint: f32, map_sz: vec2<f32>) -> f32 {
    let e = min(uv, 1.0 - uv) * map_sz;
    let d = min(e.x, e.y);
    let aa = footprint;
    let outer = 1.0 - smoothstep(footprint * 1.4 - aa, footprint * 1.4 + aa, abs(d - 14.0));
    let inner = 1.0 - smoothstep(footprint * 0.6 - aa, footprint * 0.6 + aa, abs(d - 22.0));
    return max(outer, inner * 0.8);
}

// Antialiased ink line: `d` distance in map px, `w` half width in screen px.
fn pm_line(d: f32, w: f32, footprint: f32) -> f32 {
    let ds = abs(d) / max(footprint, 1e-4);
    return 1.0 - smoothstep(w - 0.7, w + 0.7, ds);
}

// Painted rose: (colour, coverage), sampled with explicit gradients (trilinear on its mips).
fn pm_rose_painted(p: vec2<f32>, rose: vec4<f32>, gx: vec2<f32>, gy: vec2<f32>) -> vec4<f32> {
    let half_side = rose.z * PM_ROSE_EXTENT;
    let q = (p - rose.xy) / half_side;
    if (max(abs(q.x), abs(q.y)) >= 1.0) {
        return vec4<f32>(0.0);
    }
    return textureSampleGrad(rose_tex, rose_sampler, q * 0.5 + 0.5, gx * 0.5 / half_side, gy * 0.5 / half_side);
}

// Drawn 16-point wind rose, red north point (fallback without the painted rose).
fn pm_rose(p: vec2<f32>, rose: vec4<f32>, footprint: f32) -> vec4<f32> {
    let v = p - rose.xy;
    let r = length(v);
    let big_r = rose.z;
    if (r > big_r * 1.05) {
        return vec4<f32>(0.0);
    }
    let a = atan2(v.x, -v.y);
    let step16 = PI / 8.0;
    let k = round(a / step16);
    let da = a - k * step16;
    let ki = i32(k + 16.0) % 16;
    let main_w = ki % 4 == 0;
    let half_w = ki % 2 == 0 && !main_w;
    var tip = big_r * 0.45;
    if (main_w) { tip = big_r * 0.92; } else if (half_w) { tip = big_r * 0.66; }
    let valley = big_r * 0.12;
    let edge = mix(tip, valley, abs(da) / step16);
    let star = step(r, edge);
    var dark = PM_RED_INK;
    if (main_w) { dark = PM_INK * 1.2; } else if (half_w) { dark = PM_GREEN_INK * 1.3; }
    let light = PM_PAPER * 1.05;
    var col = select(light, dark, da > 0.0);
    if (ki == 0) {
        col = select(PM_RED_INK * 2.2, PM_RED_INK, da > 0.0);
    }
    let outline = pm_line(r - edge, 0.55, footprint) * step(r, tip + footprint);
    col = mix(col, PM_INK, outline);
    let ring = max(pm_line(r - big_r * 0.98, 0.9, footprint), pm_line(r - big_r * 0.9, 0.5, footprint));
    let hub = step(r, big_r * 0.05);
    let cover = max(max(star, outline), max(ring, hub));
    if (hub > 0.0) { col = PM_RED_INK; }
    if (ring > star) { col = PM_INK; }
    return vec4<f32>(col, cover);
}

// Rhumb lines (32 directions) of a rose: (colour, intensity), from `start` rose radii outwards.
fn pm_rhumbs(p: vec2<f32>, rose: vec4<f32>, footprint: f32, start: f32) -> vec4<f32> {
    let v = p - rose.xy;
    let r = length(v);
    if (r < rose.z * start) {
        return vec4<f32>(0.0);
    }
    let a = atan2(v.x, -v.y);
    let step32 = PI / 16.0;
    let k = round(a / step32);
    let dist = r * abs(sin(a - k * step32));
    let ki = i32(k + 32.0) % 32;
    var col = PM_RED_INK * 1.3;
    var w = 0.45;
    if (ki % 4 == 0) { col = PM_INK; w = 0.6; } else if (ki % 2 == 0) { col = PM_GREEN_INK * 1.6; }
    let fade = 1.0 - smoothstep(rose.z * 9.0, rose.z * 14.0, r);
    return vec4<f32>(col, pm_line(dist, w, footprint) * fade);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let w = params.map.z;
    let p = in.world.xz;
    let map_size = params.map.xy;
    let uv = p / map_size;
    let gx = dpdx(p);
    let gy = dpdy(p);
    let footprint = max(length(gx), length(gy));
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    // Signed distance to the coast (map px, negative at sea); off the map = open sea.
    var coast = -60.0;
    if (inside && params.misc.w > 0.5) {
        coast = (textureSampleLevel(coast_dist, coast_sampler, uv, 0.0).r * 255.0 - 128.0) / params.misc.x;
    }
    var parch = pm_paper_at(p, uv, footprint);
    let off = max(-coast, 0.0);
    // Like Godot, the wash, rhumbs and roses also cover the margin beyond the map (open sea).
    // Wash: deeper along the coasts (pigment pooling at the edge), pale out at sea.
    let wash = mix(0.55, 0.9, exp(-off / 10.0)) * (0.9 + 0.2 * pm_noise(p * 0.05));
    parch = mix(parch, PM_SEA_WASH * (parch / PM_PAPER), wash);
    // Water lines parallel to the coast, farther apart and paler outwards.
    var lines = 0.0;
    for (var i = 1; i <= 4; i = i + 1) {
        let fi = f32(i);
        lines = max(lines, pm_line(off - fi * fi * 1.8 - 1.5, 0.75, footprint) * (1.0 - fi * 0.18));
    }
    parch = parch * mix(vec3<f32>(1.0), vec3<f32>(0.78, 0.9, 0.9), exp(-off / 5.0));
    parch = mix(parch, PM_SEA_WASH * 0.35, lines * 0.75 * step(0.3, off));
    let painted = params.misc.z > 0.5;
    let count = i32(params.map.w);
    var start = 0.9;
    if (painted) { start = PM_ROSE_EXTENT * PM_ROSE_CIRCLE; }
    for (var i = 0; i < count; i = i + 1) {
        let rh = pm_rhumbs(p, params.roses[i], footprint, start);
        parch = mix(parch, rh.rgb, rh.a * params.misc.y);
    }
    for (var i = 0; i < count; i = i + 1) {
        var ro = vec4<f32>(0.0);
        if (painted) {
            ro = pm_rose_painted(p, params.roses[i], gx, gy);
        } else {
            ro = pm_rose(p, params.roses[i], footprint);
        }
        parch = mix(parch, ro.rgb, ro.a);
    }
    parch = mix(parch, PM_INK, pm_line(coast, 1.0, footprint) * 0.9);
    parch = mix(parch, PM_INK, pm_frame(uv, footprint, map_size) * 0.85);
    // Opaque at sea; the coast stays cut by the raster (land hides the plane by depth anyway).
    let sea = 1.0 - smoothstep(-0.2, 0.6, coast);
    return vec4<f32>(parch * 0.95, sea * w);
}
