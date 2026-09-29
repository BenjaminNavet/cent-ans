// Lot R1 — relief fin et occupation du sol de la carte de campagne (ADR 0019).
// Inclus par terrain.gdshader (après ses fonctions utilitaires : value_noise, hash12, uniforms
// map_size, lake_color...). Quatre crochets d'une ligne dans terrain.gdshader :
//   rl_relief (gradient du relief), rl_landcover (poids des matériaux), rl_surface (couleur, eau).
// Toutes les entrées sont optionnelles (drapeaux has_*), posées par ReliefLandcover.apply.
//
// - relief_shade (LA8 ou BC5 RG, 2 px par unité) : L = détail d'altitude (Copernicus 90 m moyenné) par rapport à la
//   heightmap de rendu (taille de la carte) filtrée bilinéairement, 128 + m / rl_detail_scale_m ; A = occlusion /
//   courbure multi-échelle (128 neutre, < 128 creux et fonds de vallée, > 128 crêtes).
// - wetlands (RGB8, taille de la carte) : R marais (roselières, mares), G pays d'étangs (Dombes, Brenne...),
//   B prés humides et tourbières (data/map/wetlands.json + fonds de vallée).

uniform sampler2D relief_shade : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform bool has_relief_shade = false;
// OMR-R2 : relief_shade en RG (BC5, R = L, G = A) au lieu de LA8 (occlusion lue en .g, pas en .a).
uniform bool relief_shade_rg = false;
uniform float rl_detail_scale_m = 1.5;     // mètres par niveau du canal L (relief_shade.py)
uniform float rl_detail_normal = 1.0;      // part du détail fin dans la normale d'ombrage
uniform float rl_occlusion = 0.42;         // assombrissement des creux (occlusion = -1)
uniform float rl_ridge_light = 0.14;       // éclaircissement des crêtes (occlusion = +1)
uniform float rl_valley_green = 0.25;      // fonds de vallée plus verts (prés)

uniform sampler2D wetlands : filter_linear, repeat_disable, hint_default_black;
uniform bool has_wetlands = false;
uniform vec3 rl_reed_color = vec3(0.105, 0.115, 0.042);
uniform vec3 rl_wet_meadow = vec3(0.06, 0.12, 0.05);
uniform vec3 rl_marsh_water = vec3(0.04, 0.075, 0.07);
uniform float rl_pond_cell_px = 2.8;       // taille d'une cellule d'étang (pixels carte ≈ 2 km)
uniform float rl_forest_edge = 0.35;       // ondulation des lisières (poids de forêt)

float rl_occ_at(vec2 uv) {
	vec4 t = texture(relief_shade, uv);
	return ((relief_shade_rg ? t.g : t.a) * 255.0 - 128.0) / 127.0;
}

// Crochet 1 : détail du relief sous le pixel de la heightmap de la carte (gradient en m / pixel carte).
void rl_relief(vec2 uv, float footprint, inout vec2 grad_m) {
	if (!has_relief_shade) {
		return;
	}
	// Pas en pixels carte (le raster fait 2 texels par pixel carte) et niveau de mipmap assorti.
	float k = max(footprint * 1.5, 0.5);
	float lod = max(log2(k * 2.0), 0.0);
	vec2 e = vec2(k / map_size.x, 0.0);
	vec2 f = vec2(0.0, k / map_size.y);
	float dl = textureLod(relief_shade, uv - e, lod).r;
	float dr = textureLod(relief_shade, uv + e, lod).r;
	float du = textureLod(relief_shade, uv - f, lod).r;
	float dd = textureLod(relief_shade, uv + f, lod).r;
	grad_m += vec2(dr - dl, dd - du) * 255.0 * rl_detail_scale_m / (2.0 * k) * rl_detail_normal;
}

// Crochet 2 : lisières nettes et ondulées des massifs forestiers (le raster est binaire lissé :
// on seuille autour de 0,5 avec un bruit, trait de plus en plus net en s'approchant).
vec4 rl_landcover(vec4 s, vec2 p, float footprint) {
	float f = s.b;
	if (f <= 0.002 || f >= 0.998) {
		return s;
	}
	float n = value_noise(p * 0.8) * 0.5 + value_noise(p * 2.3 + 7.1) * 0.3 + value_noise(p * 6.9 + 3.3) * 0.2;
	float w = clamp(footprint * 0.3, 0.03, 0.5);
	float fs = smoothstep(0.5 - w, 0.5 + w, f + (n - 0.5) * rl_forest_edge);
	float rest = s.r + s.g + s.a;
	if (rest > 1e-3) {
		float scale = (1.0 - fs) / rest;
		s.r *= scale;
		s.g *= scale;
		s.a *= scale;
	} else {
		s.r = 1.0 - fs;
	}
	s.b = fs;
	return s;
}

