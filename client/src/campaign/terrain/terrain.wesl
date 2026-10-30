// Campaign ground (port of game/shaders/terrain.gdshader + relief_quadtree.gdshaderinc, ADR 0142
// satellite colormap, ADR 0353 exaggerated relief). The CPU meshes carry the altitude in metres
// and the relief fields; the displayed height `y = s*(h - K*max(h-base,0) + g*(1-K)*max(h-floor,0)
// + c*D)` is computed here so that the dynamic vertical scale needs no mesh rebuild.

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{view, fog};
import cent_ans_client::campaign::terrain::atmosphere::{camera_distance_fog, campaign_grade, cloud_shadow_light, fog_of_war_veil, rvd_apply, weather_ground, weather_mist};
import cent_ans_client::campaign::terrain::ground_albedo::{base_ground, regional_ground, sun_shadow};
import cent_ans_client::campaign::terrain::parchment::{parchment_ground, pm_height};
import cent_ans_client::campaign::terrain::snow::{fw_glacier_apply, mountain_stages, rock_apply, season_ground, season_snow};
import cent_ans_client::campaign::terrain::terrain_params::{ab_dist, cache_albedo, cache_dn, cache_shade, ground, look, params, season, shade, terroir_sampler};
import cent_ans_client::campaign::terrain::water_edge::{coast_band};

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
    // geomorph delta (m, parent level minus own), vertex spacing, ground cache layer (-1: none),
    // packed node uv (`ui + 65 vi + 4225 q`, ui/vi in 64ths, quads = 64 >> q; `mesh.rs`)
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
    // ground cache: node uv, layer (-1: none), world size of a cache texel
    @location(4) cache: vec4<f32>,
};

// Ground cache coordinates of a vertex (ADR 0742): uv in the node, layer, texel size.
fn cache_coords(morph: vec4<f32>) -> vec4<f32> {
    let packed = morph.w;
    let q = floor(packed / 4225.0);
    let rest = packed - q * 4225.0;
    let vi = floor(rest / 65.0);
    let ui = rest - vi * 65.0;
    let extent = morph.y * f32(64u >> u32(q));
    return vec4<f32>(ui / 64.0, vi / 64.0, morph.z, extent / f32(constants::GC_TEXELS));
}

fn displayed_y(h: f32, fa: vec2<f32>, fb: vec2<f32>) -> f32 {
    let s = params.scale.x;
    let g = params.scale.y;
    let c = params.scale.z;
    let k = c * fb.x;
    return s * (h - k * max(h - fa.y, 0.0) + g * (1.0 - k) * max(h - fa.x, 0.0) + c * fb.y);
}

// River bed carved into the close terrain (`river_carve` of `river_banks.gdshaderinc`, lot V4):
// the ribbons sit on the altitude sampled on their centre line, while the terrain between
// vertices can stay a few metres above it in narrow valleys. Godot carves the fine relief tiles
// near the camera; here the carve fades out with the camera distance (showfix, 16/10).
const RIVER_CARVE_DEPTH: f32 = 0.075; // world units (1 m = 0.006)

const RIVER_BANK_PX: f32 = 1.1;

const RIVER_CARVE_NEAR: vec2<f32> = vec2<f32>(220.0, 380.0);

fn river_carve(xz: vec2<f32>, local: vec3<f32>) -> f32 {
    let near = 1.0 - smoothstep(RIVER_CARVE_NEAR.x, RIVER_CARVE_NEAR.y, distance(local, view.world_position));
    if (near <= 0.0) {
        return 0.0;
    }
    // Signed distance to the bank (map px, negative in the bed), as in `map_cell`; an unbound
    // texture (white) reads as far from any river.
    let sd = (textureSampleLevel(ab_dist, terroir_sampler, xz / params.map.xy, 0.0).a * 255.0 - 128.0) / 16.0;
    return RIVER_CARVE_DEPTH * (1.0 - smoothstep(-0.3, RIVER_BANK_PX, sd)) * near;
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
    y = y - v.slope.z - river_carve(v.position.xz, vec3<f32>(v.position.x, y, v.position.z));
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position.x, y, v.position.z, 1.0));
    out.world = world.xyz;
    out.altitude = h;
    out.slope_xz = v.slope.xy;
    out.cache = cache_coords(v.morph);
    out.clip = position_world_to_clip(world.xyz);
    // Displayed gradient: dY/dh of the exaggeration formula times the altitude gradient.
    let k = params.scale.z * v.fields_b.x;
    let above_base = select(0.0, 1.0, h > v.fields_a.y);
    let above_floor = select(0.0, 1.0, h > v.fields_a.x);
    let ratio = params.scale.x * (1.0 - k * above_base + params.scale.y * (1.0 - k) * above_floor);
    out.normal = normalize(vec3<f32>(-ratio * v.slope.x, 1.0, -ratio * v.slope.y));
    return out;
}

