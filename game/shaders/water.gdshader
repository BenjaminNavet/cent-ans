shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled;

// Mer (lot V2) : plan à Y = 0. Couleur et opacité selon la profondeur (heightmap), vagues par
// normales animées (estompées au dézoom pour éviter le moiré), fresnel, reflet du soleil
// borné (pas de tache blanche au dézoom), écume douce le long des côtes (distance à la côte).

uniform sampler2D heightmap : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform int height_bpp = 1;
uniform bool height_little_endian = false;
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;
uniform vec2 map_size = vec2(4096.0, 4096.0);
uniform bool has_heightmap = false;
uniform sampler2D coast_dist : filter_linear, repeat_disable, hint_default_black;
uniform bool has_coast_dist = false;
uniform float coast_dist_scale = 2.0;

// Couleurs linéaires.
uniform vec3 shallow_color = vec3(0.045, 0.14, 0.14);
uniform vec3 deep_color = vec3(0.006, 0.028, 0.050);
uniform vec3 foam_color = vec3(0.62, 0.66, 0.66);
uniform float depth_falloff_m = 70.0;
uniform float alpha_shallow = 0.7;
uniform float alpha_deep = 0.97;
uniform float wave_scale = 0.6;      // vagues par pixel carte
uniform float wave_speed = 0.35;
uniform float wave_strength = 0.08;
uniform float foam_px = 1.4;          // largeur de la bande d'écume (pixels carte)
uniform float roughness_near = 0.3;
uniform float roughness_far = 0.55;
uniform float specular_amount = 0.06;
// Reflet du soleil (modèle de Blinn-Phong borné, remplace le spéculaire par défaut).
uniform float sun_spec_power = 180.0;
uniform float sun_spec_intensity = 0.1;
uniform float sun_spec_max = 0.1;

varying float v_far;

varying vec3 world_pos;

float height_m(vec2 uv) {
	if (height_bpp == 2) {
		vec4 t = texture(heightmap, uv);
		float v = height_little_endian ? (t.a * 255.0 * 256.0 + t.r * 255.0) : (t.r * 255.0 * 256.0 + t.a * 255.0);
		return mix(height_min_m, height_max_m, v / 65535.0);
	}
	return mix(height_min_m, height_max_m, texture(heightmap, uv).r);
}

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash12(i), hash12(i + vec2(1.0, 0.0)), u.x),
		mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Hauteur de houle (somme de trains d'ondes orientés + bruit), en unités arbitraires.
float swell(vec2 p, float t) {
	float h = 0.0;
	h += sin(dot(p, vec2(0.82, 0.57)) * 1.0 + t * 1.1) * 0.5;
	h += sin(dot(p, vec2(-0.35, 0.94)) * 1.7 - t * 1.4) * 0.3;
	h += sin(dot(p, vec2(0.99, -0.12)) * 2.9 + t * 2.1) * 0.15;
	h += (value_noise(p * 1.3 + t * 0.3) - 0.5) * 0.6;
	return h;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = (p + vec2(0.5)) / map_size;
	float footprint = max(length(dFdx(p)), length(dFdy(p)));
	float far_t = smoothstep(0.6, 2.5, footprint);
	v_far = far_t;

	bool inside = all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
	float depth = (has_heightmap && inside) ? max(-height_m(uv), 0.0) : 200.0;
	float depth_t = 1.0 - exp(-depth / depth_falloff_m);
	vec3 col = mix(shallow_color, deep_color, depth_t);
	float alpha = mix(alpha_shallow, alpha_deep, smoothstep(0.0, 45.0, depth));

	// Vagues : normale par différences finies de la houle, estompée au dézoom.
	float t = TIME * wave_speed;
	vec2 q = p * wave_scale;
	float e = 0.15;
	float h0 = swell(q, t);
	float hx = swell(q + vec2(e, 0.0), t) - h0;
	float hz = swell(q + vec2(0.0, e), t) - h0;
	float ws = wave_strength * (1.0 - smoothstep(0.15, 0.8, footprint));
	vec3 n = normalize(vec3(-hx / e * ws, 1.0, -hz / e * ws));
	// Creux et crêtes légèrement contrastés dans la couleur (lisibles sans reflet du soleil).
	col *= 1.0 + h0 * 0.06 * (1.0 - smoothstep(0.25, 1.2, footprint));

	// Écume : bande près du rivage, lignes qui avancent vers la côte, cassées par le bruit.
	float coast = has_coast_dist && inside ? (texture(coast_dist, uv).r * 255.0 - 128.0) / coast_dist_scale : -50.0;
	float off = max(-coast, 0.0);
	float band = 1.0 - smoothstep(0.0, foam_px, off);
	float lines = smoothstep(0.55, 0.95, sin(off * 5.5 + TIME * 1.6 + value_noise(p * 0.8) * 4.0) * 0.5 + 0.5);
	float broken = smoothstep(0.35, 0.75, value_noise(p * 1.6 + vec2(TIME * 0.05, 0.0)));
	float foam = band * mix(0.35 + 0.65 * lines * broken, 0.6, far_t);
	foam *= 1.0 - smoothstep(foam_px * 0.2, foam_px, off) * 0.5;
	col = mix(col, foam_color, foam * 0.55);
	alpha = mix(alpha, 0.9, foam * 0.5);
	// Côte précise (raster) plutôt que l'intersection avec le maillage (LOD grossier) : l'eau
	// s'efface sur la terre même si la géométrie y passe sous le niveau de la mer.
	alpha *= 1.0 - smoothstep(-0.2, 0.6, coast);

	// Fresnel discret : le large reflète un peu plus le ciel en vue rasante.
	float fres = pow(1.0 - clamp(dot(normalize(VIEW), NORMAL), 0.0, 1.0), 5.0);
	col += vec3(0.02, 0.035, 0.045) * fres;

	ALBEDO = col;
	ALPHA = alpha;
	ROUGHNESS = mix(mix(roughness_near, roughness_far, far_t), 0.8, foam);
	SPECULAR = specular_amount * (1.0 - foam);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}

void light() {
	float ndl = max(dot(NORMAL, LIGHT), 0.0);
	DIFFUSE_LIGHT += ndl * ATTENUATION * LIGHT_COLOR / PI;
	vec3 h = normalize(VIEW + LIGHT);
	float power = mix(sun_spec_power, sun_spec_power * 0.25, v_far);
	float spec = pow(max(dot(NORMAL, h), 0.0), power) * sun_spec_intensity * (1.0 - 0.7 * v_far);
	SPECULAR_LIGHT += min(spec, sun_spec_max) * ATTENUATION * LIGHT_COLOR / PI;
}
