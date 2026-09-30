// Paramètres et fonctions communs du feuillage de la carte (lot FC2, extraits de
// `foliage.gdshaderinc`) : éclaircissement avec la distance, variété des massifs (PO3), teinte
// saisonnière par essence (V4, CV1). Inclus par `foliage.gdshaderinc` (maillages) et
// `campaign_tree_impostor.gdshader` (imposteurs) : mêmes uniformes, posées par `vegetation.gd`
// sur les deux matériaux.

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
uniform vec3 tone_warm = vec3(1.07, 1.04, 0.9);
uniform float tone_period = 22.0;
uniform float scale_jitter = 0.25;
uniform sampler2D forest_cover : filter_linear, repeat_disable, hint_default_black;
uniform bool has_forest_cover = false;
uniform vec2 cover_map_size = vec2(1.0, 1.0);  // taille du monde (map.json), fournie par le script
uniform vec4 cover_channel = vec4(0.0, 0.0, 1.0, 0.0);
uniform float cover_low = 0.35;
uniform float cover_high = 0.65;
uniform float edge_height = 0.62;
uniform float edge_thin = 0.45;

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

// Facteur d'échelle de l'éclaircissement : chaque arbre rétrécit puis disparaît à un seuil qui
// dépend de sa graine (distance à `view_origin`, densité globale, seuil de l'instance).
float foliage_keep(vec3 origin, float seed) {
	float dist = distance(origin.xz, view_origin.xz) + abs(origin.y - view_origin.y) * 0.5;
	float t = clamp((dist - fade_start) / max(fade_end - fade_start, 1.0), 0.0, 1.0);
	t = max(max(t, 1.0 - density), instance_cut);
	return smoothstep(0.0, instance_band, seed - t) * (1.0 + 0.5 * t) * inversesqrt(density);
}

// PO3 : échelle par arbre et lisières (haies exclues : appeler pour les essences 0-2 seulement) ;
// `tone` reçoit la teinte du peuplement.
vec3 foliage_variety(vec3 origin, float seed, out vec3 tone) {
	vec3 variety = vec3(1.0 + scale_jitter * (2.0 * fract(seed * 53.17) - 1.0));
	if (has_forest_cover) {
		float cover = dot(textureLod(forest_cover, origin.xz / cover_map_size, 0.0), cover_channel);
		float d = smoothstep(cover_low, cover_high, cover);
		float edge = smoothstep(0.02, 0.2, d) * (1.0 - smoothstep(0.55, 0.9, d));
		variety *= vec3(mix(1.0, 0.85, edge), mix(1.0, edge_height, edge), mix(1.0, 0.85, edge));
		variety *= step(edge * edge_thin, fract(seed * 91.7));
	}
	float n = value_noise_2d(origin.xz / tone_period) + 0.18 * (fract(seed * 13.1) - 0.5);
	tone = n < 0.5 ? mix(tone_dark, vec3(1.0), smoothstep(0.3, 0.45, n)) : mix(vec3(1.0), tone_warm, smoothstep(0.55, 0.7, n));
	return variety;
}

// Teinte saisonnière d'un houppier (`essence` : 0 chêne, 1 hêtre, 2 conifère, 3 haie) ; `leaves` :
// bruit de feuillage [0, 1] ; `up` : composante verticale de la normale monde ; `world_y` :
// altitude du pied. Le discard ajouré d'hiver des feuillus reste chez l'appelant.
vec3 foliage_season(vec3 albedo, float essence, float seed, float leaves, float up, float world_y) {
	vec4 w = campaign_season;
	float lum = dot(albedo, vec3(0.2126, 0.7152, 0.0722));
	bool broadleaf = essence < 1.5;
	bool conifer = essence > 1.5 && essence < 2.5;
	bool hedge = essence > 2.5;
	// Printemps : feuillus et haies plus tendres.
	vec3 spring = conifer ? albedo : albedo * spring_tint;
	// Automne : chênes roux-olive, hêtres cuivrés, haies rousses ; quelques retardataires verts.
	float turn = conifer ? 0.0 : smoothstep(0.15, 0.45, fract(seed * 7.13)) * mix(0.7, 1.0, leaves);
	vec3 target = essence < 0.5 ? autumn_oak : (essence < 1.5 ? autumn_beech : autumn_hedge);
	vec3 autumn = mix(albedo, target * (lum / 0.09) * mix(0.8, 1.2, leaves), turn * 0.85);
	// Hiver : ramure nue gris-brun ; conifères plus sombres, poudrés de neige en altitude et
	// sur les faces tournées vers le ciel.
	vec3 winter;
	if (broadleaf) {
		winter = winter_bare * mix(0.8, 1.25, leaves);
	} else if (hedge) {
		winter = mix(albedo, winter_bare, 0.7);
	} else {
		float snow = max(smoothstep(winter_snow_y - 1.0, winter_snow_y + 1.0, world_y), 0.5 * smoothstep(0.45, 0.9, up));
		winter = mix(albedo * 0.8, vec3(0.75, 0.78, 0.82), snow * smoothstep(0.35, 0.8, leaves) * 0.7);
	}
	return spring * w.x + albedo * w.y + autumn * w.z + winter * w.w;
}
