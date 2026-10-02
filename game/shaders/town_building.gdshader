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
// Lot TF : dernière couche unie ; les couches suivantes (`TimberFrame`) sont texturées.
uniform int plain_last = 13;
uniform float snow = 0.0;
uniform int roof_first = 7;
uniform int roof_last = 10;
// 0 : base dans INSTANCE_CUSTOM.r ; 1 : base dans UV2.x ; 2 (RS-G) : monuments fusionnés par
// cellule, base, ancrage xz (repère du nœud) et teinte dans CUSTOM0.
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
// Lot SZ4 : sol bâti vu de loin (maisons sous-pixel) teinté par la couleur moyenne (dernier niveau
// de mip) d'une couche de toit de l'atlas, tirée par cellule de `roofscape_cell_m` ; part bâtie
// dans `UV2.y` ; fondu selon la distance caméra (unités) de `roofscape_near` à `roofscape_far`.
// Uniformes `roofscape*` et teinte dans `roofscape.gdshaderinc` (partagée avec `town_far`).
#include "res://shaders/roofscape.gdshaderinc"
// Lot SR5 : usure procédurale douce (`building_aging.gdshaderinc`), 0 = rendu d'avant SR5.
// Hauteur au-dessus du pied = y du sommet relatif à l'origine de l'instance (avant pose sur le
// relief), en mètres ; position locale (m) relative à l'ancrage pour le bruit.
uniform float aging : hint_range(0.0, 2.0) = 0.5;
// Lot TB3 (ADR 0162) : suie d'une ville saccagée, prise d'assaut, assiégée ou dévastée, **par
// ville** : paramètre d'instance posé sur les nœuds de la ville (`TownBuilder.set_soot`), ou par
// instance de MultiMesh dans `INSTANCE_CUSTOM.b` (faubourgs et bâtiments hors les murs partagés
// entre villes, `OutbuildingLayer`). 0 : aucune.
instance uniform float town_soot : hint_range(0.0, 1.0) = 0.0;
uniform vec3 soot_color : source_color = vec3(0.035, 0.03, 0.028);

#include "res://shaders/building_aging.gdshaderinc"

varying flat int layer;
varying float tint_shift;
varying vec3 world_m;
varying vec3 world_n;
varying float v_built;
varying vec3 v_age_pos;
varying flat float v_seed;
varying float v_soot;

void vertex() {
	layer = clamp(int(COLOR.a * 16.0), 0, 15);
	float base = base_source == 0 ? INSTANCE_CUSTOM.r : (base_source == 2 ? CUSTOM0.r : UV2.x);
	tint_shift = base_source == 0 ? INSTANCE_CUSTOM.g - 0.5 : (base_source == 2 ? CUSTOM0.a - 0.5 : 0.0);
	// Ancrage : origine de l'instance (bâtiments rigides), celle du monument fusionné, ou le
	// sommet (maillages drapés).
	vec2 anchor_xz = base_source == 0 ? MODEL_MATRIX[3].xz : VERTEX.xz;
	if (base_source == 2) {
		anchor_xz = (MODEL_MATRIX * vec4(CUSTOM0.g, 0.0, CUSTOM0.b, 1.0)).xz;
	}
	// Maillages drapés (base_source 1) : l'ancrage est le sommet lui-même ; bruit en position monde.
	vec2 age_origin = base_source == 1 ? vec2(0.0) : anchor_xz;
	v_age_pos = vec3(VERTEX.x - age_origin.x, VERTEX.y - MODEL_MATRIX[3].y, VERTEX.z - age_origin.y) * meters_per_unit;
	v_seed = base_source == 1 ? 0.5 : ba_seed(vec3(anchor_xz.x, 0.0, anchor_xz.y), INSTANCE_ID);
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
	v_built = UV2.y;
	v_soot = max(town_soot, base_source == 0 ? INSTANCE_CUSTOM.b : 0.0);
}

void fragment() {
	vec4 tint = layer_tint[layer];
	vec3 base = vec3(1.0);
	bool textured = layer < first_plain || layer > plain_last;
	if (textured) {
		vec2 uv = UV;
		if (box_uv) {
			vec3 n = abs(world_n);
			uv = n.y > max(n.x, n.z) ? world_m.xz : (n.x > n.z ? world_m.zy : world_m.xy);
			uv.y = -uv.y;
		}
		base = texture(albedo_array, vec3(uv * layer_tile[layer], float(layer))).rgb;
	}
	vec3 albedo = base * tint.rgb * COLOR.rgb * (1.0 + tint_shift * 0.35);
	float far = ba_far(v_age_pos);
	float snow_k = roof_snow(world_m);  // TB1 : neige du matériau ou de la saison (paramètre global)
	if (aging > 0.0 && textured) {
		bool roof = layer >= roof_first && layer <= roof_last;
		float age = aging * (0.6 + 0.8 * v_seed);
		vec3 n = normalize(world_n);
		albedo = ba_age(albedo, v_age_pos, n, v_age_pos.y, v_seed, roof ? 0.0 : 1.0, roof ? 1.0 - snow_k : 0.25, age, far);
	}
	if (snow_k > 0.0 && layer >= roof_first && layer <= roof_last) {
		albedo = mix(albedo, vec3(0.9, 0.92, 0.97) * (0.85 + 0.15 * base.r), snow_k);
	}
	float roughness = tint.a;
	if (roofscape > 0.0 && v_built > 0.0) {
		float far_w = smoothstep(roofscape_near, roofscape_far, distance(world_m / meters_per_unit, CAMERA_POSITION_WORLD));
		float w = clamp(v_built * 1.6, 0.0, 1.0) * far_w * roofscape;
		if (w > 0.0) {
			float roof_roughness;
			vec3 mean = roofscape_color(world_m, roof_roughness);
			albedo = mix(albedo, mean, w);
			roughness = mix(roughness, roof_roughness, w);
		}
	}
	// ADR 0144 : toits des blocs vus de loin → même teinte « masse de toits » que le lointain
	// (`town_far.gdshader`) : pas de saut de couleur au passage bloc → lointain.
	if (lod_mode == 2 && roofscape > 0.0 && layer >= roof_first && layer <= roof_last) {
		float far_w = smoothstep(roofscape_near, roofscape_far, distance(world_m / meters_per_unit, CAMERA_POSITION_WORLD));
		if (far_w > 0.0) {
			float footprint = length(fwidth(world_m.xz));
			float detail = 1.0 - smoothstep(roofscape_cell_m * 0.5, roofscape_cell_m * 2.0, footprint);
			float r_cell;
			float r_avg;
			vec3 mean = mix(roofscape_average(world_m, r_avg), roofscape_color(world_m, r_cell), detail);
			albedo = mix(albedo, mean, far_w);
			roughness = mix(roughness, mix(r_avg, r_cell, detail), far_w);
		}
	}
	if (v_soot > 0.0) {
		// Plaques de suie (≈ 16 m, bord déchiqueté à ≈ 3 m) : la part couverte suit `v_soot` ;
		// toits presque noirs, murs et sol noircis à moitié.
		float n = ba_noise(world_m.xz * 0.06) * 0.65 + ba_noise(world_m.xz * 0.33) * 0.35;
		float patch = smoothstep(1.0 - v_soot - 0.12, 1.0 - v_soot + 0.12, n);
		bool roof = layer >= roof_first && layer <= roof_last;
		float k = patch * (roof || v_built > 0.0 ? 0.9 : 0.5);
		albedo = mix(albedo, soot_color, k);
		roughness = mix(roughness, 1.0, k);
	}
	ALBEDO = albedo;
	ROUGHNESS = roughness;
}
