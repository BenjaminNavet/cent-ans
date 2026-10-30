// Campaign terrain module `snow` (VIS-L1, split of terrain.wesl): snow cover, alpine white, mountain stages (VIS-R4), rock and glaciers.
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::noise::{fbm, luma, vnoise};
import cent_ans_client::campaign::terrain::terrain_params::{extras, params, season};

// Season grade of the ground colour (SeasonLook.grade / ground, snow of terrain.gdshader simplified).
fn season_ground(albedo: vec3<f32>, cover: SnowCover, lit: f32) -> vec3<f32> {
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
    return snow_apply(col, cover, lit);
}

// TS-T1 snow cover of a ground pixel: `snow` the cover, `mud` the brown fringe at its edge,
// `grain` the noise of the patches (shade of the snow).
struct SnowCover {
    snow: f32,
    mud: f32,
    grain: f32,
    // EAU (ADR 0750): share of forest canopy under the cover (frosted, not buried).
    frost: f32,
    // VIS-R4: 0 plain grey snow .. 1 white alpine snow (by altitude).
    alpine: f32,
};

// Multi-scale patch noise of the snow (x, 0..1), its fine octaves fading with the footprint, and
// an independent detail noise (y) for the thin snow that lets the soil show through.
fn snow_patch_noise(p: vec2<f32>, footprint: f32) -> vec2<f32> {
    let f = max(season.snow_mask.x, 1e-4);
    let fine = 1.0 - smoothstep(0.15, 0.8, footprint);
    let very_fine = 1.0 - smoothstep(0.04, 0.2, footprint);
    // The coarse octave fades with distance too, so no giant patch repeats in the far view.
    let coarse = 1.0 - smoothstep(1.5, 5.0, footprint);
    let n = 0.5 * mix(0.5, vnoise(p * f + vec2<f32>(4.7, 1.3)), coarse)
        + 0.3 * mix(0.5, vnoise(p * f * 4.1 + vec2<f32>(9.2, 3.8)), fine)
        + 0.2 * mix(0.5, vnoise(p * f * 17.0 + vec2<f32>(2.1, 7.4)), very_fine);
    let d = mix(0.5, vnoise(p * f * 9.0 + vec2<f32>(6.6, 0.4)) * 0.6 + vnoise(p * f * 31.0 + vec2<f32>(1.2, 5.5)) * 0.4 * very_fine
        + 0.2 * (1.0 - very_fine), fine);
    return vec2<f32>(n, d);
}

// Snow of a raw cover `c` (0..1, before the patches): patches of noise `pn`, sharp edge, mud fringe
// outside it; `flat_y` the vertical component of the displayed normal (steep = bare).
fn snow_cover_of(c: f32, noise: vec2<f32>, flat_y: f32) -> SnowCover {
    var out: SnowCover;
    let pn = noise.x;
    // The noise shifts the threshold: a cover c leaves about c of the ground white, in patches.
    let t = c + (pn - 0.5) * 0.72 * (1.0 - smoothstep(0.85, 1.0, c));
    let fringe = max(season.mud.w, 0.0);
    // EAU: a wide ramp (no binary mask) and a slope fade that starts earlier, so the cover thins
    // out over the steep faces instead of ending on a line.
    out.snow = smoothstep(0.34, 0.76, t) * smoothstep(season.snow_mask.z - 0.1, season.snow_mask.w + 0.05, flat_y);
    // Thin snow towards the edge of the cover and in the plains: the soil shows through in
    // small holes (detail noise), the deep snow of the heights stays whole.
    let thin = 1.0 - smoothstep(0.65, 1.1, t);
    out.snow = out.snow * (1.0 - thin * 0.6 * (1.0 - smoothstep(0.3, 0.6, noise.y)));
    // Mud only in the band just outside the snow, on the ground that could hold it (not on the
    // bare rock of the steep faces above the snow line).
    out.mud = smoothstep(0.5 - fringe, 0.5, t) * (1.0 - smoothstep(0.34, 0.76, t)) * smoothstep(0.0, 0.3, c)
        * smoothstep(season.snow_mask.z - 0.15, season.snow_mask.z + 0.05, flat_y);
    out.grain = noise.y;
    out.frost = 0.0;
    out.alpine = 0.0;
    return out;
}

