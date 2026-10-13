import bevy_pbr::{
    prepass::io::{Vertex, VertexOutput, decompress_vertex},
    prepass::bindings as prepass_bindings,
    render::{mesh_functions, view_transformations::position_world_to_clip},
};

// Procedural walk, identical in both engines (tools/proto_bevy/CONTRACT.md):
//   phase = fract(sin(instance_index * 12.9898) * 43758.5453) * 2pi
//   s     = sin(2pi * hz * t + phase)
//   translate Y by bob_m * |s|, rotate about the local Z axis by sway * s (pivot at the feet).
// `instance_index` is the soldier's record index, passed through `MeshTag`.

// The prepass view layout binds the globals uniform at binding 1.
@group(0) @binding(1) var<uniform> globals: bevy_render::globals::Globals;

struct WalkParams {
    t0: f32, // Time::elapsed_secs() when the path clock started (1e9 before)
    hz: f32,
    bob_m: f32,
    sway_rad: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> walk: WalkParams;

const TAU: f32 = 6.283185307179586;

fn walk_phase_sine(tag: u32) -> f32 {
    let phase = fract(sin(f32(tag) * 12.9898) * 43758.5453) * TAU;
    return sin(TAU * walk.hz * max(globals.time - walk.t0, 0.0) + phase);
}

fn walk_rotate_z(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(c * v.x - s * v.y, s * v.x + c * v.y, v.z);
}

fn walk_position(position: vec3<f32>, tag: u32) -> vec3<f32> {
    let s = walk_phase_sine(tag);
    return walk_rotate_z(position, walk.sway_rad * s) + vec3<f32>(0.0, walk.bob_m * abs(s), 0.0);
}

fn walk_direction(direction: vec3<f32>, tag: u32) -> vec3<f32> {
    return walk_rotate_z(direction, walk.sway_rad * walk_phase_sine(tag));
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let tag = mesh_functions::get_tag(vertex_in.instance_index);

    vertex.position = walk_position(vertex.position, tag);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
@if(VERTEX_NORMALS) {
    vertex.normal = walk_direction(vertex.normal, tag);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
}
@if(VERTEX_TANGENTS) {
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vec4<f32>(walk_direction(vertex.tangent.xyz, tag), vertex.tangent.w),
        vertex_in.instance_index
    );
}
}

@if(VERTEX_COLORS)
    out.color = vertex.color;

@if(MOTION_VECTOR_PREPASS) {
    // Previous-frame position ignores the walk animation (approximation).
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_in.instance_index);
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(prev_model, vec4<f32>(vertex.position, 1.0));
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(vertex_in.instance_index, world_from_local[3]);

    return out;
}
