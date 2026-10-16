// Godot `apply_bcs` (contrast then saturation) on display values, after the tonemapper.
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct CampaignAdjust {
    // x: contrast, y: saturation.
    params: vec4<f32>,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> adjust: CampaignAdjust;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(screen, screen_sampler, in.uv);
    var s = clamp(to_srgb(source.rgb), vec3<f32>(0.0), vec3<f32>(1.0));
    s = mix(vec3<f32>(0.5), s, adjust.params.x);
    s = mix(vec3<f32>(dot(s, vec3<f32>(1.0 / 3.0))), s, adjust.params.y);
    return vec4<f32>(to_linear(clamp(s, vec3<f32>(0.0), vec3<f32>(1.0))), source.a);
}
