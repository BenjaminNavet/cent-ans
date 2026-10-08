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

#include "res://shaders/battle_bone_anim.gdshaderinc"
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
// Lot AN1a (ADR 0096, `data/fx/atmosphere.json` `secondary_motion.flag`) : amplitude relative et
// vitesse de l'onde (vent nul → fort), ondulation le long de la hampe, vent apparent ajouté par
// l'allure du porteur (par clip du jeu : arrêt, marche, course, sonnerie), échelle du vent.
uniform vec2 flag_amp = vec2(0.08, 0.3);
uniform vec2 flag_wave_speed = vec2(2.5, 7.5);
uniform float flag_ripple = 0.0;
uniform vec4 flag_gait_wind = vec4(0.0);
uniform float flag_wind_scale = 1.0;
// NT10 : fondu au changement de clip du porteur (même empaquetage de INSTANCE_CUSTOM.y que le
// shader skinné, `custom_fade` = 1) ; 0 = changement sec.
uniform float role_blend = 0.0;

varying float v_fold;
varying float v_far;
varying float v_state;
varying float v_layer;
varying float v_full;

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
	float gait = 0.0;
	if (skinned) {
		int n = max(clip_set_size, 1);
		int slot = int(mod(mod(inst.y, 32.0), float(n)));
		gait = slot == 0 ? flag_gait_wind.x : (slot == 1 ? flag_gait_wind.y : (slot == 2 ? flag_gait_wind.z : flag_gait_wind.w));
		int clip = pick_clip(clip_set, slot);
		int r0;
		int r1;
		float f;
		clip_rows(clip, anim_time - inst.x, r0, r1, f);
		vec4 a = bone_row(0, r0, r1, f);
		vec4 b = bone_row(1, r0, r1, f);
		vec4 c = bone_row(2, r0, r1, f);
		if (inst.y >= 32.0 && role_blend > 0.001) {
			int pclip = int(mod(floor(inst.y / 32.0), 128.0) + 0.5) - 1;
			float since = mod(anim_time - floor(inst.y / 4096.0) / 32.0, 64.0);
			float k = smoothstep(0.0, role_blend, since);
			if (pclip >= 0 && k < 1.0) {
				clip_rows(pclip, anim_time - inst.x, r0, r1, f);
				a = mix(bone_row(0, r0, r1, f), a, k);
				b = mix(bone_row(1, r0, r1, f), b, k);
				c = mix(bone_row(2, r0, r1, f), c, k);
			}
		}
		vec4 p = vec4(pole_top, 1.0);
		top = vec3(dot(a, p), dot(b, p), dot(c, p));
		axis = normalize(vec3(dot(a.xyz, pole_axis), dot(b.xyz, pole_axis), dot(c.xyz, pole_axis)));
	}
	// Vent dans le repère de la figurine (rotation pure : transposée = inverse).
	mat3 rot = mat3(MODEL_MATRIX);
	vec3 wind_local = transpose(rot) * vec3(wind_dir.x, 0.0, wind_dir.y);
	float wscale = length(wind_local);
	wind_local = wscale > 1e-4 ? wind_local / wscale : vec3(1.0, 0.0, 0.0);
	// AN1a : vent apparent = vent + allure du porteur (l'étoffe traîne vers l'arrière, -Z).
	float base = clamp(wind_strength * flag_wind_scale, 0.0, 1.0);
	vec3 apparent = wind_local * base + vec3(0.0, 0.0, -gait);
	float app_len = length(apparent);
	if (app_len > 1e-3) {
		wind_local = apparent / app_len;
	}
	vec3 fly = wind_local - axis * dot(wind_local, axis);
	if (length(fly) < 0.05) {
		fly = cross(axis, vec3(0.0, 0.0, 1.0));
	}
	fly = normalize(fly);
	float u = UV.x;
	float v = UV.y;
	float gust = sin(TIME * 0.7 + phase * 0.37) * 0.5 + sin(TIME * 1.9 + phase) * 0.25;
	float s = clamp(max(app_len, base) + wind_gust * gust, 0.0, 1.0);
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
	float t = TIME * mix(flag_wave_speed.x, flag_wave_speed.y, s) + phase;
	float amp = mix(flag_amp.x, flag_amp.y, s);
	// Onde le long du battant, plus une ondulation le long de la hampe (AN1a) qui croît vers
	// le bord libre.
	float ripple = sin(v * 5.0 + u * 3.0 - t * 1.3 + 0.7) * amp * flag_ripple * u;
	float wave = sin(u * 7.0 - t) * amp + sin(u * 13.0 - t * 1.7 + 1.3) * amp * 0.27 + ripple;
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
