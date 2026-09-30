// Lot SS2 (ADR 0142) — sol « satellite » : carte de couleur précalculée (`cent-ans geo colormap`,
// BC1 14336×12288 ≈ 360 m/texel, mipmaps) comme couleur de base en vue large et moyenne.
// Inclus par terrain.gdshader ; un crochet d'une ligne après le parcellaire fin (fp_parcels).
// De près, le procédural (parcelles, sillons, haies) garde son dessin : la carte n'en donne plus
// que la teinte (part `sg_near_keep`), sa luminance module encore la couleur.
// Posé par ReliefLandcover.apply (`has_colormap` faux sans copie GPU : ancien rendu).

uniform sampler2D colormap : source_color, filter_linear_mipmap_anisotropic, repeat_disable, hint_default_black;
uniform bool has_colormap = false;
uniform float sg_strength = 1.0;       // part de la carte en vue large et moyenne
uniform float sg_near_keep = 0.45;     // part de la carte au plus près (le procédural dessine)
uniform float sg_near_footprint = 0.05;  // empreinte (px carte / px écran) du palier « près »
uniform float sg_far_footprint = 0.5;  // au-delà : carte seule (un texel ≥ un pixel écran)
uniform float sg_detail_lum = 0.55;    // part de la luminance procédurale gardée par-dessus la carte

float sg_luma(vec3 c) {
	return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// `col` : couleur procédurale (albédo linéaire) ; `snow` : poids de neige (la carte est une saison
// moyenne, la neige saisonnière reste au-dessus) ; `land_f` : 1 à terre.
void sg_apply(vec2 uv, float footprint, float snow, float land_f, inout vec3 col) {
	if (!has_colormap || sg_strength <= 0.0) {
		return;
	}
	vec3 cm = texture(colormap, uv).rgb;
	float far_w = smoothstep(sg_near_footprint, sg_far_footprint, footprint);
	float w = sg_strength * mix(sg_near_keep, 1.0, far_w) * (1.0 - snow) * land_f;
	// Luminance du détail procédural, rapportée à sa teinte : grain et parcelles restent lisibles
	// sur la teinte de la carte, d'autant plus qu'on est près.
	float ratio = clamp(sg_luma(col) / max(sg_luma(cm), 1e-3), 0.55, 1.6);
	vec3 tinted = cm * mix(1.0, ratio, sg_detail_lum * (1.0 - far_w));
	col = mix(col, tinted, w);
}
