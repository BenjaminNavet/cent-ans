// Corps commun des shaders de feuillage de la carte (lot V4) : `foliage.gdshader` (sans
// `discard`, garde le test de profondeur anticipé) et `foliage_winter.gdshader` (hiver : houppiers
// des feuillus ajourés par `discard`, activé par FOLIAGE_WINTER_CUTOUT). Uniformes, variété et
// teinte saisonnière : `foliage_common.gdshaderinc` (partagés avec les imposteurs du lot FC2).

#include "res://shaders/foliage_common.gdshaderinc"

#ifdef FOLIAGE_CARDS
// Lot FC5 : atlas des cartes (couleur normalisée à 0,5 : ×2 ici), découpe alpha.
uniform sampler2D card_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform float card_alpha_cut = 0.5;
// Lot FC6 : les arbres proches portent aussi un quadrilatère d'imposteur (UV2.y ≥ 2 : coin + 2) ;
// fondu tramé par arbre entre `near_start` et `near_end` (distance horizontale à la caméra + moitié
// de sa hauteur, même mesure que `Vegetation._apply_lod`) : cartes en deçà, imposteur au-delà.
uniform float near_start = 37.0;
uniform float near_end = 45.0;
#include "res://shaders/campaign_tree_impostor.gdshaderinc"
varying float v_near;
varying float v_impostor;
varying vec2 v_uv0;
varying vec2 v_uv1;
varying float v_blend;
varying vec3 v_right;
varying vec3 v_up;
varying vec3 v_face;
varying vec2 v_quad;
#endif

varying float v_ao;
varying vec3 v_tint;
varying vec3 v_local;
varying float v_essence;
varying float v_crown;
varying float v_seed;
varying float v_world_y;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float seed = INSTANCE_CUSTOM.a;
#ifdef FOLIAGE_CARDS
	vec3 cam = MAIN_CAM_INV_VIEW_MATRIX[3].xyz;
	float d_lod = distance(origin.xz, cam.xz) + abs(cam.y) * 0.5;
	v_near = 1.0 - smoothstep(near_start, near_end, d_lod);
	v_impostor = UV2.y > 1.5 ? 1.0 : 0.0;
	if (v_impostor > 0.5) {
		v_essence = UV.x;
		v_crown = 1.0;
		v_seed = seed;
		v_world_y = origin.y;
		v_ao = 1.0;
		v_local = vec3(0.0);
		if (v_near >= 1.0) {
			VERTEX = vec3(0.0);  // arbre entièrement en cartes : quadrilatère dégénéré
		} else {
			ImpostorVertex iv = impostor_vertex(MODEL_MATRIX, cam, UV2 - vec2(0.0, 2.0), UV.x, seed, INSTANCE_CUSTOM.rgb);
			VERTEX = (inverse(MODEL_MATRIX) * vec4(iv.world, 1.0)).xyz;
			v_uv0 = iv.uv0;
			v_uv1 = iv.uv1;
			v_blend = iv.blend;
			v_right = iv.right;
			v_up = iv.up;
			v_face = iv.face;
			v_tint = iv.tint;
			v_quad = iv.local;
		}
	} else if (v_near <= 0.0) {
		VERTEX = vec3(0.0);  // arbre entièrement en imposteur : cartes dégénérées
		v_essence = UV.x;
		v_crown = UV.y;
		v_seed = seed;
		v_world_y = origin.y;
		v_ao = 1.0;
		v_local = vec3(0.0);
		v_tint = vec3(1.0);
	} else {
#endif
	float keep = foliage_keep(origin, seed);
	// Vent : rafales lentes qui balaient la carte, plus un frémissement propre à chaque arbre.
	float phase = TIME * wind_speed + origin.x * 0.045 + origin.z * 0.03;
	float gust = 0.6 + 0.4 * sin(phase * 0.37);
	float sway = (sin(phase + seed * 6.2831) * gust + 0.3 * sin(TIME * 2.3 + seed * 20.0)) * wind_strength;
	float weight = COLOR.a * COLOR.a;
	v_essence = UV.x;
	v_crown = UV.y;
	v_seed = seed;
	v_world_y = origin.y;
	// Hiver : houppiers des feuillus dénudés, plus légers (on devine la ramure).
	if (UV.y > 0.5 && UV.x < 1.5) {
		VERTEX.xz *= mix(1.0, 0.82, campaign_season.w);
	}
	v_local = VERTEX * 9.0 + vec3(seed * 17.0);
	VERTEX.x += sway * weight;
	VERTEX.z += sway * 0.6 * weight;
	// PO3 : échelle par arbre et lisières (haies exclues).
	vec3 variety = vec3(1.0);
	vec3 tone = vec3(1.0);
	if (UV.x < 2.5) {
		variety = foliage_variety(origin, seed, tone);
	}
	VERTEX *= keep * campaign_prop_scale * variety;
	// Lot SZ4b : le pied de l'instance est enfoncé de GROUND_SINK × sa hauteur de carte
	// (`VegetationTileJob.GROUND_SINK`, crate `vegetation`) ; réduit avec l'arbre, sinon un arbre à
	// taille réelle reste enterré sous 80 m de sol (forêts invisibles au palier vallée).
	VERTEX.y += FOLIAGE_GROUND_SINK * (1.0 - campaign_prop_scale);
	v_ao = mix(0.55, 1.0, COLOR.a);
	v_tint = INSTANCE_CUSTOM.rgb * tone;
#ifdef FOLIAGE_CARDS
	}
