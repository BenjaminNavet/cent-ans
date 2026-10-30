// Campaign terrain module `ground_albedo` (VIS-L1, split of terrain.wesl): regional ground albedo: ground_proc, regional_ground and the colour map base.
// Pure move of the code, no change of behaviour.

import bevy_pbr::render::mesh_view_bindings::{view, lights};
import bevy_pbr::render::mesh_view_types;
import bevy_pbr::render::shadows;
import cent_ans_client::campaign::terrain::ground_detail::gd_macro;
import cent_ans_client::campaign::terrain::fields_paint::{fields_apply};
import cent_ans_client::campaign::terrain::ground_look::ground_look;
import cent_ans_client::campaign::terrain::ground_layers::{LayerAcc, canopy_field, hb_clearing_splat, sample_role, terroir_at, terroir_landuse, terroir_splat};
import cent_ans_client::campaign::terrain::noise::{fbm, luma, vnoise};
import cent_ans_client::campaign::terrain::parcels::{fp_parcels, fp_splat, v2b_parcels};
import cent_ans_client::campaign::terrain::seasons::{season_forest, season_forest_mean, season_ratio, season_vine_leaf};
import cent_ans_client::campaign::terrain::terrain_params::{VINE_LEAF, ab_dist, colormap, colormap_sampler, fields, gd, ground, ground_sampler, landuse_tex, micro_albedo, params, season, splat, terroir_sampler};

const TINT_FOREST = vec3<f32>(0.054, 0.1023, 0.0275);

const TINT_ROCK = vec3<f32>(0.19, 0.17, 0.15);

const TINT_HEATH = vec3<f32>(0.15, 0.11, 0.08);

const ROCK_LINE_M = 1700.0;

const TRACK_COLOR = vec3<f32>(0.26, 0.20, 0.12);

const HEDGE_COLOR = vec3<f32>(0.035, 0.05, 0.02);

// TB1 (campaign_life.gdshaderinc): background tints of the meadows / crops / forests by season.
const AUTUMN_RUSSET = vec3<f32>(1.03, 0.80, 0.86);

// `sg_season_strength`, `hb_season_strength`.
const SG_SEASON_STRENGTH = 1.3;

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
    // VIS-R7: the floor lifts the hue (chroma of the fully lifted colour) but only `value_lift` of
    // the value, so the forest masses stay darker than the crops.
    let full = max(l, mix(floor_l, l, ground.dark.w));
    let c_full = cm * (full / l);
    let lifted_l = mix(l, full, ground.macro_amp.z);
    let toned = max(vec3<f32>(lifted_l) + (c_full - vec3<f32>(full)), vec3<f32>(0.0));
    let crowns = (vnoise(q * 0.22 + vec2<f32>(7.9)) - 0.5) * 0.9
        + (vnoise(q * 0.9) - 0.5) * 0.7 * (1.0 - smoothstep(0.4, 1.0, footprint))
        + (vnoise(q * 2.7 + vec2<f32>(3.1)) - 0.5) * 0.5 * (1.0 - smoothstep(0.15, 0.4, footprint));
    cm = mix(cm, toned * (1.0 + ground.macro_amp.w * crowns * 2.0), dark);
    return cm;
}

