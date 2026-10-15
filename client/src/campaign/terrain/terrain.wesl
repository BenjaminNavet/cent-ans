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
    // x: saturation, y: contrast, z: shadow saturation, w: strength of the grade (Godot Environment adjustment).
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
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

// Regional ground (ground.rs, TX ADR 0243): 45 layers by (biome, role), biome blend, splat, grain.
struct GroundParams {
    // layer of (biome, role): index biome * 7 + role.
    layers: array<vec4<u32>, 27>,
    means: array<vec4<f32>, 48>,
    hues: array<vec4<f32>, 48>,
    micro_layers: array<vec4<u32>, 2>,
    micro_mean: array<vec4<f32>, 2>,
    // 1 when bound, blend width (texels), min blend weight, hue keep.
    bg: vec4<f32>,
    // tile screen px, base tile (map px), detail strength, detail normal strength.
    tiling: vec4<f32>,
    // micro on, micro tile (m), micro strength, metres per map px.
    micro: vec4<f32>,
    // detail fade footprints (xy), micro fade footprints (zw).
    fades: vec4<f32>,
    // splat bound, rock line (m).
    splat: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(7) var<uniform> ground: GroundParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) var bg_albedo: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) var ground_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) var bg_normal: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(11) var ab_dist: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(12) var ab_dist_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(13) var splat: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(14) var splat_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(15) var micro_albedo: texture_2d_array<f32>;