#endif
}

void fragment() {
#ifdef FOLIAGE_CARDS
	// FC6 : tramage complémentaire (même seuil de Bayer) entre cartes et imposteur.
	float dither = bayer4(FRAGCOORD.xy);
	if (v_impostor > 0.5) {
		if (dither < v_near) {
			discard;
		}
		ImpostorFragment f = impostor_fragment(FRAGCOORD.xy, v_uv0, v_uv1, v_blend, v_right, v_up, v_face, v_tint, v_quad, v_seed, v_essence, v_world_y);
		if (f.cut) {
			discard;
		}
		NORMAL = normalize((VIEW_MATRIX * vec4(f.world_normal, 0.0)).xyz);
		ALBEDO = f.albedo;
		ALPHA = f.alpha;
		ALPHA_SCISSOR_THRESHOLD = alpha_cut;
		ROUGHNESS = roughness_value;
		SPECULAR = 0.25;
		AO = f.ao;
		AO_LIGHT_AFFECT = 0.4;
		BACKLIGHT = f.albedo * (v_essence < 1.5 ? mix(0.35, 0.1, campaign_season.w) : 0.35);
	} else {
		if (dither >= v_near) {
			discard;
		}
#endif
	vec3 tint = v_tint;
	float leaves = value_noise(v_local) * 0.6 + value_noise(v_local * 2.3) * 0.4;
	vec3 albedo = COLOR.rgb * tint * mix(0.72, 1.18, leaves);
#ifdef FOLIAGE_CARDS
	ALPHA = 1.0;
	ALPHA_SCISSOR_THRESHOLD = card_alpha_cut;
	if (v_crown > 0.5) {
		float lod = clamp(textureQueryLod(card_texture, UV2).x, 0.0, 4.0);
		vec4 card = textureLod(card_texture, UV2, lod);
		ALPHA = card.a * (1.0 + lod * 0.35);
		albedo = COLOR.rgb * tint * card.rgb * 2.0 * mix(0.88, 1.08, leaves);
	}
	if (!FRONT_FACING) {
		NORMAL = -NORMAL;  // houppier : même normale arrondie des deux côtés de la carte
	}
#endif
	if (v_crown > 0.5) {
#ifdef FOLIAGE_WINTER_CUTOUT
		// Hiver : houppier des feuillus ajouré (on voit le sol à travers).
		if (v_essence < 1.5 && campaign_season.w > 0.5 && value_noise(v_local * 2.4) < 0.5) {
			discard;
		}
#endif
		float up = (INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).y;
		albedo = foliage_season(albedo, v_essence, v_seed, leaves, up, v_world_y);
	}
	ALBEDO = albedo;
	ROUGHNESS = roughness_value;
	SPECULAR = 0.25;
	AO = v_ao;
	AO_LIGHT_AFFECT = 0.4;
	// Lumière transmise à travers le feuillage, vue à contre-jour.
	BACKLIGHT = albedo * (v_essence < 1.5 ? mix(0.35, 0.1, campaign_season.w) : 0.35);
#ifdef FOLIAGE_CARDS
	}
#endif
}
