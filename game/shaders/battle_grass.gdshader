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
// EP6 : parcelles du décor (voir `bt_decor_field`).
uniform sampler2D decor_fields : filter_nearest, repeat_disable;
uniform float decor_on = 0.0;
uniform float spacing = 0.9;
uniform float radius = 80.0;
uniform float inner_radius = 0.0; // couche lointaine : vide au centre (couvert par la proche)
uniform float blade_scale = 1.0;
uniform float snow = 0.0;
uniform vec3 base_tint : source_color = vec3(1.0);
uniform vec2 wind_dir = vec2(0.8, 0.6);
uniform float wind_strength = 1.0;
uniform vec3 focus = vec3(0.0);
// BV3 : herbe couchée (R) et tachée de sang (G), carte `BattleGrassFlatten`.
uniform sampler2D flatten_map : filter_linear, repeat_disable;
uniform vec4 flatten_rect = vec4(0.0, -100.0, 1200.0, 1000.0);
uniform float flatten_on = 0.0;
uniform vec3 grass_blood_color : source_color = vec3(0.2, 0.02, 0.02);

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
	float dist = length(wp - focus.xz);
	float fade = (1.0 - smoothstep(radius * 0.6, radius, dist)) * smoothstep(inner_radius * 0.75, inner_radius, dist);
	// Touffes clairsemées par endroits (bruit) : test bon marché avant les lectures de textures.
	float density = smoothstep(0.15, 0.45, bt_value_noise(wp * 0.05)) * (1.0 - snow * 0.93);
	v_tint = vec3(0.0);
	v_shade = 0.0;
	if (fade <= 0.0 || r3 >= density) {
		// Touffe absente : triangle dégénéré sous le sol (aucun pixel).
		VERTEX = vec3(0.0, -500.0, 0.0);
	} else {
		// Densité : zones de sol nu et parcelles.
		vec2 suv = (wp - splat_rect.xy) / splat_rect.zw;
		vec4 sa = texture(splat_a, suv);
		vec4 sb = texture(splat_b, suv);
		density *= 1.0 - smoothstep(0.2, 0.5, max(max(sa.g, sa.b), sa.a));
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
		v_tint = mix(base_tint * ground * vec3(1.12, 1.18, 1.0), vec3(0.45, 0.45, 0.4), snow * 0.6);
		// Taches de prairie (mêmes formules que `battle_ground.gdshader`, lot V4b).
		float patch = texture(macro_noise, wp * 0.0013 + vec2(0.61, 0.13)).r;
		float patch2 = texture(macro_noise, wp * 0.0047 + vec2(0.27, 0.83)).r;
		float dry = smoothstep(0.42, 0.66, patch * 0.7 + patch2 * 0.45);
		float lush = smoothstep(0.45, 0.7, (1.0 - patch) * 0.75 + patch2 * 0.35);
		v_tint = mix(v_tint, v_tint * vec3(1.35, 1.12, 0.62), dry * 0.85);
		v_tint = mix(v_tint, v_tint * vec3(0.7, 0.86, 0.66), lush * 0.8);
		height_mul *= mix(1.0, 1.25, lush) * mix(1.0, 0.8, dry);
		float in_field = outside_calm * (1.0 - sb.g) * field.z;
		if (decor_on > 0.5) {
			vec4 df = texture(decor_fields, suv);
			if (df.a > 0.5) {
				field = bt_decor_field(df, wp);
				in_field = field.z;
				if (field.x == 4.0) {
					// Semis : herbe courte et claire en rangs.
					density *= 0.55;
					height_mul *= 0.45;
					field.x = 0.0;
				} else if (field.x == 5.0) {
					// Vigne : herbe rase entre les rangs.
					density *= 1.0 - in_field * 0.7;
					field.x = 0.0;
				} else if (field.x == 6.0) {
					// Chaume : tiges courtes et blondes.
					height_mul *= 0.5;
					v_tint = mix(v_tint, vec3(1.2, 1.02, 0.62), in_field * 0.8);
					field.x = 0.0;
				}
			}
		}
		if (field.x == 1.0) {
			density *= 1.0 - in_field;
		} else if (field.x == 2.0) {
			height_mul = mix(1.0, 2.1, in_field);
			v_tint = mix(v_tint, vec3(0.5, 0.36, 0.11), in_field);
		} else if (field.x == 3.0) {
			height_mul = mix(1.0, 1.4, in_field);
			v_tint = mix(v_tint, vec3(1.25, 1.1, 0.7), in_field * 0.7);
		}
		height_mul *= 1.0 - snow * 0.5;
		// BV3 : herbe couchée par les troupes et sous les corps, sang sur les brins.
		float flat_k = 0.0;
		float blood_k = 0.0;
		if (flatten_on > 0.0) {
			vec2 fuv = (wp - flatten_rect.xy) / flatten_rect.zw;
			vec2 fl = texture(flatten_map, fuv).rg;
			// Bords irréguliers (bruit à ~1,5 m) et touffes plus ou moins couchées.
			float ragged = bt_value_noise(wp * 0.7) - 0.5;
			flat_k = smoothstep(0.08, 0.9, fl.r * (1.0 + ragged * 0.6) + (rnd.x - 0.5) * 0.25 * fl.r);
			blood_k = smoothstep(0.05, 0.6, fl.g) * smoothstep(-0.45, 0.1, ragged + fl.g * 0.5);
			density *= 1.0 - flat_k * 0.3;
			height_mul *= mix(1.0, 0.22, flat_k);
			v_tint = mix(v_tint, v_tint * vec3(1.08, 1.0, 0.62), flat_k * 0.6);
			v_tint = mix(v_tint, grass_blood_color, blood_k * mix(0.55, 0.9, r3));
		}
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
		v.xz += wind_dir * sway * top * top * wind_strength * s * height_mul * (1.0 - flat_k * 0.8);
		// Brins couchés : le haut de la touffe s'étale au ras du sol (sens propre à la touffe).
		v.xz += vec2(cos(angle * 1.7), sin(angle * 1.7)) * top * 0.42 * s * flat_k;
		vec3 world = vec3(wp.x, terrain_h(wp) - 0.05, wp.y) + v;
		VERTEX = world - origin;
		NORMAL = normalize(vec3(0.0, 1.0, 0.0) + vec3(wind_dir.x, 0.0, wind_dir.y) * sway * 0.3);
		v_shade = mix(0.85, 1.12, rnd.x) * mix(0.9, 1.05, gust);
		v_tint *= mix(vec3(0.95, 1.0, 0.9), vec3(1.08, 1.03, 0.9), r3);
	}
}

void fragment() {
	// Normale « verticale » pour toutes les faces (pas d'assombrissement des faces arrière).
	NORMAL = normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
	vec4 tex = texture(grass_texture, UV);
	// La carte ne donne que la découpe et les nuances claires/sombres des brins ; la couleur
	// vient du sol (v_tint) pour que l'herbe prolonge la texture du terrain.
	float l = dot(tex.rgb, vec3(0.3, 0.59, 0.11)) / 0.14;
	vec3 hue = tex.rgb / max(dot(tex.rgb, vec3(0.333)), 0.001) * 0.333;
	ALBEDO = v_tint * clamp(l, 0.45, 1.5) * mix(vec3(1.0), hue * 3.0, 0.2) * v_shade;
	ALPHA = tex.a;
	ALPHA_SCISSOR_THRESHOLD = 0.4;
	ROUGHNESS = 0.9;
	BACKLIGHT = vec3(0.18, 0.22, 0.08);
}
