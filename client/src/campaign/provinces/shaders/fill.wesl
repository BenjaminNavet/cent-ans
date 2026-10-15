// Province colour wash, draped on the exaggerated relief. Mesh: grid, position.y = altitude (m),
// UV0 over the whole map; the texture holds one colour per province (nearest sampling).
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::view;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};

struct Params {
    tint: vec4<f32>,
    lift_base: f32,
    lift_per_distance: f32,
    pad0: f32,
    pad1: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var fill_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var fill_sampler: sampler;

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
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = get_world_from_local(in.instance_index) * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let dist = length(view.world_position - world.xyz);
    world.y = world.y + params.lift_base + params.lift_per_distance * dist;
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.uv = in.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(fill_texture, fill_sampler, in.uv);
    return vec4<f32>(texel.rgb * params.tint.rgb, texel.a * params.tint.a);
}
