// Draped roads at close range (road_fine.gdshader, lot ZG5b, ADR 0036), meshed by water/fine/job.rs.
// Vertices on the axis: position.y = road height in METRES (resident pages or tile `z`, the higher:
// embankments, decks), draped by relief.wesl; normal = signed perpendicular; UV0 = (real width in
// world units, side +-1); UV1 = (distance, width m); COLOR = (main, causeway, paved, computed).
// Real width (3 to 6 m), never under `min_px` screen pixels: rutted dirt tracks, lighter and wider
// main roads, cobbles near towns, raised causeway (dark banks) through marshes.
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    fine_zone: vec4<f32>,
    dirt_color: vec4<f32>,
    main_color: vec4<f32>,
    rut_color: vec4<f32>,
    paved_color: vec4<f32>,
    joint_color: vec4<f32>,
    grass_edge: vec4<f32>,
    bank_color: vec4<f32>,
    fine_weight: f32,
    min_px: f32,
    main_min_px: f32,
    depth_bias: f32,
    meters_per_unit: f32,
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
    @location(2) road: vec2<f32>,
    @location(3) kind: vec4<f32>,
    @location(4) xz: vec2<f32>,
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = (world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0)).xyz;
    let to_cam = view.world_position - world;
    let dist = max(length(to_cam), 0.01);
    let world_per_px = dist * 2.0 / abs(view.clip_from_view[1][1] * view.viewport.w);
    let floor_px = mix(params.min_px, params.main_min_px, in.color.r);
    // Causeway: banks on both sides (+60 % width).
    let real_half = 0.5 * in.uv.x * (1.0 + 0.6 * in.color.g);
    let half_w = max(real_half, floor_px * world_per_px * 0.5);
    let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
    world = world + dir * half_w;
    world = world + to_cam / dist * min(params.depth_bias * dist, 0.4);
    var out: VertexOutput;
    out.alpha = mix(0.55, 1.0, clamp(in.uv.x / world_per_px / floor_px, 0.0, 1.0));
    out.side = in.uv.y;
    out.road = vec2<f32>(in.uv2.x * params.meters_per_unit, in.uv.y * half_w * params.meters_per_unit);
    out.kind = in.color;
    out.xz = world.xz;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let z = params.fine_zone;
    var zone = 0.0;
    if z.z > 0.0 {
        zone = params.fine_weight * (1.0 - smoothstep(z.z * 0.85, z.z, distance(in.xz, z.xy)));
    }
    if zone <= 0.001 {
        discard;
    }
    let side = abs(in.side);
    let n = noise(vec2<f32>(in.road.x * 0.35, in.road.y * 0.8));
    // Raised causeway: central running band, dark banks on the edges.
    let causeway_bank = in.kind.g * smoothstep(0.55, 0.7, side);
    let s = mix(side, side / 0.62, in.kind.g);
    let ruts = smoothstep(0.1, 0.0, abs(s - 0.42)) * 0.6 * (1.0 - in.kind.b);
    let base = mix(params.dirt_color.rgb, params.main_color.rgb, in.kind.r);
    var col = mix(base, params.rut_color.rgb, ruts);
    // Cobbles (towns): joints of a ~0.6 m sett, faded when they get sub-pixel.
    let cell = in.road / 0.6;
    var joints = max(smoothstep(0.42, 0.5, abs(fract(cell.x + 0.5 * floor(cell.y)) - 0.5)), smoothstep(0.42, 0.5, abs(fract(cell.y) - 0.5)));
    joints = joints * (1.0 - smoothstep(0.3, 0.8, fwidth(cell.x)));
    let paved = mix(params.paved_color.rgb * (0.9 + 0.2 * hash(floor(cell))), params.joint_color.rgb, joints);
    col = mix(col, paved, in.kind.b);
    col = mix(col, params.grass_edge.rgb, smoothstep(0.6, 1.0, s + (n - 0.5) * 0.3) * (1.0 - in.kind.b * 0.8));
    col = mix(col, params.bank_color.rgb, causeway_bank);
    col = col * (0.9 + 0.2 * n);
    let edge = 1.0 - smoothstep(0.75, 1.0, side + (n - 0.5) * 0.2);
    if params.debug_flat > 0.5 {
        return vec4<f32>(1.0, 0.9, 0.0, 1.0);
    }
    return vec4<f32>(col, edge * in.alpha * zone);
}
