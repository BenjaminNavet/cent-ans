// Stone of the siege walls and towers (BattleSiege._textured("stone") of Godot): the player's
// castle_wall_varriation diffuse + normal in world triplanar, `params.x` repetitions per metre,
// `params.y` blend sharpness, `params.z` normal strength. The colour tint and the roughness are
// those of the StandardMaterial it extends.
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xyz * params.x;
    let n = normalize(in.world_normal);
    var w = pow(abs(n), vec3<f32>(params.y));
    w = w / max(w.x + w.y + w.z, 1e-5);
    let uv_x = p.zy;
    let uv_y = p.xz;
    let uv_z = p.xy;

    let col_x = textureSample(diffuse_tex, diffuse_sampler, uv_x).rgb;
    let col_y = textureSample(diffuse_tex, diffuse_sampler, uv_y).rgb;
    let col_z = textureSample(diffuse_tex, diffuse_sampler, uv_z).rgb;
    let albedo = col_x * w.x + col_y * w.y + col_z * w.z;

    // Whiteout blend of the three tangent-space normals (OpenGL style, +Y up).
    let nx = textureSample(normal_tex, normal_sampler, uv_x).xyz * 2.0 - 1.0;
    let ny = textureSample(normal_tex, normal_sampler, uv_y).xyz * 2.0 - 1.0;
    let nz = textureSample(normal_tex, normal_sampler, uv_z).xyz * 2.0 - 1.0;
    let k = params.z;
    let tx = vec3<f32>(nx.xy * k + n.zy, abs(nx.z) * n.x);
    let ty = vec3<f32>(ny.xy * k + n.xz, abs(ny.z) * n.y);
    let tz = vec3<f32>(nz.xy * k + n.xy, abs(nz.z) * n.z);
    let blended = tx.zyx * w.x + ty.xzy * w.y + tz.xyz * w.z;
    pbr_input.N = normalize(blended);

    pbr_input.material.base_color = vec4<f32>(albedo * pbr_input.material.base_color.rgb, 1.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
