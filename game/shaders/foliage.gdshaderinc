// Corps commun des shaders de feuillage de la carte (lot V4) : `foliage.gdshader` (sans
// `discard`, garde le test de profondeur anticipé) et `foliage_winter.gdshader` (hiver : houppiers
// des feuillus ajourés par `discard`, activé par FOLIAGE_WINTER_CUTOUT).

uniform vec3 view_origin = vec3(0.0);
uniform float fade_start = 400.0;
uniform float fade_end = 700.0;
// Densité globale (1 = toutes les instances) : au dézoom, seules les graines > 1 − density
// restent, agrandies de 1/√density pour garder le même couvert.
uniform float density : hint_range(0.05, 1.0) = 1.0;
// Lot SZ4b : seuil de graine propre à une instance de MultiMesh (forêt dense, `ForestDetail`) :
// les arbres dont la graine dépasse à peine ce seuil sont petits et grandissent quand il baisse.
instance uniform float instance_cut = 0.0;
// Largeur de la bande de croissance au-dessus du seuil (0,18 : éclaircissement des tuiles de base).
instance uniform float instance_band = 0.18;
uniform float wind_strength = 0.035;
uniform float wind_speed = 1.1;
uniform float roughness_value : hint_range(0.0, 1.0) = 0.88;
// Lot CV1 : saison (poids printemps, été, automne, hiver, somme 1), publiée par season_visuals.gd.
// Lot V4 : teintes par essence (UV.x) pondérées par ces poids.
global uniform vec4 campaign_season;
// Lot ZG4 : échelle des arbres aux paliers vallée / site (1 au palier comté et au-delà ; les arbres
// de la carte font ~1 km de haut, lisibles de loin, absurdes vus à 200 m). Lot SZ4 :
// MapPropScale.tree_scale, taille réelle dès le palier vallée.
global uniform float campaign_prop_scale;
const float FOLIAGE_GROUND_SINK = 0.08;
uniform vec3 spring_tint = vec3(1.03, 1.10, 0.9);
uniform vec3 autumn_oak = vec3(0.19, 0.13, 0.035);
uniform vec3 autumn_beech = vec3(0.30, 0.10, 0.018);
uniform vec3 autumn_hedge = vec3(0.20, 0.07, 0.03);
uniform vec3 winter_bare = vec3(0.105, 0.090, 0.082);
// Altitude monde (unités) au-dessus de laquelle les conifères sont poudrés de neige en hiver.
uniform float winter_snow_y = 4.0;

// Lot PO3 : variété des massifs (pas de nouvel asset). Teinte : trois tons (sombre, moyen, doré)
// choisis par un bruit de basse fréquence sur la position (peuplements de ~`tone_period` unités),
// un peu de jitter par arbre. Échelle : ±`scale_jitter` par arbre (graine). Lisières : sur la rampe
// de la couverture forestière (`forest_cover`, splat réduit, `Vegetation.build`), arbres plus bas
// (`edge_height`) et clairsemés (`edge_thin` de la rangée retirée) ; arbres des champs (couverture
// nulle) et cœur des massifs inchangés. Haies (essence 3) exclues.
uniform vec3 tone_dark = vec3(0.84, 0.9, 0.86);
uniform vec3 tone_warm = vec3(1.1, 1.06, 0.84);
uniform float tone_period = 22.0;
uniform float scale_jitter = 0.25;
uniform sampler2D forest_cover : filter_linear, repeat_disable, hint_default_black;
uniform bool has_forest_cover = false;
uniform vec2 cover_map_size = vec2(4096.0, 4096.0);
uniform vec4 cover_channel = vec4(0.0, 0.0, 1.0, 0.0);
uniform float cover_low = 0.35;
uniform float cover_high = 0.65;
uniform float edge_height = 0.62;
uniform float edge_thin = 0.45;

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

