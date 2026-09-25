// Lot CM2 : météo de la carte de campagne (source : cœur, `CampaignSim.get_campaign_weather`,
// ADR 0027). Masque par province (R pluie, G neige, B brouillard, A orage ; intensités 0..1)
// posé par CampaignWeatherView. Inclus par terrain.gdshader (sol mouillé, neige fraîche,
// brouillard matinal, ombre des nuées, éclairs) et campaign_clouds.gdshader (nuées qui passent).
// Nécessite `province_at(vec2)` et `map_size` déclarés avant l'inclusion.

uniform sampler2D weather_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool weather_enabled = false;
// Vent (px carte par seconde) : les nuées défilent dans les régions touchées.
uniform vec2 weather_wind = vec2(3.0, 0.6);
// Brouillard matinal : 1 au début du tour, se lève ensuite (reste dans les vallées).
uniform float weather_fog_morning = 1.0;
// Frange irrégulière des zones météo (px carte) : pas de bord de province net.
uniform float weather_edge_px = 30.0;

float wx_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float wx_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(wx_hash(i), wx_hash(i + vec2(1.0, 0.0)), u.x),
		mix(wx_hash(i + vec2(0.0, 1.0)), wx_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float wx_fbm(vec2 p) {
	return wx_noise(p) * 0.55 + wx_noise(p * 2.1 + 5.2) * 0.3 + wx_noise(p * 4.3 + 1.7) * 0.15;
}

// Météo au point p (px carte) : province lue à une position déformée par le bruit (frange).
vec4 wx_sample(vec2 p) {
	if (!weather_enabled) {
		return vec4(0.0);
	}
	// Trois lectures déformées moyennées : frange large et fondue entre deux provinces.
	vec2 warp = (vec2(wx_fbm(p * 0.011), wx_fbm(p * 0.011 + 7.3)) - 0.5) * weather_edge_px * 2.0;
	vec4 sum = vec4(0.0);
	int width = textureSize(weather_mask, 0).x - 1;
	for (int i = 0; i < 3; i++) {
		float ang = float(i) * 2.094 + 0.5;
		vec2 off = warp + vec2(cos(ang), sin(ang)) * weather_edge_px * 0.8;
		int id = province_at(p + off);
		if (id > 0) {
			sum += texelFetch(weather_mask, ivec2(clamp(id, 0, width), 0), 0);
		}
	}
	return sum / 3.0;
}

// Densité des nuées [0, 1] : masses qui défilent avec le vent dans les zones de pluie, d'orage
// et de neige ; `w` = wx_sample(p).
float wx_cloud(vec2 p, vec4 w) {
	float wet = max(max(w.r, w.g), w.a);
	if (wet <= 0.0) {
		return 0.0;
	}
	vec2 q = (p - TIME * weather_wind) * 0.012;
	float c = wx_fbm(q) * 0.7 + wx_fbm(q * 2.3 + vec2(TIME * 0.01, 0.0)) * 0.3;
	float cover = mix(0.62, 0.35, wet);  // plus il pleut, plus le ciel est couvert
	return smoothstep(cover, cover + 0.22, c) * (0.55 + 0.45 * wet);
}

// Éclair [0, 1] dans une zone d'orage : cellules qui s'illuminent brièvement.
float wx_lightning(vec2 p, vec4 w) {
	if (w.a <= 0.0) {
		return 0.0;
	}
	vec2 cell = floor(p / 90.0);
	float slot = floor(TIME * 1.7 + wx_hash(cell) * 13.0);
	float strike = step(0.985 - 0.02 * w.a, wx_hash(cell + slot * 0.137));
	float t = fract(TIME * 1.7 + wx_hash(cell) * 13.0);
	float flicker = (1.0 - smoothstep(0.0, 0.35, t)) * (0.6 + 0.4 * step(0.5, fract(t * 9.0)));
	float shape = 1.0 - smoothstep(0.0, 70.0, length(p - (cell + 0.5) * 90.0));
	return strike * flicker * shape;
}

// Sol : mouillé sous la pluie, neige fraîche, brouillard matinal (vallées), ombre des nuées,
// lueur des éclairs. `hc` : altitude (m), `emission` reçoit la lueur des éclairs.
vec3 weather_ground(vec3 col, vec2 p, float hc, float slope, inout float roughness, inout vec3 emission) {
	if (!weather_enabled) {
		return col;
	}
	vec4 w = wx_sample(p);
	if (dot(w, vec4(1.0)) <= 0.0) {
		return col;
	}
	float rain = max(w.r, w.a);
	// Sol mouillé : plus sombre, plus lisse (flaques).
	col *= 1.0 - 0.28 * rain;
	roughness = mix(roughness, 0.45, rain * 0.6);
	// Neige fraîche : sur le plat surtout, en plaques au bord.
	if (w.g > 0.0) {
		float flakes = smoothstep(0.45, 0.85, wx_fbm(p * 0.09) * 0.7 + w.g * 0.6);
		float snow = w.g * flakes * (1.0 - smoothstep(0.35, 0.8, slope));
		col = mix(col, vec3(0.80, 0.82, 0.86), snow * 0.7);
	}
	// Brouillard matinal : nappe dans les basses terres, qui se lève au fil du tour.
	if (w.b > 0.0) {
		float low = 1.0 - smoothstep(150.0, 700.0, hc);
		float patches = smoothstep(0.3, 0.75, wx_fbm(p * 0.02 - TIME * weather_wind * 0.004));
		float mist = w.b * mix(0.3, 0.75, low) * mix(0.35, 1.0, weather_fog_morning) * (0.35 + 0.65 * patches);
		col = mix(col, vec3(0.62, 0.64, 0.66), clamp(mist, 0.0, 0.8));
	}
	// Ombre des nuées qui passent.
	col *= 1.0 - 0.35 * wx_cloud(p + weather_wind * 6.0, w);
	emission += vec3(0.55, 0.6, 0.8) * wx_lightning(p, w) * 0.6;
	return col;
}
