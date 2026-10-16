// Pose of a tethered camp horse in the vertex stage (port of game/shaders/camp_horse.gdshader,
// `AnimalMotion.camp_horse_material`): breathing, weight shifting from one hind hoof to the other,
// head that rises now and then and chews when low, tail swishing the flies. The kit models stand
// on y = 0, nose towards -X, tail towards +X. Shared by `camp_horse.wesl` (main pass) and
// `camp_horse_shadow.wesl` (shadows).
// Import: import cent_ans_client::battle::decor::camp_horse_pose::{CampHorseParams, camp_pose};

struct CampHorseParams {
    head: vec4<f32>,   // pivot x, pivot y, neck start x, head end x
    head2: vec4<f32>,  // raised head (rad), cycle (s), share of the cycle with the head up, nod
    head3: vec4<f32>,  // nod frequency, chew frequency, chew amplitude (rad), ramp (s; 0 = 5 %)
    tail: vec4<f32>,   // start x, end x, top y, length
    tail2: vec4<f32>,  // swing, frequency, swish period, 0
    shift: vec4<f32>,  // period, hoof lift, hoof back, sway
    breath: vec4<f32>, // vertical, lateral, frequency, 0
    misc: vec4<f32>,   // hair (1 on the mane and tail), on (1 = animated), 0, 0
};

const TAU: f32 = 6.2831853;

fn ch_hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// `origin_xz`: world x, z of the instance (the seed of the horse), `time`: seconds.
fn camp_pose(ch: CampHorseParams, v_in: vec3<f32>, origin_xz: vec2<f32>, time: f32) -> vec3<f32> {
    if ch.misc.y < 0.5 {
        return v_in;
    }
    let seed = ch_hash(origin_xz * 0.37);
    let t = time + seed * 97.0;
    var v = v_in;
    let side = select(-1.0, 1.0, v.z >= 0.0);

    // Breathing: flanks and back.
    let barrel = (1.0 - smoothstep(0.5, 0.8, abs(v.x))) * smoothstep(0.6, 0.95, v.y);
    let breath = sin(t * TAU * ch.breath.z);
    v.z *= 1.0 + ch.breath.y * breath * barrel;
    v.y += ch.breath.x * breath * smoothstep(0.8, 1.5, v.y) * (1.0 - smoothstep(0.7, 1.1, abs(v.x)));

    // Weight passing from one hind hoof to the other.
    let sv = sin(t * TAU / ch.shift.x);
    let cock = select(smoothstep(0.55, 0.95, -sv), smoothstep(0.55, 0.95, sv), side > 0.0);
    let leg_w = clamp((0.8 - v.y) / 0.8, 0.0, 1.0) * (1.0 - ch.misc.x) * smoothstep(0.35, 0.5, v.x);
    v.y += ch.shift.y * cock * leg_w;
    v.x -= ch.shift.z * cock * leg_w;
    let lean = smoothstep(0.55, 0.95, sv) - smoothstep(0.55, 0.95, -sv);
    v.z -= ch.shift.w * lean * smoothstep(0.3, 1.2, v.y);

    // Head: raised from time to time, chews when low, slight nod.
    var hw = clamp((ch.head.z - v.x) / (ch.head.z - ch.head.w), 0.0, 1.0);
    hw = hw * hw * (3.0 - 2.0 * hw);
    if hw > 0.0 {
        let u = fract(t / ch.head2.y);
        var ramp = 0.05;
        if ch.head3.w > 0.0 {
            ramp = clamp(ch.head3.w / ch.head2.y, 0.01, 0.2);
        }
        let up = smoothstep(0.0, ramp, u) * (1.0 - smoothstep(ch.head2.z - ramp, ch.head2.z, u));
        var angle = -ch.head2.x * up;
        angle += sin(t * TAU * ch.head3.y) * ch.head3.z * (1.0 - up);
        angle += sin(t * TAU * ch.head3.x) * ch.head2.w;
        angle *= hw;
        let d = v.xy - ch.head.xy;
        let c = cos(angle);
        let s = sin(angle);
        let r = ch.head.xy + vec2<f32>(d.x * c - d.y * s, d.y * c + d.x * s);
        v = vec3<f32>(r.x, r.y, v.z);
    }

    // Tail: swishes the flies in bursts.
    let tw = ch.misc.x * smoothstep(ch.tail.x, ch.tail.y, v.x) * clamp((ch.tail.z - v.y) / ch.tail.w, 0.0, 1.0);
    if tw > 0.0 {
        let swish = 0.3 + 0.7 * pow(max(0.0, sin(t * TAU / ch.tail2.z)), 3.0);
        v.z += tw * ch.tail2.x * swish * sin(t * TAU * ch.tail2.y);
    }
    return v;
}
