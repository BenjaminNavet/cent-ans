// Imposteurs des arbres de la carte de campagne (lot FC2) : un quadrilatère (2 triangles) par arbre,
// tourné vers la caméra principale et incliné vers elle (pied fixe), texturé par les atlas cuits
// sous Blender (`tools/blender_scripts/campaign_tree_impostors.py`) : une ligne par essence
// (chêne, hêtre, conifère), une colonne par azimut cuit (8 vues, caméra orthographique à 35° de
// tangage). Corps commun : `campaign_tree_impostor.gdshaderinc` (aussi employé par les arbres
// proches en cartes, lot FC6). Même éclaircissement, variété des massifs, teinte saisonnière et
// uniformes que le feuillage maillé (`foliage_common.gdshaderinc`).
// Employé par `vegetation.gd` au-delà de `near_distance` (maillage `VegetationMeshes.impostor`,
// UV = coin du quadrilatère, UV2.x = ligne de l'essence dans l'atlas).
shader_type spatial;
render_mode world_vertex_coords, cull_disabled, diffuse_lambert_wrap, specular_schlick_ggx;

#include "res://shaders/foliage_common.gdshaderinc"
#include "res://shaders/campaign_tree_impostor.gdshaderinc"
#include "res://shaders/campaign_cloud_shadow.gdshaderinc"  // RV-C : ombres de cumulus (même champ que le sol)

varying vec2 v_uv0;
varying vec2 v_uv1;
varying float v_blend;
varying vec3 v_right;
varying vec3 v_up;
varying vec3 v_face;
varying vec3 v_tint;
varying vec2 v_local;
varying float v_seed;
varying float v_essence;
varying float v_world_y;
varying vec3 v_world;  // RV-C

void vertex() {
	ImpostorVertex iv = impostor_vertex(MODEL_MATRIX, MAIN_CAM_INV_VIEW_MATRIX[3].xyz, UV, impostor_row(INSTANCE_CUSTOM, UV2.x), INSTANCE_CUSTOM.a, species_tint(INSTANCE_CUSTOM));
	VERTEX = iv.world;
	v_uv0 = iv.uv0;
	v_uv1 = iv.uv1;
	v_blend = iv.blend;
	v_right = iv.right;
	v_up = iv.up;
	v_face = iv.face;
	v_tint = iv.tint;
	v_local = iv.local;
	v_seed = INSTANCE_CUSTOM.a;
	v_essence = species_season(INSTANCE_CUSTOM, UV2.x);
	v_world_y = MODEL_MATRIX[3].y;
	v_world = iv.world;
}

void fragment() {
	ImpostorFragment f = impostor_fragment(FRAGCOORD.xy, v_uv0, v_uv1, v_blend, v_right, v_up, v_face, v_tint, v_local, v_seed, v_essence, v_world_y);
	if (f.cut) {
		discard;
	}
	NORMAL = normalize((VIEW_MATRIX * vec4(f.world_normal, 0.0)).xyz);
	// RV-C : sous un cumulus, l'arbre perd la même part de lumière que le sol voisin.
	vec3 cloud = cs_light_factor(cloud_shadow_fair(v_world.xz, max(length(dFdx(v_world)), length(dFdy(v_world))), 1.0));
	ALBEDO = f.albedo * cloud;
	ALPHA = f.alpha;
	ALPHA_SCISSOR_THRESHOLD = alpha_cut;
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = f.ao;
	AO_LIGHT_AFFECT = 0.4;
	BACKLIGHT = f.albedo * cloud * (season_deciduous(v_essence) ? mix(0.35, 0.1, campaign_season.w) : 0.35);
}
