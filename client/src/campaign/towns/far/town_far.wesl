// Far mesh of the 1:1 towns (port of game/shaders/town_far.gdshader, ADR 0138). Vertex stage only;
// the fragment is StandardMaterial's (matte, vertex colours).
// Vertex contract (`far/builder.rs`): position.xz world (map units), position.y ABSOLUTE altitude
// (m); uv_b.x town index (mask texel), uv_b.y height (m) above the local ground (negative on the
// skirt), so the vertex ground is position.y - uv_b.y.
// Ground placed by `relief_display_height` (dynamic exaggerated relief), building above it at 1:1
// (metres / metres per unit, like under the root of a 1:1 town scaled 1 / metres per unit).
// Mask (Rgba8, 64 x 64, texel (i % 64, i / 64)): R = 1 when the 1:1 town is built (the far mesh is
// sunk -sink_depth_m under the ground), G = soot 0-1 (roofs brown-black, walls a little).
// The layer follows the fade of the 1:1 layer: a town not built in 1:1 sinks as the layer weight
// falls (the maquette of the markers lot rises at the same time).
import bevy_pbr::render::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
};
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};

struct FarParams {
    // x: metres per unit, y: layer weight, z: sink depth (m), w: soot on roofs (max).
    a: vec4<f32>,
    // rgb: soot colour (linear), w: soot on walls (max).
    soot: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> far: FarParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var relief_floor: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var town_mask: texture_2d<f32>;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    var town = -1;
    var above = 0.0;
    @if(VERTEX_UVS_B)
    {
        town = i32(vertex.uv_b.x + 0.5);
        above = vertex.uv_b.y;
    }
    var mask = vec4<f32>(0.0);
    if town >= 0 && town < 4096 {
        mask = textureLoad(town_mask, vec2<i32>(town % 64, town / 64), 0);
    }
    let xz = vertex.position.xz;
    let ground_y = relief_display_height(relief_floor, relief, vertex.position.y - above, xz);
    var sink = 1.0 - far.a.y;
    if mask.r > 0.5 {
        sink = 1.0;
    }
    let h_m = mix(above, -far.a.z, clamp(sink, 0.0, 1.0));
    let local = vec4<f32>(xz.x, ground_y + h_m / far.a.x, xz.y, 1.0);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, local);
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);

    var up = 0.0;
    @if(VERTEX_NORMALS)
    {
        out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
        up = smoothstep(0.35, 0.7, vertex.normal.y);
    }
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_COLORS)
    {
        let amount = mask.g * mix(far.soot.w, far.a.w, up);
        out.color = vec4<f32>(mix(vertex.color.rgb, far.soot.rgb, amount), vertex.color.a);
    }
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}
