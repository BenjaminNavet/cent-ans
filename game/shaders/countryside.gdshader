shader_type spatial;
render_mode depth_draw_opaque, cull_back, diffuse_lambert_wrap, specular_disabled, skip_vertex_transform;

// Lot DN-PAYS : campagne vivante hors champs (`CountrysideLayer`) : haies, puits, croix, moulins,
// salines, ruines, piloris, charrettes. Pas d'animation : taille tenue a l'ecran et pose sur le
// relief affiche.
//
// Donnees par instance :
//  - MODEL_MATRIX : colonnes = axes locaux (rotation de lacet) x echelle par axe en unites monde par
//    metre de maillage (1 / metres-par-pixel ; l'axe X d'une clôture est etire a la longueur du
//    segment) ; origine = (x, altitude en METRES, z) ; l'altitude affichee vient de
//    `campaign_display_height` (source unique, comme les maquettes) ;
//  - INSTANCE_CUSTOM : x, y = ecart (unites monde) au centre du groupe (enclos, village), grossi par
//    `spread_mult` ; z = profil de saison (entier) ; w = rang (0-1, amincissement et fondu).

#include "res://shaders/campaign_relief.gdshaderinc"

global uniform vec4 campaign_season;

uniform sampler2D albedo_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_texture = false;
uniform vec4 albedo : source_color = vec4(0.55, 0.45, 0.35, 1.0);
uniform float albedo_gain = 1.0;
// Fondu de distance de ce prop (0-1) et amincissement par plafond, grossissement d'ecran.
uniform float fade : hint_range(0.0, 1.0) = 1.0;
uniform float thin : hint_range(0.0, 1.0) = 1.0;
uniform float size_mult = 1.0;
uniform float spread_mult = 1.0;
// Multiplicateur de l'axe X : `size_mult` (props isoles) ou `spread_mult` (segments d'enclos, dont la
// longueur suit l'ecartement).
uniform float x_mult = 1.0;
// Maillage : (centre x, pied y, centre z).
uniform vec3 mesh_origin = vec3(0.0);
uniform vec4 season_profiles[24];

varying float v_tint;

float hash12(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

void vertex() {
	float profile = floor(INSTANCE_CUSTOM.z + 0.0001);
	vec4 weights = season_profiles[int(clamp(profile, 0.0, 23.0))];
	float presence = dot(campaign_season, weights);
	// Apparition progressive : le prop grandit quand son rang passe sous le seuil.
	float vis = clamp((min(fade, thin) * presence - INSTANCE_CUSTOM.w) * 10.0, 0.0, 1.0);
	vec3 root = MODEL_MATRIX[3].xyz;
	vec3 ax = MODEL_MATRIX[0].xyz;
	vec3 ay = MODEL_MATRIX[1].xyz;
	vec3 az = MODEL_MATRIX[2].xyz;
	float sx = length(ax);
	float sy = length(ay);
	float sz = length(az);
	mat3 rot = mat3(ax / max(sx, 1e-9), ay / max(sy, 1e-9), az / max(sz, 1e-9));
	vec3 p = VERTEX - mesh_origin;
	p *= vec3(sx * x_mult, sy * size_mult, sz * size_mult) * vis;
	vec3 oriented = rot * p;
	vec2 xz = root.xz + INSTANCE_CUSTOM.xy * (spread_mult - 1.0);
	v_tint = hash12(root.xz * 7.7);
	float ground = campaign_display_height(root.y, xz);
	vec3 world = vec3(xz.x, ground, xz.y) + oriented;
	VERTEX = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	NORMAL = normalize((VIEW_MATRIX * vec4(rot * NORMAL, 0.0)).xyz);
}

void fragment() {
	vec3 col = albedo.rgb;
	if (has_texture) {
		col = texture(albedo_texture, UV).rgb * albedo.rgb;
	}
	col *= albedo_gain * (0.94 + 0.12 * v_tint);
	ALBEDO = col;
	ROUGHNESS = 0.9;
}
