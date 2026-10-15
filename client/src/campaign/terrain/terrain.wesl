// Campaign ground (port of game/shaders/terrain.gdshader + relief_quadtree.gdshaderinc, ADR 0142
// satellite colormap, ADR 0353 exaggerated relief). The CPU meshes carry the altitude in metres
// and the relief fields; the displayed height `y = s*(h - K*max(h-base,0) + g*(1-K)*max(h-floor,0)
// + c*D)` is computed here so that the dynamic vertical scale needs no mesh rebuild.

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::view,
};

struct TerrainParams {
    // xyz: direction towards the sun.
    sun_dir: vec4<f32>,
    // rgb: sun colour x energy.
    sun_color: vec4<f32>,
    // rgb: ambient colour x energy.
    ambient: vec4<f32>,
    // rgb: distance fog colour, a: fog strength.
    fog: vec4<f32>,
    // x: vertical scale s, y: local gain g, z: squash weight c, w: edge haze width (px, 0 = off).
    scale: vec4<f32>,
    // xy: world size (map units), z: fog begin, w: 1 when the colormap is bound.
    map: vec4<f32>,
    // rgb: edge haze colour (linear), a: fog end.
    edge: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: TerrainParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var colormap: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var colormap_sampler: sampler;

struct SeasonParams {
    // spring, summer, autumn, winter weights.
    weights: vec4<f32>,
    // per season: rgb gain, a saturation.
    gain: array<vec4<f32>, 4>,
    snow_lines: vec4<f32>,
    summer_gold: vec4<f32>,
    // rgb: winter ground tint, a: winter desaturation.
    winter_ground: vec4<f32>,
    // x, y: plain-snow south fade (fractions of the map height), z: plain snow share.
    snow_plain: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> season: SeasonParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    // x, altitude (m), z
    @location(0) position: vec3<f32>,
    // dh/dx, dh/dz (m per unit), skirt depth (world units)
    @location(1) slope: vec3<f32>,
    // valley floor (m), base (m)
    @location(2) fields_a: vec2<f32>,
    // squash factor k, massif lift D (m)
    @location(3) fields_b: vec2<f32>,
};

struct Varyings {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    // altitude (m)
    @location(2) altitude: f32,
};

fn displayed_y(h: f32, fa: vec2<f32>, fb: vec2<f32>) -> f32 {
    let s = params.scale.x;
    let g = params.scale.y;
    let c = params.scale.z;
    let k = c * fb.x;
    return s * (h - k * max(h - fa.y, 0.0) + g * (1.0 - k) * max(h - fa.x, 0.0) + c * fb.y);
}

@vertex
fn vertex(v: Vertex) -> Varyings {
    var out: Varyings;
    let h = v.position.y;
    var y = displayed_y(h, v.fields_a, v.fields_b);
    let edge_px = params.scale.w;
    if (edge_px > 0.0) {
        // The land sinks to sea level towards the map edge (no slab, no flank).
        let to_edge = min(v.position.xz, params.map.xy - v.position.xz);
        let sink = 1.0 - smoothstep(0.0, edge_px, min(to_edge.x, to_edge.y));
        y = mix(y, min(y, 0.2), sink);
    }
    y = y - v.slope.z;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position.x, y, v.position.z, 1.0));
    out.world = world.xyz;
    out.altitude = h;
    out.clip = position_world_to_clip(world.xyz);
    // Displayed gradient: dY/dh of the exaggeration formula times the altitude gradient.
    let k = params.scale.z * v.fields_b.x;
    let above_base = select(0.0, 1.0, h > v.fields_a.y);
    let above_floor = select(0.0, 1.0, h > v.fields_a.x);
    let ratio = params.scale.x * (1.0 - k * above_base + params.scale.y * (1.0 - k) * above_floor);
    out.normal = normalize(vec3<f32>(-ratio * v.slope.x, 1.0, -ratio * v.slope.y));
    return out;
}

// Season grade of the ground colour (SeasonLook.grade / ground, snow of terrain.gdshader simplified).
fn season_ground(albedo: vec3<f32>, n: vec3<f32>, altitude: f32, xz: vec2<f32>) -> vec3<f32> {
    let w = season.weights;
    var gain = vec3<f32>(0.0);
    var saturation = 0.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        gain = gain + season.gain[i].rgb * w[i];
        saturation = saturation + season.gain[i].a * w[i];
    }
    var col = albedo * gain * mix(vec3<f32>(1.0), season.summer_gold.rgb, w.y);
    // Cold, greyer winter ground.
    col = col * mix(vec3<f32>(1.0), season.winter_ground.rgb, w.w);
    let luma = dot(col, vec3<f32>(0.299, 0.587, 0.114));
    col = mix(vec3<f32>(luma), col, saturation * (1.0 - season.winter_ground.a * w.w));
    // Snow: above the seasonal snow line, plus a share of the plains of the north in winter; it
    // slides off steep slopes.
    let line = dot(w, season.snow_lines);
    var snow = 0.0;
    if (line > 0.0) {
        snow = smoothstep(line - 150.0, line + 150.0, altitude);
    }
    let north = 1.0 - smoothstep(season.snow_plain.x * params.map.y, season.snow_plain.y * params.map.y, xz.y);
    snow = max(snow, w.w * north * season.snow_plain.z);
    snow = snow * smoothstep(0.55, 0.85, n.y);
    return mix(col, vec3<f32>(0.78, 0.80, 0.84), snow);
}

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    var albedo = vec3<f32>(0.10, 0.15, 0.06);
    if (params.map.w > 0.5) {
        albedo = textureSample(colormap, colormap_sampler, in.world.xz / params.map.xy).rgb;
    }
    let n = normalize(in.normal);
    albedo = season_ground(albedo, n, in.altitude, in.world.xz);
    let ndl = max(dot(n, params.sun_dir.xyz), 0.0);
    var color = albedo * (params.ambient.rgb + params.sun_color.rgb * ndl);
    // Distance fog (depth fog of the Godot environment, simplified).
    let d = distance(in.world, view.world_position);
    let begin = params.map.z;
    let fog_t = clamp((d - begin) / max(params.edge.a - begin, 1.0), 0.0, 1.0);
    color = mix(color, params.fog.rgb, pow(fog_t, 1.6) * params.fog.a);
    // Map edge drowned in haze.
    let edge_px = params.scale.w;
    if (edge_px > 0.0) {
        let q = min(in.world.xz, params.map.xy - in.world.xz);
        let haze = 1.0 - smoothstep(0.0, edge_px, min(q.x, q.y));
        color = mix(color, params.edge.rgb, haze);
    }
    return vec4<f32>(color, 1.0);
}
