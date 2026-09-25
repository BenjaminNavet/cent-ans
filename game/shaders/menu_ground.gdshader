shader_type spatial;
render_mode diffuse_burley;

// Lot MM1 — campagne autour de la maquette de Paris dans le décor du menu : bras de la Seine
// prolongé (bande d'eau ondulée), berges, puis mosaïque de champs (blé, labours, prés, jachères)
// avec sillons et bruit de détail. Rendu seulement.

uniform float river_z = 170.0;
uniform float river_half_width = 85.0;
uniform float bank_width = 14.0;
uniform vec3 meadow : source_color = vec3(0.22, 0.27, 0.13);
uniform vec3 wheat : source_color = vec3(0.42, 0.37, 0.22);
uniform vec3 ploughed : source_color = vec3(0.27, 0.21, 0.15);
uniform vec3 fallow : source_color = vec3(0.31, 0.31, 0.18);
// Pré forcé (x0, z0, x1, z1) : sous l'ost.
uniform vec4 meadow_zone = vec4(0.0);
uniform vec3 bank : source_color = vec3(0.36, 0.3, 0.22);
uniform vec3 water_color : source_color = vec3(0.3, 0.36, 0.4);

varying vec3 world_pos;

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
	for (int i = 0; i < 4; i++) {
		v += a * noise(p);
		p *= 2.03;
		a *= 0.5;
	}
	return v;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	float center = river_z + sin(p.x * 0.0011) * 38.0 + sin(p.x * 0.0031 + 1.7) * 12.0;
	float d = abs(p.y - center);
	if (d < river_half_width) {
		float t = TIME * 0.35;
		vec2 w = vec2(noise(p * 0.08 + vec2(t, 0.0)), noise(p * 0.08 + vec2(0.0, t) + 7.3)) - 0.5;
		NORMAL = normalize(NORMAL + (VIEW_MATRIX * vec4(w.x * 0.07, 0.0, w.y * 0.07, 0.0)).xyz);
		ALBEDO = water_color;
		ROUGHNESS = 0.06;
		SPECULAR = 0.7;
		METALLIC = 0.0;
	} else {
		// Parcelles : grille tournée et déformée, une culture par parcelle.
		vec2 q = mat2(vec2(0.94, 0.34), vec2(-0.34, 0.94)) * p;
		q += vec2(fbm(p * 0.004), fbm(p * 0.004 + 3.1)) * 90.0;
		vec2 cell = floor(q / vec2(170.0, 95.0));
		float pick = hash(cell);
		vec3 crop = pick < 0.3 ? meadow : pick < 0.55 ? wheat : pick < 0.78 ? ploughed : fallow;
		// Sillons dans le sens de la parcelle.
		float furrow = 0.5 + 0.5 * sin(q.y * (hash(cell + 3.0) < 0.5 ? 2.1 : 0.0) + q.x * (hash(cell + 3.0) < 0.5 ? 0.0 : 2.1));
		float furrow_fade = 1.0 - smoothstep(0.3, 1.2, fwidth(q.x + q.y) * 2.1);
		crop *= mix(1.0, 0.9 + 0.1 * furrow, (pick > 0.3 && pick < 0.78 ? 1.0 : 0.25) * furrow_fade);
		// Haies et chemins entre les parcelles.
		vec2 f = fract(q / vec2(170.0, 95.0));
		float edge = min(min(f.x, 1.0 - f.x) * 170.0, min(f.y, 1.0 - f.y) * 95.0);
		crop = mix(meadow * 0.85, crop, smoothstep(0.6, 2.5, edge));
		vec2 in_zone = step(meadow_zone.xy, p) * step(p, meadow_zone.zw);
		float zone = in_zone.x * in_zone.y;
		crop = mix(crop, meadow * (0.9 + 0.2 * fbm(p * 0.02)), zone);
		float detail = fbm(p * 0.35) * 0.5 + fbm(p * 0.03) * 0.5;
		crop *= 0.78 + 0.44 * detail;
		float shore = smoothstep(river_half_width, river_half_width + bank_width, d);
		ALBEDO = mix(bank, crop, shore);
		ROUGHNESS = 0.95;
		SPECULAR = 0.2;
	}
}
