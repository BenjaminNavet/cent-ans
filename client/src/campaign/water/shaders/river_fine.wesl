// Fine rivers at close range (river_fine.gdshader, lot ZG5b, ADR 0036), meshed by water/fine/job.rs.
// Vertices on the axis: position.y = water level in METRES (draped by relief.wesl, no rebuild when
// the exaggeration changes); normal = signed perpendicular; UV0 = (real width in world units,
// side +-1); UV1 = (distance downstream, display rank); COLOR = (divagating, tidal, wetland,
// intermittent). Ribbon widened to at least `min_px` screen pixels (faded when thinner), small ranks
// fading with the distance, bank bands by flag: sand bars (divagating), mud flats uncovered by the
// tide (tidal), reeds (wetland), gravel bed (intermittent).
// Departure from Godot: no depth texture (the bed is not carved in the Bevy terrain); the water
// depth tint comes from the river width instead of the depth-buffer thickness.
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    fine_zone: vec4<f32>,
    order_px_lo: vec4<f32>,
    order_px_hi: vec4<f32>,
    shallow_color: vec4<f32>,
    deep_color: vec4<f32>,
    brackish_color: vec4<f32>,
    sky_color: vec4<f32>,
    foam_color: vec4<f32>,
    sand_color: vec4<f32>,
    gravel_color: vec4<f32>,
    mud_color: vec4<f32>,
    reed_color: vec4<f32>,
    reed_dry_color: vec4<f32>,
    fine_weight: f32,
    min_px: f32,
    thin_fade: f32,
    depth_bias: f32,
    order_distance: f32,
    tidal_band: f32,
    wetland_band: f32,
    divagating_band: f32,
    band_min_m: f32,
    meters_per_unit: f32,
    tide_period: f32,
    tide_amount: f32,
    flow_speed: f32,
    ripple_scale: f32,
    ripple_strength: f32,
    sky_reflect: f32,
    shallow_alpha: f32,
    gen_width_scale: f32,
    gen_width_ref: f32,
    deep_width: f32,
    debug_flat: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv2: vec2<f32>,
    @location(5) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) side: f32,
    @location(1) alpha: f32,
    @location(2) band: f32,
    @location(3) flow: vec2<f32>,
    @location(4) world: vec3<f32>,
    @location(5) flags: vec4<f32>,
    @location(6) width: f32,
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn order_px_scale(order: f32) -> f32 {
    let i = clamp(i32(order + 0.5), 3, 10) - 3;
    if i < 4 {
        return params.order_px_lo[i];
    }
    return params.order_px_hi[i - 4];
}

// GC (ADR 0158): generalised width, max(real, scale * sqrt(real * ref)).
fn gen_width(w: f32) -> f32 {
    if params.gen_width_scale > 0.0 {
        return max(w, params.gen_width_scale * sqrt(max(w, 0.0) * params.gen_width_ref));
    }
    return w;
}

