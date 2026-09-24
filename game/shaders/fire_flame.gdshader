// Flammes des incendies de siège (lot V3, A1-13) : planche 8×8 procédurale
// (`tools/cent_ans_tools/fire_flipbooks.py` : RGB = chaleur, A = couverture), deux images mêlées
// pour un mouvement continu, chaleur convertie en couleur de corps noir (rouge sombre → orange →
// jaune → cœur presque blanc) émise en HDR (le halo s'en empare), alpha prémultiplié : la flamme
// éclaire sans virer au crème comme un mélange additif. Particules de `GPUParticles3D` :
// INSTANCE_CUSTOM.y = âge (0-1), .z = phase d'animation, .x = angle. Fondu doux au contact des
// murs et du sol (texture de profondeur).
shader_type spatial;
render_mode blend_premul_alpha, unshaded, cull_disabled, depth_draw_never, shadows_disabled, fog_disabled;

uniform sampler2D flipbook : filter_linear_mipmap, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;
uniform float frames_h = 8.0;
uniform float frames_v = 8.0;
uniform float emission = 3.2;
uniform float opacity = 0.55;
uniform float soft_distance = 1.5;
uniform vec3 color_cold : source_color = vec3(0.35, 0.03, 0.0);
uniform vec3 color_mid : source_color = vec3(1.0, 0.32, 0.04);
uniform vec3 color_hot : source_color = vec3(1.0, 0.72, 0.28);
uniform vec3 color_core : source_color = vec3(1.0, 0.93, 0.78);

varying vec2 uv_a;
varying vec2 uv_b;
varying float frame_blend;
varying float age;

vec2 frame_uv(vec2 uv, float frame) {
	float column = mod(frame, frames_h);
	float row = floor(frame / frames_h);
	return (vec2(column, row) + uv) / vec2(frames_h, frames_v);
}

void vertex() {
	// Panneau face caméra conservant l'échelle de la particule (billboard de particules).
	mat4 world = mat4(normalize(INV_VIEW_MATRIX[0]), normalize(INV_VIEW_MATRIX[1]), normalize(INV_VIEW_MATRIX[2]), MODEL_MATRIX[3]);
	MODELVIEW_MATRIX = VIEW_MATRIX * world * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
	float total = frames_h * frames_v;
	float f = fract(INSTANCE_CUSTOM.z) * total;
	float first = floor(f);
	frame_blend = f - first;
	uv_a = frame_uv(UV, first);
	uv_b = frame_uv(UV, mod(first + 1.0, total));
	age = INSTANCE_CUSTOM.y;
}

vec3 blackbody(float heat) {
	vec3 c = mix(color_cold, color_mid, smoothstep(0.0, 0.35, heat));
	c = mix(c, color_hot, smoothstep(0.35, 0.7, heat));
	return mix(c, color_core, smoothstep(0.7, 1.0, heat));
}

void fragment() {
	vec4 texel = mix(texture(flipbook, uv_a), texture(flipbook, uv_b), frame_blend);
	// La flamme se refroidit et s'amincit en vieillissant.
	float heat = texel.r * mix(1.0, 0.55, age);
	float coverage = texel.a * COLOR.a * smoothstep(0.0, 0.12, age) * (1.0 - smoothstep(0.65, 1.0, age));
	float depth = texture(depth_texture, SCREEN_UV).r;
	vec4 scene = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	float scene_distance = -scene.z / scene.w;
	float soft = clamp((scene_distance + VERTEX.z) / soft_distance, 0.0, 1.0);
	coverage *= soft;
	vec3 glow = blackbody(heat) * COLOR.rgb * emission * (0.35 + heat * 1.2);
	ALBEDO = glow * coverage;
	ALPHA = coverage * opacity;
}
