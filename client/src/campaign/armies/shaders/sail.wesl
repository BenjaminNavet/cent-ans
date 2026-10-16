// Square sail of the cogs and ships of the campaign map (port of game/shaders/campaign_sail.gdshader,
// lot armies6): cloth dyed in the faction colour, sewn vertical strips, the arms of the faction
// painted in the middle, a dirtied lower edge; swollen by the wind (parabolic belly that slowly
// breathes). Sail UV: (0, 0) top left, (1, 1) bottom right (campaign_fleet.py). Lit by Bevy's PBR.
import bevy_pbr::render::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput, FragmentOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
import bevy_pbr::render::mesh_view_bindings::globals;

struct SailParams {
    faction_color: vec4<f32>,
    // x belly, y breathe, z has_heraldry.
    shape: vec4<f32>,
    // xyz: local axis of the swelling (the sail is in the YZ plane, the wind pushes towards +X).
    wind_axis: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> sail: SailParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var heraldry: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var heraldry_sampler: sampler;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    var local = vertex.position;
@if(VERTEX_UVS_A)
    {
        let across = vertex.uv.x * 2.0 - 1.0;
        let down = vertex.uv.y;
        let shape = (1.0 - across * across)
            * (0.35 + 0.65 * sin(3.14159 * clamp(down, 0.0, 1.0) * 0.9 + 0.15));
        let pulse = 1.0 + sail.shape.y * sin(globals.time * 0.9 + world_from_local[3].x * 0.05);
        local = local + sail.wind_axis.xyz * sail.shape.x * shape * pulse;
    }
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
@if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
@if(VERTEX_COLORS)
    out.color = vertex.color;
@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    var uv = vec2<f32>(0.5, 0.5);
@if(VERTEX_UVS_A)
    uv = in.uv;
    // Strips: vertical bands of cloth, dark seams.
    let strips = 7.0;
    let seam = abs(fract(uv.x * strips) - 0.5);
    let seam_line = 1.0 - smoothstep(0.0, 0.04, 0.5 - seam);
    let strip_id = floor(uv.x * strips);
    let tone = 0.92 + 0.08 * fract(sin(strip_id * 12.9898) * 43758.5453);
    let grey = dot(sail.faction_color.rgb, vec3<f32>(0.3, 0.59, 0.11));
    var cloth = mix(sail.faction_color.rgb, vec3<f32>(grey), 0.15) * tone;
    cloth = cloth * (1.0 - 0.25 * seam_line);
    // Arms in the middle of the sail.
    let arms_uv = (uv - vec2<f32>(0.2, 0.18)) / vec2<f32>(0.6, 0.62);
    if sail.shape.z > 0.5 && arms_uv.x >= 0.0 && arms_uv.x <= 1.0 && arms_uv.y >= 0.0 && arms_uv.y <= 1.0 {
        let arms = textureSampleLevel(heraldry, heraldry_sampler, arms_uv, 0.0);
        cloth = mix(cloth, arms.rgb * 0.9, arms.a);
    }
    // Lower edge dirtied by the spray, shadow of the belly.
    cloth = cloth * mix(1.0, 0.72, smoothstep(0.8, 1.0, uv.y));
    cloth = cloth * mix(0.85, 1.0, sin(3.14159 * uv.x));
    pbr_input.material.base_color = vec4<f32>(cloth, 1.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
