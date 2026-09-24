// Lot R1 — relief fin et occupation du sol de la carte de campagne (ADR 0019).
// Inclus par terrain.gdshader ; toutes les entrées sont optionnelles (drapeaux has_*).
//
// - relief_shade (LA8, 8192² en général) : L = détail d'altitude (Copernicus 90 m moyenné) par
//   rapport à la heightmap de rendu, 128 + m / rl_detail_scale_m ; A = occlusion/courbure
//   multi-échelle (128 neutre, < 128 creux et fonds de vallée, > 128 crêtes).
// - wetlands (RGB8) : R marais (roselières, eau stagnante), G étangs (Dombes, Sologne, Brenne...),
//   B prés humides et tourbières. Données : data/map/wetlands.json (cent-ans geo landcover).

uniform sampler2D relief_shade : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform bool has_relief_shade = false;
uniform float rl_detail_scale_m = 2.0;
uniform float rl_detail_normal = 1.0;
uniform float rl_occlusion = 0.55;
uniform float rl_ridge_light = 0.25;

uniform sampler2D wetlands : filter_linear, repeat_disable, hint_default_black;
uniform bool has_wetlands = false;

// Crochet 1 : normale et teinte de relief fin (après le calcul du gradient de la heightmap).
void rl_relief(vec2 uv, float footprint, inout vec2 grad_m) {
}

// Crochet 2 : poids des matériaux (lisières nettes des forêts, clairières).
vec4 rl_landcover(vec4 s, vec2 p, float footprint) {
	return s;
}

// Crochet 3 : ombrage d'occlusion, marais, étangs et prés humides.
void rl_surface(vec2 p, vec2 uv, float footprint, float land_f, inout vec3 col, inout float lake,
		inout vec3 water_col, inout float roughness) {
}
