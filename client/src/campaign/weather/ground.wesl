// Per-province weather and atmosphere terms for the campaign ground shader (lot weather5).
// Rust side: `weather::ground` (baked field + selection textures) and `weather::atmosphere`.
//
//     import cent_ans_client::campaign::weather::ground::{wx_field, wx_selection_clear,
//         wx_scatter, wx_aerial_fog_color};
//
// Textures (both linear, clamp), bound by the terrain material from `Res<WeatherGround>`:
//   * `field` (Rgba8, map / 8): R rain, G snow, B fog, A storm at a map point, fringe included.
//     Equivalent of Godot `wx_sample`.
//   * `clear` (R8, map / 4): share inside the selected province (feathered edge).
//     Equivalent of `1 - wx_selection_clear`.

// Weather at map point `p` (map px); `map_size` in map px.
fn wx_field(field: texture_2d<f32>, samp: sampler, p: vec2<f32>, map_size: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(field, samp, p / map_size, 0.0);
}

// 1 outside the selected province, 0 inside: multiply the mist / cloud shade by it.
fn wx_selection_clear(clear: texture_2d<f32>, samp: sampler, p: vec2<f32>, map_size: vec2<f32>) -> f32 {
    return 1.0 - textureSampleLevel(clear, samp, p / map_size, 0.0).r;
}

// Sun scatter of the fog (Godot `fog_sun_scatter`): the fog brightens towards the sun.
// `view_dir`: camera -> fragment, `sun_dir`: towards the sun, `amount`: season `fog_sun_scatter`.
// Add the result to the fog colour before the mix: fog_rgb + wx_scatter(...).
fn wx_scatter(view_dir: vec3<f32>, sun_dir: vec3<f32>, sun_color: vec3<f32>, amount: f32, exponent: f32) -> vec3<f32> {
    let facing = max(dot(normalize(view_dir), normalize(sun_dir)), 0.0);
    return sun_color * amount * pow(facing, exponent);
}

// Aerial perspective (Godot `fog_aerial_perspective`): the fog takes the sky tint, strongest when
// looking along the horizon, nil looking straight down. Mirrors `atmosphere::aerial_weight`.
fn wx_aerial_weight(aerial: f32, view_dir_y: f32) -> f32 {
    return aerial * (1.0 - smoothstep(0.0, 0.8, -view_dir_y));
}

fn wx_aerial_fog_color(fog_rgb: vec3<f32>, sky_rgb: vec3<f32>, aerial: f32, view_dir_y: f32) -> vec3<f32> {
    return mix(fog_rgb, sky_rgb, wx_aerial_weight(aerial, view_dir_y));
}