fn zone_mask(p: vec2<f32>) -> f32 {
    let z = params.fine_zone;
    if z.z <= 0.0 {
        return 0.0;
    }
    return z.w * (1.0 - smoothstep(z.z * 0.85, z.z, distance(p, z.xy)));
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = (world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0)).xyz;
    let to_cam = view.world_position - world;
    let dist = max(length(to_cam), 0.01);
    let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
    let order = in.uv2.y;
    let px_min = params.min_px * order_px_scale(order);
    // The screen widening is WATER; the bank bands add their real width beyond it.
    let gw = gen_width(in.uv.x);
    let water_half = max(0.5 * gw, px_min * world_per_px * 0.5);
    let c = in.color;
    let band_m = max(c.g * params.tidal_band, max(c.b * params.wetland_band, c.r * params.divagating_band));
    var band = 0.0;
    if band_m > 0.0 {
        band = max(0.5 * in.uv.x * band_m, params.band_min_m / params.meters_per_unit);
    }
    let half_w = water_half + band;
    let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
    world = world + dir * half_w;
    world = world + to_cam / dist * min(params.depth_bias * dist, 0.5);
    let true_px = gw / world_per_px;
    let reach = params.order_distance * exp2(order - 3.0);
    var out: VertexOutput;
    out.alpha = mix(1.0, clamp(true_px / px_min, 0.25, 1.0), params.thin_fade) * (1.0 - smoothstep(reach * 0.75, reach, dist));
    out.side = in.uv.y * half_w / max(water_half, 1e-5);
    out.band = half_w / max(water_half, 1e-5);
    out.flow = vec2<f32>(in.uv2.x, in.uv.y * half_w);
    out.flags = c;
    out.width = gw;
    out.world = world;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let edge = abs(in.side);
    let zone = zone_mask(in.world.xz) * params.fine_weight;
    if zone <= 0.001 || in.alpha <= 0.001 {
        discard;
    }
    // Depth tint from the width (Godot: depth-buffer thickness of the carved bed).
    let depth_t = smoothstep(0.0, params.deep_width, in.width) * (1.0 - smoothstep(0.3, 1.0, edge));

    let q = vec2<f32>(in.flow.x * 0.8, in.flow.y * 2.2) * params.ripple_scale;
    let t = globals.time * params.flow_speed * params.ripple_scale * 0.012;
    let n1 = vnoise(q + vec2<f32>(-t, 0.0));
    let n2 = vnoise(q * 1.9 + vec2<f32>(-t * 1.6, 3.7));
    let nx = vnoise(q + vec2<f32>(-t + 0.07, 0.0)) - n1;
    let ny = vnoise(q + vec2<f32>(-t, 0.07)) - n1;
    let ripple_fade = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));

    // Water edge (in water half widths) by regime.
    var water_edge = 1.0;
    let tide = 0.5 + 0.5 * sin(globals.time * 6.2832 / params.tide_period + in.world.x * 0.7);
    water_edge = mix(water_edge, 1.0 - params.tide_amount * (1.0 - tide), in.flags.g);
    water_edge = mix(water_edge, 0.45, in.flags.a);
    let coarse = vnoise(vec2<f32>(in.flow.x * 40.0, in.flow.y * 40.0));
    let wobble = (vnoise(vec2<f32>(in.flow.x * 25.0, 1.3)) - 0.5) * 0.25;
    var water = 1.0 - smoothstep(water_edge + wobble - 0.06, water_edge + wobble + 0.02, edge);
    // Sand bars and shoals in the divagating bed (Loire, Rhone, upper Rhine).
    let bars = in.flags.r * smoothstep(0.62, 0.7, vnoise(vec2<f32>(in.flow.x * 9.0, in.flow.y * 14.0)) * 0.7 + coarse * 0.3) * (1.0 - smoothstep(0.55, 0.95, edge));
    water = water * (1.0 - bars);

    var water_col = mix(params.shallow_color.rgb, params.deep_color.rgb, depth_t);
    water_col = mix(water_col, params.brackish_color.rgb, in.flags.g * 0.6);
    water_col = water_col * (0.92 + 0.16 * n2);
    water_col = mix(water_col, params.foam_color.rgb, smoothstep(0.85, 1.0, edge / max(water_edge, 0.1)) * 0.12 * (0.5 + n2));

    // Banks in the band outside the water.
    var bank_col = params.gravel_color.rgb * (0.85 + 0.3 * coarse);
    var bank_alpha = 0.0;
    if edge > water_edge - 0.1 || bars > 0.0 {
        let wet = 1.0 - smoothstep(water_edge, water_edge + 0.35, edge);
        let sand = mix(params.sand_color.rgb, params.gravel_color.rgb, in.flags.a) * (0.85 + 0.3 * coarse);
        let mud = params.mud_color.rgb * (0.8 + 0.35 * coarse) * mix(1.0, 0.7, wet);
        let reeds = smoothstep(0.35, 0.65, vnoise(in.world.xz * 900.0) * 0.6 + vnoise(in.world.xz * 2600.0) * 0.4);
        let reed = mix(params.reed_color.rgb, params.reed_dry_color.rgb, vnoise(in.world.xz * 300.0)) * (0.7 + 0.5 * reeds);
        bank_col = sand;
        bank_col = mix(bank_col, mud, in.flags.g);
        bank_col = mix(bank_col, reed, in.flags.b * (0.6 + 0.4 * reeds));
        bank_col = mix(bank_col, sand * vec3<f32>(1.02, 1.0, 0.95), bars);
        let outer = 1.0 - smoothstep(in.band * 0.8, in.band, edge);
        bank_alpha = max(outer * select(0.0, 0.95, in.band > 1.02), bars);
    }

    var col = mix(bank_col, water_col, water);
    let n = normalize(vec3<f32>(-nx * params.ripple_strength * 10.0 * ripple_fade * water, 1.0, -ny * params.ripple_strength * 10.0 * ripple_fade * water));
    let to_eye = normalize(view.world_position - in.world);
    let fresnel = pow(1.0 - clamp(dot(n, to_eye), 0.0, 1.0), 4.0);
    // Sun-less shading of the Godot spatial shader: a soft lambert term on the ripples.
    col = col * (0.9 + 0.1 * n.y);
    col = mix(col, params.sky_color.rgb, clamp(fresnel * params.sky_reflect + 0.12, 0.0, 0.8) * water);
    let water_alpha = mix(params.shallow_alpha, 0.97, smoothstep(0.0, 0.35, depth_t));
    let alpha = max(water * water_alpha, bank_alpha * (1.0 - water));
    if params.debug_flat > 0.5 {
        return vec4<f32>(mix(vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(1.0, 1.0, 0.0), step(0.98, edge)), 1.0);
    }
    return vec4<f32>(col, clamp(alpha * in.alpha * zone, 0.0, 1.0));
}
