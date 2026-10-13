// GPU-instanced vegetation: one draw per (mesh, material); per-instance records are the raw
// 6-f32 records of the .f32 files (x, y, z, yaw_deg, scale, model_index) read as vertex attributes.
// Lit with Bevy's own PBR lighting (sun, cascaded shadows received, ambient, fog) but without a
// normal map, SSAO or shadow casting.
import bevy_pbr::render::{
    pbr_functions,
    pbr_types,
    view_transformations::position_world_to_clip,
};

struct Params {
    base_color: vec4<f32>,
    alpha_cutoff: f32,
    roughness: f32,
    metallic: f32,
    double_sided: f32,
}

@group(2) @binding(0) var base_texture: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;
@group(2) @binding(2) var<uniform> params: Params;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(8) at_yaw: vec4<f32>,
    @location(9) scale_model: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

fn rotate_y(v: vec3<f32>, c: f32, s: f32) -> vec3<f32> {
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let yaw = radians(in.at_yaw.w);
    let c = cos(yaw);
    let s = sin(yaw);
    let world = rotate_y(in.position * in.scale_model.x, c, s) + in.at_yaw.xyz;
    out.world_position = world;
    out.world_normal = rotate_y(in.normal, c, s);
    out.position = position_world_to_clip(world);
    out.uv = in.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var color = params.base_color * textureSample(base_texture, base_sampler, in.uv);
    if params.alpha_cutoff > 0.0 && color.a < params.alpha_cutoff {
        discard;
    }
    color.a = 1.0;

    var normal = normalize(in.world_normal);
    if !is_front && params.double_sided > 0.5 {
        normal = -normal;
    }

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = params.roughness;
    pbr_input.material.metallic = params.metallic;
    pbr_input.material.flags = pbr_input.material.flags | pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = vec4<f32>(in.world_position, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = pbr_functions::calculate_view(pbr_input.world_position, false);

    let lit = pbr_functions::apply_pbr_lighting(pbr_input);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
