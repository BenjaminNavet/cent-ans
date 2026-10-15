// Campaign ground (port of game/shaders/terrain.gdshader + relief_quadtree.gdshaderinc, ADR 0142
// satellite colormap, ADR 0353 exaggerated relief). The CPU meshes carry the altitude in metres
// and the relief fields; the displayed height `y = s*(h - K*max(h-base,0) + g*(1-K)*max(h-floor,0)
// + c*D)` is computed here so that the dynamic vertical scale needs no mesh rebuild.

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals, fog},
    mesh_view_types,
    fog as bevy_fog,
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
    // rgb: plain snow colour (sRGB).
    snow_color: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> season: SeasonParams;

struct LookParams {
    // fog of war: desaturation, dim, veil strength, 1 when a mask is bound.
    fog_a: vec4<f32>,
    // veil colour (linear), jitter of the veil edge (map px).
    fog_veil: vec4<f32>,
    // mist colour (linear), mist max opacity.
    fog_mist: vec4<f32>,
    // mask extent (map px), mist min opacity, mist drift scale.
    fog_map: vec4<f32>,
    // wet ground, fresh snow, parchment weight.
    weather: vec4<f32>,
    // geomorph: 1 when on, quadtree vertex px, morph ratio.
    morph: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> look: LookParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var fog_mask: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var fog_mask_sampler: sampler;

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
    // geomorph delta (m, parent level minus own), vertex spacing
    @location(5) morph: vec4<f32>,
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
    var h = v.position.y;
    // Geomorph: the altitude glides to the one of the parent level before the node is replaced
    // by it (the quadtree subdivides a node of spacing s while dist < 2 s k / max_vertex_px).
    if (look.morph.x > 0.5 && v.morph.y > 0.0) {
        let y_own = displayed_y(h, v.fields_a, v.fields_b);
        let k_proj = view.clip_from_view[1][1] * view.viewport.w * 0.5;
        let switch_d = 2.0 * v.morph.y * k_proj / max(look.morph.y, 0.5);
        let dist = distance(vec3<f32>(v.position.x, y_own, v.position.z), view.world_position);
        h = h + smoothstep(look.morph.z * switch_d, switch_d, dist) * v.morph.x;
    }
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
    // Plain winter snow is light (data `snow.plain_color`); the snow of the heights stays cooler.
    let plain_snow = vec3<f32>(0.78, 0.80, 0.84);
    let snow_tint = mix(season.snow_color.rgb, plain_snow, smoothstep(300.0, 1500.0, altitude));
    return mix(col, snow_tint, snow);
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * vnoise(p);
        p = p * 2.07 + vec2<f32>(13.7, 5.3);
        a = a * 0.5;
    }
    return v;
}

// Wet ground (darker, glossier) and fresh snow of the current weather.
fn weather_ground(col: vec3<f32>, n: vec3<f32>, altitude: f32, p: vec2<f32>) -> vec3<f32> {
    var out = col;
    let land = smoothstep(0.0, 4.0, altitude);
    let wet = look.weather.x;
    out = out * (1.0 - 0.28 * wet * land);
    let luma = dot(out, vec3<f32>(0.299, 0.587, 0.114));
    out = mix(out, vec3<f32>(luma) * 1.05, 0.12 * wet * land);
    // Fresh snow: patchy at its start, flat ground first, slides off steep slopes.
    let fresh = look.weather.y;
    let patches = smoothstep(0.30, 0.62, fbm(p * 0.05) + fresh * 0.45);
    let cover = clamp(fresh * 1.15, 0.0, 1.0) * patches * smoothstep(0.55, 0.85, n.y) * land;
    return mix(out, season.snow_color.rgb, cover);
}

