// Anneau d'horizon des batailles (lot EP2) : relief réel lointain (7 à 13 km). Couleur par
// occupation du sol réelle (forêts de la tuile cuite, `class_map` : 0-200 part de forêt, 255 mer),
// roche sur les pentes, neige au-dessus de la limite de saison (altitude réelle). Brume propre
// (`FOG`) : même densité que le brouillard de la scène sur les premiers `full_density_m`, puis
// allégée avec l'altitude moyenne du trajet (les sommets lointains restent lisibles).
shader_type spatial;
render_mode diffuse_burley, specular_disabled;

uniform sampler2D class_map : filter_linear, repeat_disable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
// Centre du champ (x, z) et rotation (cos, sin) monde -> tuile réelle.
uniform vec4 tile_xform = vec4(600.0, 400.0, 1.0, 0.0);
uniform float tile_half = 13000.0;
uniform float offset_y = 0.0;
uniform float snow_line = 2500.0;
uniform float snowy = 0.0;
uniform vec3 field_color : source_color = vec3(0.34, 0.37, 0.21);
uniform vec3 meadow_color : source_color = vec3(0.27, 0.34, 0.17);
uniform vec3 forest_color : source_color = vec3(0.1, 0.16, 0.08);
uniform vec3 rock_color : source_color = vec3(0.42, 0.4, 0.37);
uniform vec3 snow_color : source_color = vec3(0.92, 0.94, 0.97);
uniform vec3 fog_color : source_color = vec3(0.7, 0.76, 0.84);
uniform float fog_density = 0.0002;
uniform float full_density_m = 6000.0;
uniform float altitude_scale = 1400.0;
uniform float ref_y = 0.0;

varying vec3 world_pos;
varying vec3 world_normal;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	float e = world_pos.x - tile_xform.x;
	float n = tile_xform.y - world_pos.z;
	vec2 real = vec2(e * tile_xform.z - n * tile_xform.w, n * tile_xform.z + e * tile_xform.w);
	vec2 uv = vec2(real.x + tile_half, tile_half - real.y) / (2.0 * tile_half);
	float cls = texture(class_map, uv).r * 255.0;
	float sea = step(250.0, cls);
	float forest = sea > 0.5 ? 0.0 : clamp(cls / 200.0, 0.0, 1.0);
	float macro = texture(macro_noise, world_pos.xz * 0.00021).r;
	float macro2 = texture(macro_noise, world_pos.xz * 0.0011 + 0.37).r;
	vec3 col = mix(field_color, meadow_color, smoothstep(0.35, 0.65, macro2));
	col *= mix(0.82, 1.12, macro);
	col = mix(col, forest_color * mix(0.85, 1.1, macro2), smoothstep(0.35, 0.6, forest + (macro2 - 0.5) * 0.3));
	float slope = 1.0 - clamp(world_normal.y, 0.0, 1.0);
	float altitude = world_pos.y - offset_y;
	col = mix(col, rock_color * mix(0.8, 1.1, macro), smoothstep(0.25, 0.5, slope) * smoothstep(300.0, 1200.0, altitude));
	float snow_edge = snow_line + (macro - 0.5) * 300.0;
	float snow = max(smoothstep(snow_edge - 80.0, snow_edge + 80.0, altitude) * (1.0 - smoothstep(0.55, 0.8, slope)), snowy * 0.85);
	col = mix(col, snow_color, snow);
	col = mix(col, vec3(0.6, 0.57, 0.48), sea);
	ALBEDO = col;
	ROUGHNESS = 1.0;
	// Brume : trajet caméra -> point, densité allégée en altitude au-delà de `full_density_m`.
	float d = length(VERTEX);
	vec3 cam = INV_VIEW_MATRIX[3].xyz;
	float mid = max((world_pos.y + cam.y) * 0.5 - ref_y, 0.0);
	float tau = fog_density * (min(d, full_density_m) + max(d - full_density_m, 0.0) * exp(-mid / altitude_scale));
	FOG = vec4(fog_color, 1.0 - exp(-tau));
}
