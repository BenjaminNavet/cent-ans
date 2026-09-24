shader_type spatial;

// Lot BR1 (ADR 0021) : matériau unique des maquettes de campagne du kit de bâtiments.
// Toutes les matières (enduit, moellons, pierre, bois, tuiles, ardoise, chaume, vitres…) sont
// les couches d'un Texture2DArray ; la couche d'une face est codée dans l'alpha de la couleur
// de sommet ((indice + 0,5) / 16), le RVB portant teinte du bâtiment × occlusion × patine.
// Un seul appel de dessin par maquette (ou par MultiMesh de hameaux) au lieu d'un par matière.
// Table des couches : `BuildingMaterials.ATLAS_LAYERS` (même ordre que `kit_campaign.LAYERS`).

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
// rgb = teinte de la matière, a = rugosité.
uniform vec4 layer_tint[16];
// Répétitions par mètre (1 / taille de tuile de la texture).
uniform float layer_tile[16];
// Première couche unie (sans texture) : vitres, fer, toile.
uniform int first_plain = 11;
// Neige sur les toits (0 = aucune) : couches `roof_first`..`roof_last`.
uniform float snow = 0.0;
uniform int roof_first = 7;
uniform int roof_last = 10;

varying flat int layer;

void vertex() {
	layer = clamp(int(COLOR.a * 16.0), 0, 15);
}

void fragment() {
	vec4 tint = layer_tint[layer];
	vec3 base = vec3(1.0);
	if (layer < first_plain) {
		base = texture(albedo_array, vec3(UV * layer_tile[layer], float(layer))).rgb;
	}
	vec3 albedo = base * tint.rgb * COLOR.rgb;
	if (snow > 0.0 && layer >= roof_first && layer <= roof_last) {
		albedo = mix(albedo, vec3(0.9, 0.92, 0.97) * (0.85 + 0.15 * base.r), snow);
	}
	ALBEDO = albedo;
	ROUGHNESS = tint.a;
}
