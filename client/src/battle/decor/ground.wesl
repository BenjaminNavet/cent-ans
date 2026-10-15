// Battlefield ground: regional layers blended by the per-vertex splat (port of
// game/shaders/battle_ground.gdshader, albedo only), lit by Bevy's PBR (sun, shadows, fog).
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::view,
};

struct GroundParams {
    // rgb: mean linear colour of the layer, a: tile size (m).
    layers: array<vec4<f32>, 13>,
    // rgb: grass tint, a: snow cover.
    season: vec4<f32>,
    // rgb: water colour, a: distance where the textures fade to their mean.
    water: vec4<f32>,
    // x: normal-map strength, y: fade distance of the normals, z: 1 with normals, w: macro contrast.
    detail: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> ground: GroundParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var albedo_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_sampler: sampler;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

// One layer sampled at world position `wp`: two taps at different scales and orientations to hide
// the repetition (mip-mapped repeating array, anisotropic), faded to the layer mean with the
// distance.
fn warp_weight(w: f32, n: f32) -> f32 {
    return smoothstep(0.35, 0.65, w + (n - 0.5) * 1.5);
}

fn layer(k: i32, wp: vec2<f32>, dist: f32) -> vec3<f32> {
    let tile = ground.layers[k].a;
    let rotated = vec2<f32>(wp.x * 0.8 - wp.y * 0.6, wp.x * 0.6 + wp.y * 0.8) * 0.37 + vec2<f32>(17.3, 5.1);
    let a = textureSample(albedo_array, albedo_sampler, wp / tile, k).rgb;
    let b = textureSample(albedo_array, albedo_sampler, rotated / tile, k).rgb;
    let detail = mix(a, b, 0.4);
    let mean = ground.layers[k].rgb;
    let fade = smoothstep(ground.water.a * 0.35, ground.water.a * 1.6, dist);
    return mix(detail, mean, fade);
}

// Tangent-space normal of one layer (one tap).
fn layer_normal(k: i32, wp: vec2<f32>) -> vec2<f32> {
    let tile = ground.layers[k].a;
    let n = textureSample(normal_array, normal_sampler, wp / tile, k).xy * 2.0 - 1.0;
    return n;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let wp = in.world_position.xz;
    let dist = distance(in.world_position.xyz, view.world_position);
    let splat = in.color;
    let stone_w = in.uv_b.x;
    let plough_w = in.uv_b.y;
    let land = splat.r + splat.g + splat.b + splat.a + stone_w + plough_w;
    // The splat is per vertex (10 m cells): break its straight, hard edges with noise so dirt,
    // mud and ploughland meet the grass in ragged, organic borders.
    let edge_n = value_noise(wp / 13.0) * 0.5 + value_noise(wp / 4.5) * 0.3 + value_noise(wp / 1.6) * 0.2;
    let sg = warp_weight(splat.g, edge_n);
    let sb = warp_weight(splat.b, value_noise(wp / 4.3 + vec2<f32>(11.0, 3.0)));
    let sa = warp_weight(splat.a, edge_n);
    let sst = warp_weight(stone_w, edge_n);
    let spl = warp_weight(plough_w, value_noise(wp / 3.1 + vec2<f32>(5.0, 23.0)));
    let others = sg + sb + sa + sst + spl;
    let sr = max(land - others, 0.0);

    // Macro variation: meadow and flowering patches inside the grass.
    let macro_n = value_noise(wp / 38.0) * 0.6 + value_noise(wp / 9.0) * 0.4;
    var grass = layer(0, wp, dist);
    grass = mix(grass, layer(1, wp, dist), smoothstep(0.45, 0.7, macro_n));
    grass = grass * ground.season.rgb;
    // Large patches of lighter and darker grass, so the field does not read as one flat green.
    let patches = value_noise(wp / 140.0) * 0.55 + value_noise(wp / 23.0) * 0.45;
    grass = grass * (1.0 + (patches - 0.5) * 2.0 * ground.detail.w);
    // Breakup at the scale the mip-mapped photo cannot show (tussocks, trodden and lush patches),
    // strongest near the camera, plus warm dry and cool lush drifts at the field scale.
    let near = 1.0 - smoothstep(60.0, 300.0, dist);
    let fine = value_noise(wp / 1.3) * 0.45 + value_noise(wp / 3.7) * 0.55;
    grass = grass * (1.0 + (fine - 0.5) * 0.6 * near);
    let drift = value_noise(wp / 61.0 + vec2<f32>(40.0, 9.0));
    grass = mix(grass, grass * vec3<f32>(1.16, 1.02, 0.72), smoothstep(0.52, 0.78, drift) * 0.7);
    grass = mix(grass, grass * vec3<f32>(0.86, 1.0, 0.95), smoothstep(0.45, 0.2, drift) * 0.5);
    let steep = 1.0 - clamp(in.world_normal.y, 0.0, 1.0);
    let rock_w = smoothstep(0.28, 0.5, steep);

    var colour = grass * sr;
    colour = colour + layer(2, wp, dist) * sg;
    colour = colour + layer(3, wp, dist) * sb;
    colour = colour + layer(4, wp, dist) * sa;
    colour = colour + mix(layer(5, wp, dist), layer(6, wp, dist), 0.3) * sst;
    colour = colour + layer(7, wp, dist) * spl;
    // Steep slopes show rock.
    colour = mix(colour, layer(6, wp, dist), rock_w * clamp(land, 0.0, 1.0));
    // Snow cover.
    let snow_amount = ground.season.a * (1.0 - 0.5 * rock_w);
    colour = mix(colour, layer(8, wp, dist), snow_amount);
    // Dirt, mud and stone breakup too (the photo's detail is lost in the mips).
    colour = colour * (1.0 + (fine - 0.5) * 0.45 * near * (1.0 - sr));
    // Deep water where no splat is left.
    let water_w = 1.0 - clamp(land * 4.0, 0.0, 1.0);
    colour = mix(colour, ground.water.rgb, water_w);

    pbr_input.material.base_color = vec4<f32>(colour, 1.0);

    // Normal maps of the dominant layers (grass, dirt, mud, stone, plough), faded with the distance.
    if ground.detail.z > 0.5 {
        let strength = ground.detail.x * (1.0 - smoothstep(ground.detail.y * 0.4, ground.detail.y, dist));
        if strength > 0.001 {
            var nxy = layer_normal(0, wp) * sr;
            nxy = nxy + layer_normal(2, wp) * sg;
            nxy = nxy + layer_normal(3, wp) * sb;
            nxy = nxy + layer_normal(4, wp) * sa;
            nxy = nxy + layer_normal(5, wp) * sst;
            nxy = nxy + layer_normal(7, wp) * spl;
            nxy = nxy * strength;
            // Terrain UVs are world x and z: tangent along +x, bitangent along +z.
            let up = normalize(pbr_input.world_normal);
            let tangent = normalize(vec3<f32>(1.0, 0.0, 0.0) - up * up.x);
            let bitangent = cross(up, tangent);
            pbr_input.N = normalize(up + tangent * nxy.x + bitangent * nxy.y);
        }
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
