// Corps commun des shaders de feuillage de la carte (lot V4) : `foliage.gdshader` (sans
// `discard`, garde le test de profondeur anticipé) et `foliage_winter.gdshader` (hiver : houppiers
// des feuillus ajourés par `discard`, activé par FOLIAGE_WINTER_CUTOUT).

uniform vec3 view_origin = vec3(0.0);
uniform float fade_start = 400.0;
uniform float fade_end = 700.0;
// Densité globale (1 = toutes les instances) : au dézoom, seules les graines > 1 − density
// restent, agrandies de 1/√density pour garder le même couvert.
uniform float density : hint_range(0.05, 1.0) = 1.0;
uniform float wind_strength = 0.035;
uniform float wind_speed = 1.1;
uniform float roughness_value : hint_range(0.0, 1.0) = 0.88;
// Lot V4 (A1-10) : saison de la carte (réglée par vegetation.gd depuis la date de la simulation).
uniform int season : hint_range(0, 3) = 0;
uniform vec3 spring_tint = vec3(1.03, 1.10, 0.9);
uniform vec3 autumn_oak = vec3(0.19, 0.13, 0.035);
uniform vec3 autumn_beech = vec3(0.30, 0.10, 0.018);
uniform vec3 autumn_hedge = vec3(0.20, 0.07, 0.03);
uniform vec3 winter_bare = vec3(0.105, 0.090, 0.082);
// Altitude monde (unités) au-dessus de laquelle les conifères sont poudrés de neige en hiver.
uniform float winter_snow_y = 4.0;

varying float v_ao;
varying vec3 v_tint;
varying vec3 v_local;
varying float v_essence;
varying float v_crown;
varying float v_seed;
varying float v_world_y;

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
	v_essence = UV.x;
	v_crown = UV.y;
	v_seed = seed;
	v_world_y = origin.y;
	// Hiver : houppiers des feuillus dénudés, plus légers (on devine la ramure).
	if (season == 3 && UV.y > 0.5 && UV.x < 1.5) {
		VERTEX.xz *= 0.82;
	}
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
	vec3 albedo = COLOR.rgb * tint * mix(0.72, 1.18, leaves);
	if (v_crown > 0.5) {
		float lum = dot(albedo, vec3(0.2126, 0.7152, 0.0722));
		bool broadleaf = v_essence < 1.5;
		bool hedge = v_essence > 2.5;
		if (season == 0 && (broadleaf || hedge)) {
			albedo *= spring_tint;
		} else if (season == 2) {
			// Automne : chênes roux-olive, hêtres cuivrés, haies rousses ; quelques retardataires verts.
			float turn = smoothstep(0.15, 0.45, fract(v_seed * 7.13)) * mix(0.7, 1.0, leaves);
			vec3 target = v_essence < 0.5 ? autumn_oak : (v_essence < 1.5 ? autumn_beech : autumn_hedge);
			if (v_essence > 1.5 && v_essence < 2.5) {
				turn = 0.0;
			}
			albedo = mix(albedo, target * (lum / 0.09) * mix(0.8, 1.2, leaves), turn * 0.85);
		} else if (season == 3) {
			if (broadleaf) {
				// Ramure nue : houppier ajouré (on voit le sol à travers), gris-brun violacé.
#ifdef FOLIAGE_WINTER_CUTOUT
				if (value_noise(v_local * 2.4) < 0.5) {
					discard;
				}
#endif
				albedo = winter_bare * mix(0.8, 1.25, leaves);
			} else if (hedge) {
				albedo = mix(albedo, winter_bare, 0.7);
			} else {
				// Conifères : plus sombres, neige sur les branches exposées en altitude.
				albedo *= 0.8;
				float snow = smoothstep(winter_snow_y - 1.0, winter_snow_y + 1.0, v_world_y) * smoothstep(0.35, 0.8, leaves);
				albedo = mix(albedo, vec3(0.75, 0.78, 0.82), snow * 0.7);
			}
		}
	}
	ALBEDO = albedo;
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = v_ao;
	AO_LIGHT_AFFECT = 0.4;
	// Lumière transmise à travers le feuillage, vue à contre-jour.
	BACKLIGHT = albedo * (season == 3 && v_essence < 1.5 ? 0.1 : 0.35);
}
