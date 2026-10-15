// Campaign ground (port of game/shaders/terrain.gdshader + relief_quadtree.gdshaderinc, ADR 0142
// satellite colormap, ADR 0353 exaggerated relief). The CPU meshes carry the altitude in metres
// and the relief fields; the displayed height `y = s*(h - K*max(h-base,0) + g*(1-K)*max(h-floor,0)
// + c*D)` is computed here so that the dynamic vertical scale needs no mesh rebuild.

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals, fog, lights},
    mesh_view_types,
    fog as bevy_fog,
    shadows,
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
    // RV-C cumulus shadows: strength (0 off), scale, edge, drift / cover low, high, cloud height,
    // fade cell px / cool tint, fade cell px (none) / CPU-side tuning.
    cumulus_a: vec4<f32>,
    cumulus_b: vec4<f32>,
    cumulus_c: vec4<f32>,
    cumulus_d: vec4<f32>,
    // Morning mist: fog weight under the focus, morning share, wet dim, valley mist opacity.
    mist: vec4<f32>,
    // TB6 valley mist: depth (m, from, full), plain altitude (m, full, none) / crest height (m),
    // plain share, bank scale / drift, grazing gain, lifted share, near share / colour.
    valley_a: vec4<f32>,
    valley_b: vec4<f32>,
    valley_c: vec4<f32>,
    valley_color: vec4<f32>,
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
    // GA4 macro variation: periods (map px), octave weights, (tint amp, lum amp, -, TB6 canopy).
    macro_periods: vec4<f32>,
    macro_weights: vec4<f32>,
    macro_amp: vec4<f32>,
    // TB6 dark masses: floor, knee, feather (mip levels), kept contrast.
    dark: vec4<f32>,
    // RV-D valley occlusion: bound, valley darkening, crest lift, valley ambient occlusion.
    occlusion: vec4<f32>,
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
    // per (row, slot): parcels material layer, tile (m), season class.
    slot_tex: array<vec4<f32>, 288>,
    // per row: canopy layer / tile, rock layer / tile.
    rows_c: array<vec4<f32>, 24>,
    // per row: the two wild materials (layer, tile).
    rows_d: array<vec4<f32>, 24>,
    // per row: material tint, 1 when the row has materials.
    rows_e: array<vec4<f32>, 24>,
    // vine layer / tile, meadow layer / tile.
    tex_extra: vec4<f32>,
    // season class of the vine, of the meadow; 1 when the parcels array is bound.
    tex_flags: vec4<f32>,
    tex_means: array<vec4<f32>, 48>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(16) var<uniform> fields: FieldParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(17) var height_lod: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(18) var wash_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(19) var wash_sampler: sampler;
// Close patchwork (campvis3): parcels materials, land use, terroir mask.
@group(constants::MATERIAL_BIND_GROUP) @binding(20) var parcels_albedo: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(21) var parcels_normal: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(22) var landuse_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(23) var landuse_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(24) var terroir_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(25) var terroir_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(26) var occlusion_tex: texture_2d<f32>;

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
    // The summer gold is the crops' only (k_farm, wheat parcels), as in Godot, not the whole ground.
    var col = albedo * gain;
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

// Value noise with analytic derivatives (x: value in [0, 1], yz: gradient per unit of p).
fn vnoised(p: vec2<f32>) -> vec3<f32> {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let du = 6.0 * f * (1.0 - f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    let k = a - b - c + d;
    return vec3<f32>(a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y,
        du * vec2<f32>(b - a + k * u.y, c - a + k * u.x));
}

// Forest canopy carried by the terrain (VT3 of terrain.gdshader): four octaves of crowns (finest
// period 0.05 map units, then x4), each faded out when its period drops under 6 px.
// Returns (centred relief in [-1, 1], dimensionless slope).
fn canopy_field(p: vec2<f32>, footprint: f32) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    var period = 0.05;
    var amp = 1.0;
    var wsum = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let w = 1.0 - smoothstep(period / 12.0, period / 6.0, footprint);
        if (w > 0.0) {
            let nd = vnoised(p / period + vec2<f32>(f32(i) * 17.3, f32(i) * 5.1));
            acc = acc + w * amp * vec3<f32>(nd.x * 2.0 - 1.0, nd.y * 2.0, nd.z * 2.0);
        }
        wsum = wsum + amp;
        period = period * 4.0;
        amp = amp * 0.8;
    }
    return acc / max(wsum, 1e-3);
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

// Wind of the weather (`weather_wind` of campaign_weather.gdshaderinc), map px per second.
const WX_WIND = vec2<f32>(3.0, 0.6);
// Morning mist sheet: max opacity, share kept near the camera, footprint below which it thins
// (`weather_mist_max`, `weather_mist_near`, `weather_mist_near_footprint`).
const WX_MIST_MAX: f32 = 0.07;
const WX_MIST_NEAR: f32 = 0.2;
const WX_MIST_NEAR_FOOTPRINT: f32 = 0.06;

// Regional mean altitude (m): 3 x 3 taps of the 16 px block raster over ~190 map px, in place of
// the Godot heightmap mip 7 (`weather_valley_lods.y`, about 90 km).
fn wx_height_wide(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            sum = sum + pm_height(p + vec2<f32>(f32(i), f32(j)) * 64.0);
        }
    }
    return sum / 9.0;
}

// Morning mist of the "fog" weather (port of the B channel of `weather_ground`): a thin sheet
// capped at 7 % in the lowlands, then TB6 wide banks in the large valleys and low plains, both
// thinned near the camera and lifted over the turn.
fn weather_mist(col: vec3<f32>, p: vec2<f32>, hc: f32, footprint: f32, grazing: f32) -> vec3<f32> {
    let wb = look.mist.x;
    if (wb <= 0.001) {
        return col;
    }
    let morning = look.mist.y;
    let t = globals.time;
    let near_k = smoothstep(WX_MIST_NEAR_FOOTPRINT * 0.5, WX_MIST_NEAR_FOOTPRINT * 2.0, footprint);
    let low = 1.0 - smoothstep(150.0, 700.0, hc);
    let patches = smoothstep(0.3, 0.75, fbm(p * 0.02 - t * WX_WIND * 0.004));
    var mist = wb * mix(0.3, 0.75, low) * mix(0.35, 1.0, morning) * (0.35 + 0.65 * patches);
    mist = mist * mix(WX_MIST_NEAR, 1.0, near_k);
    let veil = mix(vec3<f32>(0.62, 0.64, 0.66), col * 1.6, 0.35);
    var out = mix(col, veil, clamp(mist, 0.0, WX_MIST_MAX));
    let opacity = look.mist.w;
    if (opacity > 0.0 && hc >= 1.0) {
        let h_mid = pm_height(p);
        let h_wide = wx_height_wide(p);
        let basin = smoothstep(look.valley_a.x, look.valley_a.y, h_wide - h_mid);
        let plain = (1.0 - smoothstep(look.valley_a.z, look.valley_a.w, h_mid)) * look.valley_b.z;
        let crest = 1.0 - smoothstep(look.valley_b.x, look.valley_b.y, hc - h_mid);
        let bq = (p - t * WX_WIND * look.valley_c.x) * look.valley_b.w;
        let banks = smoothstep(0.2, 0.8, fbm(bq) * 0.75 + vnoise(bq * 0.37 + 4.1) * 0.25);
        let optical = max(basin, plain) * crest * banks * (1.0 + look.valley_c.y * grazing);
        let lifted = mix(look.valley_c.z, 1.0, morning);
        let near = mix(look.valley_c.w, 1.0, near_k);
        let valley = clamp(wb * optical * lifted, 0.0, 1.0) * opacity * near;
        out = mix(out, look.valley_color.rgb, valley);
    }
    return out;
}