// Étang médiéval dans la cellule de `p` : ellipse orientée coupée par une chaussée droite.
// Renvoie la distance signée (unités de cellule, négative dans l'eau) ou 1 s'il n'y a pas d'étang.
float rl_pond_sd(vec2 p, float density) {
	vec2 q = p / rl_pond_cell_px;
	vec2 cell = floor(q);
	vec2 local = fract(q) - 0.5;
	float h = hash12(cell + vec2(3.7, 11.3));
	if (h > density * 0.75) {
		return 1.0;
	}
	vec2 jitter = (vec2(hash12(cell + 1.3), hash12(cell + 5.9)) - 0.5) * 0.28;
	float ang = hash12(cell + 9.1) * 6.2832;
	vec2 dir = vec2(cos(ang), sin(ang));
	vec2 d = local - jitter;
	vec2 r = vec2(dot(d, dir), dot(d, vec2(-dir.y, dir.x)));
	float ra = mix(0.14, 0.34, hash12(cell + 2.2));
	float rb = ra * mix(0.45, 0.85, hash12(cell + 4.4));
	float wob = (value_noise(p * 5.0) - 0.5) * 0.08 + (value_noise(p * 13.0) - 0.5) * 0.03;
	float e = length(r / vec2(ra, rb)) - 1.0;
	float sd = e * rb + wob;
	// Chaussée (digue) : un côté droit en aval.
	sd = max(sd, r.x - ra * 0.55);
	return sd;
}

// Crochet 3 : occlusion du relief, prés humides, marais et étangs. Avant le mélange de l'eau.
void rl_surface(vec2 p, vec2 uv, float footprint, float land_f, inout vec3 col, inout float lake,
		inout vec3 water_col, inout float roughness) {
	if (has_relief_shade) {
		float occ = rl_occ_at(uv);
		float shade = occ < 0.0 ? 1.0 + occ * rl_occlusion : 1.0 + occ * rl_ridge_light;
		col *= mix(1.0, shade, land_f);
		// Fonds de vallée : prés plus verts.
		float valley = clamp(-occ * 1.6, 0.0, 1.0) * land_f;
		col = mix(col, col * vec3(0.82, 1.04, 0.78), valley * rl_valley_green);
	}
	if (!has_wetlands) {
		return;
	}
	vec3 wl = texture(wetlands, uv).rgb * land_f;
	if (wl.r + wl.g + wl.b < 0.01) {
		return;
	}
	float aa = max(footprint, 1e-3);
	// Prés humides et tourbières : herbe plus sombre et plus fraîche, mouchetée.
	float moss = value_noise(p * 3.7);
	col = mix(col, rl_wet_meadow * (0.8 + 0.4 * moss), wl.b * 0.55);
	// Marais : roselières (teinte paille, bandes) et mares d'eau libre en taches.
	float water = 0.0;
	if (wl.r > 0.01) {
		float reeds = value_noise(p * vec2(5.0, 1.7)) * 0.6 + value_noise(p * 13.0) * 0.4;
		col = mix(col, rl_reed_color * (0.75 + 0.5 * reeds), smoothstep(0.0, 0.5, wl.r) * 0.85);
		float n = value_noise(p * 1.9) * 0.5 + value_noise(p * 5.3 + 5.0) * 0.33 + value_noise(p * 14.1) * 0.17;
		float t = 1.0 - 0.36 * wl.r;
		float fw = max(fwidth(n), 0.015);
		water = max(water, smoothstep(t - fw, t + fw, n));
	}
	// Pays d'étangs : étangs de ~200-500 m ; au dézoom, leur couverture moyenne.
	if (wl.g > 0.01) {
		float sd = rl_pond_sd(p, wl.g);
		float px = aa / rl_pond_cell_px;
		float edge = clamp(fwidth(sd), 1e-4, px) * 0.8;
		float pond = 1.0 - smoothstep(-edge, edge, sd);
		float bank = (1.0 - smoothstep(0.0, 0.05 + edge, sd)) * (1.0 - pond);
		float vis = smoothstep(2.0, 6.0, 1.0 / max(px, 1e-3));
		float coverage = wl.g * 0.75 * 0.25 * 2.0;  // renforcé : lisible au dézoom
		pond = mix(coverage, pond, vis);
		col = mix(col, rl_reed_color, bank * vis * 0.8);
		water = max(water, pond);
	}
	water *= land_f;
	float before = lake;
	lake = max(lake, water);
	vec3 pond_col = mix(rl_marsh_water, lake_color, 0.5);
	water_col = mix(water_col, pond_col, clamp(water - before, 0.0, 1.0));
	roughness = mix(roughness, 0.95, wl.r * 0.3);
}
