shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot CM2 : nuées de la carte de campagne (plan au-dessus du relief) : masses grises de pluie,
// sombres d'orage (éclairs), blanches de neige, qui défilent avec le vent dans les régions
// touchées (météo du cœur, masque par province de CampaignWeatherView). Estompées de près
// (la caméra passe dessous) et en vue parchemin.

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform vec2 map_size = vec2(1.0, 1.0);  // taille du monde (map.json size_px), fournie par le script
// Opacité selon la distance caméra (CampaignWeatherView) et part max.
uniform float cloud_alpha = 0.0;
uniform vec3 rain_color : source_color = vec3(0.50, 0.52, 0.56);
uniform vec3 storm_color : source_color = vec3(0.24, 0.25, 0.30);
uniform vec3 snow_color : source_color = vec3(0.86, 0.88, 0.92);
uniform vec3 sun_color : source_color = vec3(1.0, 0.97, 0.9);

int province_at(vec2 p) {
	ivec2 sz = textureSize(province_ids, 0);
	ivec2 texel = clamp(ivec2(floor(p)), ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

#include "res://shaders/campaign_weather.gdshaderinc"
#include "res://shaders/parchment_common.gdshaderinc"

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	float a = cloud_alpha * (1.0 - pm_weight());
	vec4 w = (a > 0.001) ? wx_sample(p) : vec4(0.0);
	float density = wx_cloud(p, w);
	if (density <= 0.001) {
		discard;
	}
	float wet = max(max(w.r, w.g), w.a);
	vec3 base = mix(rain_color, storm_color, clamp(w.a * 1.2, 0.0, 1.0));
	base = mix(base, snow_color, clamp(w.g / max(wet, 1e-3), 0.0, 1.0));
	// Relief des nuées : sommets éclairés (nord-ouest), creux plus sombres.
	vec2 q = (p - TIME * weather_wind) * 0.012;
	float lit = wx_fbm(q + vec2(-0.08, -0.08)) - wx_fbm(q);
	vec3 col = base * (0.85 + 0.9 * clamp(lit * 4.0 + 0.2, 0.0, 0.6)) * mix(vec3(1.0), sun_color, 0.3);
	col += vec3(0.75, 0.8, 1.0) * wx_lightning(p, w) * 1.6;
	ALBEDO = col;
	ALPHA = clamp(density * a, 0.0, 0.85);
}