// Painted fields table (fields.rs): crops by biome / landscape, view and geo tuning.
struct FieldParams {
    rows_a: array<vec4<f32>, 24>,
    rows_b: array<vec4<f32>, 24>,
    slots: array<vec4<f32>, 288>,
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    v5: vec4<f32>,
    v6: vec4<f32>,
    v7: vec4<f32>,
    v8: vec4<f32>,
    vine: vec4<f32>,
    meadow: vec4<f32>,
    hedge: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(16) var<uniform> fields: FieldParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(17) var height_lod: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(18) var wash_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(19) var wash_sampler: sampler;

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
    // altitude gradient (m per map unit)
    @location(3) slope_xz: vec2<f32>,
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
    out.slope_xz = v.slope.xy;
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

// Colour grade of the campaign environment (`campaign.seasons.<season>.grade`): contrast about the
// mid grey, lift in the shadows, highlight tint, saturation that drops further in the shadows.
fn campaign_grade(c: vec3<f32>) -> vec3<f32> {
    if (params.grade.w <= 0.0) {
        return c;
    }
    var col = (c - vec3<f32>(0.5)) * params.grade.y + vec3<f32>(0.5);
    col = max(col, vec3<f32>(0.0));
    let luma = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = col + params.lift.rgb * (1.0 - smoothstep(0.0, 0.5, luma));
    col = col * mix(vec3<f32>(1.0), params.highlights.rgb, smoothstep(0.3, 0.9, luma));
    let sat = mix(params.grade.z, params.grade.x, smoothstep(0.02, 0.35, luma));
    col = vec3<f32>(luma) + (col - vec3<f32>(luma)) * sat;
    return mix(c, max(col, vec3<f32>(0.0)), params.grade.w);
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

// Strategic parchment look (port of `parchment_map.gdshaderinc`, lot CM2): aged paper, watercolour
// wash of the realms (pigment pooling at the borders), hatched slopes, drawn mountains and trees,
// washed sea, burnt margins. `base` is the ground colour (unlit, seasonal), `n` the displayed
// normal, `footprint` the map px per screen px. Province / realm borders are the draped lines of
// the provinces module.
const PM_MOUNT_CELL = 26.0;
const PM_MOUNT_MIN_M = 750.0;
const PM_TREE_CELL = 12.0;
const PM_TREE_MIN = 0.56;
const PM_WASH_STRENGTH = 0.55;
const PM_PAPER = vec3<f32>(0.80, 0.67, 0.43);
const PM_PAPER_DARK = vec3<f32>(0.52, 0.36, 0.17);
const PM_INK = vec3<f32>(0.035, 0.020, 0.010);
const PM_GREEN_INK = vec3<f32>(0.07, 0.17, 0.05);

fn pm_hash2(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash21(p), hash21(p + vec2<f32>(19.19, 7.13)));
}

fn pm_line(d: f32, w: f32, footprint: f32) -> f32 {
    let ds = abs(d) / max(footprint, 1e-4);
    return 1.0 - smoothstep(w - 0.7, w + 0.7, ds);
}

// Coarse altitude (m) at a map point: bilinear on the 16 px block raster.
fn pm_height(p: vec2<f32>) -> f32 {
    let dims = vec2<i32>(textureDimensions(height_lod));
    let t = p / 16.0 - 0.5;
    let i0 = vec2<i32>(floor(t));
    let f = fract(t);
    let hi = dims - vec2<i32>(1);
    let a = textureLoad(height_lod, clamp(i0, vec2<i32>(0), hi), 0).r;
    let b = textureLoad(height_lod, clamp(i0 + vec2<i32>(1, 0), vec2<i32>(0), hi), 0).r;
    let c = textureLoad(height_lod, clamp(i0 + vec2<i32>(0, 1), vec2<i32>(0), hi), 0).r;
    let d = textureLoad(height_lod, clamp(i0 + vec2<i32>(1, 1), vec2<i32>(0), hi), 0).r;
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// Mountain of a cell: bell shape, east slope hatched; returns (cover, ink, shade).
fn pm_mountain(p: vec2<f32>, cell: vec2<f32>, footprint: f32) -> vec3<f32> {
    let jitter = pm_hash2(cell) - 0.5;
    let base = (cell + vec2<f32>(0.5, 0.85) + jitter * vec2<f32>(0.5, 0.2)) * PM_MOUNT_CELL;
    let h = pm_height(base + vec2<f32>(0.0, -PM_MOUNT_CELL * 0.3));
    if (h < PM_MOUNT_MIN_M) {
        return vec3<f32>(0.0);
    }
    let size = PM_MOUNT_CELL * mix(0.55, 1.05, smoothstep(PM_MOUNT_MIN_M, 2600.0, h)) * (0.85 + 0.3 * hash21(cell + 3.1));
    let q = (p - base) / size;
    let x = q.x * 2.0;
    var peak = 1.0 - pow(abs(x), 1.25);
    peak = peak * (1.0 + 0.12 * sin(x * 9.0 + hash21(cell) * 6.0));
    let top = -peak * mix(0.75, 1.05, hash21(cell + 7.7));
    let inside = step(abs(x), 1.0) * step(top, q.y) * step(q.y, 0.0);
    if (inside <= 0.0 && abs(x) > 1.1) {
        return vec3<f32>(0.0);
    }
    let d_top = abs(q.y - top) * size;
    var outline = 0.0;
    if (inside > 0.0 || d_top < footprint * 1.5) {
        outline = pm_line(d_top, 1.0, footprint) * step(abs(x), 1.0);
    }
    let shade_side = smoothstep(-0.05, 0.1, x) * inside;
    let hatch = smoothstep(0.55, 0.8, abs(fract(p.x / max(footprint * 3.0, 1.2)) - 0.5) * 2.0);
    let ridge = pm_line((x - 0.05 * sin(q.y * 7.0)) * size * 0.5, 0.5, footprint) * inside * step(-0.85 * peak, q.y);
    return vec3<f32>(inside, max(outline, ridge * 0.6), shade_side * (0.25 + 0.75 * hatch));
}

// Tree of a cell (round crown + trunk): (cover, ink, shade).
fn pm_tree(p: vec2<f32>, cell: vec2<f32>, footprint: f32) -> vec3<f32> {
    let jitter = pm_hash2(cell + 11.0) - 0.5;
    let base = (cell + vec2<f32>(0.5, 0.8) + jitter * 0.35) * PM_TREE_CELL;
    let sp = textureSampleLevel(splat, splat_sampler, base / params.map.xy, 0.0);
    let forest = sp.b / max(sp.r + sp.g + sp.b + sp.a, 1e-3);
    if (forest < PM_TREE_MIN + (hash21(cell) - 0.5) * 0.25 || hash21(cell + 2.7) < 0.35 || pm_height(base) <= 0.0) {
        return vec3<f32>(0.0);
    }
    let r = PM_TREE_CELL * (0.3 + 0.08 * hash21(cell + 5.0));
    let crown = base + vec2<f32>(0.0, -r * 1.35);
    let dc = length(p - crown) - r;
    let trunk_d = max(abs(p.x - base.x), abs(p.y - (base.y - r * 0.35)) - r * 0.45);
    let cover = max(step(dc, 0.0), step(trunk_d, footprint * 0.5));
    let ink = max(pm_line(dc, 0.7, footprint), pm_line(trunk_d, 0.55, footprint));
    let shade = step(dc, 0.0) * smoothstep(-0.2, 0.6, dot(normalize(p - crown + vec2<f32>(1e-4)), vec2<f32>(0.7, 0.7)));
    return vec3<f32>(cover, ink, shade);
}

fn parchment_ground(base: vec3<f32>, n: vec3<f32>, altitude: f32, p: vec2<f32>, uv: vec2<f32>, footprint: f32) -> vec3<f32> {
    let stain = fbm(p * 0.0045 + vec2<f32>(3.1, 1.7));
    let blot = smoothstep(0.62, 0.8, fbm(p * 0.011 + vec2<f32>(7.3, 2.9)));
    let fibre = vnoise(p * vec2<f32>(0.9, 0.22) / max(footprint, 0.5)) - 0.5;
    var col = PM_PAPER * (0.88 + 0.2 * stain) * (1.0 + 0.035 * fibre);
    col = mix(col, PM_PAPER_DARK, blot * 0.22);
    let land = smoothstep(-0.5, 1.5, altitude);

    // Watercolour wash of the realms: owner pigment, granular, pooling near the borders.
    let jitter = (vec2<f32>(vnoise(p * 0.6), vnoise(p * 0.6 + vec2<f32>(5.0, 3.0))) - 0.5) * 3.0;
    let q = (p + jitter) / params.map.xy;
    let wash_here = textureSampleLevel(wash_tex, wash_sampler, q, 0.0);
    if (wash_here.a > 0.5) {
        let step_uv = 6.0 / params.map.xy;
        var diff = 0.0;
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, wash_sampler, q + vec2<f32>(step_uv.x, 0.0), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, wash_sampler, q - vec2<f32>(step_uv.x, 0.0), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, wash_sampler, q + vec2<f32>(0.0, step_uv.y), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, wash_sampler, q - vec2<f32>(0.0, step_uv.y), 0.0)));
        let pool = smoothstep(0.02, 0.2, diff);
        var wash = wash_here.rgb / max(max(wash_here.r, max(wash_here.g, wash_here.b)), 0.05);
        let gran = 0.8 + 0.4 * vnoise(p * 0.35) * vnoise(p * 0.07 + vec2<f32>(4.0));
        let strength = PM_WASH_STRENGTH * gran * (1.0 + 0.9 * pool);
        col = col * mix(vec3<f32>(1.0), wash, clamp(strength, 0.0, 0.95) * land);
    }

    // Relief: diagonal hatching on slopes facing away from the north-west light.
    let light = normalize(vec3<f32>(-0.7, 1.1, -0.7));
    let shade = clamp(1.0 - dot(n, light) * 1.05, 0.0, 1.0);
    let slope = length(n.xz) / max(n.y, 0.05);
    let hc = pm_height(p);
    let mountain_zone = smoothstep(PM_MOUNT_MIN_M - 150.0, PM_MOUNT_MIN_M + 250.0, hc);
    let amount = smoothstep(0.35, 1.2, slope) * smoothstep(0.05, 0.35, shade) * (1.0 - mountain_zone * 0.85);
    let spacing = 5.0 * exp2(max(floor(log2(max(footprint, 0.5) * 1.6)), 0.0));
    let s1 = abs(fract(dot(p, vec2<f32>(0.7071, 0.7071)) / spacing) - 0.5) * 2.0;
    let s2 = abs(fract(dot(p, vec2<f32>(0.7071, -0.7071)) / spacing) - 0.5) * 2.0;
    let width = clamp(amount, 0.0, 1.0) * 0.55;
    let aa = footprint / spacing * 2.0;
    let h1 = 1.0 - smoothstep(width - aa, width + aa, s1);
    let h2 = (1.0 - smoothstep(width * 0.8 - aa, width * 0.8 + aa, s2)) * step(0.55, shade);
    let hatch = max(h1, h2 * 0.8) * step(0.001, amount);
    col = mix(col, PM_INK * 3.0, hatch * 0.55 * land);

    // Drawn mountains: the cell and its southern neighbours, which pass in front.
    if (hc > PM_MOUNT_MIN_M - 500.0) {
        let cm = floor(p / PM_MOUNT_CELL);
        for (var j = -1; j <= 1; j = j + 1) {
            for (var i = -1; i <= 1; i = i + 1) {
                let m = pm_mountain(p, cm + vec2<f32>(f32(i), f32(j)), footprint);
                if (m.x > 0.0 || m.y > 0.0) {
                    let fill = mix(PM_PAPER * vec3<f32>(1.04, 1.0, 0.92), PM_PAPER_DARK * 0.55, m.z * 0.8);
                    col = mix(col, fill, m.x * land);
                    col = mix(col, PM_INK, m.y * land);
                }
            }
        }
    }

    // Forests: green wash and small drawn trees (the cell and its southern row).
    let sp = textureSampleLevel(splat, splat_sampler, uv, 0.0);
    var forest_here = sp.b / max(sp.r + sp.g + sp.b + sp.a, 1e-3);
    if (ground.splat.x < 0.5) {
        let luma_base = dot(base, vec3<f32>(0.299, 0.587, 0.114));
        forest_here = smoothstep(0.015, 0.05, base.g - max(base.r, base.b)) * (1.0 - smoothstep(0.07, 0.16, luma_base));
    }
    col = mix(col, col * vec3<f32>(0.86, 0.95, 0.78), smoothstep(PM_TREE_MIN - 0.15, PM_TREE_MIN + 0.2, forest_here) * land * 0.6);
    if (mountain_zone < 0.6 && forest_here > PM_TREE_MIN - 0.3 && ground.splat.x > 0.5) {
        let ct = floor(p / PM_TREE_CELL);
        for (var j = 0; j <= 1; j = j + 1) {
            for (var i = -1; i <= 1; i = i + 1) {
                let t = pm_tree(p, ct + vec2<f32>(f32(i), f32(j)), footprint);
                if (t.x > 0.0 || t.y > 0.0) {
                    let leaf = mix(PM_GREEN_INK * 3.2 + PM_PAPER * 0.25, PM_GREEN_INK * 1.4, t.z);
                    col = mix(col, leaf, t.x * land);
                    col = mix(col, PM_INK * 1.5, t.y * land);
                }
            }
        }
    }
    // Sea wash.
    col = mix(vec3<f32>(0.40, 0.56, 0.54) * (0.9 + 0.15 * stain), col, land);
    return col;
}

