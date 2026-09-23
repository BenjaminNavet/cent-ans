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
uniform float wind_strength = 0.035;
uniform float wind_speed = 1.1;
uniform float roughness_value : hint_range(0.0, 1.0) = 0.88;

varying float v_ao;
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
	float keep = smoothstep(0.0, 0.18, seed - t);
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
}

void fragment() {
	vec3 tint = v_tint;
	float leaves = value_noise(v_local) * 0.6 + value_noise(v_local * 2.3) * 0.4;
	ALBEDO = COLOR.rgb * tint * mix(0.72, 1.18, leaves);
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = v_ao;
	AO_LIGHT_AFFECT = 0.4;
	// Lumière transmise à travers le feuillage, vue à contre-jour.
	BACKLIGHT = COLOR.rgb * tint * 0.35;
}
