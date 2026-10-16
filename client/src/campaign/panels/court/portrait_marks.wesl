// Port of game/shaders/portrait_marks.gdshader (DA2, ADR 0063): marks applied to the picture
// only (frame and head-dresses stay apart). grey: greyscale of the dead; pallor: waxy pallor of
// the sick; dim: captivity (shade). `flip` mirrors the picture horizontally (the Godot portrait
// mirrors by a negative scale; here the UV is flipped).
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct Marks {
    grey: f32,
    pallor: f32,
    dim: f32,
    flip: f32,
}

@group(1) @binding(0) var<uniform> marks: Marks;
@group(1) @binding(1) var picture: texture_2d<f32>;
@group(1) @binding(2) var picture_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    if marks.flip > 0.5 {
        uv.x = 1.0 - uv.x;
    }
    let c = textureSample(picture, picture_sampler, uv);
    let luma = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    // Pallor: half desaturated, lightened towards a greenish wax white.
    let pale = mix(c.rgb, vec3<f32>(luma), 0.5) * 0.82 + vec3<f32>(0.15, 0.17, 0.12);
    var col = mix(c.rgb, pale, marks.pallor);
    // Greyscale (manuscript in camaieu): warm grey, slightly lightened.
    let grisaille = vec3<f32>(luma) * vec3<f32>(0.93, 0.91, 0.87) + vec3<f32>(0.06);
    col = mix(col, grisaille, marks.grey);
    col = col * (1.0 - 0.25 * marks.dim);
    return vec4<f32>(col, c.a);
}
