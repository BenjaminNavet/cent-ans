// Arbres de la carte de campagne (lot V3, `vegetation.gd`), instanciés par MultiMesh.
// COLOR (sommet) : RVB = albédo de base, A = poids du vent (0 au pied, 1 en haut).
// INSTANCE_CUSTOM : RVB = teinte multiplicative de l'instance, A = graine aléatoire [0, 1].
// Disparition progressive avec la distance à `view_origin` (fournie par le script, identique
// dans la passe d'ombre) : chaque arbre rétrécit puis disparaît à un seuil qui dépend de sa
// graine, ce qui éclaircit la forêt au lieu de la couper net.
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx;

uniform vec3 view_origin = vec3(0.0);
uniform float fade_start = 400.0;
uniform float fade_end = 700.0;
// Densité globale (1 = toutes les instances) : au dézoom, seules les graines > 1 − density
// restent, agrandies de 1/√density pour garder le même couvert.
uniform float density : hint_range(0.05, 1.0) = 1.0;
uniform float wind_strength = 0.035;
uniform float wind_speed = 1.1;
uniform float roughness_value : hint_range(0.0, 1.0) = 0.88;

// Lot CV1 : saison (poids printemps, été, automne, hiver), voir season_visuals.gd.
global uniform vec4 campaign_season;

varying float v_ao;
varying float v_seed;  // CV1
varying vec3 v_tint;
varying vec3 v_local;

// Bruit de valeur 3D : touffes de feuillage (variation d'albédo à l'échelle des feuilles).
float hash3(vec3 p) {
	p = fract(p * 0.3183099 + 0.1);
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float value_noise(vec3 p) {
	vec3 i = floor(p);
	vec3 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(
		mix(mix(hash3(i), hash3(i + vec3(1, 0, 0)), f.x), mix(hash3(i + vec3(0, 1, 0)), hash3(i + vec3(1, 1, 0)), f.x), f.y),
		mix(mix(hash3(i + vec3(0, 0, 1)), hash3(i + vec3(1, 0, 1)), f.x), mix(hash3(i + vec3(0, 1, 1)), hash3(i + vec3(1, 1, 1)), f.x), f.y),
		f.z);
}

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float seed = INSTANCE_CUSTOM.a;
	float dist = distance(origin.xz, view_origin.xz) + abs(origin.y - view_origin.y) * 0.5;
	float t = clamp((dist - fade_start) / max(fade_end - fade_start, 1.0), 0.0, 1.0);
	t = max(t, 1.0 - density);
	float keep = smoothstep(0.0, 0.18, seed - t) * (1.0 + 0.5 * t) * inversesqrt(density);
	// Vent : rafales lentes qui balaient la carte, plus un frémissement propre à chaque arbre.
	float phase = TIME * wind_speed + origin.x * 0.045 + origin.z * 0.03;
	float gust = 0.6 + 0.4 * sin(phase * 0.37);
	float sway = (sin(phase + seed * 6.2831) * gust + 0.3 * sin(TIME * 2.3 + seed * 20.0)) * wind_strength;
	float weight = COLOR.a * COLOR.a;
	v_local = VERTEX * 9.0 + vec3(seed * 17.0);
	VERTEX.x += sway * weight;
	VERTEX.z += sway * 0.6 * weight;
	VERTEX *= keep;
	v_ao = mix(0.55, 1.0, COLOR.a);
	v_tint = INSTANCE_CUSTOM.rgb;
	v_seed = seed;  // CV1
}

// Lot CV1 : feuillus roux et dorés à l'automne, nus (gris-brun, givrés) l'hiver ; résineux
// (aiguilles, rapport R/V bas) et troncs inchangés, sinon un peu de neige sur le dessus.
vec3 season_foliage(vec3 base, float seed, float up) {
	vec4 w = campaign_season;
	float ratio = base.r / max(base.g, 1e-3);
	float leafy = smoothstep(0.55, 0.62, ratio) * (1.0 - smoothstep(1.0, 1.2, ratio));
	vec3 spring = base * vec3(0.95, 1.2, 0.85);
	vec3 autumn_leaf = seed < 0.4 ? vec3(0.30, 0.10, 0.02) : (seed < 0.75 ? vec3(0.34, 0.20, 0.03) : vec3(0.20, 0.16, 0.05));
	vec3 autumn = mix(base, autumn_leaf * 0.85, leafy * 0.85);
	vec3 winter = mix(base * 0.8, vec3(0.115, 0.098, 0.085), leafy);
	vec3 col = mix(base, spring, leafy) * w.x + base * w.y + autumn * w.z + winter * w.w;
	float snow = w.w * smoothstep(0.45, 0.9, up) * mix(0.4, 0.7, 1.0 - leafy);
	return mix(col, vec3(0.75, 0.77, 0.80), snow);
}

void fragment() {
	vec3 tint = v_tint;
	float leaves = value_noise(v_local) * 0.6 + value_noise(v_local * 2.3) * 0.4;
	vec3 base_color = season_foliage(COLOR.rgb, v_seed, (INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).y);  // CV1
	ALBEDO = base_color * tint * mix(0.72, 1.18, leaves);
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = v_ao;
	AO_LIGHT_AFFECT = 0.4;
	// Lumière transmise à travers le feuillage, vue à contre-jour.
	BACKLIGHT = base_color * tint * 0.35;
}
