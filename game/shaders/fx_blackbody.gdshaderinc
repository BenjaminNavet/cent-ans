// Chaleur (0-1) -> couleur de corps noir des flammes (rouge sombre, orange, jaune, cœur presque
// blanc), partagée par fire_flame et life_flame.
#ifndef FX_BLACKBODY_INC
#define FX_BLACKBODY_INC

uniform vec3 color_cold : source_color = vec3(0.35, 0.03, 0.0);
uniform vec3 color_mid : source_color = vec3(1.0, 0.32, 0.04);
uniform vec3 color_hot : source_color = vec3(1.0, 0.72, 0.28);
uniform vec3 color_core : source_color = vec3(1.0, 0.93, 0.78);

vec3 blackbody(float heat) {
	vec3 c = mix(color_cold, color_mid, smoothstep(0.0, 0.35, heat));
	c = mix(c, color_hot, smoothstep(0.35, 0.7, heat));
	return mix(c, color_core, smoothstep(0.7, 1.0, heat));
}

#endif
