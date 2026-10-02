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
