// Godot `apply_bcs` (contrast then saturation) on display values, after the tonemapper.
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct CampaignAdjust {
    // x: contrast, y: saturation.
    params: vec4<f32>,
    // Environment grade: x saturation, y contrast, z shadow saturation, w strength.
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
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

// Colour grade of the campaign environment (`campaign.seasons.<season>.grade`), once for the whole
// frame (ground, sea, rivers, towns, trees, armies): contrast about the mid grey, lift in the
// shadows, highlight tint, saturation that drops further in the shadows. Linear values.
fn campaign_grade(c: vec3<f32>) -> vec3<f32> {
    if (adjust.grade.w <= 0.0) {
        return c;
    }
    var col = (c - vec3<f32>(0.5)) * adjust.grade.y + vec3<f32>(0.5);
    col = max(col, vec3<f32>(0.0));
    let luma = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = col + adjust.lift.rgb * (1.0 - smoothstep(0.0, 0.5, luma));
    col = col * mix(vec3<f32>(1.0), adjust.highlights.rgb, smoothstep(0.3, 0.9, luma));
    let sat = mix(adjust.grade.z, adjust.grade.x, smoothstep(0.02, 0.35, luma));
    col = vec3<f32>(luma) + (col - vec3<f32>(luma)) * sat;
    return mix(c, max(col, vec3<f32>(0.0)), adjust.grade.w);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(screen, screen_sampler, in.uv);
    let graded = clamp(campaign_grade(source.rgb), vec3<f32>(0.0), vec3<f32>(1.0));
    var s = to_srgb(graded);
    s = mix(vec3<f32>(0.5), s, adjust.params.x);
    s = mix(vec3<f32>(dot(s, vec3<f32>(1.0 / 3.0))), s, adjust.params.y);
    return vec4<f32>(to_linear(clamp(s, vec3<f32>(0.0), vec3<f32>(1.0))), source.a);
}
