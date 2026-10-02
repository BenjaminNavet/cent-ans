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
uniform float rl_pond_cell_px = 2.8;       // taille d'une cellule d'étang (pixels carte ≈ 2 km), palier 0
// HC2 (ADR 0161 §3) : étangs et mares généralisés. Au dézoom, les cellules d'étangs doublent par
// paliers d'empreinte (comme les octaves de canopée du terrain) : une zone d'étangs montre toujours
// des nappes distinctes d'au moins `rl_pond_min_px` pixels au lieu de se fondre en teinte moyenne.
// Tailles en pixels d'un écran de `rl_water_ref_height` de haut (même cadrage à toute résolution ;
// 0 = pixels écran bruts).
uniform float rl_water_ref_height = 1080.0;
uniform float rl_pond_min_px = 5.0;        // plus petit diamètre d'un étang (px de référence, moyenne des deux axes écran)
uniform float rl_pond_max_cell_px = 22.4;  // cellule du dernier palier (px carte, cellule × 2^n) ; au-delà, couverture moyenne
uniform float rl_pond_fill = 0.8;          // part des cellules portant un étang, × densité de la zone
uniform float rl_pond_bank = 0.05;         // largeur de la berge (part de cellule ; au moins 1 px écran)
uniform float rl_pond_bank_amount = 0.75;  // opacité de la berge (couleur `lake_bank_color`)
uniform float rl_pond_far_cover = 0.35;    // couverture moyenne au-delà du dernier palier (× densité)
uniform float rl_pool_min_px = 8.0;        // plus petite mare de marais (px de référence)
uniform float rl_pool_amount = 0.45;       // part d'eau libre d'un marais dense (roselière dominante)
uniform float rl_pool_opacity = 0.85;      // opacité des mares
uniform float rl_pool_max_scale = 16.0;    // grossissement maximal des mares
uniform float rl_pool_lake_tint = 0.85;    // 0 = eau de marais (`rl_marsh_water`), 1 = eau des lacs
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

// Plus petit diamètre d'un étang, en part de cellule (voir `rl_pond_sd`).
const float RL_POND_MIN_SPAN = 0.22;

// Étang médiéval dans la cellule de `p` : ellipse orientée coupée par une chaussée droite.
// `cell_px` : taille de la cellule du palier (px carte) ; la densité de la zone est lue autour du
// centre de la cellule (un étang grossi n'est pas rogné par la limite de la zone).
// Renvoie la distance signée (unités de cellule, négative dans l'eau) ou 1 s'il n'y a pas d'étang.
float rl_pond_sd(vec2 p, float cell_px) {
	vec2 q = p / cell_px;
	// Rangées décalées au hasard : pas de trame carrée d'étangs alignés.
	float shift = hash12(vec2(floor(q.y) + 0.37, cell_px * 1.91));
	q.x += shift;
	vec2 cell = floor(q);
	vec2 local = fract(q) - 0.5;
	vec2 key = cell + vec2(cell_px * 7.31, cell_px * 3.17);  // semis propre à chaque palier
	// Plus faible densité aux quatre coins de l'emprise possible de l'étang : pas d'étang à cheval
	// sur la limite de la zone.
	vec2 mid = (cell + vec2(0.5 - shift, 0.5)) * cell_px / map_size;
	vec2 reach = vec2(0.18 * cell_px) / map_size;
	float density = min(
		min(textureLod(wetlands, mid + reach, 0.0).g, textureLod(wetlands, mid - reach, 0.0).g),
		min(textureLod(wetlands, mid + vec2(reach.x, -reach.y), 0.0).g, textureLod(wetlands, mid + vec2(-reach.x, reach.y), 0.0).g));
	float h = hash12(key + vec2(3.7, 11.3));
	if (h > density * rl_pond_fill) {
		return 1.0;
	}
	vec2 jitter = (vec2(hash12(key + 1.3), hash12(key + 5.9)) - 0.5) * 0.28;
	float ang = hash12(key + 9.1) * 6.2832;
	vec2 dir = vec2(cos(ang), sin(ang));
	vec2 d = local - jitter;
	vec2 r = vec2(dot(d, dir), dot(d, vec2(-dir.y, dir.x)));
	// Petit axe d'au moins 0,22 cellule (`RL_POND_MIN_SPAN`), grand axe d'au plus 0,70.
	float ra = mix(0.2, 0.35, hash12(key + 2.2));
	float rb = ra * mix(0.55, 0.9, hash12(key + 4.4));
	float wob = (value_noise(q * 14.0) - 0.5) * 0.07 + (value_noise(q * 36.0) - 0.5) * 0.025;
	float e = length(r / vec2(ra, rb)) - 1.0;
	float sd = e * rb + wob;
	// Chaussée (digue) : un côté droit en aval.
	sd = max(sd, r.x - ra * 0.55);
	return sd;
}

// HC2 : étangs d'un palier (cellule de `cell_px`). Renvoie (eau, berge, eau peu profonde).
vec3 rl_pond_layer(vec2 p, float cell_px, float footprint) {
	float sd = rl_pond_sd(p, cell_px);
	if (sd >= 1.0) {
		return vec3(0.0);
	}
	float edge = clamp(0.7 * footprint / cell_px, 1e-4, 0.2);
	float pond = 1.0 - smoothstep(-edge, edge, sd);
	float bank_w = max(rl_pond_bank, 1.2 * footprint / cell_px);
	float bank = (1.0 - smoothstep(0.0, bank_w + edge, sd)) * (1.0 - pond);
	float shallow = smoothstep(-0.07, 0.0, sd) * pond;
	return vec3(pond, bank, shallow);
}

