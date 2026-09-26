shader_type spatial;
render_mode world_vertex_coords;

// Lot ZG6 (ADR 0036) : villes ordinaires à l'échelle réelle sur la carte de campagne.
// Même matériau atlas que `building_atlas.gdshader` (BR1 : couche de la matière dans l'alpha de
// la couleur de sommet, `BuildingMaterials.ATLAS_LAYERS`), plus :
// - hauteur de base en MÈTRES non exagérés, posée par le shader : `base_m × campaign_vertical_scale`
//   (paramètre global de ZG4) ; aucun recalage quand l'exagération verticale change ;
//   source de la base : `INSTANCE_CUSTOM.r` (MultiMesh : maisons, tours, monuments) ou `UV2.x`
//   (maillages drapés : murailles, rues) ;
// - `box_uv` : UV en mètres projetées selon la normale (blocs du HLOD, tours, murailles remises à
//   l'échelle), `meters_per_unit` convertissant la position monde ;
// - variation de teinte par instance (`INSTANCE_CUSTOM.g`, 0,5 = neutre).

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 layer_tint[16];
uniform float layer_tile[16];
uniform int first_plain = 11;
uniform float snow = 0.0;
uniform int roof_first = 7;
uniform int roof_last = 10;
// 0 : base dans INSTANCE_CUSTOM.r ; 1 : base dans UV2.x.
uniform int base_source = 0;
uniform bool box_uv = false;
uniform float meters_per_unit = 719.0;
// Décalage vertical (m) ajouté à la base (rues au-dessus du sol).
uniform float lift_m = 0.0;
// ZG8 : base posée à la hauteur affichée (relief local exagéré), source unique.
#include "res://shaders/campaign_relief.gdshaderinc"
// ZG7a : HLOD par instance (un MultiMesh par modèle et par ville au lieu d'un par cellule).
// 0 : toujours ; 1 : maison détaillée, repliée au-delà de `lod_range` ; 2 : bloc, replié en deçà.
// Fondu sur les derniers 15 % (la maison s'enfonce, le bloc monte). `lod_camera` : caméra
// principale (monde), la même pour la passe d'ombre.
uniform int lod_mode = 0;
uniform vec3 lod_camera = vec3(0.0, 1e6, 0.0);
uniform float lod_range = 1.6;

varying flat int layer;
varying float tint_shift;
varying vec3 world_m;
varying vec3 world_n;

void vertex() {
	layer = clamp(int(COLOR.a * 16.0), 0, 15);
	float base = base_source == 0 ? INSTANCE_CUSTOM.r : UV2.x;
	tint_shift = base_source == 0 ? INSTANCE_CUSTOM.g - 0.5 : 0.0;
	// Ancrage : origine de l'instance (bâtiments rigides) ou le sommet (maillages drapés).
	vec2 anchor_xz = base_source == 0 ? MODEL_MATRIX[3].xz : VERTEX.xz;
	float keep = 1.0;
	if (lod_mode > 0) {
		// Hauteur approchée de l'instance (sans le relief ZG8) : suffit au choix du niveau.
		vec3 origin = vec3(anchor_xz.x, MODEL_MATRIX[3].y + base * campaign_vertical_scale, anchor_xz.y);
		float grow = smoothstep(lod_range * 0.85, lod_range, distance(origin, lod_camera));
		keep = lod_mode == 1 ? 1.0 - grow : grow;
	}
	if (keep <= 0.0) {
		// Instance repliée sur son origine : triangles dégénérés, aucun fragment.
		VERTEX = MODEL_MATRIX[3].xyz;
	} else {
		VERTEX.y = MODEL_MATRIX[3].y + (VERTEX.y - MODEL_MATRIX[3].y) * keep;
		VERTEX.y += campaign_display_height(base, anchor_xz) + lift_m * campaign_vertical_scale;
	}
	world_m = VERTEX * meters_per_unit;
	world_n = NORMAL;
}

void fragment() {
	vec4 tint = layer_tint[layer];
	vec3 base = vec3(1.0);
	if (layer < first_plain) {
		vec2 uv = UV;
		if (box_uv) {
			vec3 n = abs(world_n);
			uv = n.y > max(n.x, n.z) ? world_m.xz : (n.x > n.z ? world_m.zy : world_m.xy);
			uv.y = -uv.y;
		}
		base = texture(albedo_array, vec3(uv * layer_tile[layer], float(layer))).rgb;
	}
	vec3 albedo = base * tint.rgb * COLOR.rgb * (1.0 + tint_shift * 0.35);
	if (snow > 0.0 && layer >= roof_first && layer <= roof_last) {
		albedo = mix(albedo, vec3(0.9, 0.92, 0.97) * (0.85 + 0.15 * base.r), snow);
	}
	ALBEDO = albedo;
	ROUGHNESS = tint.a;
}
