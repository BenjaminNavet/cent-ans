// Lot FK3 (carte vivante, `docs/design/2026-09-29-carte-vivante-folk.md` § 2.2) : accessoires
// des figurines de la carte (charrettes, bêtes des troupeaux…), un `MultiMesh` par modèle.
// Même déplacement en shader que les figurines (`FK_TRAVEL` de `battle_soldier_skinned`) :
// INSTANCE_CUSTOM = (longueur, vitesse, phase, dénivelé) en unités du modèle, trajet en +Z,
// retour au départ masqué par un rétrécissement ; longueur nulle = immobile.
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform float anim_time = 0.0;
uniform vec4 albedo : source_color = vec4(1.0);
uniform sampler2D albedo_tex : source_color, filter_linear_mipmap;
uniform bool use_tex = false;
uniform bool use_vertex_color = false;

varying vec3 v_color;

void vertex() {
	vec4 c = INSTANCE_CUSTOM;
	if (c.x > 0.0) {
		float s = mod(anim_time * c.y + c.z, c.x);
		float u = s / c.x;
		VERTEX *= smoothstep(0.0, 0.08, u) * (1.0 - smoothstep(0.92, 1.0, u));
		VERTEX += vec3(0.0, c.w * u, s);
		// Léger cahot des roues.
		VERTEX.y += 0.03 * sin(s * 5.0);
	}
	v_color = use_vertex_color ? COLOR.rgb : vec3(1.0);
}

void fragment() {
	vec3 a = albedo.rgb * v_color;
	if (use_tex) {
		a *= texture(albedo_tex, UV).rgb;
	}
	ALBEDO = a;
	ROUGHNESS = 0.9;
}
