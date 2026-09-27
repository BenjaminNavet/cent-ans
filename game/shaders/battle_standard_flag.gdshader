// Étendards portés (lot EP5, ADR 0034) : l'étoffe est attachée à la pointe de la hampe du
// porte-étendard, lue dans la texture d'os de sa figurine (os virtuel `Prop`, même clip et même
// instant que la figurine : INSTANCE_CUSTOM identique), et flotte dans le vent de la bataille.
// Un seul MultiMesh par rig (à pied, à cheval) et par état (porté, à terre) pour tous les
// régiments : les étoffes sont les couches d'un Texture2DArray (`cloths`), leur taille dans
// `cell_info`. Au loin, le drapeau grandit et s'éclaire pour rester lisible (jusqu'à ~600 m).
//
// INSTANCE_CUSTOM = (instant, clip dans le jeu, couche d'étoffe, état + phase) ; état (partie
// entière de w) : 0 porté, 2 pris (porté bas par le vainqueur, étoffe renversée, sans vent),
// 3 à terre (couché sur le sol). Fraction de w : phase propre de l'onde.
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform sampler2D bone_tex : filter_nearest, repeat_disable;
uniform float bone_fps = 24.0;
uniform vec4 clips[64];
uniform ivec4 clip_set = ivec4(0);
uniform int clip_set_size = 1;
uniform float anim_time = 0.0;
uniform int prop_bone = 22;
// Sans figurine dédiée (repli) : hampe fixe, pointe en `pole_top`, axe `pole_axis`.
uniform bool skinned = true;
uniform vec3 pole_top = vec3(0.0, 4.2, 0.0);
uniform vec3 pole_axis = vec3(0.0, 1.0, 0.0);
uniform sampler2DArray cloths : source_color, filter_linear_mipmap, repeat_disable;
// Par couche : (longueur m, hauteur m, 1 = étoffe peinte entière, 0 = écu seul).
uniform vec4 cell_info[64];
uniform vec2 wind_dir = vec2(1.0, 0.0);
uniform float wind_strength = 0.6;
uniform float wind_gust = 0.2;
uniform float far_scale = 2.2;
uniform float far_from = 140.0;
uniform float far_to = 600.0;
uniform vec3 livery : source_color = vec3(0.5);

varying float v_fold;
varying float v_far;
varying float v_state;
varying float v_layer;
varying float v_full;

int pick(ivec4 s, int i) {
	return i == 0 ? s.x : (i == 1 ? s.y : (i == 2 ? s.z : s.w));
}

void clip_rows(int clip, float t, out int r0, out int r1, out float f) {
	vec4 c = clips[clip];
	float frames = max(c.y, 1.0);
	float x = max(t, 0.0) * bone_fps;
	float f0;
	float f1;
	if (c.z > 0.5) {
		x = mod(x, frames);
		f0 = floor(x);
		f1 = mod(f0 + 1.0, frames);
	} else {
		x = min(x, frames - 1.0);
		f0 = floor(x);
		f1 = min(f0 + 1.0, frames - 1.0);
	}
	f = x - f0;
	r0 = int(c.x + f0);
	r1 = int(c.x + f1);
}

vec4 bone_row(int k, int r0, int r1, float f) {
	int x = prop_bone * 3 + k;
	return mix(texelFetch(bone_tex, ivec2(x, r0), 0), texelFetch(bone_tex, ivec2(x, r1), 0), f);
}

