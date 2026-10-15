// River ribbons and lake sheets. Port of river_water.gdshader (without carved bed, fords, detail
// textures: those depend on terrain/fine layers). Mesh: centre line, NORMAL = signed perpendicular,
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
    map_size: vec2<f32>,
    distance_fade: vec2<f32>,
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
        out.flow = world.xz * vec2<f32>(0.35, 0.5);
    } else {
        let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
        let gw = gen_width(in.uv.x);
        let true_px = gw / world_per_px;
        let imp_px = params.min_px * importance_px_scale(in.uv2.y);
        let half_w = 0.5 * max(gw, imp_px * world_per_px);
        let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
        world = vec4<f32>(world.xyz + dir * half_w, 1.0);
        world.y = world.y + params.lift_base + params.lift_per_distance * dist;
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

    var col = mix(params.shallow_color.rgb, params.deep_color.rgb, deep);
    col = col * (0.92 + 0.16 * n2);
    if sheet {
        let cur = vnoise(in.world.xz * 0.045) * 0.6 + vnoise(in.world.xz * 0.13 + 7.0) * 0.4;
        col = col * (0.8 + 0.4 * cur);
        col = mix(col, col * vec3<f32>(0.9, 1.08, 1.0), smoothstep(0.35, 0.7, vnoise(in.world.xz * 0.02 + 3.0)));
    }
    let bank_foam = smoothstep(0.82, 1.0, edge) * (0.5 + 0.5 * n2) * 0.25;
    let foam = clamp(bank_foam, 0.0, 1.0);
    col = mix(col, params.foam_color.rgb, foam * 0.7);

    let half_px = select(1.0 / max(fwidth(in.side), 1e-4), 1000.0, sheet);
    let px_edge = max(1.0 - edge, 0.0) * half_px;
    let ink = (1.0 - smoothstep(0.0, params.bank_ink_px, px_edge)) * smoothstep(1.2, 3.0, half_px) * params.bank_ink_strength;

    let fine_ripples = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));
    let n = normalize(vec3<f32>(-nx * params.ripple_strength * 10.0 * fine_ripples, 1.0, -ny * params.ripple_strength * 10.0 * fine_ripples));
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
        alpha = alpha * (1.0 - (1.0 - smoothstep(0.0, params.edge_soft_px, px_edge)));
    } else {
        alpha = params.sheet_alpha;
        if params.has_coast_dist > 0.5 {
            let shore_noise = vnoise(in.world.xz * 3.1) * 0.5 + vnoise(in.world.xz * 7.7) * 0.5;
            alpha = alpha * (1.0 - smoothstep(-1.6, 0.3 + 0.5 * (shore_noise - 0.5), lake_coast));
        }
    }
    return vec4<f32>(col, clamp(alpha, 0.0, 1.0));
}
