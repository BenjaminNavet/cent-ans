// Lots CV1 puis ME3 : oiseaux de la carte de campagne (map_bird_flocks.gd). Chaque instance est
// un oiseau (maillage en V, ailes selon ±X, bec vers -Z) ; la position de l'instance est le
// centre du vol (ou la place dans le V), mouvement et battement d'ailes calculés ici.
// INSTANCE_CUSTOM, selon `mode` :
//   0 orbite / 3 patauge : r rayon, g vitesse angulaire (rad/s, signe = sens), b phase, a décalage vertical ;
//   1 transit en V       : r cap (rad), g vitesse (unités/s), b phase de départ (0..1), a décalage vertical ;
//   2 posé               : r cap (rad), g inutilisé, b phase, a décalage vertical.
shader_type spatial;
render_mode cull_disabled, unshaded, shadows_disabled;

uniform vec3 bird_color : source_color = vec3(0.10, 0.09, 0.085);
uniform int mode = 0;
uniform float flap_rate = 11.0;
uniform float flap_amount = 0.55;
uniform float wing_fold = 0.0;
uniform float span = 60.0;
// DN-FLEUVE : oiseau en glb généré, albédo baké (sinon couleur unie).
uniform bool use_texture = false;
uniform sampler2D albedo_tex : source_color, filter_linear;

void vertex() {
	float radius = INSTANCE_CUSTOM.r;
	float speed = INSTANCE_CUSTOM.g;
	float phase = INSTANCE_CUSTOM.b * 6.2831;
	vec3 offset;
	vec3 tangent;
	if (mode == 1) {
		tangent = vec3(cos(radius), 0.0, sin(radius));
		float travel = fract(INSTANCE_CUSTOM.b + TIME * speed / (2.0 * span)) * 2.0 - 1.0;
		offset = tangent * travel * span + vec3(0.0, INSTANCE_CUSTOM.a + sin(TIME * 0.5 + phase) * 0.3, 0.0);
	} else if (mode == 2) {
		tangent = vec3(cos(radius), 0.0, sin(radius));
		offset = vec3(0.0, INSTANCE_CUSTOM.a, 0.0);
	} else {
		float angle = phase + TIME * speed;
		// Orbite légèrement elliptique et ondulante : le vol dérive, ne tourne pas au compas.
		offset = vec3(cos(angle) * radius, INSTANCE_CUSTOM.a + sin(TIME * 0.7 + phase) * (mode == 3 ? 0.0 : 0.4), sin(angle) * radius * 0.7);
		tangent = normalize(vec3(-sin(angle) * radius, 0.0, cos(angle) * radius * 0.7) * sign(speed));
	}
	vec3 side = normalize(cross(vec3(0.0, 1.0, 0.0), tangent));
	// Battement : bout des ailes qui monte et descend ; ailes repliées si posé ou à l'eau.
	float flap = sin(TIME * flap_rate + phase * 7.0) * flap_amount;
	vec3 v = VERTEX;
	v.y += abs(v.x) * flap;
	v.x *= 1.0 - wing_fold;
	vec3 local = side * v.x + vec3(0.0, 1.0, 0.0) * v.y - tangent * v.z;
	float scale = length(MODEL_MATRIX[0].xyz);
	vec3 world = MODEL_MATRIX[3].xyz + offset + local * scale;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
}

void fragment() {
	ALBEDO = use_texture ? texture(albedo_tex, UV).rgb : bird_color;
}