// TS-T1: the transitions between the open soils (grass, farmland, heath / rock) in irregular
// patches instead of gradients: per-soil noise on the weights, then a sharpening. The forest share
// stays as it is (the trees follow it).
fn soil_patches(s_in: vec4<f32>, p: vec2<f32>, footprint: f32) -> vec4<f32> {
    let k = season.patches.y;
    let open_sum = s_in.r + s_in.g + s_in.a;
    if (k <= 0.0 || open_sum < 1e-3) {
        return s_in;
    }
    let f = season.patches.x;
    let fine = 1.0 - smoothstep(0.1, 0.6, footprint);
    let q = p * f;
    let nr = vnoise(q + vec2<f32>(1.7, 9.2)) * 0.65 + mix(0.5, vnoise(q * 3.7 + vec2<f32>(5.1, 2.3)), fine) * 0.35;
    let ng = vnoise(q + vec2<f32>(8.3, 4.4)) * 0.65 + mix(0.5, vnoise(q * 3.7 + vec2<f32>(0.6, 7.7)), fine) * 0.35;
    let na = vnoise(q + vec2<f32>(3.9, 6.1)) * 0.65 + mix(0.5, vnoise(q * 3.7 + vec2<f32>(4.2, 1.9)), fine) * 0.35;
    var o = vec3<f32>(s_in.r, s_in.g, s_in.a) * mix(vec3<f32>(1.0), vec3<f32>(0.2) + 1.6 * vec3<f32>(nr, ng, na), k);
    o = pow(max(o, vec3<f32>(0.0)), vec3<f32>(max(season.patches.z, 1.0)));
    o = o * open_sum / max(o.x + o.y + o.z, 1e-4);
    return vec4<f32>(o.x, o.y, s_in.b, o.z);
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
        s = textureSampleLevel(splat, terroir_sampler, uv, 0.0);
        @if(!PERF_TNOFPSPLAT) {
            s = fp_splat(s, p, footprint);
        }
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
    s = soil_patches(s, p, footprint);
    // Land use (R vine, G dryness, B bocage) with the vines of the terroirs; V2b parcels.
    var lu = vec3<f32>(0.0);
    if (ground.splat.z > 0.5) {
        lu = textureSampleLevel(landuse_tex, terroir_sampler, uv, 0.0).rgb;
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
    let dist_texels = textureSampleLevel(ab_dist, terroir_sampler, uv, 0.0).g * 255.0;
    let wb = 0.5 * (1.0 - smoothstep(0.0, ground.bg.y, dist_texels));
    var acc = LayerAcc(vec3<f32>(0.0), vec2<f32>(0.0));
    var role_min = 0.01;
    @if(PERF_TONEROLE) {
        role_min = max(max(w_grass, w_farm), max(max(w_forest, w_rock), w_heath)) - 1e-4;
    }
    if (w_grass > role_min) { acc = sample_role(0u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, v2.grass_tint, det, detail_fade, w_grass, acc); }
    if (w_farm > role_min) { acc = sample_role(1u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, v2.farm_tint, det, detail_fade, w_farm, acc); }
    if (w_forest > role_min) { acc = sample_role(2u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_FOREST, det, detail_fade, w_forest, acc); }
    if (w_rock > role_min) { acc = sample_role(3u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_ROCK, det, detail_fade, w_rock, acc); }
    if (w_heath > role_min) { acc = sample_role(4u, ba, bb, wb, uv_a, uv_b, da, db, mix_b, TINT_HEATH, det, detail_fade, w_heath, acc); }
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
    var micro_on = micro_k > 0.0;
    @if(PERF_TNOMICRO) {
        micro_on = false;
    }
    if (micro_on) {
        let grain = textureSampleGrad(micro_albedo, ground_sampler, muv, i32(mlayer), mdx, mdy).rgb;
        let g_ratio = luma(grain) / max(ground.micro_mean[mlayer / 4u][mlayer % 4u], 0.01);
        col = col * mix(1.0, clamp(g_ratio, 0.4, 1.8), micro_k);
    }
    // GA4 macro variation of hue (fine octaves first) and luminance (wide octaves first), the
    // same 4 noise samples (ga_macro.gdshaderinc); the luminance part fades out with the footprint.
    var macro_on = ground.macro_amp.y > 0.0 || ground.macro_amp.x > 0.0;
    @if(PERF_TNOMACRO) {
        macro_on = false;
    }
    if (macro_on) {
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
    @if(!PERF_TNOFP) {
        let fp = fp_parcels(p, footprint, grad, slope, altitude, s, lu, terroir, farm_open, w_forest / wsum, col);
        col = fp.color;
        out.dn = out.dn + fp.dn;
    }
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

// Regional ground (TX + painted fields + close detail + forest floor and canopy) of a map point:
// the costly, view-independent part of the ground colour, shared by the live fragment and the
// ground-cache bake (ADR 0742). `dn` offsets the normal: n = normalize(n + (dn.x, 0, dn.y)).
struct RegionalOut {
    albedo: vec3<f32>,
    dn: vec2<f32>,
    forest_f: f32,
};

fn regional_ground(p: vec2<f32>, footprint: f32, altitude: f32, slope_xz: vec2<f32>, base_albedo: vec3<f32>) -> RegionalOut {
    var albedo = base_albedo;
    var forest_f = 0.0;
    var nd = vec2<f32>(0.0);
    // Regional ground (TX): the colormap gives the colour far away, the textured soil draws
    // close up (sg_apply of satellite_ground.gdshaderinc: share 0.45 at the closest, 1 far).
    let slope_m = length(slope_xz) / max(ground.micro.w, 1.0);
    let g = ground_proc(p, p / params.map.xy, footprint, altitude, slope_m, slope_xz);
    let land = smoothstep(0.0, 2.0, altitude);
    var painted = g.color;
    if (params.map.w > 0.5) {
        let far_w = smoothstep(0.05, 0.5, footprint);
        let ratio = clamp(luma(g.color) / max(luma(base_albedo), 1e-3), 0.55, 1.6);
        // TB1: the season shift of the land cover over the mean-season colour map.
        let tinted = base_albedo * mix(1.0, ratio, 0.55 * (1.0 - far_w)) * pow(g.season_k, vec3<f32>(SG_SEASON_STRENGTH));
        // VIS-R3b: the share of the colormap at the closest is data (view.colormap_near_share).
        painted = mix(g.color, tinted, mix(fields.pc0.x, 1.0, far_w));
    }
    // Painted fields over the ground colour (after the colormap, like hb_apply).
    var dn = g.dn;
    var fields_on = fields.v8.w > 0.5;
    @if(PERF_TNOFIELDS) {
        fields_on = false;
    }
    if (fields_on) {
        let fo = fields_apply(p, p / params.map.xy, footprint, land,
            g.open_w, g.forest_w, g.rock_w, g.farm_share, slope_m, slope_xz, altitude, g.k_wild,
            g.k_forest, painted);
        painted = fo.color;
        dn = dn + fo.dn;
    }
    @if(!PERF_TNOGDMACRO)
    painted = gd_macro(gd, painted, p, footprint, land, g.open_w);
    albedo = mix(base_albedo, painted, land);
    nd = nd + vec2<f32>(dn.x, -dn.y) * land;
    // Forest crowns: relief and tone contrast of the canopy (0.45 bump, 0.35 contrast).
    forest_f = g.forest_w * land;
    // TS-T1: green-black ground under the forests.
    albedo = albedo * mix(vec3<f32>(1.0), season.forest_floor.rgb, season.forest_floor.a * forest_f);
    if (forest_f > 0.01) {
        let cf = canopy_field(p, footprint);
        nd = nd - cf.yz * 0.45 * forest_f;
        albedo = albedo * (1.0 + 0.35 * forest_f * cf.x);
    }
    // VIS-L1: saturation, hue and value of the vegetation (data), after the canopy tone.
    albedo = ground_look(albedo, forest_f, land, altitude);
    return RegionalOut(albedo, nd, forest_f);
}

// Colour map of the ground (with the dark masses of the regional ground).
fn base_ground(p: vec2<f32>, footprint: f32) -> vec3<f32> {
    var albedo = vec3<f32>(0.10, 0.15, 0.06);
    if (params.map.w > 0.5) {
        let cm_uv = p / params.map.xy;
        albedo = textureSample(colormap, colormap_sampler, cm_uv).rgb;
        let soft = textureSampleBias(colormap, colormap_sampler, cm_uv, ground.dark.z).rgb;
        if (ground.bg.x > 0.5) {
            albedo = dark_masses(albedo, soft, p, footprint);
        } else {
            // Regional ground still loading (worker thread, 2 to 6 s): the parcels baked in the
            // colour map would show as blocky rose patches, so show its blurred regional tone
            // until the textured ground is bound (lot SOL).
            albedo = textureSampleBias(colormap, colormap_sampler, cm_uv, ground.dark.z + 2.0).rgb;
        }
    }
    return albedo;
}
