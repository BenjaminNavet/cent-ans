// Lot CV1 : panaches de fumée de la carte de campagne (cheminées, incendies), instanciés par
// MultiMesh (life_effects.gd). Quad vertical (x ∈ [-0.5, 0.5], y ∈ [0, 1]) tourné vers la
// caméra autour de l'axe vertical, courbé par le vent ; densité en bruit qui monte.
// INSTANCE_CUSTOM : r graine, g noirceur (0 cheminée, 1 incendie), b opacité, a vitesse.
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled, fog_disabled;

uniform vec2 wind = vec2(0.8, 0.35);
uniform vec3 light_smoke : source_color = vec3(0.78, 0.76, 0.72);
uniform vec3 dark_smoke : source_color = vec3(0.10, 0.09, 0.085);
uniform vec3 ember : source_color = vec3(1.0, 0.45, 0.12);
// Fondu global (palier de zoom), piloté par le script.
uniform float fade = 1.0;
// Lot SZ4 : échelle du panache (1 au loin, taille réelle au palier vallée, `MapPropScale`),
// pilotée par le script. L'origine de l'instance est au sol ; la levée au-dessus du sol (sommet de
// la maquette, toit du hameau) est dans la colonne z de la base (`MODEL_MATRIX[2].y`) et suit
// l'échelle, comme la largeur et la hauteur.
uniform float prop_scale = 1.0;
// Lumière ambiante approximative (unshaded) : teinte chaude de la carte.
uniform vec3 sun_tint = vec3(1.0, 0.96, 0.9);

// LR-14 : panaches d'incendie en bouffées de la planche de fumée libre FA (WispySmoke03, CC0) au
// lieu du bruit procédural (traits noirs droits vus de dessus). Désactivé si `use_flipbook` = faux.
uniform bool use_flipbook = false;
uniform sampler2D flipbook : filter_linear_mipmap, repeat_disable;
uniform float frames_h = 8.0;
uniform float frames_v = 8.0;
// Hauteur / largeur du quad (corrige l'aspect des bouffées).
uniform float aspect = 2.2;
uniform int puffs = 6;

varying vec4 v_custom;
varying vec2 v_uv;

float hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise2(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), f.x), mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), f.x), f.y);
}

void vertex() {
	v_custom = INSTANCE_CUSTOM;
	v_uv = vec2(VERTEX.x + 0.5, VERTEX.y);
	vec3 origin = MODEL_MATRIX[3].xyz;
	float sx = length(MODEL_MATRIX[0].xyz) * prop_scale;
	float sy = length(MODEL_MATRIX[1].xyz) * prop_scale;
	origin.y += MODEL_MATRIX[2].y * prop_scale;
	// Face à la caméra : axe x de la caméra, axe « haut » entre la verticale et le haut de
	// l'écran (vu de dessus, le panache reste un ruban large, pas un trait).
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	right = normalize(vec3(right.x, 0.0, right.z) + vec3(1e-4, 0.0, 0.0));
	vec3 up = normalize(mix(vec3(0.0, 1.0, 0.0), INV_VIEW_MATRIX[1].xyz, 0.55));
	float t = VERTEX.y;
	// Le panache s'élargit et se couche avec le vent en montant.
	float widen = mix(0.45, 1.0, sqrt(t));
	float sway = sin(TIME * 0.7 + v_custom.r * 40.0 + t * 3.0) * 0.08 * t;
	vec3 world = origin + right * VERTEX.x * sx * widen
		+ up * t * sy
		+ vec3(wind.x, 0.0, wind.y) * (t * t * sy * 0.45)
		+ right * sway * sx;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
}

vec4 puff_sample(vec2 uv, float frame) {
	float f = clamp(frame, 0.0, frames_h * frames_v - 1.001);
	float first = floor(f);
	float next = min(first + 1.0, frames_h * frames_v - 1.0);
	vec2 a = (vec2(mod(first, frames_h), floor(first / frames_h)) + uv) / vec2(frames_h, frames_v);
	vec2 b = (vec2(mod(next, frames_h), floor(next / frames_h)) + uv) / vec2(frames_h, frames_v);
	return mix(texture(flipbook, a), texture(flipbook, b), f - first);
}

void flipbook_plume(float seed, float dark) {
	float speed = mix(0.08, 0.14, v_custom.a);
	float total = 0.0;
	float light = 0.0;
	for (int k = 0; k < puffs; k++) {
		float phase = fract(TIME * speed + float(k) / float(puffs) + seed * 7.0);
		float cy = 0.06 + phase * 0.82;
		float cx = 0.5 + sin(seed * 31.0 + float(k) * 2.4) * 0.12 * (0.4 + phase);
		float half_t = 0.10 + 0.16 * phase;
		float half_x = min(half_t * aspect, 0.5);
		vec2 local = vec2((v_uv.x - cx) / (2.0 * half_x) + 0.5, 1.0 - ((v_uv.y - cy) / (2.0 * half_t) + 0.5));
		if (local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0) {
			continue;
		}
		vec4 texel = puff_sample(local, phase * (frames_h * frames_v - 1.0));
		float life = smoothstep(0.0, 0.08, phase) * (1.0 - smoothstep(0.6, 1.0, phase));
		float a = texel.a * life;
		total += a;
		light += texel.g * a;
	}
	float alpha = clamp(total * 0.9, 0.0, 1.0) * v_custom.b * fade;
	if (alpha < 0.01) {
		discard;
	}
	float lit = total > 0.001 ? light / total : 0.5;
	vec3 col = mix(light_smoke, dark_smoke * 2.2, dark) * sun_tint * (0.6 + 0.6 * lit);
	col += ember * dark * (1.0 - smoothstep(0.0, 0.12, v_uv.y)) * (0.6 + 0.4 * sin(TIME * 7.0 + seed * 30.0));
	ALBEDO = col;
	ALPHA = alpha;
}

void fragment() {
	if (use_flipbook) {
		flipbook_plume(v_custom.r, v_custom.g);
		return;
	}
	float seed = v_custom.r;
	float dark = v_custom.g;
	float speed = mix(0.25, 0.45, v_custom.a);
	vec2 uv = v_uv;
	float t = uv.y;
	// Bruit qui monte : deux octaves décalées.
	vec2 q = vec2(uv.x * 1.2, t * 2.2 - TIME * speed) + seed * 17.0;
	float n = noise2(q * 1.6) * 0.6 + noise2(q * 3.7 + 5.0) * 0.4;
	// Colonne : bouffées rondes, cœur au centre, s'estompe en haut et sur les bords.
	float dx = abs(uv.x - 0.5) * 2.0;
	float core = 1.0 - smoothstep(0.25, 1.0, dx + (n - 0.5) * 0.6);
	float body = smoothstep(0.0, 0.1, t) * (1.0 - smoothstep(0.4, 1.0, t));
	float density = clamp((n - 0.25 + 0.3 * (1.0 - t)) * 1.8, 0.0, 1.0) * core * body;
	float alpha = density * v_custom.b * fade;
	if (alpha < 0.01) {
		discard;
	}
	vec3 col = mix(light_smoke, dark_smoke, dark) * sun_tint * mix(0.85, 1.05, n);
	// Lueur de braises au pied des incendies.
	col += ember * dark * (1.0 - smoothstep(0.0, 0.12, t)) * (0.6 + 0.4 * sin(TIME * 7.0 + seed * 30.0));
	ALBEDO = col;
	ALPHA = alpha;
}
