// Campaign terrain module `atmosphere` (VIS-L1, split of terrain.wesl): weather mist, cloud shadows, aerial fog, fog of war, the final grade.
// Pure move of the code, no change of behaviour.

import bevy_pbr::render::mesh_view_bindings::{view, globals, fog, lights};
import bevy_pbr::render::mesh_view_types;
import bevy_pbr::render::fog as bevy_fog;
import cent_ans_client::campaign::weather::ground::wx_field;
import cent_ans_client::campaign::weather::ground::wx_selection_clear;
import cent_ans_client::campaign::weather::ground::wx_scatter;
import cent_ans_client::campaign::weather::ground::wx_aerial_fog_color;
import cent_ans_client::campaign::terrain::noise::{fbm, luma, vnoise};
import cent_ans_client::campaign::terrain::parchment::{pm_height};
import cent_ans_client::campaign::terrain::snow::{snow_apply, snow_cover_of, snow_patch_noise};
import cent_ans_client::campaign::terrain::terrain_params::{extras, fog_mask, ground, look, occlusion_tex, params, season, shade, terroir_sampler, wx_clear_tex, wx_field_tex};

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
    let wb = wx_at(p).b;
    if (wb <= 0.001) {
        return col;
    }
    // TB2: nothing veils the selected province.
    let clear = wx_clear_at(p);
    let morning = look.mist.y;
    let t = globals.time;
    let near_k = smoothstep(WX_MIST_NEAR_FOOTPRINT * 0.5, WX_MIST_NEAR_FOOTPRINT * 2.0, footprint);
    let low = 1.0 - smoothstep(150.0, 700.0, hc);
    let patches = smoothstep(0.3, 0.75, fbm(p * 0.02 - t * WX_WIND * 0.004));
    var mist = wb * mix(0.3, 0.75, low) * mix(0.35, 1.0, morning) * (0.35 + 0.65 * patches);
    mist = mist * mix(WX_MIST_NEAR, 1.0, near_k);
    let veil = mix(vec3<f32>(0.62, 0.64, 0.66), col * 1.6, 0.35);
    var out = mix(col, veil, clamp(mist, 0.0, WX_MIST_MAX) * clear);
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
        let valley = clamp(wb * optical * lifted, 0.0, 1.0) * opacity * near * clear;
        out = mix(out, look.valley_color.rgb, valley);
    }
    return out;
}

// Weather at the map point `p` (Godot `wx_sample`): R rain, G snow, B fog, A storm, with the
// noisy fringe between provinces baked in `WeatherGround::field`. Zero without weather.
fn wx_at(p: vec2<f32>) -> vec4<f32> {
    if (extras.wx.x < 0.5) {
        return vec4<f32>(0.0);
    }
    return wx_field(wx_field_tex, terroir_sampler, p, params.map.xy);
}

// 1 outside the selected province, 0 inside (feathered); 1 without a selection.
fn wx_clear_at(p: vec2<f32>) -> f32 {
    if (extras.wx.y < 0.5) {
        return 1.0;
    }
    return wx_selection_clear(wx_clear_tex, terroir_sampler, p, params.map.xy);
}

// Wet ground (slightly darker, TB6 `weather_wet_dim`) and fresh snow where it rains / snows
// (`weather_ground` of campaign_weather.gdshaderinc, per province).
fn weather_ground(col: vec3<f32>, altitude: f32, p: vec2<f32>, disp_y: f32, footprint: f32, lit: f32,
        forest: f32) -> vec3<f32> {
    let w = wx_at(p);
    if (dot(w, vec4<f32>(1.0)) <= 0.0) {
        return col;
    }
    let land = smoothstep(0.0, 4.0, altitude);
    let rain = max(w.r, w.a);
    var out = col * (1.0 - look.mist.z * rain * land);
    // Fresh snow (TS-T1): patches of the season snow mask, off the steep displayed slopes, tinted
    // like the season snow, with its mud fringe.
    if (w.g > 0.0) {
        let pn = snow_patch_noise(p + vec2<f32>(31.7, 12.9), footprint);
        let c = w.g * 0.7 * mix(1.0, season.snow_shade.a, forest);
        var cover = snow_cover_of(c, pn, disp_y);
        cover.snow = cover.snow * land;
        cover.mud = cover.mud * land;
        cover.frost = forest;
        out = snow_apply(out, cover, lit);
    }
    return out;
}

// Fog-of-war veil: sepia ground under a wash, with a drifting remnant of mist (TB2, BR).
fn fog_of_war_veil(col: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    if (look.fog_a.w < 0.5) {
        return col;
    }
    let jitter = vec2<f32>(fbm(p * 0.035), fbm(p * 0.035 + vec2<f32>(31.7, 5.3))) - 0.5;
    let q = (p + jitter * look.fog_veil.w) / look.fog_map.xy;
    let seen = textureSampleLevel(fog_mask, terroir_sampler, q, 0.0).r;
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
// The fog brightens towards the sun (Godot `fog_sun_scatter`, set by `weather::atmosphere` as the
// `DistanceFog` directional light colour): same term as the PBR meshes, through `wx_scatter`.
fn camera_distance_fog(color: vec3<f32>, world: vec3<f32>, sun_vis: f32) -> vec3<f32> {
    if (fog.mode == mesh_view_types::FOG_MODE_OFF) {
        return color;
    }
    let distance = length(world - view.world_position);
    let c = vec4<f32>(color, 1.0);
    // Aerial perspective per pixel (Godot `fog_aerial_perspective`): the fog takes the sky tint
    // along the pixel's own view ray. The CPU sets the centre-ray blend as `base_color`; the
    // blend is redone in sRGB (as the CPU does) from the season colour.
    var haze = fog;
    if (look.aerial_base.w > 0.5) {
        let ray_y = normalize(world - view.world_position).y;
        let srgb = wx_aerial_fog_color(look.aerial_base.rgb, look.aerial_sky.rgb, look.aerial_sky.w, ray_y);
        let lo = srgb / 12.92;
        let hi = pow((srgb + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
        haze.base_color = vec4<f32>(select(hi, lo, srgb <= vec3<f32>(0.04045)), fog.base_color.a);
    }
    var none = vec3<f32>(0.0);
    if (fog.directional_light_color.a > 0.0 && lights.n_directional_lights > 0u) {
        let light = lights.directional_lights[0];
        none = wx_scatter(world - view.world_position, light.direction_to_light,
            light.color.rgb * view.exposure, 1.0, fog.directional_light_exponent) * sun_vis;
    }
    if (fog.mode == mesh_view_types::FOG_MODE_LINEAR) {
        return bevy_fog::linear_fog(haze, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL) {
        return bevy_fog::exponential_fog(haze, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED) {
        return bevy_fog::exponential_squared_fog(haze, c, distance, none).rgb;
    }
    return color;
}
