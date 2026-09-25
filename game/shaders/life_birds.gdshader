// Lot CV1 : vols d'oiseaux de la carte de campagne (life_ambient.gd). Chaque instance est un
// oiseau (maillage en V, ailes selon ±X, bec vers -Z) qui tourne autour de l'origine de son
// instance (centre du vol) ; battement d'ailes et orbite calculés ici.
// INSTANCE_CUSTOM : r rayon d'orbite, g vitesse angulaire (rad/s, signe = sens), b phase, a
// décalage vertical.
shader_type spatial;
render_mode cull_disabled, unshaded, shadows_disabled;

uniform vec3 bird_color : source_color = vec3(0.10, 0.09, 0.085);

void vertex() {
	float radius = INSTANCE_CUSTOM.r;
	float speed = INSTANCE_CUSTOM.g;
	float phase = INSTANCE_CUSTOM.b * 6.2831;
	float angle = phase + TIME * speed;
	// Orbite légèrement elliptique et ondulante : le vol dérive, ne tourne pas au compas.
	vec3 offset = vec3(cos(angle) * radius, INSTANCE_CUSTOM.a + sin(TIME * 0.7 + phase) * 0.4, sin(angle) * radius * 0.7);
	vec3 tangent = normalize(vec3(-sin(angle) * radius, 0.0, cos(angle) * radius * 0.7) * sign(speed));
	vec3 side = normalize(cross(vec3(0.0, 1.0, 0.0), tangent));
	// Battement : bout des ailes qui monte et descend.
	float flap = sin(TIME * 11.0 + phase * 7.0) * 0.55;
	vec3 v = VERTEX;
	v.y += abs(v.x) * flap;
	vec3 local = side * v.x + vec3(0.0, 1.0, 0.0) * v.y - tangent * v.z;
	float scale = length(MODEL_MATRIX[0].xyz);
	vec3 world = MODEL_MATRIX[3].xyz + offset + local * scale;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
}

void fragment() {
	ALBEDO = bird_color;
}
