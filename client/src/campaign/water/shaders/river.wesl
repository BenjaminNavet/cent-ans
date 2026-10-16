// River ribbons and lake sheets. Port of river_water.gdshader (carved bed, fords, fine-zone fade). Mesh: centre line, NORMAL = signed perpendicular,
// UV0 = (world width, side +-1), UV1 = (distance downstream, importance 0-6). Lakes: `sheet` = 1.
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    shallow_color: vec4<f32>,
    deep_color: vec4<f32>,
    sky_color: vec4<f32>,
    foam_color: vec4<f32>,
    bank_ink_color: vec4<f32>,
    importance_px_lo: vec4<f32>,
    importance_px_hi: vec4<f32>,
    detail: vec4<f32>,
    detail_mean: vec4<f32>,
    silt_color: vec4<f32>,
    fine_zone: vec4<f32>,
    fords: array<vec4<f32>, 32>,
    map_size: vec2<f32>,
    distance_fade: vec2<f32>,
    ford_count: f32,
    min_px: f32,
    thin_fade: f32,
    far_alpha: f32,
    bank_ink_strength: f32,
    bank_ink_px: f32,
    edge_soft_px: f32,
    sky_reflect: f32,
    lift_base: f32,
    lift_per_distance: f32,
    flow_speed: f32,
    ripple_scale: f32,
    ripple_strength: f32,
    sheet: f32,
    sheet_alpha: f32,
    gen_width_scale: f32,
    gen_width_ref: f32,
    has_coast_dist: f32,
    coast_dist_scale: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var coast_dist: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var coast_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var relief_floor: texture_2d<f32>;
// RC5 detail (water_detail.gdshaderinc): normal, shared repeating sampler, albedo (alpha = foam).
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var detail_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var detail_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var detail_albedo: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv2: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) side: f32,
    @location(1) alpha: f32,
    @location(2) flow: vec2<f32>,
    @location(3) world: vec3<f32>,
    @location(4) far: f32,
    @location(5) carved: f32,
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

// river_bed.gdshaderinc `fine_zone_mask`: 1 inside the fine hydrography disc, soft edge at 85-100 %.
fn fine_zone_mask(p: vec2<f32>) -> f32 {
    let z = params.fine_zone;
    if z.z <= 0.0 || z.w <= 0.0 {
        return 0.0;
    }
    return z.w * (1.0 - smoothstep(z.z * 0.85, z.z, distance(p, z.xy)));
}

fn importance_px_scale(importance: f32) -> f32 {
    let i = clamp(i32(importance + 0.5), 0, 6);
    if i < 4 {
        return params.importance_px_lo[i];
    }
    return params.importance_px_hi[i - 4];
}

fn gen_width(w: f32) -> f32 {
    if params.gen_width_scale > 0.0 {
        return max(w, params.gen_width_scale * sqrt(max(w, 0.0) * params.gen_width_ref));
    }
    return w;
}