// Fog-of-war veil: sepia ground under a wash, with a drifting remnant of mist (TB2, BR).
fn fog_of_war_veil(col: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    if (look.fog_a.w < 0.5) {
        return col;
    }
    let jitter = vec2<f32>(fbm(p * 0.035), fbm(p * 0.035 + vec2<f32>(31.7, 5.3))) - 0.5;
    let q = (p + jitter * look.fog_veil.w) / look.fog_map.xy;
    let seen = textureSampleLevel(fog_mask, fog_mask_sampler, q, 0.0).r;
    let fogged = 1.0 - smoothstep(0.32, 0.68, seen);
    if (fogged <= 0.0) {
        return col;
    }
    let lum_w = vec3<f32>(0.2126, 0.7152, 0.0722);
    let lum = dot(col, lum_w);
    let veil = look.fog_veil.rgb;
    let sepia = veil / max(dot(veil, lum_w), 1e-3);
    var ground = mix(col, lum * sepia, look.fog_a.x) * look.fog_a.y;
    ground = mix(ground, veil * look.fog_a.y, look.fog_a.z);
    let drift = vec2<f32>(0.020, 0.008) * globals.time;
    let s = p * look.fog_map.w;
    let warp = vec2<f32>(fbm(s * 0.5 + drift * 0.6), fbm(s * 0.5 + vec2<f32>(7.3, 2.9) - drift * 0.4));
    let banks = fbm(s + warp * 1.6 + drift);
    let density = smoothstep(0.26, 0.78, banks);
    let mist = mix(look.fog_map.z, look.fog_mist.w, density) * smoothstep(0.0, 1.0, fogged);
    return mix(mix(col, ground, fogged), look.fog_mist.rgb * look.fog_a.y, mist);
}

