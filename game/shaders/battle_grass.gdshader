// Herbe des batailles (lot V4) : touffes en cartes croisées d'un MultiMesh en grille qui suit le
// regard (nœud déplacé par pas entiers de la grille). La position réelle de chaque touffe est
// dérivée de sa cellule monde (hachage) : l'herbe ne « glisse » pas quand la grille se déplace.
// Hauteur lue dans la carte de hauteurs du terrain ; densité selon la splatmap (pas d'herbe sur
// les chemins, la boue, les galets, les labours, sous l'eau), les parcelles (blé doré, jachère
// haute) et la neige. Balancement au vent dans le sommet, fondu au bord du disque.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_disabled, depth_draw_opaque;

#include "res://shaders/battle_common.gdshaderinc"

uniform sampler2D grass_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
uniform sampler2D height_map : filter_linear, repeat_disable;
uniform vec4 height_rect = vec4(-400.0, -400.0, 2010.0, 1610.0);
uniform sampler2D splat_a : filter_linear, repeat_disable;
uniform sampler2D splat_b : filter_linear, repeat_disable;
uniform vec4 splat_rect = vec4(-400.0, -400.0, 2000.0, 1600.0);
uniform vec4 calm_rect = vec4(150.0, 60.0, 1050.0, 740.0);
uniform float spacing = 0.9;
uniform float radius = 80.0;
uniform float inner_radius = 0.0; // couche lointaine : vide au centre (couvert par la proche)
uniform float blade_scale = 1.0;
uniform float snow = 0.0;
uniform vec3 base_tint : source_color = vec3(1.0);
uniform vec2 wind_dir = vec2(0.8, 0.6);
uniform float wind_strength = 1.0;
uniform vec3 focus = vec3(0.0);

varying float v_shade;
varying vec3 v_tint;

float terrain_h(vec2 p) {
	vec2 uv = (p - height_rect.xy) / height_rect.zw;
	return texture(height_map, uv).r;
}

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec2 cell = floor(origin.xz / spacing + 0.5);
	vec2 rnd = bt_hash22(cell);
	float r3 = bt_hash21(cell + 31.7);
	vec2 wp = (cell + rnd - 0.5) * spacing;

	// Densité : zones de sol nu et parcelles.
	vec2 suv = (wp - splat_rect.xy) / splat_rect.zw;
	vec4 sa = texture(splat_a, suv);
	vec4 sb = texture(splat_b, suv);
	float density = 1.0 - smoothstep(0.2, 0.5, max(max(sa.g, sa.b), sa.a));
	density *= 1.0 - smoothstep(0.3, 0.7, sb.r) * 0.8;
	density *= mix(1.0, 0.45, smoothstep(0.3, 0.8, sa.r)); // sous-bois : clairsemé
	vec2 calm_d = max(calm_rect.xy - wp, wp - calm_rect.zw);
	float outside_calm = smoothstep(0.0, 60.0, max(calm_d.x, calm_d.y));
	vec3 field = bt_field_info(wp);
	float height_mul = 1.0;
	// Couleur de base : celle du sol dessous (couches herbe et prairie, très floues), pour que
	// l'herbe se fonde dans la texture du terrain.
	float macro2 = texture(macro_noise, wp * 0.00043 + vec2(0.3, 0.7)).r;
	vec3 ground = mix(textureLod(albedo_array, vec3(wp / 5.0, 0.0), 7.0).rgb, textureLod(albedo_array, vec3(wp / 7.0, 1.0), 7.0).rgb, smoothstep(0.25, 0.55, macro2));
	v_tint = base_tint * ground * vec3(3.3, 3.1, 2.3);
	float in_field = outside_calm * (1.0 - sb.g) * field.z;
	if (field.x == 1.0) {
		density *= 1.0 - in_field;
	} else if (field.x == 2.0) {
		height_mul = mix(1.0, 2.1, in_field);
		v_tint = mix(v_tint, vec3(1.6, 1.25, 0.55), in_field);
	} else if (field.x == 3.0) {
		height_mul = mix(1.0, 1.4, in_field);
		v_tint = mix(v_tint, vec3(1.25, 1.1, 0.7), in_field * 0.7);
	}
	density *= 1.0 - snow * 0.75;
	height_mul *= 1.0 - snow * 0.5;
	// Touffes clairsemées par endroits (bruit), plus de variation.
	density *= smoothstep(0.15, 0.45, bt_value_noise(wp * 0.05));

	float dist = length(wp - focus.xz);
	float fade = (1.0 - smoothstep(radius * 0.6, radius, dist)) * smoothstep(inner_radius * 0.75, inner_radius, dist);
	float keep = step(r3, density);
	float s = (0.6 + 0.8 * rnd.x) * blade_scale * fade * keep;

	float angle = rnd.y * 6.2832;
	mat2 rot = mat2(vec2(cos(angle), sin(angle)), vec2(-sin(angle), cos(angle)));
	vec3 v = VERTEX;
	v.xz = rot * v.xz * s;
	v.y *= s * height_mul;
	// Vent : le haut de la touffe (UV.y = 0) plie, rafales qui traversent le champ.
	float top = 1.0 - UV.y;
	float gust = sin(dot(wp, wind_dir) * 0.08 - TIME * 1.7) * 0.5 + 0.5;
	float sway = sin(TIME * 2.3 + rnd.x * 6.2832 + wp.x * 0.3) * 0.12 + gust * 0.28;
	v.xz += wind_dir * sway * top * top * wind_strength * s * height_mul;
	vec3 world = vec3(wp.x, terrain_h(wp) - 0.05, wp.y) + v;
	VERTEX = world - origin;
	NORMAL = normalize(vec3(0.0, 1.0, 0.0) + vec3(wind_dir.x, 0.0, wind_dir.y) * sway * 0.3);
	v_shade = mix(0.85, 1.12, rnd.x) * mix(0.9, 1.05, gust);
	v_tint *= mix(vec3(0.95, 1.0, 0.9), vec3(1.08, 1.03, 0.9), r3);
}

void fragment() {
	vec4 tex = texture(grass_texture, UV);
	ALBEDO = tex.rgb * v_tint * v_shade;
	ALPHA = tex.a;
	ALPHA_SCISSOR_THRESHOLD = 0.4;
	ROUGHNESS = 0.9;
	BACKLIGHT = vec3(0.18, 0.22, 0.08);
}
