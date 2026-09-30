shader_type spatial;

// Lot BR1 (ADR 0021) : matériau unique des bâtiments du kit (maquettes de campagne, batailles).
// Toutes les matières (enduit, moellons, pierre, bois, tuiles, ardoise, chaume, vitres…) sont
// les couches d'un Texture2DArray ; la couche d'une face est codée dans l'alpha de la couleur
// de sommet ((indice + 0,5) / 16), le RVB portant teinte du bâtiment × occlusion × patine.
// Un seul appel de dessin par maquette (ou par MultiMesh de hameaux, ou par modèle de bataille)
// au lieu d'un par matière.
// Table des couches : `BuildingMaterials.ATLAS_LAYERS` (même ordre que `kit_export.ATLAS_LAYERS`).

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
// Cartes normales (batailles) ; la campagne s'en passe (`use_normals` faux, lecture en moins).
uniform sampler2DArray normal_array : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool use_normals = false;
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
// Lot SR5 : usure procédurale (`building_aging.gdshaderinc`) : boue et coulures sur les murs,
// toits assombris et moussus ; 0 = rendu d'avant SR5 (`--no-sr5`). Repère local du modèle du
// kit en mètres, pied du bâtiment à y = 0.
uniform float aging : hint_range(0.0, 2.0) = 0.5;

#include "res://shaders/building_aging.gdshaderinc"

varying flat int layer;
varying vec3 local_m;
varying vec3 local_n;
varying flat float seed;

void vertex() {
	layer = clamp(int(COLOR.a * 16.0), 0, 15);
	local_m = VERTEX;
	local_n = NORMAL;
	seed = ba_seed(MODEL_MATRIX[3].xyz, INSTANCE_ID);
}

void fragment() {
	vec4 tint = layer_tint[layer];
	vec3 base = vec3(1.0);
	if (layer < first_plain) {
		vec3 uvw = vec3(UV * layer_tile[layer], float(layer));
		base = texture(albedo_array, uvw).rgb;
		if (use_normals) {
			NORMAL_MAP = texture(normal_array, uvw).rgb;
		}
	}
	vec3 albedo = base * tint.rgb * COLOR.rgb;
	// Dérivées hors de la branche (flux de contrôle uniforme).
	float far = ba_far(local_m);
	if (aging > 0.0 && layer < first_plain) {
		bool roof = layer >= roof_first && layer <= roof_last;
		float grime = roof ? 0.0 : 1.0;
		float moss_k = roof ? 1.0 - snow : 0.25;
		float age = aging * (0.6 + 0.8 * seed);
		albedo = ba_age(albedo, local_m, normalize(local_n), local_m.y, seed, grime, moss_k, age, far);
	}
	if (snow > 0.0 && layer >= roof_first && layer <= roof_last) {
		albedo = mix(albedo, vec3(0.9, 0.92, 0.97) * (0.85 + 0.15 * base.r), snow);
	}
	ALBEDO = albedo;
	ROUGHNESS = tint.a;
}