const TINT_GRASS = vec3<f32>(0.105, 0.165, 0.045);
const TINT_FARM = vec3<f32>(0.21, 0.20, 0.075);
const TINT_FOREST = vec3<f32>(0.054, 0.1023, 0.0275);
const TINT_ROCK = vec3<f32>(0.19, 0.17, 0.15);
const TINT_HEATH = vec3<f32>(0.15, 0.11, 0.08);
const ROCK_LINE_M = 1700.0;

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn bg_layer(role: u32, biome: u32) -> u32 {
    let i = biome * 7u + role;
    return ground.layers[i / 4u][i % 4u];
}

struct LayerAcc {
    albedo: vec3<f32>,
    normal: vec2<f32>,
};

// One layer at two scales (anti-repetition): tinted albedo + normal, weighted by w.
fn sample_layer(layer: u32, uv_a: vec2<f32>, uv_b: vec2<f32>, da: vec4<f32>, db: vec4<f32>, mix_b: f32,
        tint: vec3<f32>, detail: f32, w: f32, acc: LayerAcc) -> LayerAcc {
    let li = i32(layer);
    let ta = textureSampleGrad(bg_albedo, ground_sampler, uv_a, li, da.xy, da.zw).rgb;
    let tb = textureSampleGrad(bg_albedo, ground_sampler, uv_b, li, db.xy, db.zw).rgb;
    let t = mix(ta, tb, mix_b);
    let na = textureSampleGrad(bg_normal, ground_sampler, uv_a, li, da.xy, da.zw).rg;
    let nb = textureSampleGrad(bg_normal, ground_sampler, uv_b, li, db.xy, db.zw).rg;
    let nm = mix(na, nb, mix_b);
    var ratio = t / max(ground.means[layer].rgb, vec3<f32>(0.01));
    // Detail mostly in luminance: no coloured reflections of the photos.
    ratio = mix(vec3<f32>(luma(ratio)), ratio, 0.35);
    let hue = mix(vec3<f32>(1.0), ground.hues[layer].rgb, ground.bg.w * clamp(detail, 0.0, 1.0));
    var out = acc;
    out.albedo = out.albedo + w * tint * hue * mix(vec3<f32>(1.0), ratio, detail);
    out.normal = out.normal + w * (nm * 2.0 - 1.0);
    return out;
}

