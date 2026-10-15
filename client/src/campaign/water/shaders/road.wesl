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
    let side = abs(in.uv.y);
    let n = noise(vec2<f32>(in.uv.x * 3.0, in.uv.y * 2.0));
    let ruts = smoothstep(0.08, 0.0, abs(side - 0.45)) * 0.6;
    var col = mix(params.dirt_color.rgb, params.rut_color.rgb, ruts);
    col = mix(col, params.grass_edge.rgb, smoothstep(0.55, 1.0, side + (n - 0.5) * 0.3));
    col = col * (0.9 + 0.2 * n);
    let edge = 1.0 - smoothstep(0.7, 1.0, side + (n - 0.5) * 0.25);
    return vec4<f32>(col, edge * params.alpha);
}