// Unlit share of the parchment (`pm_emission`, `pm_emission_gain` of parchment_common.gdshaderinc).
const PM_EMISSION = 0.75;

const PM_EMISSION_GAIN = 0.72;

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    let k_proj_ground = view.clip_from_view[1][1] * view.viewport.w * 0.5;
    let ground_footprint = distance(in.world, view.world_position) / max(k_proj_ground, 1.0);
    var albedo = base_ground(in.world.xz, ground_footprint);
    var n = normalize(in.normal);
    let base_albedo = albedo;
    // Perf bisection (`CENT_ANS_PERF_OFF=tflat`): colour map only, flat light.
    @if(PERF_TFLAT) {
        return vec4<f32>(albedo * (params.ambient.rgb + params.sun_color.rgb * max(dot(n, params.sun_dir.xyz), 0.0)) * params.ambient.a, 1.0);
    }
    // TS-T1: slope of the displayed (exaggerated) relief, the steeper of the interpolated normal
    // and of the triangle (the vertex normal of a coarse node is smoothed), and the crests
    // (divergence of the normal along the screen axes, per map px).
    let dwx = dpdx(in.world);
    let dwy = dpdy(in.world);
    let facet = normalize(cross(dwy, dwx));
    let disp_y = min(n.y, abs(facet.y));
    let dnx = dpdx(n);
    let dny = dpdy(n);
    let curv = dot(dnx, dwx) / max(dot(dwx, dwx), 1e-8) + dot(dny, dwy) / max(dot(dwy, dwy), 1e-8);
    let crest = clamp(curv * season.rock_light.w, 0.0, 1.0);
    // Sun term of the geometric normal (snow tint, rock contrast), the shadow fetched once.
    var sun_vis = 1.0;
    @if(!PERF_TNOSUN) {
        sun_vis = sun_shadow(in.world, n, in.clip.xy);
    }
    // Baked relief shade (ADR 0745): sun visibility and horizon occlusion (1, 1 when off).
    var relief_vis = 1.0;
    var relief_ao = 1.0;
    // Ground cache (ADR 0742): a layer not baked yet (cleared, alpha 0) falls back to the base
    // ground, so a node entering the view is drawn at once instead of leaving a black hole.
    var cache_texel = vec4<f32>(0.0);
    var cache_uv = vec2<f32>(0.0);
    var cache_ok = false;
    @if(GROUND_CACHE) {
        let half_texel = 0.5 / f32(constants::GC_TEXELS);
        cache_uv = clamp(in.cache.xy, vec2<f32>(half_texel), vec2<f32>(1.0 - half_texel));
        if (in.cache.z >= 0.0) {
            cache_texel = textureSampleLevel(cache_albedo, terroir_sampler, cache_uv, i32(in.cache.z), 0.0);
            cache_ok = cache_texel.a > 0.25;
        }
        if (shade.sun.w > 0.5 && cache_ok) {
            let baked = textureSampleLevel(cache_shade, terroir_sampler, cache_uv, i32(in.cache.z), 0.0).rg;
            relief_vis = mix(1.0, baked.r, shade.mix.x);
            relief_ao = mix(1.0, baked.g, shade.mix.y);
        }
    }
    let lit0 = max(dot(n, params.sun_dir.xyz), 0.0) * sun_vis * relief_vis;
    var forest_f = 0.0;
    var regional = ground.bg.x > 0.5;
    @if(PERF_TNOPROC) {
        regional = false;
    }
    if (regional) {
        @if(GROUND_CACHE) {
            // Baked once per node (ADR 0742); the clamp keeps the bilinear inside the layer.
            // Alpha packs the forest share as 0.5 + 0.5 f (0: not baked, base ground kept).
            if (cache_ok) {
                let dn = textureSampleLevel(cache_dn, terroir_sampler, cache_uv, i32(in.cache.z), 0.0).xy;
                albedo = cache_texel.rgb;
                forest_f = saturate(cache_texel.a * 2.0 - 1.0);
                n = normalize(n + vec3<f32>(dn.x, 0.0, dn.y));
            } else {
                // Not baked yet: the same ground computed directly, so a node entering the view
                // already looks like its final self (the base ground showed as pale squares).
                let r = regional_ground(in.world.xz, ground_footprint, in.altitude, in.slope_xz, base_albedo);
                albedo = r.albedo;
                forest_f = r.forest_f;
                n = normalize(n + vec3<f32>(r.dn.x, 0.0, r.dn.y));
            }
        }
        @if(!GROUND_CACHE) {
            let r = regional_ground(in.world.xz, ground_footprint, in.altitude, in.slope_xz, base_albedo);
            albedo = r.albedo;
            forest_f = r.forest_f;
            n = normalize(n + vec3<f32>(r.dn.x, 0.0, r.dn.y));
        }
    }
    let land_g = smoothstep(0.0, 2.0, in.altitude);
    albedo = mountain_stages(albedo, in.world.xz, disp_y, in.altitude, ground_footprint, forest_f, n.xz, land_g);
    albedo = rock_apply(albedo, in.world.xz, disp_y, in.altitude, ground_footprint, lit0, crest * land_g, forest_f);
    let cover = season_snow(disp_y, in.altitude, in.world.xz, ground_footprint, forest_f, n.xz);
    albedo = season_ground(albedo, cover, lit0);
    // DN-ME4 névés and glaciers, then the TB5 coast band (cliffs, beaches, swash).
    let slope_g = length(in.slope_xz) / max(ground.micro.w, 1.0);
    albedo = fw_glacier_apply(albedo, in.world.xz, in.altitude, slope_g, ground_footprint, land_g, disp_y, lit0);
    @if(!PERF_TNOCOAST) {
        let coast_out = coast_band(in.world.xz, ground_footprint, in.altitude, in.world.y, slope_g,
            cover.snow, albedo, n);
        albedo = coast_out.color;
        n = coast_out.n;
    }
    @if(!PERF_TNOWX) {
        albedo = weather_ground(albedo, in.altitude, in.world.xz, disp_y, ground_footprint, lit0, forest_f);
        let to_eye = normalize(view.world_position - in.world);
        albedo = weather_mist(albedo, in.world.xz, in.altitude, ground_footprint, 1.0 - abs(to_eye.y));
    }
    albedo = fog_of_war_veil(albedo, in.world.xz);
    // RV-C: fair-weather cumulus shadows (gone on the parchment).
    let area_px = sqrt(length(dpdx(in.world.xz)) * length(dpdy(in.world.xz)));
    @if(!PERF_TNOCLOUD)
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
    var color = albedo * (params.ambient.rgb * rvd.ao * relief_ao + params.sun_color.rgb * ndl * sun_vis * relief_vis) * params.ambient.a;
    // TS-T9: a cast shadow is the sky light, never black: it keeps at least `1 - opacity` of the lit
    // ground, with the hue of the ambient (sky) light (see `shadows::capped_shadow`).
    let sky = params.ambient.rgb / max(dot(params.ambient.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 1e-4);
    let lit_full = albedo * (params.ambient.rgb * rvd.ao + params.sun_color.rgb * ndl) * params.ambient.a;
    let shade_floor = vec3<f32>(dot(lit_full, vec3<f32>(0.2126, 0.7152, 0.0722))) * sky * (1.0 - season.patches.w);
    color = max(color, shade_floor);
    color = campaign_grade(color + paper_emit);
    // Distance fog (depth fog of the Godot environment, simplified).
    let d = distance(in.world, view.world_position);
    let begin = params.map.z;
    let fog_t = clamp((d - begin) / max(params.edge.a - begin, 1.0), 0.0, 1.0);
    color = mix(color, params.fog.rgb, pow(fog_t, 1.6) * params.fog.a * (1.0 - parchment));
    // Weather haze of the camera (`DistanceFog`).
    color = camera_distance_fog(color, in.world, sun_vis);
    // Map edge drowned in haze.
    let edge_px = params.scale.w;
    if (edge_px > 0.0) {
        let q = min(in.world.xz, params.map.xy - in.world.xz);
        let haze = 1.0 - smoothstep(0.0, edge_px, min(q.x, q.y));
        color = mix(color, params.edge.rgb, haze);
    }
    return vec4<f32>(color, 1.0);
}