// One role of the background, blended between the pixel biome (a) and its neighbour (b).
fn sample_role(role: u32, ba: u32, bb: u32, wb: f32, uv_a: vec2<f32>, uv_b: vec2<f32>, da: vec4<f32>,
        db: vec4<f32>, mix_b: f32, tint: vec3<f32>, detail: f32, w: f32, acc: LayerAcc) -> LayerAcc {
    let la = bg_layer(role, ba);
    let lb = bg_layer(role, bb);
    if (la == lb || wb <= ground.bg.z) {
        return sample_layer(la, uv_a, uv_b, da, db, mix_b, tint, detail, w, acc);
    }
    let first = sample_layer(la, uv_a, uv_b, da, db, mix_b, tint, detail, w * (1.0 - wb), acc);
    return sample_layer(lb, uv_a, uv_b, da, db, mix_b, tint, detail, w * wb, first);
}

struct GroundOut {
    color: vec3<f32>,
    // normal perturbation (map frame), already faded by the detail range
    dn: vec2<f32>,
    // share of grass + farmland + heath, of forest, farmland among grass + farmland
    open_w: f32,
    forest_w: f32,
    farm_share: f32,
};

// Procedural ground of `terrain.gdshader` (splat weights x regional layers x micro grain), without
// the parcels, snow and sand. Called in uniform control flow (derivatives).
fn ground_proc(p: vec2<f32>, uv: vec2<f32>, footprint: f32, altitude: f32, slope: f32) -> GroundOut {
    var out: GroundOut;
    let detail_fade = 1.0 - smoothstep(ground.fades.x, ground.fades.y, footprint);
    let det = ground.tiling.z * detail_fade;
    // Tile scale adapted to the zoom: a tile keeps ~tile_screen_px on screen; two octaves blend,
    // the second one rotated.
    let lvl = max(log2(ground.tiling.x * max(footprint, 1e-4) / ground.tiling.y), -7.0);
    let tile_a = ground.tiling.y * exp2(floor(lvl));
    let ang = 0.9;
    let rot = mat2x2<f32>(vec2<f32>(cos(ang), sin(ang)), vec2<f32>(-sin(ang), cos(ang)));
    let uv_a = p / tile_a + vec2<f32>(0.21 * floor(lvl));
    let uv_b = rot * p / (tile_a * 2.0) + vec2<f32>(0.37);
    let mix_b = fract(lvl);
    let da = vec4<f32>(dpdx(uv_a), dpdy(uv_a));
    let db = vec4<f32>(dpdx(uv_b), dpdy(uv_b));
    let muv = p * ground.micro.w / max(ground.micro.y, 0.01);
    let mdx = dpdx(muv);
    let mdy = dpdy(muv);
    // Soil weights.
    var s = vec4<f32>(0.5, 0.3, 0.2, 0.0);
    if (ground.splat.x > 0.5) {
        s = textureSampleLevel(splat, splat_sampler, uv, 0.0);
    }
    s = s / max(s.r + s.g + s.b + s.a, 1e-3);
    let rock_frac = max(smoothstep(ROCK_LINE_M - 700.0, ROCK_LINE_M, altitude), smoothstep(0.35, 0.6, slope));
    // Landscape mask (agri_regions): part of the grassland turned to crops.
    let agri_raw = u32(textureLoad(ab_dist, vec2<i32>(clamp(uv * vec2<f32>(textureDimensions(ab_dist)), vec2<f32>(0.0), vec2<f32>(textureDimensions(ab_dist)) - 1.0)), 0).b * 255.0 + 0.5);
    if (agri_raw > 0u && fields.v8.w > 0.5) {
        let moved = s.r * fields.rows_b[min(agri_raw + 7u, 23u)].x;
        s = vec4<f32>(s.r - moved, s.g + moved, s.b, s.a);
    }
    let w_grass = s.r;
    let w_farm = s.g;
    let w_forest = s.b;
    let w_rock = s.a * rock_frac;
    let w_heath = s.a * (1.0 - rock_frac);
    let wsum = max(w_grass + w_farm + w_forest + w_rock + w_heath, 1e-3);
    // Biome of the pixel, nearest neighbour and its weight (0.5 on the border).
    let dims = vec2<f32>(textureDimensions(ab_dist));
    let packed = u32(textureLoad(ab_dist, vec2<i32>(clamp(uv * dims, vec2<f32>(0.0), dims - 1.0)), 0).r * 255.0 + 0.5);
    let ba = clamp(packed % 16u, 1u, 14u);
    let bb = clamp(packed / 16u, 1u, 14u);
    let dist_texels = textureSampleLevel(ab_dist, ab_dist_sampler, uv, 0.0).g * 255.0;
    let wb = 0.5 * (1.0 - smoothstep(0.0, ground.bg.y, dist_texels));
    var acc = LayerAcc(vec3<f32>(0.0), vec2<f32>(0.0));
    if (w_grass > 0.01) { acc = sample_role(0u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_GRASS, det, w_grass, acc); }
    if (w_farm > 0.01) { acc = sample_role(1u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_FARM, det, w_farm, acc); }
    if (w_forest > 0.01) { acc = sample_role(2u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_FOREST, det, w_forest, acc); }
    if (w_rock > 0.01) { acc = sample_role(3u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_ROCK, det, w_rock, acc); }
    if (w_heath > 0.01) { acc = sample_role(4u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_HEATH, det, w_heath, acc); }
    var col = acc.albedo / wsum;
    out.dn = acc.normal / wsum * ground.tiling.w * detail_fade;
    // Micro grain by dominant material (luminance only).
    let micro_k = ground.micro.x * ground.micro.z * (1.0 - smoothstep(ground.fades.z, ground.fades.w, footprint));
    var role = 0u;
    var best = w_grass;
    if (w_farm > best) { best = w_farm; role = 1u; }
    if (w_forest > best) { best = w_forest; role = 2u; }
    if (w_rock > best) { best = w_rock; role = 3u; }
    if (w_heath > best) { best = w_heath; role = 4u; }
    let mlayer = ground.micro_layers[role / 4u][role % 4u];
    let grain = textureSampleGrad(micro_albedo, ground_sampler, muv, i32(mlayer), mdx, mdy).rgb;
    let g_ratio = luma(grain) / max(ground.micro_mean[mlayer / 4u][mlayer % 4u], 0.01);
    col = col * mix(1.0, clamp(g_ratio, 0.4, 1.8), micro_k);
    // Canopy: lighter volume shading, forests darker and grainier.
    col = col * (1.0 - 0.25 * w_forest / wsum * (1.0 - vnoise(p * 1.7)) * detail_fade);
    out.color = col;
    out.open_w = (w_grass + w_farm + w_heath) / wsum;
    out.forest_w = w_forest / wsum;
    out.farm_share = w_farm / max(w_grass + w_farm, 1e-3);
    return out;
}

