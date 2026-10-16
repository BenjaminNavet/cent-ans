// Diagonal hatching fixed to the map, period in screen pixels: two scale octaves cross-faded
// (fr1_hatch_at). Shared by border.wesl (occupation bands) and fill.wesl (Feudal mode holder
// stripes); inlined at registration (`drape::with_hatch`), so it holds no binding.
fn hatch_stripes(p: vec2<f32>, world_per_px: f32, period_px: f32, duty: f32) -> f32 {
    let lv = log2(max(world_per_px, 1e-4));
    let l0 = floor(lv);
    let s0 = exp2(l0) * period_px;
    let c = (p.x + p.y) * 0.70710678;
    let a0 = fract(c / s0);
    let a1 = fract(c / (s0 * 2.0));
    let edge = 0.08;
    let h0 = smoothstep(0.0, edge, a0) * (1.0 - smoothstep(duty, duty + edge, a0));
    let h1 = smoothstep(0.0, edge, a1) * (1.0 - smoothstep(duty, duty + edge, a1));
    return mix(h0, h1, lv - l0);
}
