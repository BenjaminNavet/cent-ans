// Standard cloth of the campaign armies. Port of game/shaders/map_banner.gdshader: cloth waving in
// the wind, turned towards the camera round the pole (vertical billboard, node scale kept).
// Modes: 0 = left-attached flag, shield cropped from `heraldry` or plain faction colour;
// 1 = pennant, whole texture; 2 = vertical banner hung from a crossbar, whole texture.
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    faction_color: vec4<f32>,
    crop: vec4<f32>,
    // xyz: world direction of the pole, w: gust of the walk.
    pole_gust: vec4<f32>,
    // x: wave amplitude, y: wave speed, z: front offset (mode 2).
    wave: vec4<f32>,
    // x: mode, y: has_heraldry.
    flags: vec4<f32>,
    // Ground light of the campaign (`TerrainParams`): xyz direction to the sun, w = 1 when set;
    // sun colour x energy; ambient colour x energy (w: gain).
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var heraldry: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var heraldry_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fold: f32,
    // World normal of the cloth: the billboard faces the camera.
    @location(2) normal: vec3<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    let origin = world_from_local[3].xyz;
    let time = globals.time;
    let mode = i32(params.flags.x + 0.5);
    let phase_seed = origin.x * 0.1 + origin.z * 0.07;
    let amp = params.wave.x * (1.0 + params.pole_gust.w);
    var p = in.position;
    var fold = 0.0;
    if (mode == 2) {
        // Hung from the top: the wave grows downwards and runs across.
        let along = in.uv.y;
        let phase = time * params.wave.y * 0.8 - along * 3.5 + in.uv.x * 1.6 + phase_seed;
        p.z += sin(phase) * amp * along + params.wave.z;
        p.x += sin(phase * 0.7 + 1.3) * amp * 0.35 * along;
        fold = cos(phase) * along;
    } else {
        let u = in.uv.x;
        let phase = time * params.wave.y - u * 5.5 + phase_seed;
        p.z += sin(phase) * amp * u;
        p.y -= u * u * 0.12;
        fold = cos(phase) * u;
    }
    // The cloth leans with the pole as the camera sees it.
    let right = view.world_from_view[0].xyz;
    let back = view.world_from_view[2].xyz;
    let pole = params.pole_gust.xyz;
    let lean = atan2(dot(pole, right), max(pole.y, 0.05));
    let lc = cos(lean);
    let ls = sin(lean);
    p = vec3<f32>(p.x * lc + p.y * ls, -p.x * ls + p.y * lc, p.z);
    // Vertical billboard (BILLBOARD_FIXED_Y), scale of the node kept.
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let axis_x = normalize(cross(up, back));
    let axis_z = normalize(cross(right, up));
    let sx = length(world_from_local[0].xyz);
    let sy = length(world_from_local[1].xyz);
    let sz = length(world_from_local[2].xyz);
    let world = origin + axis_x * (p.x * sx) + up * (p.y * sy) + axis_z * (p.z * sz);
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.uv = in.uv;
    out.fold = fold;
    out.normal = axis_z;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = i32(params.flags.x + 0.5);
    var color = params.faction_color.rgb;
    var alpha = 1.0;
    if (params.flags.y > 0.5) {
        if (mode == 0) {
            let tex = textureSample(heraldry, heraldry_sampler, params.crop.xy + in.uv * params.crop.zw);
            color = mix(color, tex.rgb, tex.a);
        } else {
            let tex = textureSample(heraldry, heraldry_sampler, in.uv);
            color = tex.rgb;
            alpha = tex.a;
        }
    }
    var edge = 0.0;
    if (mode == 2) {
        // Dark crossbar on top, hems on the sides.
        edge = step(in.uv.y, 0.025) + 0.5 * (step(in.uv.x, 0.025) + step(0.975, in.uv.x));
    } else {
        edge = step(0.94, in.uv.x) + step(0.93, in.uv.y) + step(in.uv.y, 0.05);
    }
    color = color * mix(1.0, 0.5, clamp(edge, 0.0, 1.0));
    if (alpha < 0.5) {
        discard;
    }
    let albedo = color * (0.86 + 0.18 * in.fold);
    if (params.sun_dir.w < 0.5) {
        return vec4<f32>(albedo, 1.0);
    }
    // `diffuse_lambert_wrap` of the Godot shader (roughness 0.9) and its BACKLIGHT, with the
    // light of the ground (sun + ambient), not Bevy's exposure.
    let n = normalize(in.normal);
    let ndl = dot(n, normalize(params.sun_dir.xyz));
    let wrap = max((ndl + 0.9) / 1.9, 0.0);
    let back = max(-ndl, 0.0);
    let lit = albedo * (params.ambient.rgb * params.ambient.a + params.sun_color.rgb * wrap)
        + color * 0.3 * params.sun_color.rgb * back;
    return vec4<f32>(lit, 1.0);
}
