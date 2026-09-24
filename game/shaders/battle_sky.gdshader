// Ciel des batailles (lot V4) : dégradé atmosphérique, disque et halo du soleil (lumière 0),
// nuages procéduraux (fbm projeté sur une voûte) dont la couverture suit la météo. Statique
// (pas de TIME) pour que la carte de radiance ne soit calculée qu'une fois.
shader_type sky;

uniform vec3 zenith_color : source_color = vec3(0.23, 0.42, 0.72);
uniform vec3 horizon_color : source_color = vec3(0.72, 0.8, 0.88);
uniform vec3 ground_color : source_color = vec3(0.36, 0.38, 0.32);
uniform vec3 cloud_light : source_color = vec3(1.0, 0.98, 0.95);
uniform vec3 cloud_shadow : source_color = vec3(0.55, 0.6, 0.68);
uniform float cloud_coverage : hint_range(0.0, 1.0) = 0.45;
uniform float cloud_softness : hint_range(0.01, 1.0) = 0.25;
uniform float sun_disc : hint_range(0.0, 1.0) = 1.0;
uniform float exposure = 1.0;
uniform vec2 cloud_offset = vec2(0.0);

float hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
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
	mat2 r = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	for (int i = 0; i < 6; i++) {
		v += a * noise(p);
		p = r * p * 2.03 + vec2(1.7, 9.2);
		a *= 0.5;
	}
	return v;
}

void sky() {
	vec3 dir = EYEDIR;
	float h = dir.y;
	vec3 sun_dir = LIGHT0_DIRECTION;
	float sun_dot = max(dot(dir, sun_dir), 0.0);

	vec3 col = mix(horizon_color, zenith_color, pow(clamp(h, 0.0, 1.0), 0.55));
	// Halo de Mie autour du soleil et éclaircissement de l'horizon côté soleil.
	col += LIGHT0_COLOR * (pow(sun_dot, 8.0) * 0.18 + pow(sun_dot, 64.0) * 0.35) * (1.0 - cloud_coverage * 0.7);
	col = mix(col, horizon_color * 1.08, pow(1.0 - clamp(h, 0.0, 1.0), 6.0) * 0.5);

	if (h > 0.0) {
		// Voûte nuageuse : projection sur un plan à hauteur fixe.
		vec2 uv = dir.xz / (h + 0.12) * 1.6 + cloud_offset;
		float n = fbm(uv);
		float n2 = fbm(uv * 2.7 + 5.0);
		float cover = smoothstep(1.0 - cloud_coverage - cloud_softness * 0.5, 1.0 - cloud_coverage + cloud_softness, n * 0.8 + n2 * 0.25);
		// Éclairage des nuages : bord lumineux côté soleil, ventre plus sombre.
		float light = clamp(0.55 + (n2 - 0.5) * 1.2 + pow(sun_dot, 4.0) * 0.4, 0.0, 1.0);
		vec3 cloud = mix(cloud_shadow, cloud_light, light);
		float fade = smoothstep(0.0, 0.18, h);
		col = mix(col, cloud, cover * fade);
		// Soleil (disque) masqué par les nuages.
		if (!AT_CUBEMAP_PASS) {
			float disc = smoothstep(0.9994, 0.9998, sun_dot) * sun_disc * (1.0 - cover);
			col += LIGHT0_COLOR * disc * 12.0;
		}
	} else {
		col = mix(horizon_color, ground_color, smoothstep(0.0, -0.25, h));
	}
	COLOR = col * exposure;
}
