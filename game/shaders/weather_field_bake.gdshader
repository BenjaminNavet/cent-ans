shader_type canvas_item;
render_mode blend_disabled, unshaded;

// FL3 (ADR 0192) : cuisson de `wx_sample` (frange déformée entre provinces) pour toute la carte,
// une fois par tour, dans la SubViewport de CampaignWeatherView. Le sol et les nuées lisent
// ensuite `weather_field` (une lecture filtrée) au lieu de recalculer la frange à chaque pixel.

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform vec2 map_size = vec2(1.0, 1.0);

int province_at(vec2 p) {
	ivec2 sz = textureSize(province_ids, 0);
	ivec2 texel = clamp(ivec2(floor(p)), ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

#include "res://shaders/campaign_weather.gdshaderinc"

void fragment() {
	COLOR = wx_sample(UV * map_size);
}
