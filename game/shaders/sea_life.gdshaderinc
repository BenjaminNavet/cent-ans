// ME1 (chantier DN, mer vivante) : fonds clairs et caustiques, crêtes du large, saturation bornée,
// estrans (marée) et liseré de ressac lisible au dézoom. Inclus par `water.gdshader` après
// `value_noise` ; réglé par `SeaLife.apply()` depuis `data/fx/sea_life.json`.
// `sea_life_on` = false (`--no-sea-life`, ou pas de données) : rendu inchangé.
uniform bool sea_life_on = false;
uniform float sea_gain = 1.0;                 // luminosité de la mer
uniform float sea_cloud_shadow_gain = 1.0;    // part de l'ombre des cumulus gardée sur l'eau
uniform float sea_sat_max = 1.0;              // saturation maximale de la couleur de mer (bible D5)
uniform vec3 shallows_color = vec3(0.2, 0.4, 0.38);
uniform float shallows_depth_m = 40.0;
uniform float shallows_amount = 0.0;
uniform float shallows_caustic = 0.0;
uniform float shallows_caustic_scale = 0.9;
uniform vec3 crest_color = vec3(0.46, 0.55, 0.58);
uniform float crest_amount = 0.0;
uniform vec2 crest_depth_m = vec2(40.0, 140.0);
uniform float crest_frequency = 2.4;
uniform float crest_speed = 0.55;
uniform vec2 crest_dir = vec2(0.8, 0.6);
uniform float surf_min_screen_px = 0.0;       // largeur minimale à l'écran du liseré d'écume
uniform sampler2D tide_zones : filter_linear, repeat_disable, hint_default_black;
uniform float tide_period_s = 150.0;
uniform float tide_max_px = 8.0;
uniform float tide_high_cover = 0.9;
uniform vec3 tide_mud_color = vec3(0.13, 0.115, 0.085);
uniform vec3 tide_wet_color = vec3(0.22, 0.22, 0.19);
uniform float tide_mud_alpha = 0.96;

// Borne la saturation (HSV) de c à `sea_sat_max` en gardant la valeur.
vec3 sea_limit_saturation(vec3 c) {
	float mx = max(c.r, max(c.g, c.b));
	float mn = min(c.r, min(c.g, c.b));
	float sat = (mx - mn) / max(mx, 1e-5);
	float k = min(1.0, sea_sat_max / max(sat, 1e-5));
	return mix(vec3(mx), c, k);
}

// Fonds clairs près des côtes (turquoise grisé, fondu dans la profondeur) et caustiques animées.
vec3 sea_shallows(vec3 col, vec2 p, float depth, float footprint) {
	if (shallows_amount <= 0.0) {
		return col;
	}
	float g = (1.0 - smoothstep(shallows_depth_m * 0.1, shallows_depth_m, depth)) * (1.0 - 0.5 * smoothstep(1.0, 4.0, footprint));
	vec2 q = p * shallows_caustic_scale;
	float ridge = 1.0 - abs(value_noise(q + vec2(TIME * 0.11, TIME * 0.07)) * 2.0 - 1.0);
	float ridge2 = 1.0 - abs(value_noise(q * 1.7 - vec2(TIME * 0.09, -TIME * 0.13) + 9.0) * 2.0 - 1.0);
	float caustic = pow(ridge * ridge2, 4.0) * (1.0 - smoothstep(0.5, 2.0, footprint));
	col = mix(col, shallows_color * (0.55 + 0.45 * value_noise(p * 0.07)), g * shallows_amount);
	return col + vec3(0.6, 0.85, 0.8) * caustic * g * shallows_caustic;
}

// Lignes de crêtes qui avancent au large (0..1), éteintes sur les petits fonds et au dézoom.
float sea_crests(vec2 p, float depth, float footprint) {
	if (crest_amount <= 0.0) {
		return 0.0;
	}
	float deep = smoothstep(crest_depth_m.x, crest_depth_m.y, depth);
	float fade = 1.0 - smoothstep(0.8, 3.5, footprint);
	float phase = dot(p, normalize(crest_dir)) * crest_frequency - TIME * crest_speed + value_noise(p * 0.3) * 6.0;
	float line = smoothstep(0.8, 1.0, sin(phase) * 0.5 + 0.5);
	float patch_mask = smoothstep(0.45, 0.8, value_noise(p * 0.55 + vec2(TIME * 0.02, 0.0)));
	return line * patch_mask * deep * fade * crest_amount;
}

// Estran : x = vase découverte [0, 1], y = lisière humide brillante, au large de la côte
// (`off` : distance en px carte). Marée lente (`tide_period_s`) ; chenaux irréguliers.
vec2 sea_tide_flat(vec2 uv, vec2 p, float off) {
	float zone = texture(tide_zones, uv).r;
	if (zone <= 0.004 || off <= 0.0) {
		return vec2(0.0);
	}
	float tide = 0.5 + 0.5 * sin(TIME * 6.2832 / tide_period_s);  // 1 = pleine mer
	float exposed = zone * tide_max_px * (1.0 - tide * tide_high_cover);
	exposed *= 0.7 + 0.6 * value_noise(p * 0.55 + 3.0);
	float mud = 1.0 - smoothstep(exposed - 0.35, exposed, off);
	float edge = smoothstep(exposed - 0.9, exposed - 0.2, off) * (1.0 - smoothstep(exposed - 0.2, exposed + 0.1, off));
	return vec2(mud, edge);
}
