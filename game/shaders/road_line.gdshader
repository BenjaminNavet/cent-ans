shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Routes au palier moyen (lot C7b). Même maillage que `terrain_line.gdshader` (ligne médiane,
// NORMAL = perpendiculaire signée, UV = (largeur monde, côté ±1)), mais la largeur à l'écran suit
// la distance : `near_px` pixels à `near_distance`, `far_px` à `far_distance` (distance
// caméra → sommet), jamais moins que la largeur monde. Trait en deux tons (liseré sombre, cœur
// clair) lisible sur les champs comme sur les forêts ; sous ≈ 2,5 px, le liseré domine.
// Profondeur : pas de test matériel (le maillage de terrain lointain dépasse la ligne posée sur
// la heightmap bilinéaire, ce qui la hacherait) mais un test souple contre la profondeur de la
// scène : un fragment n'est masqué que par un objet plus proche de plus de `occlusion_tolerance`
// unités (crête, maquette, figurine d'armée), jamais par le relief sous la route ni par les
// arbres. `render_priority` règle l'ordre entre traits (icônes et étiquettes au-dessus).

uniform vec4 fill_color : source_color = vec4(0.95, 0.84, 0.58, 1.0);
uniform vec4 casing_color : source_color = vec4(0.27, 0.16, 0.07, 1.0);
// Opacité globale (poids du palier moyen).
uniform float alpha = 1.0;
uniform float near_px = 3.6;
uniform float far_px = 1.6;
uniform float near_distance = 150.0;
uniform float far_distance = 620.0;
// Opacité au loin (les routes s'effacent vers le palier loin sans surcharger la carte).
uniform float far_alpha = 0.7;
// Largeur du liseré de chaque côté, en pixels.
uniform float casing_px = 0.8;
// Test de profondeur souple : tolérance (unités monde) et part de la distance ajoutée.
uniform float occlusion_tolerance = 2.5;
uniform float occlusion_tolerance_per_unit = 0.006;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;

varying float v_side;
varying float v_px;
varying float v_alpha;

void vertex() {
	vec3 view_pos = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float dist = max(length(view_pos), 0.01);
	float world_per_px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	float t = smoothstep(near_distance, far_distance, dist);
	float px = max(mix(near_px, far_px, t), UV.x / world_per_px);
	VERTEX += NORMAL * 0.5 * px * world_per_px;
	v_side = UV.y;
	v_px = px;
	v_alpha = mix(1.0, far_alpha, t);
}

void fragment() {
	float depth = texture(depth_texture, SCREEN_UV).r;
	vec4 scene = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	float scene_distance = -scene.z / scene.w;
	float own_distance = -VERTEX.z;
	if (scene_distance < own_distance - occlusion_tolerance - occlusion_tolerance_per_unit * own_distance) {
		discard;
	}
	float edge = abs(v_side);
	// Bord du cœur clair (0 = tout liseré, 1 = pas de liseré).
	float core = clamp(1.0 - 2.0 * casing_px / max(v_px, 0.01), 0.0, 1.0);
	float aa = 1.0 / max(v_px, 1.0);
	float casing = smoothstep(core - aa, core + aa, edge);
	ALBEDO = mix(fill_color.rgb, casing_color.rgb, casing);
	float soft = 1.0 - smoothstep(1.0 - aa * 1.5, 1.0, edge);
	ALPHA = alpha * v_alpha * soft * mix(fill_color.a, casing_color.a, casing);
}
