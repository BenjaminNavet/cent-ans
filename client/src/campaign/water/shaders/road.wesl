// Dirt road ribbons draped on the ground. Port of road.gdshader. UV0 = (distance along, side +-1).
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    dirt_color: vec4<f32>,
    rut_color: vec4<f32>,
    grass_edge: vec4<f32>,
    alpha: f32,
    lift: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz) + params.lift;
    let world = get_world_from_local(in.instance_index) * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.uv = in.uv;
    return out;
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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // ADR 0758: worn track. Width and wear drift slowly along the road, the centre is grassed
    // over where the traffic is light, ruts wander, the edges dissolve into the grass.
    let side = abs(in.uv.y);
    let along = in.uv.x;
    let n = noise(vec2<f32>(along * 3.0, in.uv.y * 2.0));
    let slow = noise(vec2<f32>(along * 0.11, 3.7));
    let wear = noise(vec2<f32>(along * 0.23, 11.3));
    let rut_side = 0.42 + (noise(vec2<f32>(along * 0.4, 5.1)) - 0.5) * 0.14;
    let ruts = smoothstep(0.11, 0.0, abs(side - rut_side)) * (0.35 + 0.5 * wear);
    var col = mix(params.dirt_color.rgb, params.rut_color.rgb, ruts);
    // Light traffic: grass and pebbles creep into the centre line.
    let grassed = smoothstep(0.55, 0.15, wear) * 0.55 * smoothstep(0.0, 0.25, 0.5 - side);
    col = mix(col, params.grass_edge.rgb, grassed);
    col = mix(col, params.grass_edge.rgb, smoothstep(0.5, 1.0, side + (n - 0.5) * 0.35));
    col = col * (0.86 + 0.28 * n);
    let reach = 0.62 + 0.3 * slow;
    let edge = 1.0 - smoothstep(reach - 0.22, reach + 0.3, side + (n - 0.5) * 0.3);
    let thin = mix(0.55, 1.0, smoothstep(0.2, 0.6, wear + slow * 0.4));
    return vec4<f32>(col, edge * thin * params.alpha);
}
