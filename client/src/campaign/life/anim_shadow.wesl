// Shadow caster / depth prepass of the animated beasts, carts and herds (prepass vertex shader of
// `AnimMaterial`). Same pose as `anim.wesl` (module `anim_pose`), so the shadows follow the legs,
// the head, the wheels and the jolt of the box instead of the rigid mesh. Time is `globals.time`.
import cent_ans_client::campaign::life::anim_pose::{AnimParams, pose_local};
import bevy_pbr::{
    prepass::io::{Vertex, VertexOutput, decompress_vertex},
    render::{
        mesh_functions,
        view_transformations::position_world_to_clip,
        mesh_view_bindings::globals,
    },
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> anim: AnimParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let tag = mesh_functions::get_tag(vertex_in.instance_index);
    let local = pose_local(anim, vertex.position, tag, globals.time);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}
@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    return out;
}
