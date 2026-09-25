shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Parapets et créneaux des châteaux, pavois (lot NV2) : planches peintes aux couleurs du camp,
// patinées. Panneaux alternés couleur du camp / couleur d'accent, liseré ocre au sommet,
// peinture passée au soleil et au sel, écaillée sur les arêtes (le chêne reparaît), coulures
// de crasse et de sel, bas plus sale. Espace objet (1 unité = 1 m), sans texture ni UV.

uniform vec3 paint : source_color = vec3(0.2, 0.3, 0.7);
uniform vec3 accent : source_color = vec3(0.8, 0.7, 0.3);
uniform vec3 oak : source_color = vec3(0.3, 0.2, 0.12);
uniform float panel_width = 0.9;
uniform float fade = 0.25;
uniform float chipping = 0.45;
uniform float seed = 0.0;
uniform float charred = 0.0;
uniform bool panels = true;

varying vec3 obj_pos;
varying vec3 obj_normal;

void vertex() {
	obj_pos = VERTEX;
	obj_normal = NORMAL;
}

float hash1(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash1(i), hash1(i + vec2(1.0, 0.0)), u.x),
		mix(hash1(i + vec2(0.0, 1.0)), hash1(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int k = 0; k < 4; k++) {
		v += a * noise(p);
		p = p * 2.03 + vec2(17.1, 9.7);
		a *= 0.5;
	}
	return v;
}

void fragment() {
	vec3 p = obj_pos;
	vec3 n = normalize(abs(obj_normal));
	float s = seed * 5.31;
	// Coordonnée horizontale le long de la face (x ou z selon l'orientation), hauteur y.
	float h = n.x > n.z ? p.z : p.x;
	vec2 q = vec2(h, p.y);
	// Planches horizontales (15 cm) sous la peinture.
	float board = fract(q.y / 0.15);
	float seam = smoothstep(0.0, 0.08, board) * smoothstep(1.0, 0.92, board);
	float grain = fbm(vec2(q.x * 1.2, q.y * 30.0) + s);
	// Panneaux alternés et liseré ocre en haut de chaque face.
	float panel = panels ? mod(floor(q.x / panel_width + 0.5), 2.0) : 0.0;
	vec3 colour = mix(paint, mix(paint, accent, 0.75), panel);
	float top = fract(p.y / 1.1);
	colour = mix(colour, accent * 0.9, smoothstep(0.86, 0.9, top) * 0.8);
	// Passée : désaturée et blanchie par le soleil et le sel, par plaques.
	float weather = fbm(q * 1.7 + s);
	float grey = dot(colour, vec3(0.3, 0.59, 0.11));
	colour = mix(colour, vec3(grey) * 1.1 + vec3(0.04, 0.035, 0.02), fade * (0.6 + 0.6 * weather));
	// Écaillée : le chêne reparaît sur les arêtes des planches et par éclats.
	float chips = smoothstep(0.62 - chipping * 0.25, 0.7 - chipping * 0.25, fbm(q * 7.0 + s * 3.0));
	chips = max(chips, (1.0 - seam) * 0.8 * chipping);
	vec3 wood = oak * (0.75 + 0.5 * grain);
	vec3 albedo = mix(colour * (0.9 + 0.15 * grain), wood, clamp(chips, 0.0, 1.0));
	albedo *= mix(0.6, 1.0, seam);
	// Coulures de crasse et de sel, bas des faces plus sale.
	float streak = smoothstep(0.55, 0.85, fbm(vec2(q.x * 5.0, q.y * 0.5) + s));
	albedo *= 1.0 - streak * 0.25;
	albedo = mix(albedo, vec3(0.8, 0.78, 0.72), smoothstep(0.75, 0.95, fbm(vec2(q.x * 9.0, q.y * 2.0) - s)) * 0.12);
	// Brûlures : plaques noircies qui gagnent avec le feu subi.
	float burn = charred * smoothstep(1.0 - charred, 1.15 - charred, fbm(p.xy * 0.9 + p.zy * 0.7 + seed));
	albedo = mix(albedo, vec3(0.025, 0.02, 0.018), clamp(burn, 0.0, 0.95));
	ALBEDO = albedo;
	ROUGHNESS = mix(0.62, 0.85, chips) + 0.08 * weather;
	SPECULAR = 0.45;
}
