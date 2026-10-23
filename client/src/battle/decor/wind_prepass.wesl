// Vertex stage of the wind material in the prepass (depth, normals, motion vectors) and in the
// shadow passes: Bevy's prepass vertex stage (no morph targets, no skinning) with the same sway as
// `wind.wesl` (maths in `wind_common.wesl`). The fragment stage is the base material's, which
// discards the alpha-masked texels of the foliage.
import cent_ans_client::battle::decor::wind_common::{WindParams, wind_displace};
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

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> wind: WindParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var flatten_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var flatten_samp: sampler;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    let world_still = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    // No UVs: v = 1 gives the grass mode a zero weight.
    var uv_y = 1.0;
@if(VERTEX_UVS_A)
    uv_y = vertex.uv.y;
    let moved = wind_displace(wind, world_still, vertex.position.y, uv_y, globals.time, flatten_tex, flatten_samp);
    out.world_position = moved.world;
    out.position = position_world_to_clip(out.world_position.xyz);
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
}

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;

@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
@if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_in.instance_index
    );
}

@if(VERTEX_COLORS)
    out.color = vertex.color;

    // The position of the vertex one frame ago, swayed by the wind of that frame: the motion
    // vector is the true motion of the foliage, not the sway of the whole frame.
@if(MOTION_VECTOR_PREPASS) {
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_in.instance_index);
    let prev_still = mesh_functions::mesh_position_local_to_world(prev_model, vec4<f32>(vertex.position, 1.0));
    let prev_moved = wind_displace(wind, prev_still, vertex.position.y, uv_y, globals.time - globals.delta_time, flatten_tex, flatten_samp);
    out.previous_world_position = prev_moved.world;
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);

    return out;
}
