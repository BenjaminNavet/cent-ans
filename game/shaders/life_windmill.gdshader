// Lot CV1 : ailes de moulin à vent (windmill_sails.glb, moyeu à l'origine, plan XY) qui
// tournent autour de l'axe Z local. INSTANCE_CUSTOM : r phase, g vitesse (0 = arrêté : moulin
// en ruine), b, a inutilisés. Couleurs par matériau d'origine (COLOR absent) : bois et toile.
shader_type spatial;
render_mode cull_disabled;

uniform vec3 wood : source_color = vec3(0.22, 0.15, 0.09);
uniform vec3 canvas : source_color = vec3(0.70, 0.66, 0.56);

varying float v_radius;

void vertex() {
	float angle = INSTANCE_CUSTOM.r * 6.2831 + TIME * INSTANCE_CUSTOM.g;
	float c = cos(angle);
	float s = sin(angle);
	mat2 rot = mat2(vec2(c, s), vec2(-s, c));
	v_radius = length(VERTEX.xy);
	VERTEX.xy = rot * VERTEX.xy;
	NORMAL.xy = rot * NORMAL.xy;
}

void fragment() {
	// Toile sur la partie extérieure des ailes, bois au centre (moyeu et vergues).
	float cloth = smoothstep(0.05, 0.07, v_radius);
	ALBEDO = mix(wood, canvas, cloth * 0.85);
	ROUGHNESS = 0.9;
}
