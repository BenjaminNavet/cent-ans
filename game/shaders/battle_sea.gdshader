// Mer des batailles côtières (lot B5) : plan d'eau opaque, houle à deux échelles en coordonnées
// monde (pas de ruban comme la rivière), reflet de Fresnel de la couleur du ciel qui s'efface au
// loin dans la brume, frange d'écume et eau plus claire le long du rivage (`shore_x`, `flank`).
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform sampler2D wave_normal : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
uniform vec3 deep_color : source_color = vec3(0.05, 0.13, 0.17);
uniform vec3 shore_color : source_color = vec3(0.2, 0.33, 0.32);
uniform vec3 sky_color : source_color = vec3(0.62, 0.72, 0.82);
uniform float shore_x = 0.0;
// -1 : mer à l'ouest (x < shore_x), +1 : à l'est.
uniform float flank = -1.0;
uniform float ripple : hint_range(0.0, 1.0) = 0.0;

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	float t = TIME;
	vec3 n1 = texture(wave_normal, p * 0.03 + vec2(t * 0.012, t * 0.004)).rgb * 2.0 - 1.0;
	vec3 n2 = texture(wave_normal, p * 0.011 + vec2(-t * 0.004, t * 0.007)).rgb * 2.0 - 1.0;
	vec3 n = normalize(n1 + n2 * 1.3 + vec3(0.0, 0.0, 2.2));
	if (ripple > 0.0) {
		vec3 n3 = texture(wave_normal, p * 0.35 + vec2(t * 0.9, -t * 0.7)).rgb * 2.0 - 1.0;
		n = normalize(n + n3 * ripple * 0.6);
	}
	// Au loin, la houle se fond (évite le moirage des normales en incidence rasante).
	float dist = length(VERTEX);
	float calm = smoothstep(600.0, 3000.0, dist);
	n = normalize(mix(n, vec3(0.0, 0.0, 1.0), calm * 0.8));
	NORMAL_MAP = n * 0.5 + 0.5;
	NORMAL_MAP_DEPTH = 0.5;

	// Distance au rivage, vers le large.
	float offshore = max((world_pos.x - shore_x) * flank, 0.0);
	float near_shore = 1.0 - smoothstep(0.0, 90.0, offshore);
	float macro = texture(macro_noise, p * 0.0015).r;
	vec3 water = mix(deep_color, shore_color, near_shore * 0.8) * mix(0.85, 1.15, macro);
	float fresnel = pow(1.0 - clamp(dot(normalize(-VERTEX), NORMAL), 0.0, 1.0), 4.0);
	vec3 albedo = mix(water, sky_color * 0.8, clamp(fresnel * 0.75 + calm * 0.25, 0.0, 0.85));
	// Écume : liseré brisé contre la plage.
	float foam_band = 1.0 - smoothstep(0.0, 6.0 + macro * 6.0, offshore);
	float foam_noise = texture(macro_noise, p * 0.08 + vec2(t * 0.02, 0.0)).r;
	float foam = foam_band * smoothstep(0.4, 0.7, foam_noise + foam_band * 0.3);
	albedo = mix(albedo, vec3(0.86, 0.88, 0.86), foam * 0.8);
	ALBEDO = albedo;
	ROUGHNESS = mix(0.08, 0.6, foam);
	SPECULAR = 0.5;
}
