// Vertex animation of the beasts, carts and herds of the living map. Port of
// game/shaders/animal_motion.gdshaderinc (draught beasts, carts, ploughs: four-beat legs on the
// measured step curves, nodding or grazing head, tail, breath, wheels turning with the distance
// travelled, measured jolt and roll of the box) and of the gait part of game/shaders/fauna.gdshader
// (herds: diagonal legs, lift, bounce, head down when stopped).
//
// The vertex stage is a copy of Bevy's mesh vertex stage (as wind.wesl). Per-instance data come
// from the mesh tag (`MeshTag`):
//   mode 0 (prop):  low 16 bits = speed in mm/s, high 16 bits = seed / 65536;
//   mode 1 (herd):  low 16 bits = leg phase (cycles) / 65536, bits 16..24 = moving / 255.
// Time is `globals.time`. Everything is computed in the Godot frame of the model (front along +Z):
// the mesh is turned by `frame.x` (the `yaw_deg` of the prop table) and turned back afterwards.
import cent_ans_client::campaign::life::anim_pose::{AnimParams, pose_local};
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> anim: AnimParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

    let tag = mesh_functions::get_tag(vertex_in.instance_index);
    let local = pose_local(anim, vertex.position, tag, globals.time);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));

    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);

    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}
