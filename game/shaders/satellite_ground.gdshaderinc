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
uniform float sg_season_strength = 1.3;  // TB1 : force de l'écart saisonnier posé sur la carte (0 : carte d'une seule saison)

// TB6 (retouche) : massifs sombres de la carte (forêts d'Orléans, de Fontainebleau…) : presque
// noirs à bord net en vue moyenne et large. `sg_dark_floor` : luminance linéaire plancher (0 :
// carte inchangée) ; au-dessus de `sg_dark_floor * sg_dark_knee`, rien ne change. Le masque vient
// d'une lecture floutée de la carte (`sg_dark_feather` niveaux de mip en plus) : lisière fondue sur
// quelques pixels de carte. `sg_dark_keep` : part du contraste d'origine gardée sous le plancher ;
// `sg_dark_canopy` : modelé de canopée. Bloc `forest_masses` de `data/ui/campaign_map.json`.
uniform float sg_dark_floor = 0.0;
uniform float sg_dark_knee = 2.0;
uniform float sg_dark_feather = 2.5;
uniform float sg_dark_keep = 0.25;
uniform float sg_dark_canopy = 0.12;

float sg_luma(vec3 c) {
	return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// `col` : couleur procédurale (albédo linéaire) ; `snow` : poids de neige (la carte est une saison
// moyenne, la neige saisonnière reste au-dessus) ; `land_f` : 1 à terre ; `season_k` : écart
// saisonnier multiplicatif (saison / été) de l'occupation du sol au pixel (TB1) : la carte donne
// la couleur moyenne, la saison la déplace au lieu d'être effacée par elle.
void sg_apply(vec2 uv, float footprint, float snow, float land_f, vec3 season_k, inout vec3 col) {
	if (!has_colormap || sg_strength <= 0.0) {
		return;
	}
	vec3 cm = texture(colormap, uv).rgb;
	if (sg_dark_floor > 0.0) {  // TB6 : massifs sombres éclaircis, lisière fondue, modelé de canopée
		vec3 soft = texture(colormap, uv, sg_dark_feather).rgb;
		float dark = 1.0 - smoothstep(sg_dark_floor, sg_dark_floor * sg_dark_knee, sg_luma(soft));
		if (dark > 0.0) {
			cm = mix(cm, soft, dark * 0.6);
			float l = max(sg_luma(cm), 1e-3);
			float lifted = max(l, mix(sg_dark_floor, l, sg_dark_keep));
			vec2 q = uv * map_size;
			// Modelé de canopée : trois octaves, les fines s'éteignent au dézoom (pas de scintillement),
			// la plus large (≈ 4,5 px carte) reste en vue large.
			float crowns = (value_noise(q * 0.22 + 7.9) - 0.5) * 0.9
				+ (value_noise(q * 0.9) - 0.5) * 0.7 * (1.0 - smoothstep(0.4, 1.0, footprint))
				+ (value_noise(q * 2.7 + 3.1) - 0.5) * 0.5 * (1.0 - smoothstep(0.15, 0.4, footprint));
			cm *= mix(1.0, lifted / l * (1.0 + sg_dark_canopy * crowns * 2.0), dark);
		}
	}
	float far_w = smoothstep(sg_near_footprint, sg_far_footprint, footprint);
	float w = sg_strength * mix(sg_near_keep, 1.0, far_w) * (1.0 - snow) * land_f;
	// Luminance du détail procédural, rapportée à sa teinte : grain et parcelles restent lisibles
	// sur la teinte de la carte, d'autant plus qu'on est près.
	float ratio = clamp(sg_luma(col) / max(sg_luma(cm), 1e-3), 0.55, 1.6);
	vec3 tinted = cm * mix(1.0, ratio, sg_detail_lum * (1.0 - far_w));
	tinted *= pow(season_k, vec3(sg_season_strength));
	col = mix(col, tinted, w);
}
