// Lot EP3 : écume au pied d'une pile de pont, entraînée par le courant. Un quad posé sur l'eau,
// son axe long (UV.y de 0 à 1) vers l'aval : sillage en V qui s'élargit et s'efface vers l'aval,
// bouillons près de la pile, filaments qui défilent.
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

uniform float speed = 0.6;
uniform vec3 foam_color : source_color = vec3(0.86, 0.88, 0.86);

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int k = 0; k < 4; k++) {
		v += a * noise(p);
		p *= 2.03;
		a *= 0.5;
	}
	return v;
}

void fragment() {
	// UV.y : 0 au pied de la pile, 1 en aval ; UV.x : travers (0,5 au centre).
	float down = UV.y;
	float across = abs(UV.x - 0.5) * 2.0;
	// Sillage en V : étroit au pied de la pile, ouvert en aval.
	float spread = mix(0.35, 1.0, down);
	float wake = 1.0 - smoothstep(spread * 0.55, spread, across);
	float fade = smoothstep(1.0, 0.7, down) * smoothstep(0.0, 0.08, down);
	float t = TIME * speed;
	float n = fbm(vec2(UV.x * 6.0, down * 5.0 - t * 1.6));
	float streaks = fbm(vec2(UV.x * 14.0, down * 2.0 - t * 2.2));
	float boil = smoothstep(0.35, 0.0, down) * fbm(vec2(UV.x * 9.0 + t * 0.3, down * 9.0 - t));
	float foam = wake * fade * smoothstep(0.42, 0.7, n * 0.65 + streaks * 0.45) + boil * 0.8;
	ALBEDO = foam_color;
	ALPHA = clamp(foam, 0.0, 0.85);
}
