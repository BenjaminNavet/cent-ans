// Campaign terrain module `water_edge` (VIS-L1, split of terrain.wesl): coast band: shore geology, swash and foam.
// Pure move of the code, no change of behaviour.

import bevy_pbr::render::mesh_view_bindings::{globals};
import cent_ans_client::campaign::terrain::noise::{vnoise};
import cent_ans_client::campaign::terrain::terrain_params::{coast_dist_tex, coast_probe_tex, coast_types_tex, extras, params, shade, terroir_sampler};

// --- TB5 coast (coast_common.gdshaderinc + coast_band.gdshaderinc) ---

// Signed distance to the coast (map px, > 0 on land) at `uv`.
fn coast_px(uv: vec2<f32>) -> f32 {
    return (textureSampleLevel(coast_dist_tex, terroir_sampler, uv, 0.0).r * 255.0 - 128.0) / extras.coast.texel.z;
}

// EAU (ADR 0750): the same coast distance, smoothed close up. The raster has one texel per map px and
// a 0.5 px step: the bilinear read draws stair steps on the beach and the surf. Within 5 px of the
// shore and below ~1.8 map px per screen px it is read as a cubic B-spline (four bilinear reads, the
// same sampler as above); the sea shader (`water/shaders/sea.wesl`) does the same, so both agree.
fn coast_px_smooth(uv: vec2<f32>, footprint: f32) -> f32 {
    let raw = coast_px(uv);
    if (abs(raw) >= 5.0 || footprint >= 1.8) {
        return raw;
    }
    let ts = vec2<f32>(textureDimensions(coast_dist_tex));
    let st = uv * ts - vec2<f32>(0.5);
    let i = floor(st);
    let f = st - i;
    let f2 = f * f;
    let f3 = f2 * f;
    let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
    let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
    let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
    let w3 = f3 / 6.0;
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    let o0 = (i + vec2<f32>(0.5) + (-1.0 + w1 / g0)) / ts;
    let o1 = (i + vec2<f32>(0.5) + (1.0 + w3 / g1)) / ts;
    let a = textureSampleLevel(coast_dist_tex, terroir_sampler, vec2<f32>(o0.x, o0.y), 0.0).r;
    let b = textureSampleLevel(coast_dist_tex, terroir_sampler, vec2<f32>(o1.x, o0.y), 0.0).r;
    let c = textureSampleLevel(coast_dist_tex, terroir_sampler, vec2<f32>(o0.x, o1.y), 0.0).r;
    let d = textureSampleLevel(coast_dist_tex, terroir_sampler, vec2<f32>(o1.x, o1.y), 0.0).r;
    let r = g0.y * (g0.x * a + g1.x * b) + g1.y * (g0.x * c + g1.x * d);
    let smooth_c = (r * 255.0 - 128.0) / extras.coast.texel.z;
    return mix(smooth_c, raw, smoothstep(0.9, 1.8, footprint));
}

// Direction of the land (gradient of the coast distance), unit, in map px.
fn coast_to_land(uv: vec2<f32>) -> vec2<f32> {
    let ex = vec2<f32>(1.0 / extras.coast.texel.x, 0.0);
    let ey = vec2<f32>(0.0, 1.0 / extras.coast.texel.y);
    let cg = vec2<f32>(
        textureSampleLevel(coast_dist_tex, terroir_sampler, uv + ex, 0.0).r - textureSampleLevel(coast_dist_tex, terroir_sampler, uv - ex, 0.0).r,
        textureSampleLevel(coast_dist_tex, terroir_sampler, uv + ey, 0.0).r - textureSampleLevel(coast_dist_tex, terroir_sampler, uv - ey, 0.0).r);
    return cg / max(length(cg), 1e-5);
}

// Geology at p (map px): x chalk, y granite, z shingle, w beach width; region edges warped by a
// slow noise (no straight limit along the shore).
fn coast_geology(p: vec2<f32>) -> vec4<f32> {
    let warp = (vec2<f32>(vnoise(p * 0.21), vnoise(p * 0.21 + vec2<f32>(37.0, 11.0))) - 0.5) * 10.0;
    let g = textureSampleLevel(coast_types_tex, terroir_sampler, (p + warp) / params.map.xy, 0.0);
    return vec4<f32>(smoothstep(vec3<f32>(0.3), vec3<f32>(0.7), g.rgb), g.a);
}

