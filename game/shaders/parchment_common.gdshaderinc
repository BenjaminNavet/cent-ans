// Lot CM2 : vue stratégique « parchemin enluminé » (Three Kingdoms, Medieval II), fonctions
// communes au terrain (parchment_map.gdshaderinc) et à la mer (parchment_sea.gdshaderinc).
// `campaign_parchment` (paramètre global, `project.godot` § shader_globals) : 0 = carte 3D,
// 1 = parchemin ; piloté par StrategicView selon la distance caméra (fondu, sans coupure).
// Tout est procédural (aucune image) ; couleurs en linéaire.

global uniform float campaign_parchment;

// Papier, encres, lavis.
uniform vec3 pm_paper = vec3(0.80, 0.67, 0.43);
uniform vec3 pm_paper_dark = vec3(0.52, 0.36, 0.17);
uniform vec3 pm_ink = vec3(0.035, 0.020, 0.010);
uniform vec3 pm_red_ink = vec3(0.42, 0.045, 0.025);
uniform vec3 pm_green_ink = vec3(0.07, 0.17, 0.05);
uniform vec3 pm_sea_wash = vec3(0.40, 0.56, 0.54);
// Part de la couleur rendue en émission (la carte ne doit pas être ombrée par le soleil).
uniform float pm_emission = 0.75;
uniform float pm_emission_gain = 0.72;

float pm_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

vec2 pm_hash2(vec2 p) {
	return vec2(pm_hash(p), pm_hash(p + vec2(19.19, 7.13)));
}

float pm_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(pm_hash(i), pm_hash(i + vec2(1.0, 0.0)), u.x),
		mix(pm_hash(i + vec2(0.0, 1.0)), pm_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float pm_fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * pm_noise(p);
		p = p * 2.07 + vec2(13.7, 5.3);
		a *= 0.5;
	}
	return v;
}

float pm_weight() {
	return clamp(campaign_parchment, 0.0, 1.0);
}

// Papier vieilli : taches, fibres, bords roussis (uv carte ; hors carte = marge du parchemin).
vec3 pm_paper_at(vec2 p, vec2 uv, float footprint) {
	float stain = pm_fbm(p * 0.0045 + vec2(3.1, 1.7));
	float blot = smoothstep(0.62, 0.8, pm_fbm(p * 0.011 + vec2(7.3, 2.9)));
	float fibre = pm_noise(p * vec2(0.9, 0.22) / max(footprint, 0.5)) - 0.5;
	vec3 col = pm_paper * (0.88 + 0.2 * stain) * (1.0 + 0.035 * fibre);
	col = mix(col, pm_paper_dark, blot * 0.22);
	// Bords : brunis vers l'extérieur de la carte (roussi), jusqu'à la marge.
	vec2 e = min(uv, 1.0 - uv);
	float edge = min(e.x, e.y);
	float burn = 1.0 - smoothstep(-0.06, 0.07, edge + (pm_noise(p * 0.01) - 0.5) * 0.03);
	col = mix(col, pm_paper_dark * 0.75, burn * 0.7);
	return col;
}

// Cadre à double filet autour de la carte (bords de la heightmap).
float pm_frame(vec2 uv, float footprint, vec2 map_sz) {
	vec2 e = min(uv, 1.0 - uv) * map_sz;
	float d = min(e.x, e.y);
	float aa = footprint;
	float outer = 1.0 - smoothstep(footprint * 1.4 - aa, footprint * 1.4 + aa, abs(d - 14.0));
	float inner = 1.0 - smoothstep(footprint * 0.6 - aa, footprint * 0.6 + aa, abs(d - 22.0));
	return max(outer, inner * 0.8);
}

// Trait d'encre antialiasé : `d` distance en pixels carte, `w` demi-largeur en pixels écran.
float pm_line(float d, float w, float footprint) {
	float ds = abs(d) / max(footprint, 1e-4);
	return 1.0 - smoothstep(w - 0.7, w + 0.7, ds);
}

// Sortie : la couleur du parchemin passe surtout par l'émission (aplat non ombré).
void pm_output(vec3 col, vec3 parch, float w, out vec3 albedo, out vec3 emission) {
	float k = pm_emission * w;
	albedo = mix(col, parch * (1.0 - pm_emission), w);
	emission = parch * k * pm_emission_gain;
}