float value_noise_2d(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = hash3(vec3(i, 7.0));
	float b = hash3(vec3(i + vec2(1.0, 0.0), 7.0));
	float c = hash3(vec3(i + vec2(0.0, 1.0), 7.0));
	float d = hash3(vec3(i + vec2(1.0, 1.0), 7.0));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
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
	t = max(max(t, 1.0 - density), instance_cut);
	float keep = smoothstep(0.0, instance_band, seed - t) * (1.0 + 0.5 * t) * inversesqrt(density);
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
	if (UV.y > 0.5 && UV.x < 1.5) {
		VERTEX.xz *= mix(1.0, 0.82, campaign_season.w);
	}
	v_local = VERTEX * 9.0 + vec3(seed * 17.0);
	VERTEX.x += sway * weight;
	VERTEX.z += sway * 0.6 * weight;
	// PO3 : échelle par arbre et lisières (haies exclues).
	vec3 variety = vec3(1.0);
	vec3 tone = vec3(1.0);
	if (UV.x < 2.5) {
		variety *= 1.0 + scale_jitter * (2.0 * fract(seed * 53.17) - 1.0);
		if (has_forest_cover) {
			float cover = dot(textureLod(forest_cover, origin.xz / cover_map_size, 0.0), cover_channel);
			float d = smoothstep(cover_low, cover_high, cover);
			float edge = smoothstep(0.02, 0.2, d) * (1.0 - smoothstep(0.55, 0.9, d));
			variety *= vec3(mix(1.0, 0.85, edge), mix(1.0, edge_height, edge), mix(1.0, 0.85, edge));
			variety *= step(edge * edge_thin, fract(seed * 91.7));
		}
		float n = value_noise_2d(origin.xz / tone_period) + 0.18 * (fract(seed * 13.1) - 0.5);
		tone = n < 0.5 ? mix(tone_dark, vec3(1.0), smoothstep(0.3, 0.45, n)) : mix(vec3(1.0), tone_warm, smoothstep(0.55, 0.7, n));
	}
	VERTEX *= keep * campaign_prop_scale * variety;
	// Lot SZ4b : le pied de l'instance est enfoncé de GROUND_SINK × sa hauteur de carte
	// (`VegetationTileJob.GROUND_SINK`, crate `vegetation`) ; réduit avec l'arbre, sinon un arbre à
	// taille réelle reste enterré sous 80 m de sol (forêts invisibles au palier vallée).
	VERTEX.y += FOLIAGE_GROUND_SINK * (1.0 - campaign_prop_scale);
	v_ao = mix(0.55, 1.0, COLOR.a);
	v_tint = INSTANCE_CUSTOM.rgb * tone;
}

void fragment() {
	vec3 tint = v_tint;
	float leaves = value_noise(v_local) * 0.6 + value_noise(v_local * 2.3) * 0.4;
	vec3 albedo = COLOR.rgb * tint * mix(0.72, 1.18, leaves);
	if (v_crown > 0.5) {
		vec4 w = campaign_season;
		float lum = dot(albedo, vec3(0.2126, 0.7152, 0.0722));
		bool broadleaf = v_essence < 1.5;
		bool conifer = v_essence > 1.5 && v_essence < 2.5;
		bool hedge = v_essence > 2.5;
		// Printemps : feuillus et haies plus tendres.
		vec3 spring = conifer ? albedo : albedo * spring_tint;
		// Automne : chênes roux-olive, hêtres cuivrés, haies rousses ; quelques retardataires verts.
		float turn = conifer ? 0.0 : smoothstep(0.15, 0.45, fract(v_seed * 7.13)) * mix(0.7, 1.0, leaves);
		vec3 target = v_essence < 0.5 ? autumn_oak : (v_essence < 1.5 ? autumn_beech : autumn_hedge);
		vec3 autumn = mix(albedo, target * (lum / 0.09) * mix(0.8, 1.2, leaves), turn * 0.85);
		// Hiver : ramure nue gris-brun ; conifères plus sombres, poudrés de neige en altitude et
		// sur les faces tournées vers le ciel.
		vec3 winter;
		if (broadleaf) {
#ifdef FOLIAGE_WINTER_CUTOUT
			// Houppier ajouré (on voit le sol à travers).
			if (w.w > 0.5 && value_noise(v_local * 2.4) < 0.5) {
				discard;
			}
#endif
			winter = winter_bare * mix(0.8, 1.25, leaves);
		} else if (hedge) {
			winter = mix(albedo, winter_bare, 0.7);
		} else {
			float up = (INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).y;
			float snow = max(smoothstep(winter_snow_y - 1.0, winter_snow_y + 1.0, v_world_y), 0.5 * smoothstep(0.45, 0.9, up));
			winter = mix(albedo * 0.8, vec3(0.75, 0.78, 0.82), snow * smoothstep(0.35, 0.8, leaves) * 0.7);
		}
		albedo = spring * w.x + albedo * w.y + autumn * w.z + winter * w.w;
	}
	ALBEDO = albedo;
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = v_ao;
	AO_LIGHT_AFFECT = 0.4;
	// Lumière transmise à travers le feuillage, vue à contre-jour.
	BACKLIGHT = albedo * (v_essence < 1.5 ? mix(0.35, 0.1, campaign_season.w) : 0.35);
}
