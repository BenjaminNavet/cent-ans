// Shadow caster of the camp horses: same pose as `camp_horse.wesl`.
import cent_ans_client::battle::decor::camp_horse_pose::{CampHorseParams, camp_pose};
import bevy_render::globals::Globals;
import bevy_pbr::{
    prepass::io::{Vertex, VertexOutput, decompress_vertex},
    render::{
        mesh_functions,
        view_transformations::position_world_to_clip,
    },
};

// The prepass view layout has the globals at binding 1 (the main pass has them at 11).
@group(0) @binding(1) var<uniform> globals: Globals;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> horse: CampHorseParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let local = camp_pose(horse, vertex.position, world_from_local[3].xz, globals.time);
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
