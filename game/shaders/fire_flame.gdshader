// Flammes des incendies de siège (lot V3, A1-13) : planche 8×8 procédurale
// (`tools/cent_ans_tools/fire_flipbooks.py` : RGB = chaleur, A = couverture), deux images mêlées
// pour un mouvement continu, chaleur convertie en couleur de corps noir (rouge sombre → orange →
// jaune → cœur presque blanc) émise en HDR (le halo s'en empare), alpha prémultiplié : la flamme
// éclaire sans virer au crème comme un mélange additif. Particules de `GPUParticles3D` :
// INSTANCE_CUSTOM.y = âge (0-1), .z = phase d'animation, .x = angle. Fondu doux au contact des
// murs et du sol (texture de profondeur).
shader_type spatial;
render_mode blend_premul_alpha, unshaded, cull_disabled, depth_draw_never, shadows_disabled, fog_disabled;

uniform sampler2D depth_texture : hint_depth_texture, filter_nearest;
uniform float emission = 3.2;
uniform float opacity = 0.55;
uniform float soft_distance = 1.5;

#include "res://shaders/fx_flipbook.gdshaderinc"
#include "res://shaders/fx_blackbody.gdshaderinc"
#include "res://shaders/fx_particle.gdshaderinc"

varying vec2 uv_a;
varying vec2 uv_b;
varying float frame_blend;
varying float age;

void vertex() {
	MODELVIEW_MATRIX = particle_billboard(VIEW_MATRIX, INV_VIEW_MATRIX, MODEL_MATRIX, 0.0);
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
	vec3 frames = flipbook_loop(INSTANCE_CUSTOM.z);
	uv_a = frame_uv(UV, frames.x);
	uv_b = frame_uv(UV, frames.y);
	frame_blend = frames.z;
	age = INSTANCE_CUSTOM.y;
}

void fragment() {
	vec4 texel = flipbook_texel(uv_a, uv_b, frame_blend);
	// La flamme se refroidit et s'amincit en vieillissant.
	float heat = texel.r * mix(1.0, 0.55, age);
	float coverage = texel.a * COLOR.a * smoothstep(0.0, 0.12, age) * (1.0 - smoothstep(0.65, 1.0, age));
	float depth = texture(depth_texture, SCREEN_UV).r;
	coverage *= particle_contact_fade(depth, SCREEN_UV, INV_PROJECTION_MATRIX, VERTEX.z, soft_distance);
	vec3 glow = blackbody(heat) * COLOR.rgb * emission * (0.35 + heat * 1.2);
	ALBEDO = glow * coverage;
	ALPHA = coverage * opacity;
}