// Swash: a foam blade climbs the shore and withdraws, the same as the sea's (continuous at the
// coastline). x foam, y water of the blade; frozen into a fringe when zoomed out.
fn coast_swash(p: vec2<f32>, coast: f32, cliff_f: f32, footprint: f32) -> vec2<f32> {
    let c = extras.coast;
    let amount = c.swash_a.x;
    if (amount <= 0.0) {
        return vec2<f32>(0.0);
    }
    let out_px = c.swash_a.z;
    let edge_px = c.swash_b.x;
    let film_px = c.swash_b.y;
    let reach_in = c.swash_a.w * mix(1.0, c.swash_b.z, cliff_f);
    if (coast > reach_in + edge_px + 0.1 || coast < -out_px - film_px - edge_px - 0.1) {
        return vec2<f32>(0.0);
    }
    let t = globals.time;
    let far = smoothstep(0.3, 1.1, footprint);
    let cyc = fract(t / c.swash_a.y + vnoise(p * 0.45) + vnoise(p * 0.11) * 2.0);
    let up = smoothstep(0.0, 0.3, cyc);
    let down = 1.0 - smoothstep(0.3, 1.0, cyc);
    let adv = mix(up * down, 0.5, far);
    let front = mix(-out_px, reach_in, adv) + (vnoise(p * 3.1) - 0.5) * 0.08 * (1.0 - far);
    let behind = front - coast;
    let edge = 1.0 - smoothstep(0.0, edge_px, abs(behind));
    let lace = smoothstep(0.35, 0.75, vnoise(p * 6.0 + vec2<f32>(t * 0.07, 0.0)));
    let film = step(0.0, behind) * (1.0 - smoothstep(0.0, film_px, behind)) * lace * mix(0.35, 0.8, down);
    let foam = max(edge * mix(0.55, 1.0, down), film) * mix(1.0, 0.6, far);
    return vec2<f32>(foam * amount, smoothstep(-0.02, 0.02, behind) * amount);
}

struct CoastOut {
    color: vec3<f32>,
    n: vec3<f32>,
};

