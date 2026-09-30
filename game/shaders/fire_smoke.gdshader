// Fumée des incendies et des bombardes (lot V3, A1-13) : planche 8×8 procédurale d'une bouffée
// qui s'étale et se dissipe (`tools/cent_ans_tools/fire_flipbooks.py` : R/A = densité, G =
// éclairage auto-ombré). Éclairée par la scène (soleil, ciel, lueur des foyers) avec un léger
// effet de transmission ; jeune, elle garde la lueur orangée du feu par-dessous. Particules de
// `GPUParticles3D` : INSTANCE_CUSTOM.y = âge (0-1) choisit l'image, .x = angle (rotation des UV).
// Fondu doux au contact du sol et des toits (texture de profondeur).
shader_type spatial;
render_mode blend_mix, cull_disabled, depth_draw_never, specular_disabled, shadows_disabled;

uniform sampler2D flipbook : filter_linear_mipmap, repeat_disable;
uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;
uniform float frames_h = 8.0;
uniform float frames_v = 8.0;
uniform vec3 smoke_color : source_color = vec3(0.16, 0.15, 0.14);
uniform vec3 ember_glow : source_color = vec3(1.0, 0.38, 0.08);
uniform float ember_glow_energy = 0.15;
uniform float density = 1.6;
uniform float soft_distance = 3.0;
// Part de la lumière des foyers (lumières ponctuelles) reçue : la fumée, très diffusante, virerait
// sinon au rose saumon au-dessus des flammes.
uniform float point_light_share = 0.2;
// CR1 : fondu des volutes proches de la caméra (m ; 0 = aucun). En gros plan, une volute de 9 m
// à quelques mètres devenait une grande tache blanche ronde.
uniform float near_fade = 0.0;

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
	mat4 world = mat4(normalize(INV_VIEW_MATRIX[0]), normalize(INV_VIEW_MATRIX[1]), normalize(INV_VIEW_MATRIX[2]), MODEL_MATRIX[3]);
	float angle = INSTANCE_CUSTOM.x;
	mat4 spin = mat4(vec4(cos(angle), -sin(angle), 0.0, 0.0), vec4(sin(angle), cos(angle), 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_MATRIX = VIEW_MATRIX * world * spin * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
	age = clamp(INSTANCE_CUSTOM.y, 0.0, 1.0);
	float total = frames_h * frames_v;
	float f = age * (total - 1.0);
	float first = floor(f);
	frame_blend = f - first;
	uv_a = frame_uv(UV, first);
	uv_b = frame_uv(UV, min(first + 1.0, total - 1.0));
}

void fragment() {
	vec4 texel = mix(texture(flipbook, uv_a), texture(flipbook, uv_b), frame_blend);
	float fade_in = smoothstep(0.0, 0.06, age);
	float alpha = texel.a * COLOR.a * density * fade_in;
	float depth = texture(depth_texture, SCREEN_UV).r;
	vec4 scene = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	float scene_distance = -scene.z / scene.w;
	alpha *= clamp((scene_distance + VERTEX.z) / soft_distance, 0.0, 1.0);
	if (near_fade > 0.0) {
		alpha *= smoothstep(near_fade * 0.3, near_fade, -VERTEX.z);
	}
	float lighting = texel.g;
	ALBEDO = smoke_color * (0.45 + 0.75 * lighting);
	// Lueur du feu sous la fumée jeune (ventre des volutes, côté ombré).
	float glow = pow(1.0 - age, 7.0) * (1.0 - lighting * 0.7);
	EMISSION = ember_glow * glow * ember_glow_energy;
	ROUGHNESS = 1.0;
	ALPHA = clamp(alpha, 0.0, 1.0);
}

void light() {
	// Lambert enveloppé (milieu diffusant) ; lumières ponctuelles atténuées.
	float wrap = clamp((dot(NORMAL, LIGHT) + 0.6) / 1.6, 0.0, 1.0);
	float share = LIGHT_IS_DIRECTIONAL ? 1.0 : point_light_share;
	DIFFUSE_LIGHT += LIGHT_COLOR * ATTENUATION * wrap * share / PI;
}
