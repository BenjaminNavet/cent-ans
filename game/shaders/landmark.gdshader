shader_type spatial;
render_mode cull_disabled, depth_draw_opaque;

// Villes emblématiques (lot L1) : maquette posée sur le relief par sommet. UV = point d'ancrage
// (repère local de la maquette, x et z) ; la hauteur du terrain y est lue dans `height_map`
// (grille cuite par `LandmarkModel` depuis la surface affichée). Les bâtiments ont un ancrage
// commun à tous leurs sommets et restent donc rigides. Rendu seulement.
//
// Matériaux (lot L3) : l'alpha de la couleur de sommet porte la couche de l'atlas partagé
// `assets/textures/landmarks/` (calcaire appareillé, moellons, brique, ardoise, tuile, enduit,
// pans de bois, planches, chaume, pavés, plomb, vitrail, sol). Projection triplanaire dans le
// repère local (mètres = unités × `meters_per_unit`), couleur de la palette × détail de la photo,
// variation par bâtiment (hachage de l'ancrage), vieillissement : salissures au pied des murs,
// coulures sous la pluie, mousse et lichen sur les faces tournées vers le ciel.

uniform vec4 albedo : source_color = vec4(0.6, 0.6, 0.6, 1.0);
uniform float roughness : hint_range(0.0, 1.0) = 0.9;
uniform float tint_strength : hint_range(0.0, 1.0) = 1.0;
uniform sampler2D height_map : filter_linear, repeat_disable;
uniform vec2 map_origin = vec2(0.0);
uniform float map_extent = 1.0;
// ZG4 : hauteurs cuites en mètres, mises à l'échelle verticale courante de la carte (pas de
// nouvelle cuisson quand l'exagération change) ; faux : hauteurs déjà en unités monde.
uniform bool height_in_meters = false;
global uniform float campaign_vertical_scale;
// Eau : léger reflet spéculaire et teinte.
uniform float water = 0.0;
// Échelle de la maquette : mètres par unité locale (1 : toiles de fond de siège, menu).
uniform float meters_per_unit = 1.0;
// Hauteur locale des plaques de sol et des îles (unités) : le pied des murs y commence
// (défaut : Paris en toile de fond, 0,010 unité de carte × 205 m).
uniform float ground_height = 2.05;
uniform int debug_view = 0;
// 0 : couleurs unies (ancien rendu) ; 1 : matériaux de l'atlas.
uniform float material_detail : hint_range(0.0, 1.0) = 1.0;
uniform float aging : hint_range(0.0, 2.0) = 1.0;
// Atlas partagé (réglage global du projet, `shader_globals` : aucun script à modifier).
global uniform sampler2DArray landmark_albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
global uniform sampler2DArray landmark_normal_array : filter_linear_mipmap_anisotropic, repeat_enable;

varying vec3 local_pos;
varying vec2 anchor;

// Taille d'une répétition de chaque couche (m).
const float TILE[16] = float[16](4.6, 3.5, 2.2, 2.6, 2.4, 2.6, 3.0, 5.0, 2.4, 3.0, 3.0, 2.4, 4.0, 6.0, 4.0, 9.0);
// Rugosité par couche (-1 : celle de la palette).
const float ROUGH[16] = float[16](0.88, 0.92, 0.9, 0.55, 0.78, 0.85, 0.93, 0.9, 0.85, 0.97, 0.92, 0.42, 0.18, 0.97, -1.0, 0.9);
// Pied de mur (salissures, 1) et mousse sur les faces au ciel (0-1).
const float MOSS[16] = float[16](0.35, 0.6, 0.35, 0.55, 0.75, 1.0, 0.3, 0.3, 0.5, 0.8, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0);
const float GRIME[16] = float[16](1.0, 1.0, 0.8, 0.4, 0.5, 0.6, 1.0, 0.9, 0.8, 0.4, 0.3, 0.3, 0.0, 0.0, 0.0, 0.0);

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

void vertex() {
	local_pos = VERTEX;
	anchor = UV;
	vec3 anchor_world = (MODEL_MATRIX * vec4(UV.x, 0.0, UV.y, 1.0)).xyz;
	vec2 uvh = (anchor_world.xz - map_origin) / map_extent;
	float h = textureLod(height_map, clamp(uvh, vec2(0.0), vec2(1.0)), 0.0).r;
	VERTEX.y += height_in_meters ? h * campaign_vertical_scale : h;
}

// Échantillon triplanaire d'une couche ; `n` normale locale, `w` poids des trois plans.
vec4 tri(sampler2DArray tex, vec3 p, vec3 w, float layer) {
	vec4 x = texture(tex, vec3(p.zy, layer));
	vec4 y = texture(tex, vec3(p.xz, layer));
	vec4 z = texture(tex, vec3(p.xy, layer));
	return x * w.x + y * w.y + z * w.z;
}

