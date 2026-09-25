shader_type spatial;
render_mode blend_mix, cull_disabled, depth_draw_never, specular_disabled;

// Chemins de terre drapés sur le relief (lot C6, palier près). Le ruban est posé sur la surface
// exacte du maillage de terrain affiché (TerrainBuilder.surface_height_at) avec un léger
// soulèvement ; UV.x = abscisse curviligne (unités monde), UV.y = côté (-1 / 1).

uniform vec3 dirt_color : source_color = vec3(0.47, 0.38, 0.25);
uniform vec3 rut_color : source_color = vec3(0.34, 0.26, 0.16);
uniform vec3 grass_edge : source_color = vec3(0.30, 0.31, 0.16);
uniform float alpha = 1.0;
// Lot ZG5b : disque où les routes drapées fines prennent le relais (centre x, z ; rayon ; poids).
uniform vec4 fine_zone = vec4(0.0, 0.0, -1.0, 0.0);

varying vec2 v_xz;

void vertex() {
	v_xz = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xz;
}

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	float side = abs(UV.y);
	float n = noise(vec2(UV.x * 3.0, UV.y * 2.0));
	float ruts = smoothstep(0.08, 0.0, abs(side - 0.45)) * 0.6;
	vec3 col = mix(dirt_color, rut_color, ruts);
	col = mix(col, grass_edge, smoothstep(0.55, 1.0, side + (n - 0.5) * 0.3));
	col *= 0.9 + 0.2 * n;
	ALBEDO = col;
	ROUGHNESS = 1.0;
	float edge = 1.0 - smoothstep(0.7, 1.0, side + (n - 0.5) * 0.25);
	float fine = fine_zone.z > 0.0 ? fine_zone.w * (1.0 - smoothstep(fine_zone.z * 0.85, fine_zone.z, distance(v_xz, fine_zone.xy))) : 0.0;
	ALPHA = edge * alpha * (1.0 - fine);
}