// Seasonal snow cover: above the seasonal snow line (jittered by the noise), plus a share of the
// plains of the north in winter, in patches; it slides off the steep slopes of the displayed relief
// (`disp_y` the vertical component of the exaggerated normal), thinner under the forests.
fn season_snow(disp_y: f32, altitude: f32, xz: vec2<f32>, footprint: f32, forest: f32, n_xz: vec2<f32>) -> SnowCover {
    let w = season.weights;
    let line = dot(w, season.snow_lines);
    let pn = snow_patch_noise(xz, footprint);
    var c = 0.0;
    if (line > 0.0) {
        let alt = altitude + (pn.x - 0.5) * season.snow_mask.y;
        c = smoothstep(line - 250.0, line + 250.0, alt);
    }
    let north = 1.0 - smoothstep(season.snow_plain.x * params.map.y, season.snow_plain.y * params.map.y, xz.y);
    // Plains: near-continuous cover, with a low-frequency variation (some regions less snowy).
    let region = vnoise(xz * max(season.snow_mask.x, 1e-4) * 0.12 + vec2<f32>(13.1, 5.3));
    let plain = season.snow_plain.z * (1.0 - season.snow_plain.w * smoothstep(0.35, 0.75, region));
    c = max(c, w.w * north * plain);
    // EAU: accumulation by altitude and exposure. Snow thickens with height, piles on the faces
    // turned away from the sun and melts first on those that face it (smooth, never a mask).
    if (c > 0.0) {
        let sun_h = normalize(params.sun_dir.xz + vec2<f32>(1e-4, 0.0));
        let facing = clamp(dot(n_xz, sun_h) * 5.0, -1.0, 1.0);
        let high = smoothstep(0.0, 800.0, altitude);
        c = clamp(c - 0.2 * facing * (1.0 - disp_y * 0.5) + 0.1 * high * w.w, 0.0, 1.0);
    }
    c = c * mix(1.0, season.snow_shade.a, forest);
    var cover = snow_cover_of(c, pn, disp_y);
    cover.frost = forest;
    cover.alpine = alpine_share(altitude);
    return cover;
}

// Snow colour: warm in the sun, cold and bluish in the shade (`lit` = n.l x sun visibility),
// never pure white; the grain of the patches varies it slightly.
fn snow_tint(lit: f32, grain: f32, alpine: f32) -> vec3<f32> {
    let k = smoothstep(0.05, 0.7, lit);
    let sun_c = mix(season.snow_color.rgb, season.mt_snow_sun.rgb, alpine);
    let shade_c = mix(season.snow_shade.rgb, season.mt_snow_shade.rgb, alpine);
    return mix(shade_c, sun_c, k) * (0.9 + 0.16 * grain);
}