// Relief shade of a map point (ADR 0745), marched on the 16 px block heightfield: (sun visibility,
// ambient visibility). Runs in the vertex stage of the bake only (once per node vertex, amortised
// by the cache), never in the main pass.
const SHADE_MAX_ALT_M = 5200.0;

const SHADE_AO_GAIN = 6.0;

fn relief_shade(xz: vec2<f32>, h0: f32) -> vec2<f32> {
    if (shade.sun.w < 0.5) {
        return vec2<f32>(1.0);
    }
    let s_eff = max(shade.mix.w, 1e-5);
    let sun = shade.sun.xyz;
    let horiz = length(sun.xz);
    var vis = 1.0;
    if (horiz > 1e-3 && sun.y > 0.0) {
        let dir = sun.xz / horiz;
        // Metres of altitude the sun ray gains per map px.
        let rise = (sun.y / horiz) / s_eff;
        let start_h = max(h0, pm_height(xz)) + shade.march.w;
        let n = i32(shade.march.y);
        for (var i = 1; i <= n; i = i + 1) {
            let t = f32(i) / f32(n);
            let d = shade.march.x * t * t;
            let ray_h = start_h + rise * d;
            if (ray_h > SHADE_MAX_ALT_M) {
                break;
            }
            let pen = shade.march.z * (0.25 + t);
            vis = min(vis, smoothstep(-pen, pen, ray_h - pm_height(xz + dir * d)));
        }
    }
    // Horizon occlusion: highest elevation seen in 8 directions at 3 distances.
    var occ = 0.0;
    for (var k = 0u; k < 8u; k = k + 1u) {
        let a = f32(k) * 0.785398;
        let dir = vec2<f32>(cos(a), sin(a));
        var best = 0.0;
        for (var j = 1; j <= 3; j = j + 1) {
            let d = shade.mix.z * (f32(j) / 3.0) * (f32(j) / 3.0);
            let tan_e = (pm_height(xz + dir * d) - h0) * s_eff / max(d, 1.0);
            best = max(best, tan_e * inverseSqrt(1.0 + tan_e * tan_e));
        }
        occ = occ + best;
    }
    let ao = 1.0 - clamp(occ * 0.125 * SHADE_AO_GAIN, 0.0, 1.0);
    return vec2<f32>(vis, ao);
}

