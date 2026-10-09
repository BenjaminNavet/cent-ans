// Particules de feu et de fumée (fire_*) : panneau face caméra qui garde l'échelle de la particule
// (billboard de `GPUParticles3D`) et fondu doux au contact de la scène (texture de profondeur).
#ifndef FX_PARTICLE_INC
#define FX_PARTICLE_INC

// Matrice modèle-vue d'un panneau face caméra, tourné de `angle` (radians) dans son plan.
mat4 particle_billboard(mat4 view, mat4 inv_view, mat4 model, float angle) {
	mat4 world = mat4(normalize(inv_view[0]), normalize(inv_view[1]), normalize(inv_view[2]), model[3]);
	mat4 spin = mat4(vec4(cos(angle), -sin(angle), 0.0, 0.0), vec4(sin(angle), cos(angle), 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, 0.0, 1.0));
	return view * world * spin * mat4(
		vec4(length(model[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(model[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(model[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
}

// Facteur 0-1 de fondu au contact : distance entre la scène (profondeur `depth` lue au pixel
// `screen_uv`) et le fragment (`vertex_z`, espace vue), ramenée à `fade_distance`.
float particle_contact_fade(float depth, vec2 screen_uv, mat4 inv_projection, float vertex_z, float fade_distance) {
	vec4 scene = inv_projection * vec4(screen_uv * 2.0 - 1.0, depth, 1.0);
	float scene_distance = -scene.z / scene.w;
	return clamp((scene_distance + vertex_z) / fade_distance, 0.0, 1.0);
}

#endif