// VIS-R4 (ADR 0820): share of white alpine snow by altitude (the plain keeps its grey).
fn alpine_share(altitude: f32) -> f32 {
    let t = clamp((altitude - season.mt_edge.y) / max(season.mt_edge.z - season.mt_edge.y, 1.0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn snow_apply(col: vec3<f32>, cover: SnowCover, lit: f32) -> vec3<f32> {
    var out = mix(col, season.mud.rgb * (0.85 + 0.3 * cover.grain), cover.mud * 0.85);
    // EAU: under a forest the snow is a frost: greyer, bluer, and the canopy texture shows through.
    let frost_tint = vec3<f32>(0.80, 0.86, 0.95);
    let tint = snow_tint(lit, cover.grain, cover.alpine) * mix(vec3<f32>(1.0), frost_tint, cover.frost);
    return mix(out, tint, cover.snow * (1.0 - 0.28 * cover.frost));
}

// VIS-R4 (ADR 0820): stages of the mountain by real altitude and exposure, ALU only. Slope forest
// (conifers, in patches) up to the tree line, alpine meadow (moorland in the north) above it, scree
// on the steeper ground higher up; the bare rock of the steep faces and the snow come after.
// The tree line falls towards the north of the map and on the shaded faces.
fn mountain_stages(col: vec3<f32>, p: vec2<f32>, disp_y: f32, altitude: f32, footprint: f32,
        forest: f32, n_xz: vec2<f32>, land: f32) -> vec3<f32> {
    let pattern_f = season.mt_slope.w;
    if (pattern_f <= 0.0 || altitude < season.mt_band.x) {
        return col;
    }
    let height = params.map.y;
    let south = smoothstep(season.mt_line.z * height, season.mt_line.w * height, p.y);
    // The horizontal normal points to where the face looks: +y (south) is the sunny face.
    let aspect = clamp(n_xz.y * 4.0, -1.0, 1.0);
    let large = vnoise(p * pattern_f);
    let fine = mix(0.5, vnoise(p * pattern_f * 6.3 + vec2<f32>(7.1, 2.9)), 1.0 - smoothstep(0.3, 1.5, footprint));
    let tree = mix(season.mt_line.y, season.mt_line.x, south) + aspect * season.mt_band.w
        + (large - 0.5) * 2.0 * season.mt_edge.x;
    let flat_k = disp_y;
    let alp_w = season.mt_band.y;
    let patches = smoothstep(0.3, 0.55, mix(large, fine, 0.45));
    let forest_k = smoothstep(season.mt_band.x, season.mt_band.x + 250.0, altitude)
        * (1.0 - smoothstep(tree - 300.0, tree, altitude))
        * smoothstep(season.mt_slope.x - 0.1, season.mt_slope.x + 0.1, flat_k)
        * patches * season.mt_forest.a * (1.0 - forest);
    let meadow_k = smoothstep(tree - 120.0, tree + 120.0, altitude)
        * (1.0 - smoothstep(tree + alp_w - 250.0, tree + alp_w, altitude))
        * smoothstep(season.mt_slope.y, season.mt_slope.z, flat_k) * season.mt_alpage.a;
    let steep_k = 1.0 - smoothstep(season.mt_slope.y, season.mt_slope.z, flat_k);
    let scree_k = smoothstep(tree + alp_w - season.mt_band.z * 0.5, tree + alp_w + season.mt_band.z * 0.5, altitude)
        * mix(0.55, 1.0, steep_k) * season.mt_scree.a;
    let grain = 0.85 + 0.3 * fine;
    let meadow_col = mix(season.mt_alpage.rgb, season.mt_moor.rgb, 1.0 - south);
    var out = mix(col, season.mt_forest.rgb * grain, forest_k * land);
    out = mix(out, meadow_col * (0.9 + 0.2 * fine), meadow_k * land);
    out = mix(out, season.mt_scree.rgb * grain, scree_k * land);
    return out;
}

// TS-T1 dark rock of the heights and of the steep displayed slopes: grey-brown, strata, lighter
// crests, stronger sunlit / shaded contrast of the slopes. `ndl` the sun term of the normal.
fn rock_apply(col: vec3<f32>, p: vec2<f32>, disp_y: f32, altitude: f32, footprint: f32, ndl: f32,
        crest: f32, forest: f32) -> vec3<f32> {
    let steep = 1.0 - smoothstep(season.rock_band.x, season.rock_band.y, disp_y);
    let high = smoothstep(season.rock_band.z, season.rock_band.w, altitude);
    let k = season.rock.a * steep * high * (1.0 - 0.6 * forest);
    if (k <= 0.001) {
        return col;
    }
    let detail = 1.0 - smoothstep(0.2, 1.0, footprint);
    let strata = vnoise(p * vec2<f32>(0.5, 2.3) + vec2<f32>(3.3, 8.1)) * 0.6 + vnoise(p * 3.1) * 0.4 * detail;
    var rock = season.rock.rgb * (1.0 + (strata - 0.5) * 2.0 * season.rock_light.z);
    rock = rock * (1.0 + season.rock_light.y * crest);
    rock = rock * mix(1.0, 0.55 + 0.9 * ndl, season.rock_light.x);
    return mix(col, rock, k);
}

// --- DN-ME4 névés and glaciers (freshwater_ground.gdshaderinc), permanent, all seasons ---
fn fw_glacier_apply(col: vec3<f32>, p: vec2<f32>, hc: f32, slope: f32, footprint: f32, land: f32, disp_y: f32,
        lit: f32) -> vec3<f32> {
    let g = extras.glacier.line;
    if (g.w <= 0.0 || hc < g.x - 600.0) {
        return col;
    }
    let alt = hc + (fbm(p * 0.035) - 0.5) * 420.0;
    // TS-T1: the steep faces of the displayed (exaggerated) relief stay bare rock.
    let flat_k = (1.0 - smoothstep(g.z - 0.12, g.z + 0.25, slope))
        * smoothstep(season.snow_mask.z, season.snow_mask.w, disp_y);
    // Névé: white patches below the glacier line, cut by the noise.
    let patch_n = vnoise(p * 0.35) * 0.6 + vnoise(p * 1.3) * 0.4;
    let neve = smoothstep(g.x - 500.0, g.x + 150.0, alt) * smoothstep(0.35, 0.6, patch_n + 0.12) * flat_k;
    let neve_col = snow_tint(lit, patch_n, 1.0) * extras.glacier.neve.rgb / max(luma(extras.glacier.neve.rgb), 0.1);
    var out = mix(col, neve_col, neve * 0.8 * g.w * land);
    // Glacier: blue-white tongue with crevasses, on gentle slopes above the line.
    let ice = smoothstep(g.x, g.y, alt) * flat_k;
    if (ice > 0.01) {
        let fade_detail = 1.0 - smoothstep(0.5, 2.5, footprint);
        let seams = abs(vnoise(p * vec2<f32>(2.2, 0.7)) - 0.5) * 2.0;
        let crevasse = (1.0 - smoothstep(0.0, 0.1, seams)) * fade_detail;
        let ice_col = mix(extras.glacier.ice.rgb * (0.9 + 0.15 * patch_n), extras.glacier.crevasse.rgb, crevasse * 0.7)
            * mix(0.8, 1.0, smoothstep(0.05, 0.7, lit));
        out = mix(out, ice_col, ice * 0.85 * g.w * land);
    }
    return out;
}
