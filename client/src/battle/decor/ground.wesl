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
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> ground: GroundParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_strip: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var albedo_sampler: sampler;

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
// the repetition, faded to the layer mean with the distance.
fn layer(k: i32, wp: vec2<f32>, dist: f32) -> vec3<f32> {
    let tile = ground.layers[k].a;
    let uv_a = fract(wp / tile);
    let rotated = vec2<f32>(wp.x * 0.8 - wp.y * 0.6, wp.x * 0.6 + wp.y * 0.8) * 0.37 + vec2<f32>(17.3, 5.1);
    let uv_b = fract(rotated / tile);
    let rows = 13.0;
    let a = textureSampleLevel(albedo_strip, albedo_sampler, vec2<f32>(uv_a.x * 0.998 + 0.001, (f32(k) + uv_a.y * 0.998 + 0.001) / rows), 0.0).rgb;
    let b = textureSampleLevel(albedo_strip, albedo_sampler, vec2<f32>(uv_b.x * 0.998 + 0.001, (f32(k) + uv_b.y * 0.998 + 0.001) / rows), 0.0).rgb;
    let detail = mix(a, b, 0.4);
    let mean = ground.layers[k].rgb;
    let fade = smoothstep(ground.water.a * 0.35, ground.water.a * 1.6, dist);
    return mix(detail, mean, fade);
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

    // Macro variation: meadow and flowering patches inside the grass.
    let macro_n = value_noise(wp / 38.0) * 0.6 + value_noise(wp / 9.0) * 0.4;
    var grass = layer(0, wp, dist);
    grass = mix(grass, layer(1, wp, dist), smoothstep(0.45, 0.7, macro_n));
    grass = grass * ground.season.rgb;
    let steep = 1.0 - clamp(in.world_normal.y, 0.0, 1.0);
    let rock_w = smoothstep(0.28, 0.5, steep);

    var colour = grass * splat.r;
    colour = colour + layer(2, wp, dist) * splat.g;
    colour = colour + layer(3, wp, dist) * splat.b;
    colour = colour + layer(4, wp, dist) * splat.a;
    colour = colour + mix(layer(5, wp, dist), layer(6, wp, dist), 0.3) * stone_w;
    colour = colour + layer(7, wp, dist) * plough_w;
    // Steep slopes show rock.
    colour = mix(colour, layer(6, wp, dist), rock_w * clamp(land, 0.0, 1.0));
    // Snow cover.
    let snow_amount = ground.season.a * (1.0 - 0.5 * rock_w);
    colour = mix(colour, layer(8, wp, dist), snow_amount);
    // Deep water where no splat is left.
    let water_w = 1.0 - clamp(land * 4.0, 0.0, 1.0);
    colour = mix(colour, ground.water.rgb, water_w);

    pbr_input.material.base_color = vec4<f32>(colour, 1.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
