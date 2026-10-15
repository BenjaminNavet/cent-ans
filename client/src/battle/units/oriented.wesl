// GPU-instanced oriented meshes: record (x, y, z, scale | dx, dy, dz, 0), the mesh's +Z axis
// is turned to the direction. Lit with Bevy's PBR code like instanced.wesl.
import bevy_pbr::render::{
    mesh_types,
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
    @location(8) at_scale: vec4<f32>,
    @location(9) dir: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

fn rotate_to(v: vec3<f32>, fwd: vec3<f32>) -> vec3<f32> {
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if abs(fwd.y) > 0.98 {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let right = normalize(cross(up, fwd));
    let up2 = cross(fwd, right);
    return right * v.x + up2 * v.y + fwd * v.z;
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let fwd = normalize(in.dir.xyz + vec3<f32>(0.0, 0.0, 0.00001));
    let world = rotate_to(in.position * in.at_scale.w, fwd) + in.at_scale.xyz;
    out.world_position = world;
    out.world_normal = rotate_to(in.normal, fwd);
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
    // Receive the cascaded sun shadows (pbr_input_new leaves the mesh flags at 0).
    pbr_input.flags = mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT;
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = vec4<f32>(in.world_position, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = pbr_functions::calculate_view(pbr_input.world_position, false);

    let lit = pbr_functions::apply_pbr_lighting(pbr_input);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
