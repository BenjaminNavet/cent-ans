// VQ cground: close-range ground detail of the campaign terrain, in its own module so that
// `terrain.wesl` only carries two call sites.
//
//     import cent_ans_client::campaign::terrain::ground_detail::{GdParams, gd_macro, gd_parcel};
//
// At the campaign close views (camera distance 25..200) one screen pixel covers 15..300 m, so what
// reads as a landscape is the structure of 50 m .. 3 km: strips of the open fields, the tone of
// each parcel, soil and moisture patches, not the sub-metre grain (which the mips erase). Tunables
// come from `data/fx/ground_detail.json`, scaled by the `ground_detail` knob of
// `data/fx/render_quality.json` (0 off, 1 tones, 2 + strips and mottling).

struct GdParams {
    // level (0 off, 1 tones, 2 + strips / mottling), macro tone amp, macro hue amp, macro fade end footprint.
    a: vec4<f32>,
    // parcel luminance contrast, parcel saturation, parcel jitter, structure fade end footprint.
    b: vec4<f32>,
    // strip tone amp, strip balk amp, strip width min (m), strip width max (m).
    c: vec4<f32>,
    // mottle amp, mottle period (m), structure fade start footprint, mottle fade end footprint.
    d: vec4<f32>,
    // macro periods (map px, xyz), plough colour share of the dark strips.
    e: vec4<f32>,
};

fn gd_hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn gd_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(gd_hash(i), gd_hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(gd_hash(i + vec2<f32>(0.0, 1.0)), gd_hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn gd_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Soil / moisture / geology patches of 2 to 20 km: slow tone and warm-cool drift of the open
// ground, kept off the forests (their own canopy tone) and faded where the colour map already
// carries the far view. `p` map px, `footprint` map px per screen px, `open_w` open-ground share.
fn gd_macro(g: GdParams, col: vec3<f32>, p: vec2<f32>, footprint: f32, land: f32, open_w: f32) -> vec3<f32> {
    if (g.a.x < 0.5) {
        return col;
    }
    let n1 = gd_noise(p / g.e.x + vec2<f32>(4.3, 9.1));
    let n2 = gd_noise(p / g.e.y + vec2<f32>(17.7, 2.9));
    let n3 = gd_noise(p / g.e.z + vec2<f32>(8.2, 31.4));
    let tone = (n1 * 0.5 + n2 * 0.3 + n3 * 0.2 - 0.5) * 3.0;
    let hue = (n2 * 0.4 + n3 * 0.6 - 0.5) * 3.0;
    let k = (1.0 - smoothstep(g.a.w * 0.5, g.a.w, footprint)) * land * clamp(open_w, 0.0, 1.0);
    let warm = vec3<f32>(1.0 + hue * 0.10 * g.a.z, 1.0 + hue * 0.02 * g.a.z, 1.0 - hue * 0.13 * g.a.z);
    return mix(col, col * warm * (1.0 + tone * g.a.y), k);
}

// One cultivated parcel: stronger tone and saturation, strips (lanieres) with their balks, soil
// mottling. `id` the parcel draw, `ang` its field angle (rad), `cls` the season class of the crop
// (0 wheat, 1 green crop, 2 ploughed, 3 fallow, 4 meadow, 5 hay, 6 vine), `mpp` metres per map px.
fn gd_parcel(g: GdParams, col_in: vec3<f32>, p: vec2<f32>, id: vec2<f32>, ang: f32, cls: i32,
        footprint: f32, mpp: f32) -> vec3<f32> {
    if (g.a.x < 0.5) {
        return col_in;
    }
    var col = col_in;
    let px_m = max(footprint * mpp, 0.01);
    let fade = 1.0 - smoothstep(g.d.z, g.b.w, footprint);
    // Tone and saturation of the parcel (contrast about the mean luminance of the open ground).
    let lum = max(gd_luma(col), 1e-3);
    let ref_l = 0.14;
    let own = (gd_hash(id + vec2<f32>(5.7, 1.3)) - 0.5) * g.b.z;
    col = col * pow(lum / ref_l, g.b.x * fade) * (1.0 + own * fade);
    let l2 = gd_luma(col);
    col = mix(vec3<f32>(l2), col, 1.0 + g.b.y * fade);
    if (g.a.x < 1.5 || fade <= 0.0) {
        return col;
    }
    // Strips: bands across the parcel, tone of their own (maturity, sowing date), dark balks.
    let width = mix(g.c.z, g.c.w, gd_hash(id + vec2<f32>(2.9, 8.1)));
    let across = vec2<f32>(-sin(ang), cos(ang));
    let t = dot(p * mpp, across) / width;
    let band = floor(t);
    let bh = gd_hash(vec2<f32>(band, id.x * 0.37 + id.y * 1.91));
    let strips = clamp((width / px_m - 1.5) / 2.5, 0.0, 1.0);
    let cultivated = select(1.0, 0.45, cls == 4 || cls == 5 || cls == 6);
    col = col * (1.0 + (bh - 0.5) * 2.0 * g.c.x * strips * cultivated);
    let edge = (0.5 - abs(fract(t) - 0.5)) * width;
    let balk = (1.0 - smoothstep(0.0, max(px_m * 0.7, 1.2), edge)) * clamp(width / px_m / 6.0, 0.0, 1.0);
    col = col * (1.0 - g.c.y * balk * cultivated);
    // Dark strips tip towards the ploughed soil (freshly turned) on the arable classes.
    if (cls <= 3) {
        let dark = smoothstep(0.78, 0.95, bh) * strips * g.e.w;
        col = mix(col, vec3<f32>(0.17, 0.115, 0.065) * (0.8 + 0.4 * gd_hash(vec2<f32>(band, 3.1))), dark);
    }
    // Soil and crop mottling at 20 to 200 m.
    let mott = (gd_noise(p * mpp / g.d.y + id) * 0.65 + gd_noise(p * mpp / (g.d.y * 0.27) + id * 1.7) * 0.35 - 0.5) * 2.0;
    col = col * (1.0 + mott * g.d.x * (1.0 - smoothstep(g.d.z, g.d.w, footprint)));
    return col;
}
