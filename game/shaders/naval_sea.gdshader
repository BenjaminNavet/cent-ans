// Mer des batailles navales (lot NV1) : houle à quatre trains de vagues (mêmes formules que
// `NavalWaves`, les coques la suivent), déplacement des sommets près de la caméra, normales
// analytiques plus deux couches de rides animées, couleur selon l'épaisseur d'eau vue en
// incidence, reflet de Fresnel du ciel, écume sur les crêtes, étrave et sillage de chaque navire
// (`ships` : x, z, cap, longueur ; `ship_motion` : vitesse, largeur), liseré de rivage
// (`shore_x`) pour un estuaire. Étend l'idée de `battle_sea.gdshader` (B5) à la haute mer.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

#define MAX_SHIPS 40

uniform sampler2D wave_normal : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
uniform vec4 waves[4];
uniform float omegas[4];
uniform float phases[4];
uniform float sea_time = 0.0;
// Centre du déplacement (cible de la caméra) et rayon au-delà duquel la houle s'aplatit.
uniform vec2 displace_center = vec2(0.0);
uniform float displace_radius = 900.0;
uniform bool displace = true;
uniform vec3 deep_color : source_color = vec3(0.035, 0.1, 0.13);
uniform vec3 crest_color : source_color = vec3(0.09, 0.2, 0.22);
uniform vec3 shallow_color : source_color = vec3(0.16, 0.27, 0.24);
uniform vec3 sky_color : source_color = vec3(0.62, 0.72, 0.82);
uniform vec3 foam_color : source_color = vec3(0.86, 0.88, 0.87);
uniform float choppiness : hint_range(0.0, 2.0) = 1.0;
// Rivage : terre au-delà de shore_x (x croissants) ; désactivé si shore_on = false.
uniform bool shore_on = false;
uniform float shore_x = 600.0;
uniform vec4 ships[MAX_SHIPS];
uniform vec2 ship_motion[MAX_SHIPS];
uniform int ship_count = 0;

varying vec3 world_pos;
varying float crest;
varying float far_fade;

float fade_at(vec2 p) {
	return 1.0 - smoothstep(displace_radius * 0.6, displace_radius, distance(p, displace_center));
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec2 p = world_pos.xz;
	float h = 0.0;
	vec2 grad = vec2(0.0);
	float fade = displace ? fade_at(p) : 0.0;
	for (int i = 0; i < 4; i++) {
		vec4 w = waves[i];
		float arg = w.z * (w.x * p.x + w.y * p.y) - omegas[i] * sea_time + phases[i];
		h += w.w * sin(arg);
		grad += vec2(w.x, w.y) * (w.w * w.z * cos(arg));
	}
	crest = h;
	far_fade = fade;
	VERTEX.y += h * fade;
	NORMAL = normalize(vec3(-grad.x * fade * choppiness, 1.0, -grad.y * fade * choppiness));
	world_pos.y += h * fade;
}

void fragment() {
	vec2 p = world_pos.xz;
	float t = sea_time;
	vec3 n1 = texture(wave_normal, p * 0.045 + vec2(t * 0.02, t * 0.006)).rgb * 2.0 - 1.0;
	vec3 n2 = texture(wave_normal, p * 0.013 + vec2(-t * 0.006, t * 0.011)).rgb * 2.0 - 1.0;
	vec3 n = normalize(n1 + n2 * 1.2 + vec3(0.0, 0.0, 2.4));
	float dist = length(VERTEX);
	float calm = smoothstep(500.0, 4000.0, dist);
	n = normalize(mix(n, vec3(0.0, 0.0, 1.0), calm * 0.85));
	NORMAL_MAP = n * 0.5 + 0.5;
	NORMAL_MAP_DEPTH = 0.6;

	float macro = texture(macro_noise, p * 0.0012).r;
	float crest_amount = clamp(crest * 0.9 + 0.35, 0.0, 1.0) * far_fade;
	vec3 water = mix(deep_color, crest_color, crest_amount) * mix(0.85, 1.15, macro);
	float offshore = 1e6;
	if (shore_on) {
		offshore = shore_x - world_pos.x;
		water = mix(water, shallow_color, (1.0 - smoothstep(0.0, 160.0, offshore)) * 0.8);
	}
	float fresnel = pow(1.0 - clamp(dot(normalize(-VERTEX), NORMAL), 0.0, 1.0), 5.0);
	vec3 albedo = mix(water, sky_color * 0.75, clamp(fresnel * 0.8 + calm * 0.3, 0.0, 0.85));

	// Écume : crêtes des grosses vagues, étraves et sillages.
	float foam = smoothstep(0.75, 1.25, crest * 1.1 + (macro - 0.5) * 0.5) * far_fade * 0.3;
	float foam_noise = texture(macro_noise, p * 0.09 + vec2(t * 0.03, t * 0.01)).r;
	for (int i = 0; i < MAX_SHIPS; i++) {
		if (i >= ship_count) {
			break;
		}
		vec4 s = ships[i];
		vec2 d = p - s.xy;
		float c = cos(s.z);
		float sn = sin(s.z);
		// Repère du navire : avant (along), travers (across).
		float along = d.x * c + d.y * sn;
		float across = -d.x * sn + d.y * c;
		float half_len = s.w * 0.5;
		float half_beam = ship_motion[i].y * 0.5;
		float speed = ship_motion[i].x;
		// Collerette d'écume contre la coque (toujours, plus forte avec la vitesse).
		float ex = along / (half_len + 1.5);
		float ey = across / (half_beam + 1.2);
		float r = sqrt(ex * ex + ey * ey);
		float hull_ring = (1.0 - smoothstep(0.85, 1.25, r)) * smoothstep(0.7, 0.95, r);
		foam = max(foam, hull_ring * (0.2 + 0.45 * clamp(speed / 3.0, 0.0, 1.0)));
		// Moustache d'étrave et sillage en V derrière.
		if (speed > 0.2) {
			float behind = -along - half_len;
			if (behind > -2.0 && behind < 140.0) {
				float spread = 0.5 * half_beam + behind * 0.18;
				float edge = abs(abs(across) - spread);
				float trail = (1.0 - smoothstep(0.0, 0.8 + behind * 0.03, edge)) * (1.0 - behind / 110.0);
				float centre = (1.0 - smoothstep(0.0, half_beam * 0.8 + behind * 0.04, abs(across))) * (1.0 - behind / 60.0);
				foam = max(foam, (trail * 0.8 + max(centre, 0.0) * 0.45) * clamp(speed / 3.0, 0.0, 1.0));
			}
		}
	}
	if (shore_on) {
		float band = 1.0 - smoothstep(0.0, 8.0 + macro * 8.0, offshore);
		foam = max(foam, band);
	}
	foam *= smoothstep(0.25, 0.65, foam_noise + foam * 0.4);
	albedo = mix(albedo, foam_color, clamp(foam, 0.0, 1.0) * 0.85);
	ALBEDO = albedo;
	ROUGHNESS = mix(0.05, 0.55, clamp(foam + calm * 0.3, 0.0, 1.0));
	SPECULAR = 0.55;
}
