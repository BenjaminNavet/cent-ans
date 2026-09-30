// NT10 : ombre bon marché des imposteurs lointains (au-delà de 300 m) : un disque (ellipse pour
// les cavaliers, allongée selon le cap) posé au sol sous chaque figurine, en passe
// multiplicative (assombrit le sol, aucun tri, aucune carte d'ombre). Même MultiMesh que
// l'imposteur (`BattleSoldiers._impostor_layer`) : le quadrilatère 1 × 1 est couché ici.
shader_type spatial;
render_mode world_vertex_coords, unshaded, blend_mul, depth_draw_never, cull_disabled, shadows_disabled, fog_disabled;

uniform vec2 cell_m = vec2(1.3, 2.6);
uniform bool mounted = false;
uniform float strength = 0.5;
// Décalage (m, par unité d'échelle) vers l'opposé du soleil, et hauteur au-dessus du sol.
uniform vec2 offset = vec2(0.25, 0.2);
uniform float lift = 0.25;
uniform float thin_out = 0.0;

varying vec2 v_local;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec3 ax = MODEL_MATRIX[0].xyz;
	vec3 az = MODEL_MATRIX[2].xyz;
	float scale = length(ax);
	vec2 local = vec2(UV.x - 0.5, 0.5 - UV.y) * 2.0;
	vec3 side = normalize(vec3(ax.x, 0.0, ax.z) + vec3(1e-5, 0.0, 0.0));
	vec3 fore = normalize(vec3(az.x, 0.0, az.z) + vec3(0.0, 0.0, 1e-5));
	float rx = (mounted ? 0.32 : 0.5) * cell_m.x * scale;
	float rz = (mounted ? 0.55 : 0.5) * cell_m.x * scale;
	if (thin_out > 0.5) {
		if (INSTANCE_ID % 2 == 1) {
			rx = 0.0;
			rz = 0.0;
		} else {
			rx *= 1.5;
			rz *= 1.5;
		}
	}
	VERTEX = origin + side * local.x * rx + fore * local.y * rz + vec3(offset.x, lift, offset.y) * scale;
	v_local = local;
}

void fragment() {
	float d = length(v_local);
	float a = (1.0 - smoothstep(0.3, 1.0, d)) * strength;
	ALBEDO = vec3(1.0 - a);
}