// --- Painted fields (hb_ground.gdshaderinc: hb_apply, farmed parcels and the far patchwork) ---
fn hb_hash(q: vec2<f32>) -> f32 {
    return fract(sin(dot(q, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn hb_rot(a: f32) -> mat2x2<f32> {
    return mat2x2<f32>(vec2<f32>(cos(a), sin(a)), vec2<f32>(-sin(a), cos(a)));
}

struct Vor {
    id: vec2<f32>,
    site: vec2<f32>,
    edge: f32,
};

// Additively weighted Voronoi cells in parcel units: id, site, approximate distance to the edge.
fn hb_voronoi(u: vec2<f32>) -> Vor {
    let ic = floor(u);
    var d1 = 1e9;
    var d2 = 1e9;
    var id1 = ic;
    var s1 = ic;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let id = ic + vec2<f32>(f32(i), f32(j));
            let st = id + 0.1 + 0.8 * vec2<f32>(hb_hash(id + 3.1), hb_hash(id + 17.7));
            let d = length(u - st) - (hb_hash(id + 41.3) - 0.5) * 0.7;
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id1 = id;
                s1 = st;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return Vor(id1, s1, (d2 - d1) * 0.5);
}

// Biome A, landscape mask value and river distance (px, signed) at a map uv.
struct MapCell {
    biome: u32,
    agri: u32,
    river_px: f32,
};

fn map_cell(uv: vec2<f32>) -> MapCell {
    let dims = vec2<f32>(textureDimensions(ab_dist));
    let t = textureLoad(ab_dist, vec2<i32>(clamp(uv * dims, vec2<f32>(0.0), dims - 1.0)), 0);
    let packed = u32(t.r * 255.0 + 0.5);
    let river = (textureSampleLevel(ab_dist, ab_dist_sampler, uv, 0.0).a * 255.0 - 128.0) / 16.0;
    return MapCell(clamp(packed % 16u, 1u, 14u), u32(t.b * 255.0 + 0.5), river);
}

fn field_row(cell: MapCell) -> u32 {
    if (cell.agri > 0u) {
        return min(cell.agri + 7u, 23u);
    }
    return cell.biome;
}

fn hb_slot(row: u32, h: f32) -> i32 {
    for (var i = 0u; i < 12u; i = i + 1u) {
        let slot = fields.slots[row * 12u + i];
        if (slot.w > 1.5) {
            return -1;
        }
        if (h <= slot.w) {
            return i32(i);
        }
    }
    return -1;
}

// Crop of a cell: 70 % follow the dominant crop of their region; -2 meadow, -3 vine, -1 none.
fn hb_maille(id: vec2<f32>, row: u32, h_dom: f32, vine_k: f32, meadow_k: f32) -> i32 {
    if (fields.meadow.w > 0.5 && hb_hash(id + 23.3) < meadow_k) {
        return -2;
    }
    if (fields.vine.w > 0.5 && hb_hash(id + 19.1) < vine_k) {
        return -3;
    }
    var h = hb_hash(id + 27.7);
    if (hb_hash(id + 5.9) < 0.7) {
        h = clamp(h_dom + (h - 0.5) * 0.5, 0.0, 1.0);
    }
    return hb_slot(row, h);
}

fn hb_maille_far(m: i32, row: u32) -> vec3<f32> {
    if (m >= 0) {
        return fields.slots[row * 12u + u32(m)].rgb;
    }
    if (m == -3) {
        return fields.vine.rgb;
    }
    return fields.meadow.rgb;
}

fn hb_patch_color(id: vec2<f32>, far: vec3<f32>) -> vec3<f32> {
    let lumv = 1.0 + 2.0 * fields.v1.y * (hb_hash(id + 3.3) - 0.5);
    let c = far * lumv;
    return max(mix(vec3<f32>(luma(c)), c, fields.v1.w), vec3<f32>(0.0));
}

// The luminance of `c` stays within `lum_band` (relative) of the local ground colour.
fn hb_relative(base: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let lb = max(luma(base), 1e-3);
    let lc = max(luma(c), 1e-3);
    return c * (clamp(lc, lb * (1.0 - fields.v5.y), lb * (1.0 + fields.v5.y)) / lc);
}

fn hb_far_patch(uu: vec2<f32>, footprint: f32, px_u: f32, row: u32, h_dom: f32, vine_k: f32, meadow_k: f32) -> vec3<f32> {
    let k = smoothstep(fields.v3.w, fields.v4.x, footprint);
    var c = vec3<f32>(0.0);
    var e = 0.0;
    let sole_scale = fields.v2.y;
    if (k < 0.999) {
        let v = hb_voronoi(uu * sole_scale);
        c = hb_patch_color(v.id, hb_maille_far(hb_maille(v.id, row, h_dom, vine_k, meadow_k), row)) * (1.0 - k);
        e = v.edge / max(sole_scale, 1e-3) * (1.0 - k);
    }
    if (k > 0.001) {
        let v = hb_voronoi(uu * sole_scale * fields.v3.z + 53.0);
        let id = v.id + 0.5;
        c = c + hb_patch_color(id, hb_maille_far(hb_maille(id, row, h_dom, vine_k, meadow_k), row)) * k;
        e = e + v.edge / max(sole_scale * fields.v3.z, 1e-3) * k;
    }
    let line = 1.0 - smoothstep(0.0, 1.2 * px_u, e);
    return c * (1.0 - fields.v4.y * line);
}

// Paints the fields on `col` (the colormap-blended ground): near and mid view, the parcels of
// the landscape (crop colour per parcel, hedges, rims); far, the patchwork of soles and terroirs.
// Crops follow the biome / landscape table, relief (altitude, slope, south slopes for vines) and
// water (meadows by the rivers) decide where they grow (GR, ADR 0352).
fn fields_apply(p: vec2<f32>, uv: vec2<f32>, footprint: f32, land: f32, open_w: f32, farm_share: f32,
        slope: f32, grad: vec2<f32>, alt: f32, col_in: vec3<f32>) -> vec3<f32> {
    var col = col_in;
    let mpp = max(ground.micro.w, 1.0);
    let fade = smoothstep(fields.v0.x, fields.v0.y, footprint);
    let w_bare = fields.v5.x * (1.0 - fade) * land;
    var w_far = 0.0;
    if (fields.v2.y > 0.0) {
        w_far = fields.v2.w * fade * (1.0 - smoothstep(fields.v3.x, fields.v3.y, footprint)) * land;
    }
    if (w_bare <= 0.001 && w_far <= 0.001) {
        return col;
    }
    let region_cells = fields.v6.x;
    let reg = hb_voronoi(p / region_cells + vec2<f32>(vnoise(p * 0.05), vnoise(p * 0.05 + 9.0)) * 0.6);
    let h_dom = hb_hash(reg.id + 9.9);
    let ang_r = hb_hash(reg.id + 41.0) * 3.1416;
    let origin = reg.site * region_cells;
    let map_px = params.map.xy;
    let cell0 = map_cell(p / map_px);
    let row0 = field_row(cell0);
    let cell_px = max(fields.rows_a[row0].x, 50.0) * fields.v2.x / mpp;
    let strip = clamp(fields.rows_a[row0].y, 1.0, 2.2);
    let rot_f = hb_rot(ang_r);
    var uu = rot_f * (p - origin) / cell_px;
    uu.x = uu.x / strip;
    uu = uu + (vec2<f32>(vnoise(uu * 0.9), vnoise(uu * 0.9 + 17.3)) - 0.5) * 0.55
        + (vec2<f32>(vnoise(uu * 3.3 + 7.7), vnoise(uu * 3.3 + 29.1)) - 0.5) * 0.14;
    uu = uu + reg.id * 37.0;
    let parcel = hb_voronoi(uu);
    let pc = transpose(rot_f) * (vec2<f32>((parcel.site.x - reg.id.x * 37.0) * strip, parcel.site.y - reg.id.y * 37.0) * cell_px) + origin;
    // The parcel is one biome / landscape: read at its centre.
    let cell_c = map_cell(pc / map_px);
    let row = field_row(cell_c);
    let hedge_k = fields.rows_a[row].z;
    var presence = fields.rows_a[row].w;
    // Relief and water: crops give way to pasture high up and on steep slopes; vines on sunny
    // slopes; meadows along the rivers on flat ground.
    var vine_k = 0.0;
    var meadow_k = 0.0;
    if (fields.v6.y > 0.5) {
        let south = -grad.y / max(length(grad), 1e-6);
        vine_k = fields.rows_b[row].y * smoothstep(fields.v7.z, fields.v7.z * 2.0, slope)
            * (1.0 - smoothstep(fields.v7.w, fields.v7.w * 1.4, slope))
            * smoothstep(fields.v8.x, fields.v8.x + 0.35, south);
        meadow_k = fields.v8.z * (1.0 - smoothstep(fields.v8.y * 0.5, fields.v8.y, cell0.river_px))
            * (1.0 - smoothstep(0.06, 0.18, slope));
        let relief_k = (1.0 - smoothstep(fields.v6.z, fields.v6.w, alt))
            * max(1.0 - smoothstep(fields.v7.x, fields.v7.y, slope), vine_k);
        presence = presence * relief_k;
    }
    let farm_frac = clamp(max(farm_share, fields.v4.w) * presence, 0.0, 1.0);
    if (w_far > 0.001) {
        let far_col = hb_far_patch(uu, footprint, footprint / cell_px, row, h_dom, vine_k, meadow_k);
        col = mix(col, hb_relative(col, far_col), w_far * open_w * farm_frac);
    }
    if (w_bare <= 0.001) {
        return col;
    }
    // Parcel: farmed or wild, by the farm share of the soil.
    if (hb_hash(parcel.id + 71.7) >= max(farm_share, fields.v4.w) * presence) {
        return col;
    }
    var key = parcel.id;
    var sole = false;
    if (fields.v2.y > 0.0 && hb_hash(parcel.id + 5.1) < fields.v2.z) {
        key = hb_voronoi(parcel.site * fields.v2.y).id;
        sole = true;
    }
    let m = hb_maille(key, row, h_dom, vine_k, meadow_k);
    if (m == -1) {
        return col;
    }
    let far = hb_maille_far(m, row);
    // Each parcel has its own nuance (maturity, soil, exposure).
    let j1 = hb_hash(parcel.id + 3.3) - 0.5;
    let j2 = hb_hash(parcel.id + 9.1) - 0.5;
    let own = (1.0 + 2.0 * fields.v1.y * j1) * vec3<f32>(1.0 + fields.v1.z * j2, 1.0, 1.0 - fields.v1.z * j2);
    var open_col = col * own;
    let patch_k = fields.v0.z * smoothstep(fields.v0.w, fields.v1.x, footprint);
    var crop = far * (1.0 + 2.0 * fields.v1.y * j1);
    if (sole) {
        crop = hb_patch_color(key, far);
    }
    // The crop colour keeps the grain of the textured soil under it.
    let grain = clamp(luma(col) / max(luma(col_in), 1e-3), 0.7, 1.35);
    open_col = mix(open_col, crop * grain, patch_k);
    // Hedges and banks on the edges of the cultivated parcels.
    let edge_m = parcel.edge * cell_px * mpp;
    let px_m = footprint * mpp;
    let hedge_w = fields.v5.w * hedge_k;
    var hedge = 0.0;
    if (hedge_k > 0.01) {
        hedge = (1.0 - smoothstep(0.0, max(px_m, hedge_w), edge_m - hedge_w))
            * clamp(2.0 * hedge_w / max(px_m, 1e-3), max(0.35, fields.v4.z), 1.0);
    }
    let balk = 1.0 - smoothstep(1.5, 1.5 + px_m, edge_m);
    open_col = mix(open_col, open_col * 0.82, balk * 0.6);
    let rim = 1.0 - smoothstep(0.0, max(footprint * 1.4, 0.02), parcel.edge * cell_px - 0.006);
    open_col = open_col * (1.0 - 0.28 * rim * (1.0 - smoothstep(0.1, 0.3, footprint)));
    open_col = mix(open_col, fields.hedge.rgb, hedge * 0.8);
    // Hue of the colour map kept in part, luminance within the band of the local colour.
    let lum_o = luma(open_col);
    let lum_c = max(luma(col_in), 1e-3);
    open_col = mix(open_col, col_in * (lum_o / lum_c), fields.v5.z);
    open_col = hb_relative(col_in, open_col);
    return mix(col, open_col, w_bare * open_w);
}

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    var albedo = vec3<f32>(0.10, 0.15, 0.06);
    if (params.map.w > 0.5) {
        albedo = textureSample(colormap, colormap_sampler, in.world.xz / params.map.xy).rgb;
    }
    var n = normalize(in.normal);
    let base_albedo = albedo;
    let k_proj_ground = view.clip_from_view[1][1] * view.viewport.w * 0.5;
    let ground_footprint = distance(in.world, view.world_position) / max(k_proj_ground, 1.0);
    if (ground.bg.x > 0.5) {
        // Regional ground (TX): the colormap gives the colour far away, the textured soil draws
        // close up (sg_apply of satellite_ground.gdshaderinc: share 0.45 at the closest, 1 far).
        let slope_m = length(in.slope_xz) / max(ground.micro.w, 1.0);
        let g = ground_proc(in.world.xz, in.world.xz / params.map.xy, ground_footprint, in.altitude, slope_m);
        let land = smoothstep(0.0, 2.0, in.altitude);
        var painted = g.color;
        if (params.map.w > 0.5) {
            let far_w = smoothstep(0.05, 0.5, ground_footprint);
            let ratio = clamp(luma(g.color) / max(luma(base_albedo), 1e-3), 0.55, 1.6);
            let tinted = base_albedo * mix(1.0, ratio, 0.55 * (1.0 - far_w));
            painted = mix(g.color, tinted, mix(0.45, 1.0, far_w));
        }
        // Painted fields over the ground colour (after the colormap, like hb_apply).
        if (fields.v8.w > 0.5) {
            painted = fields_apply(in.world.xz, in.world.xz / params.map.xy, ground_footprint, land,
                g.open_w, g.farm_share, slope_m, in.slope_xz, in.altitude, painted);
        }
        albedo = mix(base_albedo, painted, land);
        n = normalize(vec3<f32>(n.x + g.dn.x * land, n.y, n.z - g.dn.y * land));
    }
    albedo = season_ground(albedo, n, in.altitude, in.world.xz);
    albedo = weather_ground(albedo, n, in.altitude, in.world.xz);
    albedo = fog_of_war_veil(albedo, in.world.xz);
    let ndl = max(dot(n, params.sun_dir.xyz), 0.0);
    var color = albedo * (params.ambient.rgb + params.sun_color.rgb * ndl) * params.ambient.a;
    color = campaign_grade(color);
    // Strategic view: fade to the parchment map (ADR 0124), unlit like the Godot emission.
    let parchment = look.weather.z;
    if (parchment > 0.001) {
        let k_proj = view.clip_from_view[1][1] * view.viewport.w * 0.5;
        let footprint = distance(in.world, view.world_position) / max(k_proj, 1.0);
        var paper = parchment_ground(base_albedo, n, in.altitude, in.world.xz, in.world.xz / params.map.xy, footprint);
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
