shader_type spatial;

// Lot SR5b : matériau partagé d'une matière nommée du kit de bâtiments (`Plaster`, `RoofTile`…,
// `BuildingMaterials._textured`) : reproduit le StandardMaterial3D d'avant SR5 (albédo × teinte ×
// couleur de sommet, carte normale, rugosité canal vert, UV en mètres × `uv_scale`, variante
// neige des toits) et ajoute l'usure procédurale (`building_aging.gdshaderinc`) : boue et
// éclaboussures au pied des murs (y local ≈ 0 au sol), coulures sous les appuis, crasse, mousse
// sur les faces au ciel des tuiles, chaume et bardeaux, variation par bâtiment (INSTANCE_ID et
// origine de l'instance). Un matériau par nom de matière : pas d'appel de dessin en plus.

uniform vec4 albedo_color : source_color = vec4(1.0);
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool use_albedo_texture = true;
uniform sampler2D normal_texture : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool use_normal = false;
uniform float normal_scale = 1.0;
uniform sampler2D roughness_texture : hint_roughness_g, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool use_roughness_texture = false;
uniform float roughness : hint_range(0.0, 1.0) = 0.9;
uniform vec2 uv_scale = vec2(1.0);
// Usure : 0 = rendu d'avant SR5 ; poids des murs (boue, coulures) et de la mousse (faces au ciel).
uniform float aging : hint_range(0.0, 2.0) = 0.5;
uniform float aging_grime = 1.0;
uniform float aging_moss = 0.25;

#include "res://shaders/building_aging.gdshaderinc"

varying vec3 local_m;
varying vec3 local_n;
varying flat float seed;

void vertex() {
	UV = UV * uv_scale;
	local_m = VERTEX;
	local_n = NORMAL;
	seed = ba_seed(MODEL_MATRIX[3].xyz, INSTANCE_ID);
}

void fragment() {
	vec3 albedo = albedo_color.rgb * COLOR.rgb;
	if (use_albedo_texture) {
		albedo *= texture(albedo_texture, UV).rgb;
	}
	float rough = roughness;
	if (use_roughness_texture) {
		rough *= texture(roughness_texture, UV).g;
	}
	float far = ba_far(local_m);
	if (aging > 0.0) {
		float age = aging * (0.6 + 0.8 * seed);
		vec3 n = normalize(local_n);
		vec3 aged = ba_age(albedo, local_m, n, local_m.y, seed, aging_grime, aging_moss, age, far);
		// Boue et mousse plus mates que la matière.
		float change = clamp(length(albedo - aged) * 3.0, 0.0, 1.0);
		rough = mix(rough, 0.97, change * 0.6);
		albedo = aged;
	}
	ALBEDO = albedo;
	ROUGHNESS = rough;
	if (use_normal) {
		NORMAL_MAP = texture(normal_texture, UV).rgb;
		NORMAL_MAP_DEPTH = normal_scale;
	}
}