// Bevy `DistanceFog` of the camera (weather haze), which the custom shader must honour.
fn camera_distance_fog(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if (fog.mode == mesh_view_types::FOG_MODE_OFF) {
        return color;
    }
    let distance = length(world - view.world_position);
    let c = vec4<f32>(color, 1.0);
    let none = vec3<f32>(0.0);
    if (fog.mode == mesh_view_types::FOG_MODE_LINEAR) {
        return bevy_fog::linear_fog(fog, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL) {
        return bevy_fog::exponential_fog(fog, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED) {
        return bevy_fog::exponential_squared_fog(fog, c, distance, none).rgb;
    }
    return color;
}

// Strategic parchment look (simplified `parchment_map.gdshaderinc`): aged paper, hatched slopes,
// inked mountains, greenish forests, washed sea, burnt margins. `base` is the ground colour
// (unlit, seasonal), `n` the displayed normal, `footprint` the map px per screen px.
fn parchment_ground(base: vec3<f32>, n: vec3<f32>, altitude: f32, p: vec2<f32>, footprint: f32) -> vec3<f32> {
    let paper = vec3<f32>(0.80, 0.67, 0.43);
    let paper_dark = vec3<f32>(0.52, 0.36, 0.17);
    let ink = vec3<f32>(0.035, 0.020, 0.010);
    let stain = fbm(p * 0.0045 + vec2<f32>(3.1, 1.7));
    let blot = smoothstep(0.62, 0.8, fbm(p * 0.011 + vec2<f32>(7.3, 2.9)));
    let fibre = vnoise(p * vec2<f32>(0.9, 0.22) / max(footprint, 0.5)) - 0.5;
    var col = paper * (0.88 + 0.2 * stain) * (1.0 + 0.035 * fibre);
    col = mix(col, paper_dark, blot * 0.22);
    let land = smoothstep(-0.5, 1.5, altitude);
    // Forest: dark green ground colours get a green wash.
    let luma = dot(base, vec3<f32>(0.299, 0.587, 0.114));
    let greenness = smoothstep(0.015, 0.05, base.g - max(base.r, base.b));
    let forest = greenness * (1.0 - smoothstep(0.07, 0.16, luma));
    col = mix(col, col * vec3<f32>(0.86, 0.95, 0.78), forest * 0.7);
    // Relief: diagonal hatching on slopes facing away from the north-west light.
    let light = normalize(vec3<f32>(-0.7, 1.1, -0.7));
    let shade = clamp(1.0 - dot(n, light) * 1.05, 0.0, 1.0);
    let slope = length(n.xz) / max(n.y, 0.05);
    let mountain = smoothstep(600.0, 1000.0, altitude);
    let amount = smoothstep(0.35, 1.2, slope) * smoothstep(0.05, 0.35, shade);
    let spacing = 5.0 * exp2(max(floor(log2(max(footprint, 0.5) * 1.6)), 0.0));
    let s1 = abs(fract(dot(p, vec2<f32>(0.7071, 0.7071)) / spacing) - 0.5) * 2.0;
    let s2 = abs(fract(dot(p, vec2<f32>(0.7071, -0.7071)) / spacing) - 0.5) * 2.0;
    let width = clamp(amount, 0.0, 1.0) * 0.55;
    let aa = footprint / spacing * 2.0;
    let h1 = 1.0 - smoothstep(width - aa, width + aa, s1);
    let h2 = (1.0 - smoothstep(width * 0.8 - aa, width * 0.8 + aa, s2)) * step(0.55, shade);
    let hatch = max(h1, h2 * 0.8) * step(0.001, amount);
    col = mix(col, ink * 3.0, hatch * 0.55 * land);
    // Massifs: darker paper, heavier hatching.
    col = mix(col, paper_dark * 0.9, mountain * shade * 0.45 * land);
    col = mix(col, ink * 2.5, mountain * hatch * 0.35 * land);
    // Sea wash.
    col = mix(vec3<f32>(0.40, 0.56, 0.54) * (0.9 + 0.15 * stain), col, land);
    return col;
}

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    var albedo = vec3<f32>(0.10, 0.15, 0.06);
    if (params.map.w > 0.5) {
        albedo = textureSample(colormap, colormap_sampler, in.world.xz / params.map.xy).rgb;
    }
    let n = normalize(in.normal);
    let base_albedo = albedo;
    albedo = season_ground(albedo, n, in.altitude, in.world.xz);
    albedo = weather_ground(albedo, n, in.altitude, in.world.xz);
    albedo = fog_of_war_veil(albedo, in.world.xz);
    let ndl = max(dot(n, params.sun_dir.xyz), 0.0);
    var color = albedo * (params.ambient.rgb + params.sun_color.rgb * ndl);
    // Strategic view: fade to the parchment map (ADR 0124), unlit like the Godot emission.
    let parchment = look.weather.z;
    if (parchment > 0.001) {
        let k_proj = view.clip_from_view[1][1] * view.viewport.w * 0.5;
        let footprint = distance(in.world, view.world_position) / max(k_proj, 1.0);
        var paper = parchment_ground(base_albedo, n, in.altitude, in.world.xz, footprint);
        // The veil of the fog of war survives on the parchment as a darker wash.
        paper = fog_of_war_veil(paper, in.world.xz);
        color = mix(color, paper * 0.95, parchment);
    }
    // Distance fog (depth fog of the Godot environment, simplified).
    let d = distance(in.world, view.world_position);
    let begin = params.map.z;
    let fog_t = clamp((d - begin) / max(params.edge.a - begin, 1.0), 0.0, 1.0);
    color = mix(color, params.fog.rgb, pow(fog_t, 1.6) * params.fog.a * (1.0 - parchment));
    // Weather haze of the camera (`DistanceFog`).
    color = camera_distance_fog(color, in.world);
    // Map edge drowned in haze.
    let edge_px = params.scale.w;
    if (edge_px > 0.0) {
        let q = min(in.world.xz, params.map.xy - in.world.xz);
        let haze = 1.0 - smoothstep(0.0, edge_px, min(q.x, q.y));
        color = mix(color, params.edge.rgb, haze);
    }
    return vec4<f32>(color, 1.0);
}
