// Campaign terrain module `ground_look` (VIS-L1): saturation, hue and value of the ground albedo,
// tuned by data (`view.ground_look` of `data/art/ground_biome_mix.json`, `fields.look_*`).
//
// The colour map and the crop materials reach the screen with a third of their chroma (lit by a
// cool sky, AgX, aerial perspective): the bible of the palette asks for a plain at s 0.56 and a
// meadow at 89 degrees, forests at half the value of the meadows. The grade is applied here, on the
// baked albedo (ADR 0742), once per texel, and not in the post pass on every pixel.

import cent_ans_client::campaign::terrain::terrain_params::fields;

// Hue angle (degrees, 0 red, 120 green, 240 blue) of a linear colour, from the opponent axes.
fn look_hue(c: vec3<f32>) -> f32 {
    let a = c.r - 0.5 * (c.g + c.b);
    let b = 0.8660254 * (c.g - c.b);
    let angle = degrees(atan2(b, a));
    return angle + select(0.0, 360.0, angle < 0.0);
}

// Vegetation hues are pulled towards `hue_target`, their chroma is multiplied by `saturation`, the
// forests are darkened and the open land scaled in value; `land` (0 sea .. 1 land) and `altitude`
// (the farmland altitude band of the geo rules) keep the sea, the rock and the high mountains out.
fn ground_look(c: vec3<f32>, forest_f: f32, land: f32, altitude: f32) -> vec3<f32> {
    if (fields.look_a.w < 0.5) {
        return c;
    }
    let weight = land * (1.0 - smoothstep(fields.v6.z, fields.v6.w, altitude));
    let mean = (c.r + c.g + c.b) / 3.0;
    var chroma = c - vec3<f32>(mean);
    let hue = look_hue(c);
    let lo = fields.look_b.x;
    let hi = fields.look_b.y;
    let in_range = smoothstep(lo - 12.0, lo, hue) * (1.0 - smoothstep(hi, hi + 12.0, hue)) * weight;
    // Shortest signed angle to the target hue, then a rotation of the chroma about the grey axis.
    let delta = fract((fields.look_a.y - hue) / 360.0 + 0.5) * 360.0 - 180.0;
    let angle = radians(delta) * fields.look_a.z * in_range;
    let axis = vec3<f32>(0.57735027);
    chroma = chroma * cos(angle) + cross(axis, chroma) * sin(angle);
    chroma = chroma * mix(1.0, fields.look_a.x, in_range);
    let value = mix(fields.look_b.w, fields.look_b.z, forest_f);
    return max(vec3<f32>(mean) + chroma, vec3<f32>(0.0)) * mix(1.0, value, weight);
}