// Wet ground (slightly darker, TB6 `weather_wet_dim`) and fresh snow of the current weather.
fn weather_ground(col: vec3<f32>, n: vec3<f32>, altitude: f32, p: vec2<f32>) -> vec3<f32> {
    var out = col;
    let land = smoothstep(0.0, 4.0, altitude);
    let wet = look.weather.x;
    out = out * (1.0 - look.mist.z * wet * land);
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

// --- RV-C fair-weather cumulus shadows (campaign_cloud_shadow.gdshaderinc) ---
const CS_WIND = vec2<f32>(3.0, 0.6);

fn cs_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn cs_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(cs_hash(i), cs_hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(cs_hash(i + vec2<f32>(0.0, 1.0)), cs_hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn cs_fbm(p: vec2<f32>) -> f32 {
    return cs_noise(p) * 0.55 + cs_noise(p * 2.1 + 5.2) * 0.3 + cs_noise(p * 4.3 + 1.7) * 0.15;
}

// Light factor of the ground under the fair cumulus (1 = full sun), cool at the core. `area_px`:
// map px per screen px (geometric mean of both axes), `footprint` as elsewhere.
fn cloud_shadow_light(p: vec2<f32>, footprint: f32, area_px: f32, fade_in: f32) -> vec3<f32> {
    let strength = look.cumulus_a.x;
    let scale = look.cumulus_a.y;
    let cell_px = 1.0 / max(scale * area_px, 1e-4);
    let fade = fade_in * smoothstep(look.cumulus_c.w, look.cumulus_b.w, cell_px);
    if (fade <= 0.001 || strength <= 0.0) {
        return vec3<f32>(1.0);
    }
    // Point of the cloud plane on the sun ray reaching p.
    let sun = normalize(params.sun_dir.xyz);
    let src = p + sun.xz / max(sun.y, 0.2) * look.cumulus_b.z;
    let drift = globals.time * CS_WIND * look.cumulus_a.w;
    let q = (src - drift) * scale;
    let warp = vec2<f32>(cs_fbm(q * 0.55 + vec2<f32>(1.7, 9.2)), cs_fbm(q * 0.55 + vec2<f32>(8.3, 2.8))) - 0.5;
    var c = cs_fbm(q + warp * 1.4);
    let fine_w = 1.0 - smoothstep(0.25, 0.6, footprint * scale * 3.0);
    c = c + (cs_noise(q * 3.7 + warp * 2.0 + vec2<f32>(4.1, 0.6)) - 0.5) * 0.16 * fine_w;
    let region = cs_fbm((src - drift * 0.5) * 0.0035 + vec2<f32>(3.3, 7.1));
    let cover = mix(look.cumulus_b.x, look.cumulus_b.y, smoothstep(0.3, 0.7, region));
    let threshold = mix(0.74, 0.36, cover);
    let edge = look.cumulus_a.z + footprint * scale * 0.25;
    let shade = smoothstep(threshold, threshold + edge, c) * strength * fade;
    let cool = clamp(shade / max(strength, 1e-3), 0.0, 1.0);
    return mix(vec3<f32>(1.0), look.cumulus_c.rgb, cool) * (1.0 - shade);
}

// --- RV-D valley occlusion (relief_occlusion.gdshaderinc) ---
const RVD_COOL_TINT = vec3<f32>(0.70, 0.78, 0.92);
const RVD_WARM_TINT = vec3<f32>(1.0, 0.97, 0.90);
const RVD_NEAR_FOOTPRINT = vec2<f32>(0.04, 0.3);
const RVD_NEAR_STRENGTH = 0.5;

struct Rvd {
    color: vec3<f32>,
    // ambient occlusion (1 = none)
    ao: f32,
};

// Valleys darker and cooler, crests a little lighter (low frequency, independent of the sun);
// returns the ambient occlusion of the valley floors. `land` weight, gone on the parchment.
fn rvd_apply(uv: vec2<f32>, footprint: f32, land: f32, col: vec3<f32>) -> Rvd {
    var out = Rvd(col, 1.0);
    if (ground.occlusion.x < 0.5 || land <= 0.001) {
        return out;
    }
    let o = textureSampleLevel(occlusion_tex, terroir_sampler, uv, 0.0).r * (255.0 / 127.0) - (128.0 / 127.0);
    // The fine relief pages are always there close up in Bevy (`qt_has_page`).
    let w = land * (1.0 - look.weather.z) * mix(RVD_NEAR_STRENGTH, 1.0, smoothstep(RVD_NEAR_FOOTPRINT.x, RVD_NEAR_FOOTPRINT.y, footprint));
    let valley = max(o, 0.0) * w;
    let crest = max(-o, 0.0) * w;
    let strength = ground.occlusion.y;
    let cool = RVD_COOL_TINT / max(dot(RVD_COOL_TINT, vec3<f32>(1.0 / 3.0)), 1e-3) * (1.0 - strength);
    out.color = col * mix(vec3<f32>(1.0), cool, valley) * mix(vec3<f32>(1.0), RVD_WARM_TINT * (1.0 + ground.occlusion.z), crest);
    out.ao = 1.0 - ground.occlusion.w * valley;
    return out;
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

// --- Close patchwork (CV1 terroirs, V2b parcels, ZG5b fine parcels) ---------------------------
// Ports of campaign_life.gdshaderinc (season_field, terroir_*), terrain.gdshader (field_at and the
// V2b parcel colours, hedges, tracks) and fine_parcels.gdshaderinc (fp_splat, fp_parcels).

// V2b palette (terrain.gdshader uniforms, linear).
const CROP_WHEAT = vec3<f32>(0.36, 0.265, 0.075);
const CROP_GREEN = vec3<f32>(0.12, 0.175, 0.045);
const CROP_PLOUGHED = vec3<f32>(0.15, 0.10, 0.055);
const CROP_FALLOW = vec3<f32>(0.19, 0.175, 0.075);
const MEADOW_COLOR = vec3<f32>(0.095, 0.16, 0.042);
const HAY_COLOR = vec3<f32>(0.24, 0.215, 0.085);
const VINE_SOIL = vec3<f32>(0.19, 0.135, 0.075);
const VINE_LEAF = vec3<f32>(0.07, 0.10, 0.03);
const DRY_GRASS = vec3<f32>(0.23, 0.19, 0.085);
const TRACK_COLOR = vec3<f32>(0.26, 0.20, 0.12);
const HEDGE_COLOR = vec3<f32>(0.035, 0.05, 0.02);
const HEDGE_WIDTH = 0.16;
const TRACK_WIDTH = 0.10;
const FIELD_SCALE = 0.3;
// hb_clear_floor of hb_ground.gdshaderinc (DN-SOL clearings).
const HB_CLEAR_FLOOR = 0.3;
// fine_parcels.gdshaderinc (fp_quality 2).
const FP_START = 0.05;
const FP_FULL = 0.015;
const FP_BLOCK = 8.0;
const FP_HEDGE = vec3<f32>(0.028, 0.045, 0.016);
const FP_HEADLAND = vec3<f32>(0.13, 0.17, 0.06);
const FP_WASTE = vec3<f32>(0.13, 0.13, 0.06);
const FP_ASSART = vec3<f32>(0.12, 0.12, 0.05);
const FP_TERRACE = vec3<f32>(0.16, 0.14, 0.11);

// Parcel colour by season. cls: 0 wheat, 1 green crop, 2 ploughed, 3 fallow, 4 meadow, 5 hay,
// 6 vine soil; h: draw of the parcel.
fn season_field(summer: vec3<f32>, cls: i32, h: f32) -> vec3<f32> {
    let w = season.weights;
    let young = vec3<f32>(0.10, 0.19, 0.045);
    let plough = vec3<f32>(0.19, 0.135, 0.078);
    let wet_plough = vec3<f32>(0.135, 0.095, 0.06);
    let stubble = vec3<f32>(0.25, 0.20, 0.105);
    let meadow_spring = vec3<f32>(0.085, 0.19, 0.035);
    let meadow_autumn = vec3<f32>(0.11, 0.12, 0.045);
    let meadow_winter = vec3<f32>(0.075, 0.085, 0.048);
    var spring = summer;
    var autumn = summer;
    var winter = summer;
    var ripe = summer;
    if (cls == 0) {
        ripe = summer * vec3<f32>(1.15, 1.06, 0.78);
        spring = select(plough, young, h < 0.8);
        autumn = select(plough, stubble, h < 0.45);
        winter = mix(wet_plough, young, 0.3);
    } else if (cls == 1) {
        spring = mix(young, summer, 0.25);
        autumn = select(mix(stubble, plough, 0.5), plough, h < 0.5);
        winter = wet_plough;
    } else if (cls == 2) {
        spring = plough;
        autumn = plough * 0.9;
        winter = wet_plough;
    } else if (cls == 3) {
        spring = select(mix(summer, young, 0.5), plough, h < 0.5);
        autumn = mix(summer, stubble, 0.4);
        winter = mix(wet_plough, meadow_winter, 0.5);
    } else if (cls == 4) {
        spring = meadow_spring;
        autumn = meadow_autumn;
        winter = meadow_winter;
    } else if (cls == 5) {
        spring = meadow_spring * 1.05;
        autumn = mix(meadow_autumn, stubble, 0.35);
        winter = mix(meadow_winter, stubble * 0.6, 0.4);
    } else {
        autumn = summer * vec3<f32>(1.05, 0.95, 0.9);
        winter = summer * 0.85;
    }
    return spring * w.x + ripe * w.y + autumn * w.z + winter * w.w;
}

fn season_vine_leaf(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    return summer * 1.15 * w.x + summer * w.y + vec3<f32>(0.24, 0.085, 0.02) * w.z
        + vec3<f32>(0.10, 0.07, 0.045) * w.w;
}

// TB1 (campaign_life.gdshaderinc): background tints of the meadows / crops / forests by season.
const AUTUMN_RUSSET = vec3<f32>(1.03, 0.80, 0.86);
// `sg_season_strength`, `hb_season_strength`.
const SG_SEASON_STRENGTH = 1.3;
const HB_SEASON_STRENGTH = 0.8;

fn season_grass(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = summer * vec3<f32>(0.9, 1.18, 0.85);
    let autumn = mix(summer, vec3<f32>(0.13, 0.12, 0.05), 0.5);
    let winter = mix(summer, vec3<f32>(0.08, 0.085, 0.055), 0.65);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_farm(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = mix(summer, vec3<f32>(0.11, 0.15, 0.05), 0.6);
    let autumn = mix(summer, vec3<f32>(0.15, 0.11, 0.06), 0.6);
    let winter = mix(summer, vec3<f32>(0.095, 0.075, 0.05), 0.7);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_forest(summer: vec3<f32>, grove: f32) -> vec3<f32> {
    let w = season.weights;
    if (w.y > 0.999) {
        return summer;
    }
    let spring = summer * vec3<f32>(1.05, 1.3, 0.9);
    var autumn = summer;
    if (grove < 0.35) {
        autumn = vec3<f32>(0.115, 0.042, 0.010);
    } else if (grove < 0.7) {
        autumn = vec3<f32>(0.12, 0.08, 0.015);
    }
    let winter = select(summer * 0.85, vec3<f32>(0.050, 0.043, 0.036), grove < 0.72);
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

fn season_forest_mean(summer: vec3<f32>) -> vec3<f32> {
    let w = season.weights;
    let spring = summer * vec3<f32>(1.05, 1.3, 0.9);
    let autumn = 0.35 * vec3<f32>(0.115, 0.042, 0.010) + 0.35 * vec3<f32>(0.12, 0.08, 0.015) + 0.30 * summer;
    let winter = 0.72 * vec3<f32>(0.050, 0.043, 0.036) + 0.28 * summer * 0.85;
    return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// Multiplicative season shift (season / summer), bounded: applied on the mean-season colour map
// and materials, which do not know the season.
fn season_ratio(season_col: vec3<f32>, summer_col: vec3<f32>) -> vec3<f32> {
    return clamp(season_col / max(summer_col, vec3<f32>(1e-3)), vec3<f32>(0.35), vec3<f32>(2.6));
}

// Terroir mask (terroir.rs): R fields, G vine, B burnt, A pasture; square frame of the map.
fn terroir_at(p: vec2<f32>) -> vec4<f32> {
    if (ground.splat.w < 0.5) {
        return vec4<f32>(0.0);
    }
    return textureSampleLevel(terroir_tex, terroir_sampler, p / max(params.map.x, params.map.y), 0.0);
}

// Finage: meadows and waste near the settlements turn to fields, pastures grow in a ring.
fn terroir_splat(s_in: vec4<f32>, tr: vec4<f32>) -> vec4<f32> {
    var s = s_in;
    let to_farm = s.r * tr.r * 0.75 + s.a * tr.r * 0.4;
    s.r = s.r - s.r * tr.r * 0.75;
    s.a = s.a - s.a * tr.r * 0.4;
    s.g = s.g + to_farm;
    let to_grass = s.g * tr.a * 0.35 * (1.0 - tr.r);
    s.g = s.g - to_grass;
    s.r = s.r + to_grass;
    return s;
}

// Vines around the settlements of the wine regions (land use: R vine).
fn terroir_landuse(lu: vec3<f32>, tr: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(max(lu.r, tr.g * 1.6), lu.g, lu.b);
}

// DN-SOL clearings: the crops of the splat stay on the finages and the flat valley floors; the
// rest becomes meadow (75 %) and heath. `agri`: landscape mask value (0 outside).
fn hb_clearing_splat(s: vec4<f32>, terroir: vec4<f32>, p: vec2<f32>, slope: f32, agri: u32) -> vec4<f32> {
    let valley = 1.0 - smoothstep(0.03, 0.12, slope);
    let patch_n = smoothstep(0.38, 0.62, vnoise(p * 0.07 + vec2<f32>(11.0)));
    let clear = clamp(terroir.r * 1.25 + 0.4 * valley * patch_n, 0.0, 1.0);
    let floor_keep = select(HB_CLEAR_FLOOR, 0.55, agri > 0u);
    let keep = mix(floor_keep, 1.0, clear);
    let lost = s.g * (1.0 - keep);
    return vec4<f32>(s.r + lost * 0.75, s.g - lost, s.b, s.a + lost * 0.25);
}

// --- V2b parcels (shared with vegetation_fields.gd: same constants, same hash) ---
fn hash01i(n: i32) -> f32 {
    var h = (bitcast<u32>(n) * 1103515245u + 12345u) & 0x7fffffffu;
    h = ((h ^ (h >> 13u)) * 1274126177u) & 0x7fffffffu;
    return f32(h % 100000u) / 100000.0;
}

fn region_value(p: vec2<f32>) -> f32 {
    return sin(0.0131 * p.x + 1.3 * sin(0.0093 * p.y)) + sin(0.0117 * p.y + 1.1 * sin(0.0171 * p.x)) - 0.25;
}

fn field_warp(q: vec2<f32>, ph: f32) -> vec2<f32> {
    let ox = 1.7 * sin(0.071 * q.y + 1.9 * sin(0.027 * q.x) + ph) + 0.55 * sin(0.23 * q.y + 1.3 * sin(0.061 * q.x));
    let oy = 1.7 * sin(0.063 * q.x + 1.7 * sin(0.023 * q.y) + ph * 1.3) + 0.55 * sin(0.19 * q.x + 1.1 * sin(0.047 * q.y));
    return vec2<f32>(ox, oy);
}

struct Field {
    cell_h: f32,
    strip_h: f32,
    du: f32,
    dv: f32,
    roll_u: f32,
    roll_v: f32,
    strip_d: f32,
    strip_dir: vec2<f32>,
    rows: f32,
};

// Parcel at p (layout `lay`): enclosure and strip draws, distances (parcel units) to the edges at
// constant u / of the row and the draws of those edges, distance to the strip edge, strip
// direction (map frame) and across coordinate (vine rows).
fn field_at(p: vec2<f32>, lay: i32) -> Field {
    var out: Field;
    let first = lay == 0;
    let k = select(-0.8, 0.35, first);
    let fu = select(4.6, 5.0, first);
    let fv = select(3.8, 4.2, first);
    let ph = select(2.1, 0.0, first);
    var q = p;
    for (var i = 0; i < 3; i = i + 1) {
        q = p - field_warp(q, ph);
    }
    let u = (q.x + k * q.y) / fu;
    let v = (q.y - k * q.x) / fv;
    let inv = inverseSqrt(1.0 + k * k);
    let cu = i32(floor(u));
    let vs = v + hash01i(cu * 5023 + 17 + lay * 101);
    let cv = i32(floor(vs));
    let fu_f = fract(u);
    let fv_f = fract(vs);
    out.du = min(fu_f, 1.0 - fu_f) * fu * inv;
    out.dv = min(fv_f, 1.0 - fv_f) * fv * inv;
    let uline = select(cu + 1, cu, fu_f < 0.5);
    let vline = select(cv + 1, cv, fv_f < 0.5);
    out.roll_u = hash01i(uline * 7919 + i32(floor(v)) * 104729 + lay * 7 + 1);
    out.roll_v = hash01i(vline * 7919 + cu * 104729 + lay * 7 + 31);
    let cell_key = cu * 7919 + cv * 104729 + lay * 7 + 3;
    out.cell_h = hash01i(cell_key);
    let h2 = hash01i(cell_key + 11);
    let h3 = hash01i(cell_key + 23);
    let n = 1.0 + floor(h2 * 4.0);
    let along_u = h3 < 0.5;
    var t = select(fv_f, fu_f, along_u);
    t = t + 0.035 * sin(select(u, v, along_u) * 9.0 + h2 * 6.0);
    let ts = clamp(t, 0.0, 0.999) * n;
    let strip = floor(ts);
    out.strip_h = hash01i(cell_key + i32(strip) * 13 + 5);
    let sf = fract(ts);
    out.strip_d = select(1e3, min(sf, 1.0 - sf) / n * select(fv, fu, along_u) * inv, n > 1.0);
    out.strip_dir = select(normalize(vec2<f32>(1.0, k)), normalize(vec2<f32>(-k, 1.0)), along_u);
    out.rows = ts;
    return out;
}

struct V2b {
    grass_tint: vec3<f32>,
    farm_tint: vec3<f32>,
    hedge: f32,
    track: f32,
    balk: f32,
    vine_rows: f32,
    // TB1 season shifts of the meadows and the crops (season / summer, summer gold included).
    k_grass: vec3<f32>,
    k_farm: vec3<f32>,
    grass_summer: vec3<f32>,
    farm_summer: vec3<f32>,
};

// Lot V2b of terrain.gdshader: enclosures, crop strips, meadows, vines, hedges and dirt tracks;
// the tints replace the grass / farmland tints of the textured soil. Call in uniform control
// flow (fwidth of the vine rows).
fn v2b_parcels(p: vec2<f32>, footprint: f32, s: vec4<f32>, lu: vec3<f32>) -> V2b {
    var out: V2b;
    let dry = lu.g;
    let open_land = clamp(s.g + 0.6 * s.r, 0.0, 1.0);
    let grass_share = s.r / max(s.r + s.g, 1e-3);
    let grass_summer = mix(TINT_GRASS, DRY_GRASS, dry * 0.75);
    let farm_summer = mix(TINT_FARM, mix(TINT_FARM, CROP_WHEAT, 0.35) * vec3<f32>(1.05, 0.98, 0.9), dry * 0.6);
    let grass_base = season_grass(grass_summer);
    let farm_base = season_farm(farm_summer);
    out.grass_summer = grass_summer;
    out.farm_summer = farm_summer;
    out.k_grass = season_ratio(grass_base, grass_summer);
    out.k_farm = season_ratio(farm_base * mix(vec3<f32>(1.0), season.summer_gold.rgb, season.weights.y), farm_summer);
    out.grass_tint = grass_base;
    out.farm_tint = farm_base;
    let field_fp = footprint / FIELD_SCALE;
    let cell_px = 4.4 / max(field_fp, 1e-4);
    let field_vis = smoothstep(5.0, 14.0, cell_px);
    let rv = region_value(p);
    let lay = select(0, 1, rv > 0.0);
    let f = field_at(p / FIELD_SCALE, lay);
    // Derivatives first (uniform control flow).
    let rw = fwidth(f.rows * 5.0);
    if (field_vis <= 0.0 || (s.g + s.r) <= 0.05) {
        return out;
    }
    let strip_vis = smoothstep(3.0, 8.0, cell_px * 0.4);
    let h = select(f.cell_h, f.strip_h, strip_vis > 0.5);
    let h_b = hash01i(i32(h * 99991.0) + 7);
    let p_meadow = mix(0.15, 0.85, grass_share);
    var crop: vec3<f32>;
    let vine = h_b < lu.r * 0.32 && grass_share < 0.8;
    if (vine) {
        crop = season_field(VINE_SOIL, 6, h_b);
    } else if (h < p_meadow) {
        let m = h / max(p_meadow, 1e-3);
        if (m < 0.6) {
            crop = season_field(MEADOW_COLOR * (0.85 + 0.35 * h_b), 4, h_b);
        } else if (m < 0.85) {
            crop = season_field(mix(MEADOW_COLOR, HAY_COLOR, 0.55), 5, h_b);
        } else {
            crop = season_field(HAY_COLOR, 5, h_b);
        }
        crop = mix(crop, DRY_GRASS, dry * 0.7);
    } else {
        let c = (h - p_meadow) / max(1.0 - p_meadow, 1e-3);
        if (c < 0.36) {
            crop = season_field(CROP_WHEAT, 0, h_b);
        } else if (c < 0.6) {
            crop = season_field(CROP_GREEN, 1, h_b);
        } else if (c < 0.76) {
            crop = season_field(CROP_PLOUGHED, 2, h_b);
        } else {
            crop = season_field(CROP_FALLOW, 3, h_b);
        }
        crop = crop * (0.9 + 0.2 * h_b);
        crop = mix(crop, crop * vec3<f32>(1.08, 0.96, 0.82), dry);
    }
    // Parcel colours later than their outline: no noisy checkerboard at mid zoom.
    let crop_fade = smoothstep(6.0, 24.0, cell_px);
    out.grass_tint = mix(grass_base, crop, crop_fade * 0.8);
    out.farm_tint = mix(farm_base, crop, crop_fade * 0.85);
    if (vine) {
        let r = abs(fract(f.rows * 5.0) - 0.5);
        let fine = 1.0 - smoothstep(0.25, 0.6, rw);
        out.vine_rows = mix(0.45, 1.0 - smoothstep(0.22 - rw, 0.22 + rw, r), fine) * field_vis;
    }
    // Edges: hedge (draw < probability), dirt track (other draw), else a grassy balk.
    let p_hedge = open_land * mix(0.1, 0.75, lu.b);
    let aa = field_fp * 0.75;
    let hedge_u = step(f.roll_u, p_hedge) * (1.0 - smoothstep(HEDGE_WIDTH - aa, HEDGE_WIDTH + aa, f.du));
    let hedge_v = step(f.roll_v, p_hedge) * (1.0 - smoothstep(HEDGE_WIDTH - aa, HEDGE_WIDTH + aa, f.dv));
    let cover = clamp(2.0 * HEDGE_WIDTH / max(field_fp, 1e-4), 0.0, 1.0);
    out.hedge = max(hedge_u, hedge_v) * mix(cover, 1.0, cover) * field_vis;
    let tr_u = step(fract(f.roll_u * 7.31), 0.07) * (1.0 - smoothstep(TRACK_WIDTH - aa, TRACK_WIDTH + aa, f.du));
    let tr_v = step(fract(f.roll_v * 7.31), 0.07) * (1.0 - smoothstep(TRACK_WIDTH - aa, TRACK_WIDTH + aa, f.dv));
    let tcover = clamp(2.0 * TRACK_WIDTH / max(field_fp, 1e-4), 0.0, 1.0);
    var tr_r = 0.0;
    if (tcover > 0.15 && abs(rv) < 0.05) {
        // Border between the two layouts: a continuous track.
        let rgrad = length(vec2<f32>(region_value(p + vec2<f32>(1.0, 0.0)) - rv, region_value(p + vec2<f32>(0.0, 1.0)) - rv));
        let rd = abs(rv) / max(rgrad, 1e-4);
        tr_r = 1.0 - smoothstep(TRACK_WIDTH * 1.3 - aa, TRACK_WIDTH * 1.3 + aa, rd);
    }
    out.track = max(max(tr_u, tr_v), tr_r) * tcover * field_vis;
    let edge = min(min(f.du, f.dv), f.strip_d);
    out.balk = (1.0 - smoothstep(0.03, 0.03 + aa * 2.0, edge)) * smoothstep(8.0, 20.0, cell_px) * 0.5;
    return out;
}

// --- Fine parcels (ZG5b, fine_parcels.gdshaderinc) ---
fn fp_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn fp_hash2(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn fp_mod(x: vec2<f32>, y: f32) -> vec2<f32> {
    return x - y * floor(x / y);
}

struct FpVor {
    id: vec2<f32>,
    center: vec2<f32>,
    border: f32,
};

// Voronoi on a jittered grid of `per` cells per block: global id of the cell, centre (m, local
// frame), approximate distance to the edge (m).
fn fp_voronoi(q: vec2<f32>, blk: vec2<f32>, per: f32, jitter: f32, mpp: f32) -> FpVor {
    let size = FP_BLOCK * mpp / per;
    let g = floor(q / size);
    let f = q / size - g;
    var d1 = 1e9;
    var d2 = 1e9;
    var best = vec2<f32>(0.0);
    var best_c = vec2<f32>(0.0);
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let o = vec2<f32>(f32(i), f32(j));
            let gid = g + o;
            let id = (blk + floor(gid / per)) * per + fp_mod(gid, per);
            let c = o + 0.5 + (fp_hash2(id) - 0.5) * jitter;
            let d = length(c - f);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                best = id;
                best_c = (g + c) * size;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return FpVor(best, best_c, (d2 - d1) * 0.5 * size);
}

// Hook 1: splat texel read in warped coordinates close up (edges of the 719 m squares broken).
fn fp_splat(s: vec4<f32>, p: vec2<f32>, footprint: f32) -> vec4<f32> {
    let vis = 1.0 - smoothstep(0.03, 0.12, footprint);
    if (vis <= 0.0) {
        return s;
    }
    var w = vec2<f32>(vnoise(p * 0.9), vnoise(p * 0.9 + vec2<f32>(31.7, 11.3))) - 0.5;
    w = w + (vec2<f32>(vnoise(p * 3.7 + 5.1), vnoise(p * 3.7 + 17.9)) - 0.5) * 0.45;
    w = w + (vec2<f32>(vnoise(p * 14.0 + 2.3), vnoise(p * 14.0 + 8.1)) - 0.5) * 0.18;
    let warped = textureSampleLevel(splat, splat_sampler, (p + w * 1.1) / params.map.xy, 0.0);
    return mix(s, warped, vis);
}

struct FpOut {
    color: vec3<f32>,
    dn: vec2<f32>,
};

// Hook 3: parcels at their real scale, in metres (openfield strips and three-field soles,
// bocage enclosures with hedges, vines on south slopes, assarts by the woods, waste far from the
// villages). Not ported: valley hay meadows (relief occlusion, wetlands), 1:1 town finages.
// `grad` in m per map px, `open_w` / `forest_w` normalised shares.
fn fp_parcels(p: vec2<f32>, footprint: f32, grad: vec2<f32>, slope: f32, hc: f32, s: vec4<f32>,
        lu: vec3<f32>, terroir: vec4<f32>, open_w: f32, forest_w: f32, col: vec3<f32>) -> FpOut {
    var out = FpOut(col, vec2<f32>(0.0));
    let mpp = max(ground.micro.w, 1.0);
    let blk = floor(p / FP_BLOCK);
    let q = (p - blk * FP_BLOCK) * mpp;
    let g = grad / mpp;
    let g_len = length(g);
    let downhill = select(vec2<f32>(0.0, 1.0), -g / max(g_len, 1e-6), g_len > 1e-4);
    // Derivatives first (uniform control flow): vine rows along the contour lines.
    let row = dot(q, downhill) / 1.8;
    let rw = fwidth(row);
    let vis = 1.0 - smoothstep(FP_FULL, FP_START, footprint);
    if (vis <= 0.0 || open_w + forest_w < 0.05 || hc < 0.25) {
        return out;
    }
    let px_m = footprint * mpp;
    let aa = max(px_m, 0.05);
    let bocage = smoothstep(0.25, 0.6, lu.b + (vnoise(p * 0.25) - 0.5) * 0.25);
    let village = clamp(terroir.r * 1.3 + terroir.a * 0.6, 0.0, 1.0);
    let grass_share = s.r / max(s.r + s.g, 1e-3);
    let south = select(0.0, clamp(-g.y / max(g_len, 1e-6), -1.0, 1.0), g_len > 1e-4);
    let vine_site = smoothstep(0.04, 0.10, slope) * (1.0 - smoothstep(0.45, 0.7, slope)) * smoothstep(0.1, 0.6, south);
    let vine_w = vine_site * clamp(lu.r * 2.2 + 0.12, 0.0, 1.0) * (1.0 - bocage * 0.6);
    let assart = smoothstep(0.08, 0.3, forest_w) * (1.0 - smoothstep(0.5, 0.75, forest_w));
    let waste = (1.0 - smoothstep(0.05, 0.35, village)) * smoothstep(0.3, 0.8, grass_share + slope * 2.0);
    var hedge = 0.0;
    var furrow_on = 0.0;
    // Openfield: quarters of strips.
    let fv = fp_voronoi(q, blk, 14.0, 0.8, mpp);
    let fh = fp_hash(fv.id + 3.1);
    let ang = fh * 3.14159;
    let dir = vec2<f32>(cos(ang), sin(ang));
    let across = vec2<f32>(-dir.y, dir.x);
    let strip_w = mix(15.0, 35.0, fp_hash(fv.id + 7.7));
    let t = dot(q - fv.center, across) / strip_w;
    let strip = floor(t);
    let sh = fp_hash(fv.id * 1.37 + vec2<f32>(strip, strip * 0.37));
    // Soles of the three-field rotation (about 1.5 km) shared by neighbouring quarters.
    let sole = floor(vnoise(p * 0.45 + 3.3) * 2.999 + fh * 0.4);
    let c_pick = fract(sole / 3.0 + select(sh, 0.0, sh < 0.75));
    var crop: vec3<f32>;
    if (c_pick < 0.34) {
        crop = season_field(CROP_WHEAT, 0, sh);
    } else if (c_pick < 0.67) {
        crop = season_field(mix(CROP_GREEN, CROP_WHEAT, sh * 0.4), 1, sh);
    } else {
        crop = season_field(CROP_FALLOW, 3, sh);
    }
    crop = crop * (0.8 + 0.4 * fp_hash(vec2<f32>(strip, fh * 97.0)));
    let balk_d = (0.5 - abs(fract(t) - 0.5)) * strip_w;
    let balk = (1.0 - smoothstep(0.6, 0.6 + aa, balk_d)) * (1.0 - smoothstep(1.5, 6.0, px_m));
    let headland = (1.0 - smoothstep(5.0, 5.0 + aa * 1.5, fv.border)) * (0.6 + 0.4 * vnoise(q * 0.08));
    let openfield = mix(crop, FP_HEADLAND, max(balk * 0.45, headland * 0.45));
    var furrow = dir;
    furrow_on = (1.0 - headland) * select(0.3, 1.0, c_pick < 0.67);
    // Bocage: irregular enclosures and tree hedges.
    var bocage_col = openfield;
    if (bocage > 0.01) {
        let warp = vec2<f32>(vnoise(p * 6.0), vnoise(p * 6.0 + 4.0)) * 55.0
            + vec2<f32>(vnoise(p * 25.0), vnoise(p * 25.0 + 2.0)) * 14.0;
        let bv = fp_voronoi(q + warp, blk, 38.0, 0.95, mpp);
        let bh = fp_hash(bv.id + 11.3);
        var field: vec3<f32>;
        if (bh < 0.55) {
            field = season_field(MEADOW_COLOR * (0.85 + 0.3 * bh), 4, bh);
        } else if (bh < 0.8) {
            field = season_field(CROP_WHEAT, 0, bh);
        } else {
            field = season_field(CROP_PLOUGHED, 2, bh);
        }
        let crowns = vnoise(q * 0.18) * 0.6 + vnoise(q * 0.55) * 0.4;
        let hw = 4.0 + 5.0 * crowns;
        hedge = (1.0 - smoothstep(hw - aa, hw + aa, bv.border)) * step(0.1, fp_hash(bv.id + 5.5));
        let core = 1.0 - smoothstep(0.0, hw, bv.border);
        bocage_col = mix(field, FP_HEDGE * (0.6 + 0.7 * crowns) * mix(1.15, 0.7, core), hedge);
    }
    var pc = mix(openfield, bocage_col, bocage);
    furrow_on = furrow_on * (1.0 - bocage);
    // Vines on the south slopes (rows along the contour lines), terraces on steep ones.
    if (vine_w > 0.01) {
        let contour = select(dir, vec2<f32>(-g.y, g.x) / max(g_len, 1e-6), g_len > 1e-4);
        let r = abs(fract(row) - 0.5);
        let fine = 1.0 - smoothstep(0.25, 0.6, rw);
        let leaf = mix(0.45, 1.0 - smoothstep(0.2 - rw, 0.2 + rw, r), fine);
        var vine = mix(season_field(VINE_SOIL, 6, fh), season_vine_leaf(VINE_LEAF), leaf);
        let terrace = smoothstep(0.2, 0.35, slope)
            * (1.0 - smoothstep(0.08, 0.08 + aa / 10.0, abs(fract(dot(q, downhill) / 10.0) - 0.5) - 0.42));
        vine = mix(vine, FP_TERRACE, terrace * 0.7);
        pc = mix(pc, vine, vine_w);
        furrow = contour;
        furrow_on = furrow_on * (1.0 - vine_w);
    }
    // Assarts by the woods and waste far from the villages.
    if (assart > 0.01) {
        let stumps = smoothstep(0.78, 0.82, vnoise(q * 0.4)) * (1.0 - smoothstep(0.3, 1.0, px_m));
        let cleared = mix(FP_ASSART * (0.85 + 0.3 * vnoise(q * 0.05)), FP_HEDGE, stumps);
        pc = mix(pc, cleared, assart * 0.7);
    }
    if (waste > 0.01) {
        let bush = smoothstep(0.7, 0.76, vnoise(q * 0.12) * 0.7 + vnoise(q * 0.5) * 0.3);
        let rough = mix(season_field(FP_WASTE, 4, fh), FP_HEDGE * 1.3, bush * 0.8);
        pc = mix(pc, rough, waste * 0.75);
    }
    // High-frequency detail (tufts, clods), close up only.
    if (px_m < 3.0) {
        let hf = vnoise(q * 0.9) * 0.5 + vnoise(q * 3.1) * 0.3 + vnoise(q * 9.7) * 0.2;
        pc = pc * (0.9 + 0.2 * mix(0.5, hf, 1.0 - smoothstep(0.4, 3.0, px_m)));
    }
    let w = vis * clamp(open_w + assart * forest_w, 0.0, 1.0);
    out.color = mix(col, pc, w * 0.92);
    // Furrows across the strips, hedges in relief (map-frame normal perturbation, z flipped as in
    // the fragment: n.z - dn.y).
    if (furrow_on > 0.01) {
        let kk = dot(q, vec2<f32>(-furrow.y, furrow.x)) / 0.9;
        let fade = 1.0 - smoothstep(0.2, 0.8, px_m);
        let bump = vec2<f32>(-furrow.y, furrow.x) * cos(kk * 6.2832) * 0.12 * fade * furrow_on * w;
        out.dn = out.dn + vec2<f32>(bump.x, -bump.y);
    }
    if (hedge > 0.001) {
        let hb = vec2<f32>(vnoise(q * 0.5) - 0.5, vnoise(q * 0.5 + 9.0) - 0.5) * hedge * 0.6 * w;
        out.dn = out.dn + vec2<f32>(hb.x, -hb.y);
    }
    return out;
}

struct GroundOut {
    color: vec3<f32>,
    // normal perturbation (map frame), already faded by the detail range
    dn: vec2<f32>,
    // share of grass + farmland + heath, of forest, farmland among grass + farmland
    open_w: f32,
    forest_w: f32,
    rock_w: f32,
    farm_share: f32,
    // TB1: season shift of the pixel's land cover (colour map), of the wild soil and of the canopy.
    season_k: vec3<f32>,
    k_wild: vec3<f32>,
    k_forest: vec3<f32>,
};

// Procedural ground of `terrain.gdshader` (splat weights x regional layers x micro grain), without
// the parcels, snow and sand. Called in uniform control flow (derivatives).
// `macro_lum_fade` / `macro_lum_far_keep` of terrain.gdshader (RX mapb).
const MACRO_LUM_FADE = vec2<f32>(0.6, 2.0);
const MACRO_LUM_FAR_KEEP = 0.35;

// TB6: the dark forest masses of the colour map (Orléans, Fontainebleau...) lifted to a floor with a
// feathered edge and a canopy relief (sg_apply of satellite_ground.gdshaderinc). `soft` is the
// colour map read `feather` mip levels coarser.
fn dark_masses(cm_in: vec3<f32>, soft: vec3<f32>, q: vec2<f32>, footprint: f32) -> vec3<f32> {
    let floor_l = ground.dark.x;
    if (floor_l <= 0.0) {
        return cm_in;
    }
    let dark = 1.0 - smoothstep(floor_l, floor_l * ground.dark.y, luma(soft));
    if (dark <= 0.0) {
        return cm_in;
    }
    var cm = mix(cm_in, soft, dark * 0.6);
    let l = max(luma(cm), 1e-3);
    let lifted = max(l, mix(floor_l, l, ground.dark.w));
    let crowns = (vnoise(q * 0.22 + vec2<f32>(7.9)) - 0.5) * 0.9
        + (vnoise(q * 0.9) - 0.5) * 0.7 * (1.0 - smoothstep(0.4, 1.0, footprint))
        + (vnoise(q * 2.7 + vec2<f32>(3.1)) - 0.5) * 0.5 * (1.0 - smoothstep(0.15, 0.4, footprint));
    cm = cm * mix(1.0, lifted / l * (1.0 + ground.macro_amp.w * crowns * 2.0), dark);
    return cm;
}

fn ground_proc(p: vec2<f32>, uv: vec2<f32>, footprint: f32, altitude: f32, slope: f32, grad: vec2<f32>) -> GroundOut {
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
        s = fp_splat(s, p, footprint);
    }
    s = s / max(s.r + s.g + s.b + s.a, 1e-3);
    // CV1 terroirs: finages of the settlements (fields, pasture ring).
    let terroir = terroir_at(p);
    s = terroir_splat(s, terroir);
    let rock_frac = max(smoothstep(ROCK_LINE_M - 700.0, ROCK_LINE_M, altitude), smoothstep(0.35, 0.6, slope));
    // Landscape mask (agri_regions): part of the grassland turned to crops.
    let agri_raw = u32(textureLoad(ab_dist, vec2<i32>(clamp(uv * vec2<f32>(textureDimensions(ab_dist)), vec2<f32>(0.0), vec2<f32>(textureDimensions(ab_dist)) - 1.0)), 0).b * 255.0 + 0.5);
    if (agri_raw > 0u && fields.v8.w > 0.5) {
        let moved = s.r * fields.rows_b[min(agri_raw + 7u, 23u)].x;
        s = vec4<f32>(s.r - moved, s.g + moved, s.b, s.a);
    }
    s = hb_clearing_splat(s, terroir, p, slope, agri_raw);
    // Land use (R vine, G dryness, B bocage) with the vines of the terroirs; V2b parcels.
    var lu = vec3<f32>(0.0);
    if (ground.splat.z > 0.5) {
        lu = textureSampleLevel(landuse_tex, landuse_sampler, uv, 0.0).rgb;
    }
    lu = terroir_landuse(lu, terroir);
    let v2 = v2b_parcels(p, footprint, s, lu);
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
    if (w_grass > 0.01) { acc = sample_role(0u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, v2.grass_tint, det, w_grass, acc); }
    if (w_farm > 0.01) { acc = sample_role(1u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, v2.farm_tint, det, w_farm, acc); }
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
    // GA4 macro variation of hue (fine octaves first) and luminance (wide octaves first), the
    // same 4 noise samples (ga_macro.gdshaderinc); the luminance part fades out with the footprint.
    if (ground.macro_amp.y > 0.0 || ground.macro_amp.x > 0.0) {
        let mo = vec4<f32>(vnoise(p / ground.macro_periods.x + vec2<f32>(3.1, 7.7)),
            vnoise(p / ground.macro_periods.y + vec2<f32>(11.3, 1.9)),
            vnoise(p / ground.macro_periods.z + vec2<f32>(5.3, 17.2)),
            vnoise(p / ground.macro_periods.w + vec2<f32>(23.9, 13.4)));
        let wsum4 = max(dot(ground.macro_weights, vec4<f32>(1.0)), 1e-3);
        let mh = dot(mo, ground.macro_weights) / wsum4;
        let ml = dot(mo, ground.macro_weights.wzyx) / wsum4;
        let lum_amp = ground.macro_amp.y * mix(1.0, MACRO_LUM_FAR_KEEP, smoothstep(MACRO_LUM_FADE.x, MACRO_LUM_FADE.y, footprint));
        let tint = vec3<f32>(1.0 + (mh - 0.5) * 0.06 * ground.macro_amp.x,
            1.0 + (mh - 0.5) * 0.04 * ground.macro_amp.x,
            1.0 - (mh - 0.5) * 0.05 * ground.macro_amp.x);
        col = col * tint * (1.0 + (ml - 0.5) * 2.0 * lum_amp);
    }
    // Canopy: lighter volume shading, forests darker and grainier.
    col = col * (1.0 - 0.25 * w_forest / wsum * (1.0 - vnoise(p * 1.7)) * detail_fade);
    // V2b balks, vine rows, dirt tracks and hedges.
    let farm_open = (w_grass + w_farm) / wsum;
    let macro_n = fbm(p * 0.035);
    col = mix(col, col * vec3<f32>(0.93, 0.95, 0.9), v2.balk * farm_open);
    col = mix(col, season_vine_leaf(VINE_LEAF), v2.vine_rows * farm_open * 0.8);
    col = mix(col, TRACK_COLOR * (0.9 + 0.2 * macro_n), v2.track * farm_open * 0.85);
    col = mix(col, HEDGE_COLOR, v2.hedge * farm_open * 0.55);
    // ZG5b fine parcels in metres (valley / site views).
    let fp = fp_parcels(p, footprint, grad, slope, altitude, s, lu, terroir, farm_open, w_forest / wsum, col);
    col = fp.color;
    out.dn = out.dn + fp.dn;
    out.color = col;
    out.open_w = (w_grass + w_farm + w_heath) / wsum;
    out.forest_w = w_forest / wsum;
    out.rock_w = w_rock / wsum;
    out.farm_share = w_farm / max(w_grass + w_farm, 1e-3);
    // TB1: season shift (season / summer) of the land cover, laid over the mean-season colour
    // map and materials (terrain.gdshader before sg_apply).
    let forest_far = season_forest_mean(TINT_FOREST);
    var k_forest = season_ratio(mix(season_forest(TINT_FOREST, vnoise(p * 0.45)), forest_far, smoothstep(0.3, 1.0, footprint)), TINT_FOREST);
    let russet = mix(vec3<f32>(1.0), AUTUMN_RUSSET, season.weights.z);
    let k_grass = v2.k_grass * russet;
    k_forest = k_forest * russet;
    let k_farm = v2.k_farm * mix(vec3<f32>(1.0), russet, 0.5);
    let k_heath = mix(vec3<f32>(1.0), k_grass, 0.6);
    let num = w_grass * v2.grass_summer * k_grass + w_farm * v2.farm_summer * k_farm + w_forest * TINT_FOREST * k_forest
        + w_rock * TINT_ROCK + w_heath * TINT_HEATH * k_heath;
    let den = w_grass * v2.grass_summer + w_farm * v2.farm_summer + w_forest * TINT_FOREST + w_rock * TINT_ROCK + w_heath * TINT_HEATH;
    out.season_k = select(vec3<f32>(1.0), season_ratio(num, den), dot(den, vec3<f32>(1.0)) > 1e-4);
    out.k_wild = k_grass;
    out.k_forest = k_forest;
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

fn hb_maille_far(m: i32, row: u32) -> vec4<f32> {
    if (m >= 0) {
        let i = row * 12u + u32(m);
        return vec4<f32>(fields.slots[i].rgb, fields.slot_tex[i].z);
    }
    if (m == -3) {
        return vec4<f32>(fields.vine.rgb, fields.tex_flags.x);
    }
    return vec4<f32>(fields.meadow.rgb, fields.tex_flags.y);
}

// Far colour of a mesh in its season (`hb_patch_color`): luminance jitter and saturation.
fn hb_patch_color(id: vec2<f32>, far: vec4<f32>) -> vec3<f32> {
    let lumv = 1.0 + 2.0 * fields.v1.y * (hb_hash(id + 3.3) - 0.5);
    let c = season_field(far.rgb, i32(far.a + 0.5), hb_hash(id + 61.3)) * lumv;
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

// One parcels material (`hb_sample`): layer, tile (m), rotated by `ang` around the map origin.
fn hb_sample(layer: f32, tile_m: f32, p: vec2<f32>, dpx: vec2<f32>, dpy: vec2<f32>, ang: f32) -> vec3<f32> {
    let r = hb_rot(ang);
    let k = max(ground.micro.w, 1.0) / max(tile_m, 1.0);
    return textureSampleGrad(parcels_albedo, ground_sampler, r * p * k, i32(layer + 0.5), r * dpx * k, r * dpy * k).rgb;
}

// Normal of the same material, back in the map frame (x, z).
fn hb_sample_normal(layer: f32, tile_m: f32, p: vec2<f32>, dpx: vec2<f32>, dpy: vec2<f32>, ang: f32) -> vec2<f32> {
    let r = hb_rot(ang);
    let k = max(ground.micro.w, 1.0) / max(tile_m, 1.0);
    let t = textureSampleGrad(parcels_normal, ground_sampler, r * p * k, i32(layer + 0.5), r * dpx * k, r * dpy * k).rg * 2.0 - 1.0;
    return transpose(r) * t;
}

fn tex_mean(layer: f32) -> vec3<f32> {
    return fields.tex_means[min(u32(max(layer, 0.0) + 0.5), 47u)].rgb;
}

struct FieldsOut {
    color: vec3<f32>,
    dn: vec2<f32>,
};

// Port of `hb_apply` (hb_ground.gdshaderinc) on `col` (the colormap-blended ground): near and
// mid view, the parcels of the landscape drawn with their own materials (`tx_campaign_parcels`:
// crop of the parcel, wild soil of the unfarmed ones, canopy, rock), their season, hedges and
// rims; far, the patchwork of soles and terroirs. Crops follow the biome / landscape table,
// relief and water decide where they grow (GR, ADR 0352).
fn fields_apply(p: vec2<f32>, uv: vec2<f32>, footprint_in: f32, land: f32, open_w: f32, forest_w: f32,
        rock_w: f32, farm_share: f32, slope: f32, grad: vec2<f32>, alt: f32, k_wild: vec3<f32>,
        k_forest: vec3<f32>, col_in: vec3<f32>) -> FieldsOut {
    var out = FieldsOut(col_in, vec2<f32>(0.0));
    // Derivatives first (uniform control flow).
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    // Godot's footprint is the screen derivative (`max(length(dFdx(p)), length(dFdy(p)))`), larger
    // than the distance footprint on grazing ground: the patch colour of the crops comes in there.
    let footprint = max(footprint_in, max(length(dpx), length(dpy)));
    var col = col_in;
    let mpp = max(ground.micro.w, 1.0);
    let fade = smoothstep(fields.v0.x, fields.v0.y, footprint);
    let w_bare = fields.v5.x * (1.0 - fade) * land;
    var w_far = 0.0;
    if (fields.v2.y > 0.0) {
        w_far = fields.v2.w * fade * (1.0 - smoothstep(fields.v3.x, fields.v3.y, footprint)) * land;
    }
    if (w_bare <= 0.001 && w_far <= 0.001) {
        return out;
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
    out.color = col;
    if (w_bare <= 0.001) {
        return out;
    }
    let has_tex = fields.tex_flags.z > 0.5 && fields.rows_e[row].w > 0.5;
    let sw = season.weights;
    // Unfarmed parcel: the two wild soils of the row, mixed by a slow noise.
    let wild_mix = smoothstep(0.35, 0.65, vnoise(p * 0.35 + 3.7));
    var open_col = col;
    if (has_tex) {
        let wd = fields.rows_d[row];
        open_col = mix(hb_sample(wd.x, wd.y, p, dpx, dpy, ang_r * 0.3), hb_sample(wd.z, wd.w, p, dpx, dpy, 1.1), wild_mix);
    }
    open_col = open_col * pow(k_wild, vec3<f32>(HB_SEASON_STRENGTH));  // TB1
    var hedge_mask = 0.0;
    var nrm = vec2<f32>(0.0);
    let farmed = hb_hash(parcel.id + 71.7) < max(farm_share, fields.v4.w) * presence;
    if (farmed) {
        var key = parcel.id;
        var sole = false;
        if (fields.v2.y > 0.0 && hb_hash(parcel.id + 5.1) < fields.v2.z) {
            key = hb_voronoi(parcel.site * fields.v2.y).id;
            sole = true;
        }
        let m = hb_maille(key, row, h_dom, vine_k, meadow_k);
        var layer = -1.0;
        var tile = 120.0;
        if (m >= 0) {
            let st = fields.slot_tex[row * 12u + u32(m)];
            layer = st.x;
            tile = st.y;
        } else if (m == -2) {
            layer = fields.tex_extra.z;
            tile = fields.tex_extra.w;
        } else if (m == -3) {
            layer = fields.tex_extra.x;
            tile = fields.tex_extra.y;
        }
        if (m != -1) {
            let far = hb_maille_far(m, row);
            let cls = i32(far.a + 0.5);
            let field_ang = ang_r + select(1.5708, 0.0, hb_hash(parcel.id + 5.3) < 0.85);
            let textured = has_tex && layer >= 0.0;
            var mean = vec3<f32>(max(luma(col), 1e-3));
            if (textured) {
                open_col = hb_sample(layer, tile, p, dpx, dpy, field_ang);
                nrm = hb_sample_normal(layer, tile, p, dpx, dpy, field_ang);
                mean = tex_mean(layer);
            } else {
                open_col = col;
            }
            // Each parcel has its own nuance (maturity, soil, exposure).
            let j1 = hb_hash(parcel.id + 3.3) - 0.5;
            let j2 = hb_hash(parcel.id + 9.1) - 0.5;
            open_col = open_col * (1.0 + 2.0 * fields.v1.y * j1) * vec3<f32>(1.0 + fields.v1.z * j2, 1.0, 1.0 - fields.v1.z * j2);
            // TB1: season of the parcel; the material is of a mean season (summer gilds the wheat,
            // the other seasons take the crop's season colour and keep the grain).
            if (cls == 0) {
                open_col = open_col * mix(vec3<f32>(1.0), season.summer_gold.rgb, sw.y * 0.8);
            }
            if (sw.y < 0.999) {
                let grain_s = clamp(luma(open_col) / max(luma(mean), 1e-3), 0.6, 1.5);
                let season_col = season_field(far.rgb, cls, hb_hash(parcel.id + 61.3));
                open_col = mix(open_col, season_col * grain_s, 0.8 * (1.0 - sw.y));
            }
            // ADR 0253: from mid distance the parcel takes the colour of its crop, keeping the
            // grain of the material (luminance relative to its mean).
            let patch_k = fields.v0.z * smoothstep(fields.v0.w, fields.v1.x, footprint);
            if (patch_k > 0.001) {
                let lumv = 1.0 + 2.0 * fields.v1.y * (hb_hash(parcel.id + 3.3) - 0.5);
                var far_col = season_field(far.rgb, cls, hb_hash(parcel.id + 61.3)) * lumv;
                if (sole) {
                    far_col = hb_patch_color(key, far);
                }
                let grain_l = clamp(luma(open_col) / max(luma(mean), 1e-3), 0.7, 1.35);
                open_col = mix(open_col, far_col * grain_l, patch_k);
            }
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
            hedge_mask = hedge;
            nrm = nrm * (1.0 - hedge);
        }
    }
    var canopy_col = col;
    var rock_col = col;
    if (has_tex) {
        let cr = fields.rows_c[row];
        canopy_col = hb_sample(cr.x, cr.y, p, dpx, dpy, 0.4) * pow(k_forest, vec3<f32>(HB_SEASON_STRENGTH));
        rock_col = hb_sample(cr.z, cr.w, p, dpx, dpy, 0.0);
    }
    // Rock: rock share of the splat, more on the steep slopes.
    let rock_share = clamp(rock_w + smoothstep(0.35, 0.8, slope) * (1.0 - forest_w), 0.0, 1.0);
    let total = max(open_w + forest_w + rock_share, 1e-3);
    var hb_col = (open_col * open_w + canopy_col * forest_w + rock_col * rock_share) / total;
    if (has_tex) {
        hb_col = hb_col * fields.rows_e[row].rgb;
    }
    // Hue: a share of the colour map's (consistent with the far view), luminance in the band.
    let lum_hb = luma(hb_col);
    let lum_c = max(luma(col), 1e-3);
    hb_col = mix(hb_col, col * (lum_hb / lum_c), fields.v5.z);
    hb_col = hb_relative(col, hb_col);
    out.color = mix(col, hb_col, w_bare);
    out.dn = nrm * w_bare * open_w / total * ground.tiling.w * 0.6;
    return out;
}

// Unlit share of the parchment (`pm_emission`, `pm_emission_gain` of parchment_common.gdshaderinc).
const PM_EMISSION = 0.75;
const PM_EMISSION_GAIN = 0.72;

// Visibility of the sun (cascaded shadow map of the `CampaignSun`, sized to the camera by
// `shadows.rs`): 1 lit, 0 in the shadow of the relief or of an object.
fn sun_shadow(world: vec3<f32>, n: vec3<f32>, frag_xy: vec2<f32>) -> f32 {
    if (lights.n_directional_lights == 0u) {
        return 1.0;
    }
    if ((lights.directional_lights[0].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u) {
        return 1.0;
    }
    let wp = vec4<f32>(world, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), wp);
    return shadows::fetch_directional_shadow(0u, wp, n, view_z, frag_xy);
}

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    var albedo = vec3<f32>(0.10, 0.15, 0.06);
    let k_proj_ground = view.clip_from_view[1][1] * view.viewport.w * 0.5;
    let ground_footprint = distance(in.world, view.world_position) / max(k_proj_ground, 1.0);
    if (params.map.w > 0.5) {
        let cm_uv = in.world.xz / params.map.xy;
        albedo = textureSample(colormap, colormap_sampler, cm_uv).rgb;
        let soft = textureSampleBias(colormap, colormap_sampler, cm_uv, ground.dark.z).rgb;
        if (ground.bg.x > 0.5) {
            albedo = dark_masses(albedo, soft, in.world.xz, ground_footprint);
        }
    }
    var n = normalize(in.normal);
    let base_albedo = albedo;
    if (ground.bg.x > 0.5) {
        // Regional ground (TX): the colormap gives the colour far away, the textured soil draws
        // close up (sg_apply of satellite_ground.gdshaderinc: share 0.45 at the closest, 1 far).
        let slope_m = length(in.slope_xz) / max(ground.micro.w, 1.0);
        let g = ground_proc(in.world.xz, in.world.xz / params.map.xy, ground_footprint, in.altitude, slope_m, in.slope_xz);
        let land = smoothstep(0.0, 2.0, in.altitude);
        var painted = g.color;
        if (params.map.w > 0.5) {
            let far_w = smoothstep(0.05, 0.5, ground_footprint);
            let ratio = clamp(luma(g.color) / max(luma(base_albedo), 1e-3), 0.55, 1.6);
            // TB1: the season shift of the land cover over the mean-season colour map.
            let tinted = base_albedo * mix(1.0, ratio, 0.55 * (1.0 - far_w)) * pow(g.season_k, vec3<f32>(SG_SEASON_STRENGTH));
            painted = mix(g.color, tinted, mix(0.45, 1.0, far_w));
        }
        // Painted fields over the ground colour (after the colormap, like hb_apply).
        var dn = g.dn;
        if (fields.v8.w > 0.5) {
            let fo = fields_apply(in.world.xz, in.world.xz / params.map.xy, ground_footprint, land,
                g.open_w, g.forest_w, g.rock_w, g.farm_share, slope_m, in.slope_xz, in.altitude, g.k_wild,
                g.k_forest, painted);
            painted = fo.color;
            dn = dn + fo.dn;
        }
        albedo = mix(base_albedo, painted, land);
        n = normalize(vec3<f32>(n.x + dn.x * land, n.y, n.z - dn.y * land));
        // Forest crowns: relief and tone contrast of the canopy (0.45 bump, 0.35 contrast).
        let forest_f = g.forest_w * land;
        if (forest_f > 0.01) {
            let cf = canopy_field(in.world.xz, ground_footprint);
            n = normalize(vec3<f32>(n.x - cf.y * 0.45 * forest_f, n.y, n.z - cf.z * 0.45 * forest_f));
            albedo = albedo * (1.0 + 0.35 * forest_f * cf.x);
        }
    }
    albedo = season_ground(albedo, n, in.altitude, in.world.xz);
    albedo = weather_ground(albedo, n, in.altitude, in.world.xz);
    let to_eye = normalize(view.world_position - in.world);
    albedo = weather_mist(albedo, in.world.xz, in.altitude, ground_footprint, 1.0 - abs(to_eye.y));
    albedo = fog_of_war_veil(albedo, in.world.xz);
    // RV-C: fair-weather cumulus shadows (gone on the parchment).
    let area_px = sqrt(length(dpdx(in.world.xz)) * length(dpdy(in.world.xz)));
    albedo = albedo * cloud_shadow_light(in.world.xz, ground_footprint, area_px, 1.0 - look.weather.z);
    // Strategic view: fade to the parchment map (ADR 0124). Port of `pm_output`
    // (parchment_common.gdshaderinc): the paper goes mostly through the emission (unlit flat
    // colour, `pm_emission` 0.75 x gain 0.72), a quarter of it stays in the lit albedo.
    let parchment = look.weather.z;
    var paper_emit = vec3<f32>(0.0);
    if (parchment > 0.001) {
        let k_proj = view.clip_from_view[1][1] * view.viewport.w * 0.5;
        let footprint = distance(in.world, view.world_position) / max(k_proj, 1.0);
        var paper = parchment_ground(base_albedo, n, in.altitude, in.world.xz, in.world.xz / params.map.xy, footprint);
        // The veil of the fog of war survives on the parchment as a darker wash.
        paper = fog_of_war_veil(paper, in.world.xz);
        albedo = mix(albedo, paper * (1.0 - PM_EMISSION), parchment);
        paper_emit = paper * PM_EMISSION * parchment * PM_EMISSION_GAIN;
        n = normalize(mix(n, vec3<f32>(0.0, 1.0, 0.0), parchment));
    }
    // RV-D: valley occlusion, last tint of the ground before the light (Godot: after the borders).
    let rvd = rvd_apply(in.world.xz / params.map.xy, ground_footprint, smoothstep(0.0, 2.0, in.altitude), albedo);
    albedo = rvd.color;
    let ndl = max(dot(n, params.sun_dir.xyz), 0.0);
    let sun_vis = sun_shadow(in.world, n, in.clip.xy);
    var color = albedo * (params.ambient.rgb * rvd.ao + params.sun_color.rgb * ndl * sun_vis) * params.ambient.a;
    color = campaign_grade(color + paper_emit);
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
