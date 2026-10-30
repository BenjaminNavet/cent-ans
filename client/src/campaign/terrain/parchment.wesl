// Campaign terrain module `parchment` (VIS-L1, split of terrain.wesl): parchment (far map) look: hills, mountains and tree glyphs.
// Pure move of the code, no change of behaviour.

import cent_ans_client::campaign::terrain::noise::{fbm, hash21, vnoise};
import cent_ans_client::campaign::terrain::terrain_params::{ground, height_lod, params, shade, splat, terroir_sampler, wash_tex};

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
    let sp = textureSampleLevel(splat, terroir_sampler, base / params.map.xy, 0.0);
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
    let wash_here = textureSampleLevel(wash_tex, terroir_sampler, q, 0.0);
    if (wash_here.a > 0.5) {
        let step_uv = 6.0 / params.map.xy;
        var diff = 0.0;
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, terroir_sampler, q + vec2<f32>(step_uv.x, 0.0), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, terroir_sampler, q - vec2<f32>(step_uv.x, 0.0), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, terroir_sampler, q + vec2<f32>(0.0, step_uv.y), 0.0)));
        diff = max(diff, distance(wash_here, textureSampleLevel(wash_tex, terroir_sampler, q - vec2<f32>(0.0, step_uv.y), 0.0)));
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
    let sp = textureSampleLevel(splat, terroir_sampler, uv, 0.0);
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
