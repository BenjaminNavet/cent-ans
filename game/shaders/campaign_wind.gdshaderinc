// Lot RV-F : rafales de vent qui traversent la carte de campagne. Un seul champ de rafales,
// partagé par le couvert forestier du terrain (`forest_wind`, campaign_life.gdshaderinc) et par les
// imposteurs d'arbres (campaign_tree_impostor.gdshaderinc) : la même rafale éclaircit la forêt vue
// de haut et fait plier les arbres de près. Nappes allongées dans le sens du vent, qui avancent
// avec lui ; les feuilles retournées éclaircissent et argentent le couvert au passage.
#ifndef CAMPAIGN_WIND_INC
#define CAMPAIGN_WIND_INC

// Direction du vent (plan de carte x, z), alignée sur le vent météo (`weather_wind`, d'ouest).
uniform vec2 wind_gust_dir = vec2(0.98, 0.2);
// Vitesse d'avance des rafales (unités monde par seconde ; 1 u ≈ 0,7 km : stylisé, lisible).
uniform float wind_gust_speed = 2.6;
// Taille des nappes de rafale : le long du vent, en travers (unités monde).
uniform vec2 wind_gust_size = vec2(34.0, 16.0);
// Part du temps et de l'espace sous la rafale (seuil du bruit) et douceur du front.
uniform float wind_gust_share = 0.38;
uniform float wind_gust_soft = 0.22;

float wind_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float wind_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(wind_hash(i), wind_hash(i + vec2(1.0, 0.0)), u.x),
		mix(wind_hash(i + vec2(0.0, 1.0)), wind_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Repère du vent : x le long du vent (décalé par l'avance des rafales), y en travers.
vec2 wind_frame(vec2 p) {
	vec2 d = normalize(wind_gust_dir + vec2(1e-5, 0.0));
	vec2 q = vec2(dot(p, d), dot(p, vec2(-d.y, d.x)));
	q.x -= TIME * wind_gust_speed;
	return q;
}

// Force de la rafale [0, 1] au point `p` (unités monde). Deux octaves larges, la seconde dérivant
// un peu plus vite (les nappes se déforment en avançant).
float wind_gust(vec2 p) {
	vec2 q = wind_frame(p);
	float n = wind_noise(q / wind_gust_size) * 0.7
		+ wind_noise(q / (wind_gust_size * 0.45) + vec2(TIME * 0.05, 3.7)) * 0.3;
	float edge = 1.0 - wind_gust_share;
	return smoothstep(edge - wind_gust_soft * 0.5, edge + wind_gust_soft * 0.5, n);
}

#endif
