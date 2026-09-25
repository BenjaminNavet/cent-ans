// Sang au sol (lot BV1) : décalques posés à plat sur le terrain (MultiMesh de quadrilatères
// 1 × 1 m dans le plan XZ, transformée d'instance = position, orientation, taille et inclinaison
// du sol). INSTANCE_CUSTOM = (instant d'apparition, sorte, graine, intensité) ;
// sorte 0 flaque (s'étale), 1 éclaboussures (gouttes), 2 traînée (étirée par la transformée).
// Le sang frais, rouge et luisant, brunit et se mate en séchant.
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, specular_schlick_ggx;

uniform float time_now = 0.0;
uniform vec3 fresh_color : source_color = vec3(0.25, 0.008, 0.012);
uniform vec3 dry_color : source_color = vec3(0.09, 0.03, 0.02);

varying vec2 v_uv;
varying vec4 v_custom;

float hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash21(i), hash21(i + vec2(1, 0)), f.x), mix(hash21(i + vec2(0, 1)), hash21(i + vec2(1, 1)), f.x), f.y);
}

void vertex() {
	v_custom = INSTANCE_CUSTOM;
	v_uv = VERTEX.xz * 2.0;  // -1..1
	float age = time_now - INSTANCE_CUSTOM.x;
	float kind = INSTANCE_CUSTOM.y;
	// Une flaque s'étale pendant une vingtaine de secondes ; le reste apparaît d'un coup.
	float grow = kind < 0.5 ? mix(0.3, 1.0, smoothstep(0.0, 20.0, age)) : 1.0;
	float shown = age >= 0.0 ? 1.0 : 0.0;
	VERTEX.xz *= grow * shown;
	VERTEX.y += 0.035;
}

void fragment() {
	float age = time_now - v_custom.x;
	float kind = v_custom.y;
	vec2 seed = vec2(v_custom.z, v_custom.z * 1.7);
	vec2 uv = v_uv;
	float shape;
	if (kind < 0.5) {
		// Flaque : tache irrégulière, bord déchiqueté.
		float r = length(uv);
		float n = noise(uv * 2.2 + seed) * 0.55 + noise(uv * 6.0 + seed * 3.1) * 0.25;
		shape = 1.0 - smoothstep(0.62, 0.78, r + n * 0.45 - 0.2);
	} else if (kind < 1.5) {
		// Éclaboussures : une tache centrale et des gouttes semées autour.
		shape = 1.0 - smoothstep(0.18, 0.26, length(uv) + noise(uv * 5.0 + seed) * 0.1);
		for (int k = 0; k < 9; k++) {
			vec2 c = vec2(hash21(seed + float(k)), hash21(seed * 1.3 + float(k) * 2.1)) * 1.6 - 0.8;
			float rad = 0.04 + 0.08 * hash21(seed + float(k) * 7.7);
			shape = max(shape, 1.0 - smoothstep(rad * 0.7, rad, length(uv - c)));
		}
	} else {
		// Traînée : bande étirée en x (la transformée l'allonge), stries, bout effilé.
		float band = abs(uv.y) + noise(vec2(uv.x * 3.0, uv.y * 12.0) + seed) * 0.35;
		float taper = mix(0.55, 0.15, smoothstep(-1.0, 1.0, uv.x));
		shape = (1.0 - smoothstep(taper * 0.7, taper, band)) * (1.0 - smoothstep(0.8, 1.0, abs(uv.x)));
	}
	float dry = smoothstep(4.0, 90.0, age);
	ALBEDO = mix(fresh_color, dry_color, dry) * mix(1.0, 0.8, noise(uv * 9.0 + seed));
	// Luisant mais pas miroir : un reflet trop net du ciel rendait le sang rose.
	ROUGHNESS = mix(0.38, 0.8, dry);
	SPECULAR = mix(0.3, 0.15, dry);
	ALPHA = clamp(shape, 0.0, 1.0) * mix(0.92, 0.82, dry) * v_custom.w;
}