void fragment() {
	vec3 tint = mix(vec3(1.0), COLOR.rgb * 1.2, tint_strength);
	vec3 base = albedo.rgb * tint;
	float rough = roughness;
	// Normale géométrique locale (faces planes, indépendante des normales importées).
	vec3 n_local = normalize(cross(dFdx(local_pos), dFdy(local_pos)));
	vec3 n_view_geo = normalize((VIEW_MATRIX * MODEL_MATRIX * vec4(n_local, 0.0)).xyz);
	if (dot(n_view_geo, VIEW) < 0.0) {
		n_local = -n_local;
	}
	// Alpha = couche × 16 + pas d'exagération (`material_code` de `landmark_city.py`).
	int code = int(round(COLOR.a * 255.0));
	int layer = clamp(code / 16, 0, 15);
	float exaggeration = exp2(float(code % 16 - 4) * 0.25);
	if (material_detail > 0.0 && water < 0.5 && layer != 14) {
		float seed = hash12(floor(anchor * 97.0) + 0.37);
		float seed2 = hash12(floor(anchor * 61.0) + 11.7);
		// Mètres « réels » : une pierre garde sa taille sur un monument agrandi.
		vec3 p_m = local_pos * meters_per_unit / exaggeration;
		// Décalage par bâtiment pour que deux façades voisines ne se répètent pas.
		vec3 p = p_m / TILE[layer] + vec3(seed * 7.0, seed2 * 3.0, seed2 * 5.0);
		vec3 w = pow(abs(n_local), vec3(4.0));
		w /= (w.x + w.y + w.z);
		float fl = float(layer);
		vec3 d = tri(landmark_albedo_array, p, w, fl).rgb;
		vec3 nm = tri(landmark_normal_array, p, w, fl).xyz * 2.0 - 1.0;
		vec3 detail_col = (layer == 12) ? d * 0.1 : base * d * 2.0;
		// Variation par bâtiment : valeur et chaleur.
		detail_col *= 0.9 + 0.2 * seed;
		detail_col *= mix(vec3(1.0), vec3(1.04, 1.0, 0.94), seed2 - 0.3);
		// Vieillissement.
		vec3 masks = tri(landmark_albedo_array, p_m / TILE[15], w, 15.0).rgb;
		// Grandes nuances (pierres de carrières différentes, reprises, patine) sur ~40 m.
		vec3 macro = tri(landmark_albedo_array, p_m / 41.0 + seed, w, 15.0).rgb;
		detail_col *= 0.84 + 0.32 * macro.r;
		detail_col *= mix(vec3(1.0), vec3(0.95, 0.97, 1.03), macro.b);
		float h_m = max((local_pos.y - ground_height) * meters_per_unit / exaggeration, 0.0);
		float vertical = 1.0 - abs(n_local.y);
		float age = aging * (0.55 + 0.9 * seed2);
		// Pied des murs : éclaboussures de boue et remontées d'humidité.
		float foot = (1.0 - smoothstep(0.0, 2.5 + 2.5 * masks.r, h_m)) * vertical * GRIME[layer];
		detail_col = mix(detail_col, detail_col * vec3(0.5, 0.48, 0.4), clamp(foot * 0.9 * age, 0.0, 0.85));
		// Coulures verticales sous les rebords (noircissement de la pluie et de la suie).
		float streak = masks.g * vertical * GRIME[layer] * smoothstep(1.5, 5.0, h_m);
		detail_col *= 1.0 - clamp(streak * 0.45 * age, 0.0, 0.6);
		// Salissure diffuse (plaques).
		detail_col *= 1.0 - 0.22 * age * smoothstep(0.4, 0.85, masks.r) * GRIME[layer];
		// Mousse et lichen sur les faces au ciel (toits, glacis, sommets de mur).
		float up = smoothstep(0.25, 0.8, n_local.y);
		float moss = up * MOSS[layer] * smoothstep(0.5, 0.78, masks.b + 0.18 * (seed - 0.5)) * age;
		detail_col = mix(detail_col, vec3(0.11, 0.13, 0.05) * (0.8 + 0.4 * masks.r), clamp(moss * 0.55, 0.0, 0.7));
		base = mix(base, detail_col, material_detail);
		if (debug_view == 3) { base = vec3(foot, streak, moss); }
		if (debug_view == 4) { base = masks; }
		if (debug_view == 5) { base = vec3(h_m / 20.0, exaggeration / 4.0, float(layer) / 15.0); }
		float r_layer = ROUGH[layer];
		rough = r_layer < 0.0 ? roughness : clamp(r_layer + (0.5 - d.g) * 0.2 + moss * 0.1, 0.05, 1.0);
		// Normale perturbée (mélange « whiteout » par plan), en repère local puis vue.
		vec3 nx = normalize(vec3(nm.z * sign(n_local.x), nm.y, nm.x));
		vec3 ny = normalize(vec3(nm.x, nm.z * sign(n_local.y), nm.y));
		vec3 nz = normalize(vec3(nm.x, nm.y, nm.z * sign(n_local.z)));
		vec3 bumped = normalize(nx * w.x + ny * w.y + nz * w.z);
		vec3 n_mix = normalize(n_local + (bumped - vec3(sign(n_local.x) * w.x, sign(n_local.y) * w.y, sign(n_local.z) * w.z)) * 0.8 * material_detail);
		NORMAL = normalize((VIEW_MATRIX * MODEL_MATRIX * vec4(n_mix, 0.0)).xyz);
		if (layer == 12) {
			SPECULAR = 0.7;
		}
	} else {
		// Les GLB n'ont plus de normales (L3, `drop_normals`) : normale plane dérivée.
		NORMAL = normalize((VIEW_MATRIX * MODEL_MATRIX * vec4(n_local, 0.0)).xyz);
	}
	ALBEDO = base;
	ROUGHNESS = rough;
	if (debug_view >= 3) { ALBEDO = base; }
	if (debug_view == 1) { ALBEDO = vec3(fract(local_pos.y * meters_per_unit / 5.0)); }
	if (debug_view == 2) { ALBEDO = abs(n_local); }
	if (water > 0.5) {
		ROUGHNESS = 0.08;
		SPECULAR = 0.6;
		METALLIC = 0.0;
	}
}
