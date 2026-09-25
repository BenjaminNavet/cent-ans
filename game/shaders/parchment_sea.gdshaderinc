// Lot CM2 : mer à la manière des portulans (inclus par water.gdshader) : lavis bleu-vert pâle,
// filets d'eau le long des côtes, lignes de rhumbs rayonnant des roses des vents (vents
// principaux à l'encre, demi-vents en vert, quarts en rouge), roses des vents dessinées.
// Centres des roses : StrategicView (points du large les plus éloignés des côtes).
#include "res://shaders/parchment_common.gdshaderinc"

uniform vec4 pm_roses[4];  // x, y (px carte), rayon (px carte), 1 = active
uniform int pm_rose_count = 0;
uniform float pm_rhumb_alpha = 0.55;

// Rose des vents à 16 pointes, pointe nord rouge ; renvoie (couverture, couleur).
vec4 pm_rose(vec2 p, vec4 rose, float footprint) {
	vec2 v = p - rose.xy;
	float r = length(v);
	float R = rose.z;
	if (r > R * 1.05) {
		return vec4(0.0);
	}
	float a = atan(v.x, -v.y);  // 0 = nord (haut de la carte), sens horaire
	float step16 = PI / 8.0;
	float k = round(a / step16);
	float da = a - k * step16;
	int ki = int(mod(k + 16.0, 16.0));
	bool main = ki % 4 == 0;
	bool half_w = ki % 2 == 0 && !main;
	float tip = main ? R * 0.92 : (half_w ? R * 0.66 : R * 0.45);
	float valley = R * 0.12;
	float edge = mix(tip, valley, abs(da) / step16);
	float star = step(r, edge);
	// Deux tons par pointe (moitié claire, moitié encrée).
	vec3 dark = main ? pm_ink * 1.2 : (half_w ? pm_green_ink * 1.3 : pm_red_ink);
	vec3 light = pm_paper * 1.05;
	vec3 col = da > 0.0 ? dark : light;
	if (ki == 0) {
		col = da > 0.0 ? pm_red_ink : pm_red_ink * 2.2;
	}
	float outline = pm_line(r - edge, 0.55, footprint) * step(r, tip + footprint);
	col = mix(col, pm_ink, outline);
	// Anneaux et moyeu.
	float ring = max(pm_line(r - R * 0.98, 0.9, footprint), pm_line(r - R * 0.9, 0.5, footprint));
	float hub = step(r, R * 0.05);
	float cover = max(max(star, outline), max(ring, hub));
	col = hub > 0.0 ? pm_red_ink : col;
	col = ring > star ? pm_ink : col;
	return vec4(col, cover);
}

// Lignes de rhumbs (32 directions) d'une rose : (intensité, couleur).
vec4 pm_rhumbs(vec2 p, vec4 rose, float footprint) {
	vec2 v = p - rose.xy;
	float r = length(v);
	if (r < rose.z * 0.9) {
		return vec4(0.0);
	}
	float a = atan(v.x, -v.y);
	float step32 = PI / 16.0;
	float k = round(a / step32);
	float dist = r * abs(sin(a - k * step32));
	int ki = int(mod(k + 32.0, 32.0));
	vec3 col = ki % 4 == 0 ? pm_ink : (ki % 2 == 0 ? pm_green_ink * 1.6 : pm_red_ink * 1.3);
	float w = ki % 4 == 0 ? 0.6 : 0.45;
	float fade = 1.0 - smoothstep(rose.z * 9.0, rose.z * 14.0, r);
	return vec4(col, pm_line(dist, w, footprint) * fade);
}

// Crochet unique de la mer. `coast` : distance signée à la côte (px carte, < 0 en mer).
void parchment_sea(vec2 p, vec2 uv, float footprint, float coast, float lake_f, inout vec3 col,
		inout float alpha, inout vec3 n, out vec3 emission) {
	emission = vec3(0.0);
	float w = pm_weight();
	if (w <= 0.001) {
		return;
	}
	vec3 parch = pm_paper_at(p, uv, footprint);
	float off = max(-coast, 0.0);
	// Lavis : plus soutenu le long des côtes (pigment au bord), pâle au large.
	float wash = mix(0.42, 0.85, exp(-off / 10.0)) * (0.9 + 0.2 * pm_noise(p * 0.05));
	parch *= mix(vec3(1.0), pm_sea_wash, wash);
	// Filets d'eau : lignes parallèles à la côte, de plus en plus espacées et pâles.
	float lines = 0.0;
	for (int i = 1; i <= 4; i++) {
		float fi = float(i);
		lines = max(lines, pm_line(off - fi * fi * 1.6 - 1.2, 0.45, footprint) * (1.0 - fi * 0.2));
	}
	parch = mix(parch, pm_ink * 4.0 + pm_sea_wash * 0.1, lines * 0.6 * step(0.3, off));
	// Rhumbs puis roses.
	for (int i = 0; i < 4; i++) {
		if (i >= pm_rose_count) {
			break;
		}
		vec4 rh = pm_rhumbs(p, pm_roses[i], footprint);
		parch = mix(parch, rh.rgb, rh.a * pm_rhumb_alpha);
	}
	for (int i = 0; i < 4; i++) {
		if (i >= pm_rose_count) {
			break;
		}
		vec4 ro = pm_rose(p, pm_roses[i], footprint);
		parch = mix(parch, ro.rgb, ro.a);
	}
	parch = mix(parch, pm_ink, pm_line(coast, 1.0, footprint) * 0.9);
	parch = mix(parch, pm_ink, pm_frame(uv, footprint, map_size) * 0.85);
	vec3 albedo;
	pm_output(col, parch, w, albedo, emission);
	col = albedo;
	// Le parchemin est opaque en mer (la côte reste découpée par le raster).
	float sea = 1.0 - smoothstep(-0.2, 0.6, coast);
	alpha = mix(alpha, sea * (1.0 - lake_f), w);
	n = normalize(mix(n, vec3(0.0, 1.0, 0.0), w));
}