void vertex() {
	vec4 inst = INSTANCE_CUSTOM;
	int layer = int(inst.z + 0.5);
	vec4 info = cell_info[clamp(layer, 0, 63)];
	float state = floor(inst.w + 0.001);
	float phase = fract(inst.w) * 40.0;
	vec3 top = pole_top;
	vec3 axis = pole_axis;
	if (skinned) {
		int n = max(clip_set_size, 1);
		int clip = pick(clip_set, int(mod(inst.y, float(n))));
		int r0;
		int r1;
		float f;
		clip_rows(clip, anim_time - inst.x, r0, r1, f);
		vec4 a = bone_row(0, r0, r1, f);
		vec4 b = bone_row(1, r0, r1, f);
		vec4 c = bone_row(2, r0, r1, f);
		vec4 p = vec4(pole_top, 1.0);
		top = vec3(dot(a, p), dot(b, p), dot(c, p));
		axis = normalize(vec3(dot(a.xyz, pole_axis), dot(b.xyz, pole_axis), dot(c.xyz, pole_axis)));
	}
	// Vent dans le repère de la figurine (rotation pure : transposée = inverse).
	mat3 rot = mat3(MODEL_MATRIX);
	vec3 wind_local = transpose(rot) * vec3(wind_dir.x, 0.0, wind_dir.y);
	vec3 fly = wind_local - axis * dot(wind_local, axis);
	if (length(fly) < 0.05) {
		fly = cross(axis, vec3(0.0, 0.0, 1.0));
	}
	fly = normalize(fly);
	float u = UV.x;
	float v = UV.y;
	float gust = sin(TIME * 0.7 + phase * 0.37) * 0.5 + sin(TIME * 1.9 + phase) * 0.25;
	float s = clamp(wind_strength + wind_gust * gust, 0.0, 1.0);
	float dist = length((MODELVIEW_MATRIX * vec4(top, 1.0)).xyz);
	v_far = smoothstep(far_from, far_to, dist);
	float k = 1.0 + (far_scale - 1.0) * v_far;
	float len = info.x * k;
	float height = info.y * k;
	vec3 anchor = top + axis * (k - 1.0) * height * 0.55;
	if (state > 1.5 && state < 2.5) {
		// Pris : porté bas, sans vent (pend le long de la hampe), étoffe renversée.
		s = 0.04;
		anchor = top - axis * 1.1;
	}
	float t = TIME * mix(2.5, 7.5, s) + phase;
	float amp = mix(0.08, 0.3, s);
	float wave = sin(u * 7.0 - t) * amp + sin(u * 13.0 - t * 1.7 + 1.3) * amp * 0.27;
	vec3 side = normalize(cross(axis, fly));
	vec3 pos;
	if (state > 2.5) {
		// À terre : couché à plat le long de la hampe tombée, froissé.
		vec3 flat_fly = normalize(cross(axis, vec3(0.0, 1.0, 0.0)) + vec3(0.0001));
		pos = anchor - axis * v * height + flat_fly * u * len;
		pos.y = max(pos.y, 0.0) + 0.04 + 0.03 * sin(u * 9.0 + v * 7.0 + phase);
		wave = 0.0;
	} else {
		float sag = u * u * mix(0.45, 0.06, s) * len;
		pos = anchor - axis * (v * height + sag) + fly * u * len * mix(0.8, 1.0, s) + side * wave * u * len * 0.35;
	}
	VERTEX = pos;
	float slope = (cos(u * 7.0 - t) * 7.0 * amp + cos(u * 13.0 - t * 1.7 + 1.3) * 13.0 * amp * 0.27) * u * 0.35;
	NORMAL = normalize(side - fly * slope);
	v_fold = clamp(0.5 + wave * 2.0, 0.0, 1.0);
	v_state = state;
	v_layer = float(layer);
	v_full = info.z;
}

void fragment() {
	vec2 uv = UV;
	if (v_state > 1.5 && v_state < 2.5) {
		uv.y = 1.0 - uv.y;
	}
	vec4 cloth = texture(cloths, vec3(clamp(uv, vec2(0.004), vec2(0.996)), v_layer));
	vec3 color = cloth.rgb;
	if (v_full < 0.5) {
		color = mix(livery, cloth.rgb, cloth.a);
		cloth.a = 1.0;
	}
	// Au loin, l'alpha des mipmaps s'effondre : on le relève pour garder la silhouette.
	ALPHA = clamp(cloth.a * (1.0 + v_far * 1.5), 0.0, 1.0);
	ALPHA_SCISSOR_THRESHOLD = 0.5;
	if (v_state > 1.5 && v_state < 2.5) {
		color *= 0.75;
	}
	color *= 0.85 + 0.15 * v_fold;
	ALBEDO = color;
	EMISSION = color * mix(0.22, 0.5, v_far);
	ROUGHNESS = 0.9;
}