// HC2 : palier continu d'une échelle doublée : la taille voulue vaut `wanted` fois la taille de
// base. Renvoie (palier bas, part du palier suivant) ; le palier bas est toujours ≥ la taille voulue
// (fondu vers le suivant sur les derniers 15 % du palier).
vec2 rl_tier(float wanted, float max_tier) {
	float k = clamp(log2(max(wanted, 1e-4)), -1.0, floor(max_tier + 0.01) - 1.0);
	return vec2(floor(k) + 1.0, smoothstep(0.85, 1.0, fract(k)));
}

float rl_pool_noise(vec2 p) {
	return value_noise(p * 1.9) * 0.5 + value_noise(p * 5.3 + 5.0) * 0.33 + value_noise(p * 14.1) * 0.17;
}

// Crochet 3 : occlusion du relief, prés humides, marais et étangs. Avant le mélange de l'eau.
// `viewport_h` : hauteur de l'écran en pixels (VIEWPORT_SIZE.y, lisible seulement dans fragment()).
void rl_surface(vec2 p, vec2 uv, float footprint, float viewport_h, float land_f, inout vec3 col, inout float lake,
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
	// HC2 : empreinte en pixels de référence (écran de `rl_water_ref_height` de haut), moyenne
	// géométrique des deux axes écran (la vue inclinée écrase l'axe nord-sud : surface écran égale).
	float ref_fp = sqrt(max(length(dFdx(p)) * length(dFdy(p)), 1e-6))
		* (rl_water_ref_height > 0.0 ? viewport_h / rl_water_ref_height : 1.0);
	// Prés humides et tourbières : herbe plus sombre et plus fraîche, mouchetée.
	float moss = value_noise(p * 3.7);
	col = mix(col, rl_wet_meadow * (0.8 + 0.4 * moss), wl.b * 0.55);
	// Marais : roselières (teinte paille, bandes) et mares d'eau libre en taches. HC2 : les mares
	// grossissent par paliers au dézoom (au moins `rl_pool_min_px`), la roselière domine.
	float pool = 0.0;
	if (wl.r > 0.01) {
		float reeds = value_noise(p * vec2(5.0, 1.7)) * 0.6 + value_noise(p * 13.0) * 0.4;
		col = mix(col, rl_reed_color * (0.75 + 0.5 * reeds), smoothstep(0.0, 0.5, wl.r) * 0.85);
		// Une mare du palier 0 fait ≈ 0,35 px carte.
		vec2 tier = rl_tier(ref_fp * rl_pool_min_px / 0.35, log2(max(rl_pool_max_scale, 1.0)));
		float t = 1.0 - rl_pool_amount * wl.r;
		float s0 = exp2(tier.x);
		float fw0 = clamp(aa * 2.2 / s0, 0.015, 0.25);
		pool = smoothstep(t - fw0, t + fw0, rl_pool_noise(p / s0));
		if (tier.y > 0.0) {
			float s1 = s0 * 2.0;
			float fw1 = clamp(aa * 2.2 / s1, 0.015, 0.25);
			pool = mix(pool, smoothstep(t - fw1, t + fw1, rl_pool_noise(p / s1 + 17.3)), tier.y);
		}
		pool *= rl_pool_opacity;
	}
	// Pays d'étangs : étangs de ~200-500 m de près ; au dézoom, étangs généralisés (HC2) : cellules
	// doublées par paliers, puis couverture moyenne au-delà du dernier palier.
	float pond = 0.0;
	float pond_shallow = 0.0;
	if (wl.g > 0.01) {
		float max_tier = log2(max(rl_pond_max_cell_px / rl_pond_cell_px, 1.0));
		vec2 tier = rl_tier(ref_fp * rl_pond_min_px / (RL_POND_MIN_SPAN * rl_pond_cell_px), max_tier);
		float cell0 = rl_pond_cell_px * exp2(tier.x);
		vec3 layer = rl_pond_layer(p, cell0, aa);
		if (tier.y > 0.0) {
			layer = mix(layer, rl_pond_layer(p, cell0 * 2.0, aa), tier.y);
		}
		// Au-delà du dernier palier, les étangs rapetissent : couverture moyenne quand le plus petit
		// passe sous 70 % puis 30 % de `rl_pond_min_px`.
		float vis = smoothstep(0.3, 0.7, RL_POND_MIN_SPAN * rl_pond_max_cell_px / max(ref_fp * rl_pond_min_px, 1e-4));
		pond = mix(wl.g * rl_pond_far_cover, layer.x, vis);
		pond_shallow = layer.z * vis;
		col = mix(col, lake_bank_color, layer.y * vis * rl_pond_bank_amount * land_f);
	}
	float water = max(pool, pond) * land_f;
	float before = lake;
	lake = max(lake, water);
	vec3 pool_col = mix(rl_marsh_water, lake_color, rl_pool_lake_tint);
	vec3 pond_col = mix(lake_color, lake_shallow_color, pond_shallow);
	vec3 wet_col = mix(pool_col, pond_col, pond >= pool ? 1.0 : 0.0);
	water_col = mix(water_col, wet_col, clamp(water - before, 0.0, 1.0));
	roughness = mix(roughness, 0.95, wl.r * 0.3);
}
