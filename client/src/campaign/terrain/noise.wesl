// Campaign terrain module `noise` (VIS-L1, split of terrain.wesl): hash and value-noise helpers shared by the ground, snow, weather and parchment passes.
// Pure move of the code, no change of behaviour.



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

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// --- V2b parcels (shared with vegetation_fields.gd: same constants, same hash) ---
fn hash01i(n: i32) -> f32 {
    var h = (bitcast<u32>(n) * 1103515245u + 12345u) & 0x7fffffffu;
    h = ((h ^ (h >> 13u)) * 1274126177u) & 0x7fffffffu;
    return f32(h % 100000u) / 100000.0;
}