// Carved bed (river_bed.gdshaderinc, river_water.gdshader). The Godot terrain carves the fine relief
// tiles listed in `carved_rects`; the Bevy terrain carves the close ground by camera distance
// (`river_carve` of terrain.wesl, same two radii), so the water follows the same weight: it sinks
// `WATER_SINK` under the bank and the ribbon overlaps the carved slope by `CARVED_OVERLAP`.
const WATER_SINK: f32 = 0.022;
const CARVED_OVERLAP: f32 = 1.35;
const CARVE_NEAR: vec2<f32> = vec2<f32>(220.0, 380.0);

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    // position.y is the altitude in metres: drape it on the exaggerated relief.
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let view_pos = (view.view_from_world * world).xyz;
    let dist = max(length(view_pos), 0.01);
    var out: VertexOutput;
    if params.sheet > 0.5 {
        world.y = world.y + params.lift_base + params.lift_per_distance * dist;
        out.alpha = 1.0;
        out.far = 1.0;
        out.side = 0.0;
        out.carved = 0.0;
        out.flow = world.xz * vec2<f32>(0.35, 0.5);
    } else {
        let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
        let gw = gen_width(in.uv.x);
        let true_px = gw / world_per_px;
        let imp_px = params.min_px * importance_px_scale(in.uv2.y);
        let carved = 1.0 - smoothstep(CARVE_NEAR.x, CARVE_NEAR.y, distance(world.xyz, view.world_position));
        let half_w = 0.5 * max(gw * mix(1.0, CARVED_OVERLAP, carved), imp_px * world_per_px);
        let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
        world = vec4<f32>(world.xyz + dir * half_w, 1.0);
        world.y = world.y + mix(params.lift_base + params.lift_per_distance * dist, -WATER_SINK, carved);
        out.carved = carved;
        out.alpha = mix(1.0, clamp(true_px / imp_px, 0.25, 1.0), params.thin_fade);
        out.far = 1.0;
        if params.distance_fade.y > 0.0 {
            out.far = mix(1.0, params.far_alpha, smoothstep(params.distance_fade.x, params.distance_fade.y, dist));
        }
        out.side = in.uv.y * half_w / max(0.5 * gw, 1e-4);
        out.flow = vec2<f32>(in.uv2.x, in.uv.y * 0.5 * gw);
    }
    out.clip = view.clip_from_world * world;
    out.world = world.xyz;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sheet = params.sheet > 0.5;
    let edge = abs(in.side);
    var deep = 1.0 - smoothstep(0.35, 1.05, edge);
    var lake_coast = -8.0;
    if sheet {
        deep = 0.75;
        if params.has_coast_dist > 0.5 {
            let uv = in.world.xz / params.map_size;
            lake_coast = (textureSampleLevel(coast_dist, coast_sampler, uv, 0.0).r * 255.0 - 128.0) / params.coast_dist_scale;
            deep = mix(0.12, 0.75, smoothstep(0.0, -7.0, lake_coast));
        }
    }
    let t = globals.time * params.flow_speed * params.ripple_scale * select(1.0, 0.25, sheet);
    let q = vec2<f32>(in.flow.x * 0.8, in.flow.y * 2.2) * params.ripple_scale;
    let n1 = vnoise(q + vec2<f32>(-t, 0.0));
    let n2 = vnoise(q * 1.9 + vec2<f32>(-t * 1.6, 3.7));
    let nx = vnoise(q + vec2<f32>(-t + 0.07, 0.0)) - n1;
    let ny = vnoise(q + vec2<f32>(-t, 0.07)) - n1;

    // Fords and ferries: clear shallow water, riffles and stones (only close up, like Godot).
    var ford = 0.0;
    var riffle = 0.0;
    var fords_here = 0;
    if !sheet && fwidth(in.flow.x) < 0.5 {
        fords_here = i32(params.ford_count);
    }
    for (var i = 0; i < 32; i = i + 1) {
        if i >= fords_here {
            break;
        }
        let f = params.fords[i];
        let d = distance(in.world.xz, f.xy);
        ford = max(ford, (1.0 - smoothstep(f.z * 0.4, f.z, d)) * f.w);
    }
    if ford > 0.0 {
        let stones = smoothstep(0.62, 0.7, vnoise(in.world.xz * 14.0)) * ford;
        riffle = smoothstep(0.55, 0.85, vnoise(vec2<f32>(in.flow.x * 9.0 - globals.time * 1.5, in.flow.y * 30.0))) * ford;
        deep = min(deep, 1.0 - 0.8 * ford);
        riffle = max(riffle, stones * 0.6);
    }

    var col = mix(params.shallow_color.rgb, params.deep_color.rgb, deep);
    col = col * (0.92 + 0.16 * n2);
    if sheet {
        let cur = vnoise(in.world.xz * 0.045) * 0.6 + vnoise(in.world.xz * 0.13 + 7.0) * 0.4;
        col = col * (0.8 + 0.4 * cur);
        col = mix(col, col * vec3<f32>(0.9, 1.08, 1.0), smoothstep(0.35, 0.7, vnoise(in.world.xz * 0.02 + 3.0)));
    }
    // RC5: detail coordinates follow the bed (downstream, across), scaled per unit.
    let d_on = select(0.0, 1.0, params.detail.x > 0.0);
    let d_strength = params.detail.x * d_on;
    let rc_uv = in.flow * params.detail.y;
    let ratio = textureSample(detail_albedo, detail_sampler, rc_uv).rgb / params.detail_mean.rgb;
    col = col * mix(vec3<f32>(1.0), clamp(ratio, vec3<f32>(0.5), vec3<f32>(1.6)), d_strength);
    // DN: silt covers the bed (denser in deep water), never total.
    col = mix(col, params.silt_color.rgb * (0.9 + 0.2 * n2), params.detail.w * mix(0.55, 1.0, deep));
    // DN: foam streaks along the bank, stretched along the current (albedo alpha).
    let streak = textureSample(detail_albedo, detail_sampler, rc_uv * vec2<f32>(0.6, 1.4) + vec2<f32>(-globals.time * params.detail.z * 1.5, 0.0)).a * d_on;
    var bank_foam = smoothstep(0.82, 1.0, edge) * (0.5 + 0.5 * n2) * (1.0 - in.carved) * 0.25;
    bank_foam = bank_foam + smoothstep(0.6, 1.0, edge) * streak * 0.35 * (1.0 - in.carved * 0.5);
    let foam = clamp(bank_foam + riffle, 0.0, 1.0);
    col = mix(col, params.foam_color.rgb, foam * 0.7);

    let half_px = select(1.0 / max(fwidth(in.side), 1e-4), 1000.0, sheet);
    let px_edge = max(1.0 - edge, 0.0) * half_px;
    let ink = (1.0 - smoothstep(0.0, params.bank_ink_px, px_edge)) * smoothstep(1.2, 3.0, half_px) * params.bank_ink_strength * (1.0 - in.carved);

    let fine_ripples = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));
    var n = normalize(vec3<f32>(-nx * params.ripple_strength * 10.0 * fine_ripples, 1.0, -ny * params.ripple_strength * 10.0 * fine_ripples));
    // RC5: detail normal drifting with the current, faded when the ripples get sub-pixel.
    let warp = vec2<f32>(sin(in.flow.y * 0.071 + 1.3), cos(in.flow.x * 0.057 + 0.4)) * 0.9;
    let flow_drift = vec2<f32>(-params.detail.z, 0.0) * globals.time;
    let duv1 = mat2x2<f32>(vec2<f32>(0.92, 0.39), vec2<f32>(-0.39, 0.92)) * in.flow * params.detail.y + warp;
    let duv2 = mat2x2<f32>(vec2<f32>(0.6, -0.8), vec2<f32>(0.8, 0.6)) * in.flow * params.detail.y * 0.37 + vec2<f32>(0.31, 0.57) - warp * 0.5;
    let dn1 = textureSample(detail_normal, detail_sampler, duv1 + flow_drift).xyz * 2.0 - 1.0;
    let dn2 = textureSample(detail_normal, detail_sampler, duv2 - flow_drift * 0.6).xyz * 2.0 - 1.0;
    let detail_raw = vec3<f32>(dn1.x + dn2.x, 0.0, -(dn1.y + dn2.y)) * 0.5 * d_strength;
    let detail_n = normalize(vec3<f32>(detail_raw.x, 1.0, detail_raw.z)) - vec3<f32>(0.0, 1.0, 0.0);
    n = normalize(n + detail_n * fine_ripples);
    let to_eye = normalize(view.world_position - in.world);
    let fresnel = pow(1.0 - clamp(dot(n, to_eye), 0.0, 1.0), 4.0);
    var sheet_sky = 0.0;
    if sheet {
        sheet_sky = 0.04 + 0.26 * vnoise(in.world.xz * 0.06 + globals.time * 0.02);
    }
    col = mix(col, params.sky_color.rgb, clamp(fresnel * params.sky_reflect + 0.12 + sheet_sky, 0.0, 0.85) * (1.0 - foam));
    col = mix(col, params.bank_ink_color.rgb, ink);

    var alpha = 0.9 * in.alpha * in.far;
    if !sheet {
        // The water ends on the carved slope by itself (depth test): no soft border there.
        let soft = 1.0 - (1.0 - smoothstep(0.0, params.edge_soft_px, px_edge)) * (1.0 - in.carved);
        alpha = mix(0.9, 0.97, in.carved) * in.alpha * in.far * soft;
        alpha = alpha * (1.0 - ford * 0.35) * (1.0 - fine_zone_mask(in.world.xz));
    } else {
        alpha = params.sheet_alpha;
        if params.has_coast_dist > 0.5 {
            let shore_noise = vnoise(in.world.xz * 3.1) * 0.5 + vnoise(in.world.xz * 7.7) * 0.5;
            alpha = alpha * (1.0 - smoothstep(-1.6, 0.3 + 0.5 * (shore_noise - 0.5), lake_coast));
        }
    }
    return vec4<f32>(col, clamp(alpha, 0.0, 1.0));
}
