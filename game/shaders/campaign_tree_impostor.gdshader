// Imposteurs des arbres de la carte de campagne (lot FC2) : un quadrilatère (2 triangles) par arbre,
// tourné vers la caméra principale et incliné vers elle (pied fixe), texturé par les atlas cuits
// sous Blender (`tools/blender_scripts/campaign_tree_impostors.py`) : une ligne par essence
// (chêne, hêtre, conifère), une colonne par azimut cuit (8 vues, caméra orthographique à 35° de
// tangage). Vue choisie selon l'azimut de la caméra dans le repère de l'instance (le lacet de
// l'arbre varie donc aussi l'image) ; les deux vues voisines sont mêlées par un tramage de Bayer
// (un seul échantillon par atlas). Même éclaircissement, variété des massifs, teinte saisonnière et
// uniformes que le feuillage maillé (`foliage_common.gdshaderinc`) ; hiver dans le même shader
// (le découpage alpha fait déjà un discard) : feuillus gris-brun et ajourés.
// Employé par `vegetation.gd` pour les tuiles au-delà de `detail_distance` (au lieu du maillage
// bas ≈ 20 triangles). Maillage : `VegetationMeshes.impostor(essence)` ; UV = coin du
// quadrilatère, UV2.x = ligne de l'essence dans l'atlas.
shader_type spatial;
render_mode world_vertex_coords, cull_disabled, diffuse_lambert_wrap, specular_schlick_ggx;

#include "res://shaders/foliage_common.gdshaderinc"

uniform sampler2D albedo_atlas : source_color, filter_linear_mipmap, repeat_disable;
// RVB : normale dans le repère de la vue cuite (x droite, y haut, z vers la caméra), A : occlusion.
uniform sampler2D normal_atlas : hint_default_white, filter_linear_mipmap, repeat_disable;
uniform int views = 8;
uniform int rows = 3;
// Cadrage de la cuisson (constantes ORTHO et FOOT du générateur) : côté d'une cellule pour un
// arbre de hauteur 1, et hauteur du pied dans la cellule (fraction depuis le bas).
uniform float ortho = 1.7;
uniform float foot = 0.2588;
uniform float alpha_cut = 0.5;
uniform float max_lod = 2.5;

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

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float seed = INSTANCE_CUSTOM.a;
	float essence = UV2.x;
	float keep = foliage_keep(origin, seed);
	vec3 tone = vec3(1.0);
	vec3 variety = foliage_variety(origin, seed, tone);
	vec3 bx = MODEL_MATRIX[0].xyz;
	vec3 by = MODEL_MATRIX[1].xyz;
	vec3 bz = MODEL_MATRIX[2].xyz;
	float width = length(bx);
	float height = length(by);
	vec3 cam = MAIN_CAM_INV_VIEW_MATRIX[3].xyz;
	vec3 to_cam = cam - origin;
	// Azimut de la caméra dans le repère de l'arbre (Blender (x, y) = Godot (x, −z)).
	float ax = dot(to_cam, bx / max(width, 1e-6));
	float az = dot(to_cam, bz / max(length(bz), 1e-6));
	float f = fract(atan(-az, ax) / TAU + 1.0) * float(views);
	int k0 = int(floor(f)) % views;
	int k1 = (k0 + 1) % views;
	v_blend = fract(f);
	// Panneau face à la caméra, incliné vers elle ; pied à l'origine de l'instance.
	vec3 face = normalize(to_cam + vec3(0.0, 1e-4, 0.0));
	vec3 right = normalize(cross(vec3(0.0, 1.0, 0.0), face) + vec3(1e-5, 0.0, 0.0));
	vec3 up = cross(face, right);
	float s = keep * campaign_prop_scale * ortho;
	vec2 local = vec2(UV.x - 0.5, 1.0 - UV.y);
	VERTEX = origin + right * local.x * width * variety.x * s + up * (local.y - foot) * height * variety.y * s;
	// Lot SZ4b : pied enfoncé comme le feuillage maillé (`FOLIAGE_GROUND_SINK`).
	VERTEX += by * FOLIAGE_GROUND_SINK * (1.0 - campaign_prop_scale);
	float row = clamp(floor(essence + 0.5), 0.0, float(rows - 1));
	vec2 cell = vec2(float(views), float(rows));
	v_uv0 = (vec2(float(k0), row) + UV) / cell;
	v_uv1 = (vec2(float(k1), row) + UV) / cell;
	v_right = right;
	v_up = up;
	v_face = face;
	v_tint = INSTANCE_CUSTOM.rgb * tone;
	v_local = local;
	v_seed = seed;
	v_essence = essence;
	v_world_y = origin.y;
}

// Seuil de Bayer 4 × 4 (0..1) du pixel.
float bayer4(vec2 frag) {
	int x = int(mod(frag.x, 4.0));
	int y = int(mod(frag.y, 4.0));
	const float bayer[16] = float[](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
	return (bayer[y * 4 + x] + 0.5) / 16.0;
}

void fragment() {
	vec2 uv = v_blend > bayer4(FRAGCOORD.xy) ? v_uv1 : v_uv0;
	// Niveau de mipmap borné : très loin (arbres de quelques pixels), l'alpha moyen de la cellule
	// entière, même compensé, remplirait tout le quadrilatère (carrés pleins).
	float lod = clamp(textureQueryLod(albedo_atlas, uv).x, 0.0, max_lod);
	vec4 t = textureLod(albedo_atlas, uv, lod);
	vec4 nt = textureLod(normal_atlas, uv, lod);
	float leaves = value_noise(vec3(v_local * 24.0, v_seed * 17.0));
	bool broadleaf = v_essence < 1.5;
	// Hiver : feuillus ajourés (on voit le sol à travers), comme `foliage_winter.gdshader`.
	if (broadleaf && campaign_season.w > 0.5 && value_noise(vec3(v_local * 40.0, v_seed * 29.0)) < 0.45) {
		discard;
	}
	vec3 n = nt.rgb * 2.0 - 1.0;
	vec3 world_n = normalize(v_right * n.x + v_up * n.y + v_face * max(n.z, 0.05));
	NORMAL = normalize((VIEW_MATRIX * vec4(world_n, 0.0)).xyz);
	vec3 albedo = t.rgb * v_tint * mix(0.85, 1.12, leaves);
	albedo = foliage_season(albedo, v_essence, v_seed, leaves, world_n.y, v_world_y);
	ALBEDO = albedo;
	// Mipmaps : l'alpha moyen baisse avec le niveau, compensé pour garder la silhouette.
	ALPHA = t.a * (1.0 + lod * 0.35);
	ALPHA_SCISSOR_THRESHOLD = alpha_cut;
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = mix(0.55, 1.0, nt.a);
	AO_LIGHT_AFFECT = 0.4;
	BACKLIGHT = albedo * (broadleaf ? mix(0.35, 0.1, campaign_season.w) : 0.35);
}
