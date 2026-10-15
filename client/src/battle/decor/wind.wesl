// Wind sway for grass and tree crowns: a copy of Bevy's mesh vertex stage (no morph targets, no
// skinning) that moves the vertices in world space before projection. Fragment: StandardMaterial.
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
};

struct WindParams {
    // x: sway amplitude (m), y: sway speed, z: gust amplitude (m), w: gust speed.
    sway: vec4<f32>,
    // xy: ground direction of the wind, z: weight mode (0: 1 - v of the card, 1: height / height_ref),
    // w: height_ref (m).
    shape: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> wind: WindParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    var weight = 0.0;
    @if(VERTEX_UVS_A)
    {
        if wind.shape.z < 0.5 {
            weight = clamp(1.0 - vertex.uv.y, 0.0, 1.0);
        }
    }
    if wind.shape.z > 0.5 {
        let h = clamp(vertex.position.y / max(wind.shape.w, 0.01), 0.0, 1.0);
        weight = h * h;
    }
    let t = globals.time;
    let phase = world.x * 0.21 + world.z * 0.17;
    let gust = sin(t * wind.sway.w + world.x * 0.012 + world.z * 0.009) * 0.5 + 0.5;
    let sway = (sin(t * wind.sway.y + phase) * 0.6 + sin(t * wind.sway.y * 2.3 + phase * 1.7) * 0.4) * wind.sway.x
        + gust * wind.sway.z;
    world = vec4<f32>(world.x + wind.shape.x * sway * weight, world.y, world.z + wind.shape.y * sway * weight, 1.0);

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