// Ground cache bake (ADR 0742, `ground_cache.rs`): the node mesh drawn flat over its cache layer,
// one fragment per texel, the regional ground computed with the texel as footprint. The skirts
// collapse onto the node edges.
struct BakeVaryings {
    @builtin(position) clip: vec4<f32>,
    @location(0) xz: vec2<f32>,
    @location(1) altitude: f32,
    @location(2) slope_xz: vec2<f32>,
    @location(3) texel: f32,
    // Relief shade (ADR 0745): per vertex, interpolated over the node.
    @location(4) shade: vec2<f32>,
};

@vertex
fn bake_vertex(v: Vertex) -> BakeVaryings {
    let c = cache_coords(v.morph);
    var out: BakeVaryings;
    out.clip = vec4<f32>(c.x * 2.0 - 1.0, 1.0 - c.y * 2.0, 0.5, 1.0);
    out.xz = v.position.xz;
    out.altitude = v.position.y;
    out.slope_xz = v.slope.xy;
    out.texel = c.w;
    out.shade = relief_shade(v.position.xz, v.position.y);
    return out;
}

struct BakeOut {
    @location(0) albedo: vec4<f32>,
    @location(1) dn: vec4<f32>,
    @location(2) shade: vec4<f32>,
};

@fragment
fn bake_fragment(in: BakeVaryings) -> BakeOut {
    let base = base_ground(in.xz, in.texel);
    let r = regional_ground(in.xz, in.texel, in.altitude, in.slope_xz, base);
    var out: BakeOut;
    // Alpha 0.5 + 0.5 f: never 0, which marks a layer not baked yet (cleared on allocation).
    out.albedo = vec4<f32>(r.albedo, 0.5 + 0.5 * r.forest_f);
    out.dn = vec4<f32>(r.dn, 0.0, 1.0);
    out.shade = vec4<f32>(in.shade, 0.0, 1.0);
    return out;
}
