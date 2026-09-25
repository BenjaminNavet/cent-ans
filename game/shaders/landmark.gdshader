shader_type spatial;
render_mode cull_disabled, depth_draw_opaque;

// Villes emblématiques (lot L1) : maquette posée sur le relief par sommet. UV = point d'ancrage
// (repère local de la maquette, x et z) ; la hauteur du terrain y est lue dans `height_map`
// (grille cuite par `LandmarkModel` depuis la surface affichée). Les bâtiments ont un ancrage
// commun à tous leurs sommets et restent donc rigides. Rendu seulement.

uniform vec4 albedo : source_color = vec4(0.6, 0.6, 0.6, 1.0);
uniform float roughness : hint_range(0.0, 1.0) = 0.9;
uniform float tint_strength : hint_range(0.0, 1.0) = 1.0;
uniform sampler2D height_map : filter_linear, repeat_disable;
uniform vec2 map_origin = vec2(0.0);
uniform float map_extent = 1.0;
// Eau : léger reflet spéculaire et teinte.
uniform float water = 0.0;

void vertex() {
	vec3 anchor_world = (MODEL_MATRIX * vec4(UV.x, 0.0, UV.y, 1.0)).xyz;
	vec2 uvh = (anchor_world.xz - map_origin) / map_extent;
	float h = textureLod(height_map, clamp(uvh, vec2(0.0), vec2(1.0)), 0.0).r;
	VERTEX.y += h;
}

void fragment() {
	vec3 tint = mix(vec3(1.0), COLOR.rgb * 1.2, tint_strength);
	ALBEDO = albedo.rgb * tint;
	ROUGHNESS = roughness;
	if (!FRONT_FACING) {
		NORMAL = -NORMAL;
	}
	if (water > 0.5) {
		ROUGHNESS = 0.08;
		SPECULAR = 0.6;
		METALLIC = 0.0;
	}
}
