// EP8 : ombres de nuages (texture projetée sur le champ par un `Decal`, `BattleCloudShadows`).
// Bruit fbm périodique (la texture se répète sans couture) défilant avec le vent ; sortie noire
// dont l'alpha est la densité de l'ombre. Rendu dans un petit `SubViewport` (256 px), rafraîchi
// quelques fois par seconde.
shader_type canvas_item;

uniform vec2 offset = vec2(0.0);  // décalage du vent, en tuiles
uniform vec2 tiles = vec2(1.0);  // tuiles de bruit sur l'emprise du décal (x, z)
uniform float coverage : hint_range(0.0, 1.0) = 0.5;
uniform float strength : hint_range(0.0, 1.0) = 0.4;

const float PERIOD = 8.0;

float hash(vec2 p) {
	p = mod(p, PERIOD);
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
	for (int i = 0; i < 5; i++) {
		v += a * noise(p);
		p = p * 2.0 + vec2(1.7, 9.2) * float(i);
		a *= 0.5;
	}
	return v;
}

void fragment() {
	vec2 p = (UV * tiles + offset) * PERIOD * 0.5;
	float n = fbm(p) * 0.75 + fbm(p * 0.5 + 3.1) * 0.35;
	float edge = 1.0 - coverage;
	float cloud = smoothstep(edge - 0.08, edge + 0.12, n);
	COLOR = vec4(0.0, 0.0, 0.0, cloud * strength);
}
