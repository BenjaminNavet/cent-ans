// Lot TB5 (mer et côtes) : mers différenciées par bassin. Inclus par `water.gdshader` après
// `value_noise`. `sea_basins` : poids de quatre bassins au plus (R, V, B, A ; limites fondues),
// cuits par `SeaBasins` depuis `data/map/sea_basins.json` ; les réglages sont des tableaux dont
// l'indice 0 est la mer par défaut (hors bassin). `basin_on` = false (pas de données) : rendu
// inchangé.
uniform sampler2D sea_basins : filter_linear, repeat_disable, hint_default_black;
uniform bool basin_on = false;
uniform vec3 basin_tint[5];    // teinte multipliée à l'eau
uniform vec4 basin_wave[5];    // x clapot, y longue houle, z écume du rivage, w moutons permanents
uniform vec2 basin_water[5];   // x clarté, y part du gris de saison
uniform float long_swell_length_px = 3.5;
uniform float long_swell_speed = 0.5;
uniform float long_swell_contrast = 0.12;
uniform float long_swell_normal = 0.12;

// Réglages de la mer au point uv, mêlés selon les poids des bassins.
void basin_look(vec2 uv, out vec3 tint, out vec4 wave, out vec2 water) {
	tint = vec3(1.0);
	wave = vec4(1.0, 0.0, 1.0, 0.0);
	water = vec2(1.0);
	if (!basin_on) {
		return;
	}
	vec4 w = clamp(textureLod(sea_basins, clamp(uv, vec2(0.0), vec2(1.0)), 0.0), 0.0, 1.0);
	float rest = max(1.0 - (w.r + w.g + w.b + w.a), 0.0);
	float total = rest + w.r + w.g + w.b + w.a;
	tint = (basin_tint[0] * rest + basin_tint[1] * w.r + basin_tint[2] * w.g + basin_tint[3] * w.b + basin_tint[4] * w.a) / total;
	wave = (basin_wave[0] * rest + basin_wave[1] * w.r + basin_wave[2] * w.g + basin_wave[3] * w.b + basin_wave[4] * w.a) / total;
	water = (basin_water[0] * rest + basin_water[1] * w.r + basin_water[2] * w.g + basin_water[3] * w.b + basin_water[4] * w.a) / total;
}

// Longue houle du large : deux trains d'ondes lents venus de l'ouest, phase déformée par un bruit
// lent (crêtes non rectilignes), estompés quand la longueur d'onde passe sous quelques pixels
// écran. Rend xy = pente (repère carte), z = hauteur [-1, 1].
vec3 long_swell(vec2 p, float footprint, float amount) {
	if (amount <= 0.01) {
		return vec3(0.0);
	}
	float fade = 1.0 - smoothstep(long_swell_length_px * 0.12, long_swell_length_px * 0.4, footprint);
	if (fade <= 0.0) {
		return vec3(0.0);
	}
	float k = 6.2832 / long_swell_length_px;
	float t = TIME * long_swell_speed;
	vec2 d1 = vec2(0.87, 0.5);
	vec2 d2 = vec2(0.64, -0.77);
	float ph1 = dot(p, d1) * k - t + value_noise(p * 0.07) * 4.0;
	float ph2 = dot(p, d2) * k * 0.61 - t * 0.7 + value_noise(p * 0.045 + vec2(7.0, 3.0)) * 4.0;
	float h = sin(ph1) * 0.62 + sin(ph2) * 0.38;
	vec2 slope = d1 * cos(ph1) * 0.62 + d2 * cos(ph2) * 0.38;
	return vec3(slope, h) * amount * fade;
}

// Moutons (TB5, retouche) : crêtes d'écume rares et irrégulières, allongées dans le sens du vent.
// Cellules de Voronoï à gigue pleine dans un domaine tourné (vent d'ouest-sud-ouest), étiré et
// déformé par un bruit lent : aucun alignement en lignes ou en colonnes. Seule une part `amount`
// des cellules porte une crête, par rafales (masque lent) ; chaque crête déferle puis s'efface et
// renaît ailleurs au cycle suivant. Bord lissé sur la taille d'un pixel écran (`footprint`, soit
// le `fwidth` du domaine). Au-delà de `whitecap_fade` (empreinte d'un pixel écran en px carte),
// plus rien de ponctuel : il ne reste qu'un voile de teinte très doux (y).
uniform float whitecap_cells_per_px = 1.6;   // cellules par pixel carte, en travers du vent
uniform float whitecap_stretch = 2.5;        // allongement des cellules dans le sens du vent
uniform vec2 whitecap_fade = vec2(0.062, 0.084);
uniform float whitecap_veil = 0.03;

vec2 whitecap_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.xx + p3.yz) * p3.zy);
}

// x = crêtes [0, 1], y = voile de teinte [0, 1].
vec2 sea_whitecaps(vec2 p, float footprint, float amount) {
	if (amount <= 0.005) {
		return vec2(0.0);
	}
	// Repère du vent : x dans le sens du vent, y en travers.
	vec2 w = vec2(dot(p, vec2(0.94, -0.34)), dot(p, vec2(0.34, 0.94)));
	float gust = smoothstep(0.3, 0.75, value_noise(w * vec2(0.09, 0.17) - vec2(TIME * 0.02, 0.0)));
	float near = 1.0 - smoothstep(whitecap_fade.x, whitecap_fade.y, footprint);
	vec2 result = vec2(0.0, amount * gust * (1.0 - near) * whitecap_veil);
	if (near <= 0.0 || gust <= 0.0) {
		return result;
	}
	vec2 warp = vec2(value_noise(p * 0.41 + vec2(5.2, 1.3)), value_noise(p * 0.41 + vec2(31.7, 9.1))) - 0.5;
	vec2 q = (w + warp * 2.4) * whitecap_cells_per_px * vec2(1.0 / whitecap_stretch, 1.0);
	q.x -= TIME * 0.03;
	vec2 cell = floor(q);
	vec2 f = fract(q);
	float ragged = (value_noise(p * 7.3) - 0.5) * 0.6;
	float px = footprint * whitecap_cells_per_px;  // un pixel écran, en cellules (en travers)
	float crest = 0.0;
	for (int j = -1; j <= 1; j++) {
		for (int i = -1; i <= 1; i++) {
			vec2 g = vec2(float(i), float(j));
			vec2 id = cell + g;
			float phase = hash12(id * 1.37 + 17.3);
			float clock = TIME * 0.09 * (0.7 + 0.6 * phase) + phase * 37.0;
			vec2 h = whitecap_hash(id + floor(clock) * vec2(7.1, 3.3));  // nouvelle crête à chaque cycle
			float lot = hash12(id + floor(clock) * vec2(2.7, 9.4) + 3.1);
			float life = fract(clock);
			float pulse = smoothstep(0.0, 0.15, life) * (1.0 - smoothstep(0.3, 0.85, life));
			float len = mix(0.22, 0.46, h.x);
			float wid = mix(0.05, 0.11, lot / max(amount * gust, 1e-3));
			vec2 d = (g + h - f) / vec2(len, wid);
			float e = length(d) + ragged;
			float aa = clamp(px / wid, 0.04, 1.0);
			float shape = (1.0 - smoothstep(1.0 - aa, 1.0 + aa, e)) * min(wid / max(px, 1e-4), 1.0);
			crest = max(crest, shape * pulse * step(lot, amount * gust));
		}
	}
	result.x = crest * near;
	return result;
}
