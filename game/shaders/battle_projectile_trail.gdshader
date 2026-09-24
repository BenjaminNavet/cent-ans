// Traînée légère des traits en vol (lot B4) : ruban tourné vers la caméra autour de l'axe du
// vol, de la pointe (z = 0) vers l'arrière (z = -1 × `trail_length`), qui s'estompe.
shader_type spatial;
render_mode skip_vertex_transform, unshaded, cull_disabled, depth_draw_never, shadows_disabled;

#include "res://shaders/battle_projectile.gdshaderinc"

uniform float trail_alpha = 0.45;
uniform float trail_length = 5.0;

varying float v_along;
varying float v_alpha;

void vertex() {
	Flight f = projectile(MODEL_MATRIX);
	float scale = f.flying ? f.scale : 0.0;
	vec3 to_cam = normalize(CAMERA_POSITION_WORLD - f.pos);
	vec3 across = normalize(cross(f.dir, to_cam) + vec3(0.0, 1e-4, 0.0));
	float widen = clamp(distance(CAMERA_POSITION_WORLD, f.pos) / 30.0, 1.0, 7.0);
	vec3 world = f.pos + (across * VERTEX.x * widen + f.dir * VERTEX.z * trail_length) * scale;
	v_along = clamp(-VERTEX.z, 0.0, 1.0);
	v_alpha = scale;
	VERTEX = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
}

void fragment() {
	ALBEDO = vec3(0.86, 0.85, 0.8);
	ALPHA = trail_alpha * (1.0 - v_along) * v_alpha;
}
