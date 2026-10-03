// Lot LR-18 : nappe de crue de la carte vivante (FolkScenes `flood`). Matériau propre (les
// shaders d'eau partagés ne sont pas touchés) : eau boueuse semi-transparente, bord adouci par
// l'alpha de sommet (COLOR.a, 1 au centre, 0 au bord) et par la profondeur (là où la nappe
// affleure le relief, elle s'efface : elle épouse le terrain au lieu de le couper net).
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, diffuse_lambert, specular_schlick_ggx;

uniform float anim_time = 0.0;
uniform vec4 water_color : source_color = vec4(0.30, 0.27, 0.17, 1.0);
uniform float max_alpha = 0.62;
uniform float depth_fade = 0.9;
uniform sampler2D depth_tex : hint_depth_texture, repeat_disable, filter_nearest;

varying float v_edge;
varying vec3 v_world;

void vertex() {
	v_edge = COLOR.a;
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	// Petite houle (à peine visible), hors du bord pour ne pas faire flotter la frange.
	VERTEX.y += 0.04 * sin(anim_time * 0.8 + VERTEX.x * 0.5 + VERTEX.z * 0.4) * v_edge;
}

void fragment() {
	float depth = texture(depth_tex, SCREEN_UV).x;
	vec3 ndc = vec3(SCREEN_UV * 2.0 - 1.0, depth);
	vec4 view = INV_PROJECTION_MATRIX * vec4(ndc, 1.0);
	view.xyz /= view.w;
	float thickness = max(0.0, -view.z - (-VERTEX.z));
	float soft = clamp(thickness / depth_fade, 0.0, 1.0);
	float rim = smoothstep(0.0, 0.55, v_edge);
	float ripple = 0.5 + 0.5 * sin(anim_time * 1.3 + v_world.x * 0.7 + v_world.z * 0.9);
	ALBEDO = mix(water_color.rgb, water_color.rgb * 1.25, ripple * 0.35);
	ALPHA = max_alpha * rim * mix(0.35, 1.0, soft);
	ROUGHNESS = 0.12;
	METALLIC = 0.0;
	SPECULAR = 0.6;
}