// Coast band of the ground (`coast_band`): cliffs (chalk, granite, rock) where the relief
// dominates the shore, beaches (sand, shingle) elsewhere; the relief emerging on the sea side of
// the raster coast is painted rock or sand. The slope-rule probe altitude and the lake-shore
// flag are baked in `coast_probe` (coast.rs) instead of reading the heightmap here.
fn coast_band(p: vec2<f32>, footprint: f32, hc: f32, world_y: f32, slope: f32, snow: f32, col: vec3<f32>, n: vec3<f32>) -> CoastOut {
    var out: CoastOut;
    out.color = col;
    out.n = n;
    let c = extras.coast;
    if (c.cliff.x < 0.5) {
        return out;
    }
    let uv = p / params.map.xy;
    let coast = coast_px_smooth(uv, footprint);
    let cliff_px = c.band_a.x;
    let widen = c.band_a.y;
    let beach_px = c.band_a.w;
    let reach = max(cliff_px * widen * 2.2, beach_px * 3.2);
    if (coast > reach) {
        return out;
    }
    let emerged = select(0.0, smoothstep(0.0, 2.0, hc), coast > -c.band_a.z && world_y >= 0.0);
    if (coast < -0.3 && emerged <= 0.0) {
        return out;
    }
    let probe = textureSampleLevel(coast_probe_tex, terroir_sampler, uv, 0.0);
    if (probe.g > 0.5) {
        return out;
    }
    let h = probe.r * 255.0;
    let to_land = coast_to_land(uv);
    let cliff_f = max(smoothstep(c.cliff.z, c.cliff.w, h), emerged * smoothstep(6.0, 15.0, hc));
    let geo = coast_geology(p);
    let w_rock = max(1.0 - geo.r - geo.g, 0.0);
    let fine = 1.0 - smoothstep(0.15, 0.7, footprint);
    let shore = max(smoothstep(-0.25, 0.05, coast), emerged);
    let k = c.band_b.y * mix(1.0, c.band_b.w, smoothstep(1.5, 5.0, footprint));

    // Beach: irregular width, grain, wet foreshore at the water's edge.
    let beach_c = mix(c.color[3].rgb, c.color[4].rgb, geo.b);
    let beach_s = mix(c.shade[3].rgb, c.shade[4].rgb, geo.b);
    let beach_k = mix(c.color[3].a, c.color[4].a, geo.b);
    let bw = beach_px * mix(0.5, 2.0, geo.a) * (0.55 + 0.9 * vnoise(p * 0.9 + vec2<f32>(11.0, 3.0)));
    let speck = mix(0.5, vnoise(p * 17.0) * 0.6 + vnoise(p * 41.0) * 0.4, fine);
    var beach_col = mix(beach_s, beach_c, clamp(1.0 - beach_k + beach_k * speck * 1.6, 0.0, 1.0));
    let wet = 1.0 - smoothstep(0.0, c.band_b.x, coast);
    beach_col = beach_col * mix(1.0, 0.7, wet);
    let swash = coast_swash(p, coast, cliff_f, footprint) * shore;
    beach_col = mix(beach_col, beach_col * vec3<f32>(0.55, 0.7, 0.72), swash.y * 0.5);
    beach_col = mix(beach_col, c.foam.rgb, swash.x * 0.8);
    let beach = shore * (1.0 - smoothstep(bw * 0.6, bw, coast)) * (1.0 - cliff_f) * (1.0 - 0.7 * snow);
    let color = mix(col, beach_col, beach * k);

    // Cliff: light or dark face by the rock, foot in the shade and the shingle, strata and grain;
    // the normal leans towards the sea (the face takes the light by its exposure).
    let rock_c = c.color[0].rgb * w_rock + c.color[1].rgb * geo.r + c.color[2].rgb * geo.g;
    let rock_s = c.shade[0].rgb * w_rock + c.shade[1].rgb * geo.r + c.shade[2].rgb * geo.g;
    let rock_k = c.color[0].a * w_rock + c.color[1].a * geo.r + c.color[2].a * geo.g;
    let steep = smoothstep(c.cliff.w, c.cliff.w * 2.5, h);
    let cw = cliff_px * (0.75 + 0.4 * vnoise(p * 1.7)) * mix(1.0, widen, steep);
    let t = max(clamp(coast / max(cw, 1e-3), 0.0, 1.0), emerged * smoothstep(5.0, 40.0, hc));
    let grain = vnoise(p * 5.0) * 0.6 + vnoise(p * 11.0) * 0.4;
    let strata = vnoise(vec2<f32>(t * 6.0, vnoise(p * 0.9) * 3.0));
    let mottle = mix(0.5, grain * 0.65 + strata * 0.35, fine);
    let light = clamp((1.0 - rock_k + rock_k * mottle * 1.7) * (0.7 + 0.3 * smoothstep(0.0, 0.3, t)), 0.0, 1.0);
    var cliff_col = mix(rock_s, rock_c, light);
    let foot = (1.0 - smoothstep(0.05, 0.22, coast)) * 0.7 * (1.0 - emerged);
    cliff_col = mix(cliff_col, mix(beach_col, rock_s, 0.5), foot);
    cliff_col = mix(cliff_col, c.foam.rgb, swash.x * 0.8);
    // Bevy addition: on a real cliff the bare rock follows the steep wall, not only the thin band
    // of the raster coast (Dover: the wall stands 3-4 px inland of the raster shore), so the white
    // chalk reads from the camera; fades with the slope and with the distance from the shore.
    let wall = shore * cliff_f * smoothstep(0.45, 1.1, slope) * (1.0 - smoothstep(cw * 1.2, cw * 2.6, coast));
    let face = max(shore * (1.0 - smoothstep(cw * 0.7, cw, coast)) * cliff_f, wall);
    out.color = mix(color, cliff_col, face * k);
    out.n = normalize(n + vec3<f32>(-to_land.x, 0.0, -to_land.y) * c.band_b.z * face * k);
    return out;
}
