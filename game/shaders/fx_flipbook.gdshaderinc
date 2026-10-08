// Planche d'images (flipbook) des effets de feu et de fumée (fire_*, life_*) : uniformes de la
// planche, adresse d'une image dans l'atlas et fondu entre deux images voisines.
#ifndef FX_FLIPBOOK_INC
#define FX_FLIPBOOK_INC

uniform sampler2D flipbook : filter_linear_mipmap, repeat_disable;
uniform float frames_h = 8.0;
uniform float frames_v = 8.0;

// UV (0-1 dans l'image) -> UV dans l'atlas de l'image `frame`.
vec2 frame_uv(vec2 uv, float frame) {
	float column = mod(frame, frames_h);
	float row = floor(frame / frames_h);
	return (vec2(column, row) + uv) / vec2(frames_h, frames_v);
}

// Lecture en boucle : `phase` (tours, partie fractionnaire utilisée) -> (image courante, image
// suivante avec retour au début après la dernière, poids de la suivante).
vec3 flipbook_loop(float phase) {
	float total = frames_h * frames_v;
	float f = fract(phase) * total;
	float first = floor(f);
	return vec3(first, mod(first + 1.0, total), f - first);
}

// Lecture sans boucle : `f` = position fractionnaire dans [0, total - 1] ; la dernière image
// n'a pas de suivante (elle reste la sienne). Même sortie que `flipbook_loop`.
vec3 flipbook_clamped(float f) {
	float first = floor(f);
	return vec3(first, min(first + 1.0, frames_h * frames_v - 1.0), f - first);
}

vec4 flipbook_texel(vec2 uv_a, vec2 uv_b, float blend) {
	return mix(texture(flipbook, uv_a), texture(flipbook, uv_b), blend);
}

#endif
