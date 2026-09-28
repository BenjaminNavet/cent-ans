shader_type spatial;
render_mode cull_back, depth_draw_opaque;

// Lot CM2 : terrain en vue stratégique parchemin seule (poids 1). StrategicView substitue ce
// shader à terrain.gdshader dans le matériau partagé du terrain (mêmes noms d'uniformes : les
// valeurs posées par TerrainBuilder sont conservées) ; le rendu 3D n'est alors plus calculé.
// Pendant le fondu, terrain.gdshader mélange les deux (crochet `parchment_apply`).
// Uniformes et fonctions ci-dessous : copie de ceux de terrain.gdshader dont le parchemin a besoin.

uniform sampler2D heightmap : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform int height_bpp = 1;
uniform bool height_little_endian = false;
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;
uniform vec2 map_size = vec2(1.0, 1.0);  // taille du monde (map.json size_px), fournie par le script
uniform float meters_per_px = 719.0;

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D faction_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform int hovered_id = 0;
uniform int selected_id = 0;
uniform sampler2D province_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool mask_enabled = false;
uniform sampler2D fog_mask : filter_nearest, repeat_disable, hint_default_white;
uniform bool fog_enabled = false;

uniform sampler2D splat : filter_linear, repeat_disable, hint_default_black;
uniform bool has_splat = false;
uniform sampler2D border_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_border_dist = false;
uniform float border_dist_scale = 6.0;
uniform sampler2D coast_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_coast_dist = false;
uniform float coast_dist_scale = 2.0;
#include "res://shaders/river_bed.gdshaderinc"

varying vec3 world_pos;

float height01_lod(vec2 uv, float lod) {
	if (height_bpp == 2) {
		vec4 t = texture(heightmap, uv);
		if (height_little_endian) {
			return (t.a * 255.0 * 256.0 + t.r * 255.0) / 65535.0;
		}
		return (t.r * 255.0 * 256.0 + t.a * 255.0) / 65535.0;
	}
	return textureLod(heightmap, uv, lod).r;
}

float height_m_lod(vec2 uv, float lod) {
	return mix(height_min_m, height_max_m, height01_lod(uv, lod));
}

int province_at(vec2 p) {
	ivec2 sz = textureSize(province_ids, 0);
	ivec2 texel = clamp(ivec2(floor(p)), ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

vec4 faction_of(int id) {
	if (id <= 0) {
		return vec4(0.0);
	}
	return texelFetch(faction_colors, ivec2(clamp(id, 0, textureSize(faction_colors, 0).x - 1), 0), 0);
}

float fog_seen(int id) {
	return texelFetch(fog_mask, ivec2(clamp(id, 0, textureSize(fog_mask, 0).x - 1), 0), 0).r;
}

float border_d(vec2 uv) {
	vec3 s = (texture(border_dist, uv).rgb * 255.0 - 128.0) / border_dist_scale;
	return min(abs(s.r), min(abs(s.g), abs(s.b)));
}

#include "res://shaders/parchment_map.gdshaderinc"
#include "res://shaders/faction_borders.gdshaderinc"  // FR1 : frontières de faction

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = p / map_size;  // SZ2b : pixel carte i centré en x = i + 0,5, comme les outils (ADR 0086)
	float footprint = max(length(dFdx(p)), length(dFdy(p)));
	vec3 albedo;
	vec3 emission;
	parchment_only(p, uv, footprint, albedo, emission);
	fr1_borders(p, uv, footprint, albedo, emission);  // FR1
	ALBEDO = albedo;
	EMISSION = emission;
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
	NORMAL = normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
}
