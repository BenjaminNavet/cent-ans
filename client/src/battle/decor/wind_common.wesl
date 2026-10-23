// Wind sway of grass cards and tree crowns, shared by `wind.wesl` (main pass) and
// `wind_prepass.wesl` (prepass and shadows): one computation, so the depth, the normals, the
// shadows and the picture all agree on where a vertex is.
// Import: import cent_ans_client::battle::decor::wind_common::{WindParams, WindResult, wind_displace};

struct WindParams {
    // x: sway amplitude (m), y: sway speed, z: gust amplitude (m), w: gust speed.
    sway: vec4<f32>,
    // xy: ground direction of the wind, z: weight mode (0: 1 - v of the card, 1: height / height_ref),
    // w: height_ref (m).
    shape: vec4<f32>,
    // Flatten map rectangle (x, z, width, depth); width 0 = none.
    flatten_rect: vec4<f32>,
};

struct WindResult {
    world: vec4<f32>,
    // Colour factor of the grass flattened by troops and stained by blood (main pass only).
    tint: vec4<f32>,
};

// `local_y`: height of the vertex in the mesh; `uv_y`: v of the card (1.0 when the mesh has no
// UVs, which gives the grass mode a zero weight); `time`: seconds (the previous frame's for the
// motion vectors).
fn wind_displace(
    wind: WindParams,
    world_in: vec4<f32>,
    local_y: f32,
    uv_y: f32,
    time: f32,
    flatten_tex: texture_2d<f32>,
    flatten_samp: sampler,
) -> WindResult {
    var world = world_in;
    var weight = 0.0;
    if wind.shape.z < 0.5 {
        weight = clamp(1.0 - uv_y, 0.0, 1.0);
    }
    if wind.shape.z > 0.5 {
        let h = clamp(local_y / max(wind.shape.w, 0.01), 0.0, 1.0);
        weight = h * h;
    }
    let phase = world.x * 0.21 + world.z * 0.17;
    let gust = sin(time * wind.sway.w + world.x * 0.012 + world.z * 0.009) * 0.5 + 0.5;
    let sway = (sin(time * wind.sway.y + phase) * 0.6 + sin(time * wind.sway.y * 2.3 + phase * 1.7) * 0.4) * wind.sway.x
        + gust * wind.sway.z;
    world = vec4<f32>(world.x + wind.shape.x * sway * weight, world.y, world.z + wind.shape.y * sway * weight, 1.0);

    // Grass flattened by marching troops and stained by blood (mode 0 only, grass cards).
    var tint = vec4<f32>(1.0);
    if wind.shape.z < 0.5 && wind.flatten_rect.z > 0.0 {
        let map_uv = (world.xz - wind.flatten_rect.xy) / wind.flatten_rect.zw;
        let marks = textureSampleLevel(flatten_tex, flatten_samp, map_uv, 0.0).rg;
        let lay = marks.r * weight;
        // The tuft folds down to the ground and leans over, paler and yellower.
        let h = fract(sin(dot(floor(world.xz * 0.5), vec2<f32>(12.99, 78.23))) * 43758.5);
        let lean = vec2<f32>(h - 0.5, fract(h * 7.3) - 0.5);
        world = vec4<f32>(world.x + lean.x * lay * 0.5, world.y - lay * 0.5, world.z + lean.y * lay * 0.5, 1.0);
        tint = mix(vec4<f32>(1.0), vec4<f32>(1.25, 1.12, 0.78, 1.0), marks.r * 0.6);
        tint = mix(tint, vec4<f32>(0.5, 0.06, 0.04, 1.0), clamp(marks.g * 1.2, 0.0, 1.0));
    }
    return WindResult(world, tint);
}
