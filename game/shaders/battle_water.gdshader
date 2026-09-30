// Rivière des batailles (lot V4) : profondeur lue dans le tampon de profondeur (rives qui se
// fondent, lit visible par transparence, eau sombre au milieu), réfraction de l'écran, vaguelettes
// qui s'écoulent le long du ruban (UV.y = mètres le long du cours), écume sur les hauts-fonds
// (gués), reflets du ciel et du soleil laissés au moteur (faible rugosité).
// Lot V4b : reflet de Fresnel de la couleur du ciel (l'eau vue de loin reflète le ciel au lieu
// d'être une bande sombre), eau plus claire et verte sur les hauts-fonds (bords du ruban).
shader_type spatial;
render_mode blend_mix, depth_draw_always, cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform sampler2D wave_normal : hint_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
uniform sampler2D depth_texture : hint_depth_texture, filter_linear_mipmap;
uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;
uniform vec3 shallow_color : source_color = vec3(0.34, 0.37, 0.28);
uniform vec3 deep_color : source_color = vec3(0.05, 0.1, 0.1);
uniform vec3 sky_color : source_color = vec3(0.62, 0.72, 0.82);
uniform float river_width = 38.0;
uniform float flow_speed = 0.55;
uniform float clarity = 0.9; // mètres de visibilité
uniform float turbidity : hint_range(0.0, 1.0) = 0.35;
uniform float ripple : hint_range(0.0, 1.0) = 0.0; // gouttes de pluie

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 flow = vec2(UV.x * river_width, UV.y);
	float t = TIME * flow_speed;
	vec2 uv1 = flow * vec2(0.09, 0.05) + vec2(0.0, -t * 0.12);
	vec2 uv2 = flow * vec2(0.05, 0.023) + vec2(0.31, -t * 0.07);
	vec3 n1 = texture(wave_normal, uv1).rgb * 2.0 - 1.0;
	vec3 n2 = texture(wave_normal, uv2).rgb * 2.0 - 1.0;
	vec3 n = normalize(n1 + n2 + vec3(0.0, 0.0, 1.5));
	if (ripple > 0.0) {
		vec3 n3 = texture(wave_normal, world_pos.xz * 0.35 + vec2(TIME * 0.9, -TIME * 0.7)).rgb * 2.0 - 1.0;
		n = normalize(n + n3 * ripple * 0.8);
	}
	NORMAL_MAP = n * 0.5 + 0.5;
	NORMAL_MAP_DEPTH = 0.6;

	// Épaisseur d'eau traversée par le rayon (espace vue).
	float depth = texture(depth_texture, SCREEN_UV).r;
	vec4 behind = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
	behind.xyz /= behind.w;
	float thickness = max(VERTEX.z - behind.z, 0.0);
	// Profondeur verticale approchée (le rayon est oblique).
	vec3 up_view = normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
	float vertical = thickness * abs(dot(normalize(VERTEX), up_view));

	vec2 offset = n.xy * 0.02 * clamp(thickness, 0.0, 1.0);
	float refracted_depth = texture(depth_texture, SCREEN_UV + offset).r;
	vec4 behind2 = INV_PROJECTION_MATRIX * vec4((SCREEN_UV + offset) * 2.0 - 1.0, refracted_depth, 1.0);
	if (behind2.z / behind2.w > VERTEX.z) {
		offset = vec2(0.0); // pas de réfraction de ce qui est devant l'eau
	}
	vec3 ground = texture(screen_texture, SCREEN_UV + offset).rgb;

	float murk = 1.0 - exp(-vertical / clarity);
	murk = clamp(murk + turbidity * 0.35, 0.0, 1.0);
	// Hauts-fonds : le bord du ruban (UV.x près de 0 ou 1) garde une eau claire et verte.
	float edge = 1.0 - smoothstep(0.0, 0.3, min(UV.x, 1.0 - UV.x));
	murk *= mix(1.0, 0.45, edge);
	vec3 water = mix(ground * shallow_color * 2.2, deep_color, murk);
	// Reflet du ciel (Fresnel de Schlick) : fort en incidence rasante, donc de loin.
	// CR1 : ciel reflété assombri (le ciel vu sous 20-40° est
	// plus foncé que la couleur d'horizon `sky_color`) et plafonné à 0,4 : quelques pour cent de
	// ciel clair suffisent à rendre laiteuse une eau sombre une fois passée en sRGB.
	vec3 view_n = normalize(mix(up_view, (VIEW_MATRIX * vec4(n.x, n.z, n.y, 0.0)).xyz, 0.25));
	float cos_v = clamp(dot(view_n, normalize(-VERTEX)), 0.0, 1.0);
	float fresnel = 0.03 + 0.97 * pow(1.0 - cos_v, 5.0);
	vec3 sky = sky_color * 0.7 * mix(0.85, 1.1, texture(macro_noise, world_pos.xz * 0.004).r);
	water = mix(water, sky, clamp(fresnel * 1.1, 0.0, 0.4) * smoothstep(0.02, 0.2, vertical));

	// Écume sur les hauts-fonds et le long des rives, déchiquetée et entraînée par le courant.
	float foam_noise = texture(macro_noise, flow * vec2(0.3, 0.12) + vec2(0.0, -t * 0.35)).r;
	float shore = 1.0 - smoothstep(0.02, 0.12, vertical);
	float foam = smoothstep(0.55, 0.8, foam_noise) * shore * 0.6;
	water = mix(water, vec3(0.7, 0.72, 0.68), foam * 0.4);

	ALBEDO = deep_color * 0.2 + vec3(foam * 0.4);
	EMISSION = water * (1.0 - foam * 0.3);
	// CR1 : le reflet du ciel est déjà dans EMISSION (Fresnel ci-dessus) ; le spéculaire du moteur
	// le comptait une seconde fois (ciel de l'environnement sur des vagues très bosselées :
	// traînées blanches, eau « glacée » vue de près). Spéculaire réduit, vagues moins miroir.
	ROUGHNESS = mix(0.1, 0.35, foam);
	SPECULAR = 0.25;
	METALLIC = 0.0;
	// Bord de l'eau fondu dans le sol.
	ALPHA = smoothstep(0.0, 0.1, vertical);
}
