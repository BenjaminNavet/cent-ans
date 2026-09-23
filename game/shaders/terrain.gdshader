shader_type spatial;
render_mode cull_back, depth_draw_opaque;

// Terrain de campagne : couleur par altitude/pente, teinte parchemin,
// frontières de provinces (comparaison des texels voisins de province_ids)
// et teinte de faction + surbrillance survol/sélection.

uniform sampler2D heightmap : filter_linear, repeat_disable, hint_default_black;
uniform int height_bpp = 2;              // 2 = LA8 (16 bits), 1 = L8
uniform bool height_little_endian = false; // LA8 : L = octet faible (Rust) ou fort (Png16)
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;
uniform float height_scale = 0.02;
uniform vec2 map_size = vec2(4096.0, 4096.0);

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D faction_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform int hovered_id = 0;
uniform int selected_id = 0;
// Masque 1D par province (R8) : ~0,5 = atteignable par l'armée sélectionnée, 1 = sur le chemin.
uniform sampler2D province_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool mask_enabled = false;
uniform vec3 reachable_color : source_color = vec3(0.98, 0.93, 0.55);
uniform vec3 path_color : source_color = vec3(1.0, 0.55, 0.15);
uniform float faction_alpha = 0.30;
uniform vec3 border_color : source_color = vec3(0.24, 0.15, 0.08);
uniform vec3 parchment : source_color = vec3(0.87, 0.79, 0.62);
uniform float parchment_amount = 0.12;
// Exagération du relief pour l'ombrage seulement (plus douce que la géométrie).
uniform float shading_relief = 0.6;

varying vec3 world_pos;

float height01(vec2 uv) {
	vec4 t = texture(heightmap, uv);
	if (height_bpp == 2) {
		if (height_little_endian) {
			return (t.a * 255.0 * 256.0 + t.r * 255.0) / 65535.0;
		}
		return (t.r * 255.0 * 256.0 + t.a * 255.0) / 65535.0;
	}
	return t.r;
}

float height_m(vec2 uv) {
	return mix(height_min_m, height_max_m, height01(uv));
}

int province_at(ivec2 texel) {
	ivec2 sz = textureSize(province_ids, 0);
	texel = clamp(texel, ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 uv = (world_pos.xz + vec2(0.5)) / map_size;
	// Différences finies sur ±2 px : lisse le bruit haute fréquence de la heightmap.
	vec2 du = vec2(2.0 / map_size.x, 0.0);
	vec2 dv = vec2(0.0, 2.0 / map_size.y);

	// Normale par différences finies sur la heightmap (pas de couture entre tuiles).
	float hc = height_m(uv);
	float hx = (height_m(uv + du) - height_m(uv - du)) * height_scale * shading_relief * 0.25;
	float hz = (height_m(uv + dv) - height_m(uv - dv)) * height_scale * shading_relief * 0.25;
	vec3 n = normalize(vec3(-hx, 1.0, -hz));
	float slope = 1.0 - n.y;

	// Palette par altitude.
	vec3 deep_sea = vec3(0.07, 0.22, 0.28);
	vec3 shallow_sea = vec3(0.20, 0.42, 0.45);
	vec3 shore = vec3(0.64, 0.66, 0.42);
	vec3 plain = vec3(0.44, 0.58, 0.27);
	vec3 plain_ochre = vec3(0.68, 0.60, 0.32);
	vec3 hill = vec3(0.56, 0.44, 0.26);
	vec3 mountain = vec3(0.46, 0.43, 0.40);
	vec3 snow = vec3(0.93, 0.93, 0.95);

	vec3 col;
	if (hc <= 0.0) {
		col = mix(shallow_sea, deep_sea, clamp(-hc / 200.0, 0.0, 1.0));
	} else if (hc < 40.0) {
		col = mix(shore, plain, hc / 40.0);
	} else if (hc < 300.0) {
		col = mix(plain, plain_ochre, (hc - 40.0) / 260.0);
	} else if (hc < 900.0) {
		col = mix(plain_ochre, hill, (hc - 300.0) / 600.0);
	} else if (hc < 2500.0) {
		col = mix(hill, mountain, (hc - 900.0) / 1600.0);
	} else {
		col = mix(mountain, snow, clamp((hc - 2500.0) / 400.0, 0.0, 1.0));
	}
	// Les pentes raides tirent vers la roche.
	col = mix(col, mountain * 0.9, clamp(slope * 4.0, 0.0, 0.5) * step(0.0, hc));
	col = mix(col, parchment, parchment_amount);

	// Provinces.
	ivec2 texel = ivec2(floor(world_pos.xz));
	int id = province_at(texel);
	if (id > 0) {
		vec4 fc = texelFetch(faction_colors, ivec2(clamp(id, 0, textureSize(faction_colors, 0).x - 1), 0), 0);
		col = mix(col, fc.rgb, fc.a * faction_alpha);
		if (mask_enabled) {
			float m = texelFetch(province_mask, ivec2(clamp(id, 0, textureSize(province_mask, 0).x - 1), 0), 0).r;
			if (m > 0.75) {
				col = mix(col, path_color, 0.5);
			} else if (m > 0.25) {
				col = mix(col, reachable_color, 0.5);
			} else {
				// Hors de portée : assombri pour faire ressortir les provinces atteignables.
				col *= 0.7;
			}
		}
		if (id == selected_id) {
			col = mix(col, vec3(0.95, 0.80, 0.30), 0.35);
		} else if (id == hovered_id) {
			col = mix(col, vec3(1.0, 0.97, 0.85), 0.22);
		}
	}
	// Frontières : largeur ~constante à l'écran (texels par fragment).
	float px_per_frag = max(fwidth(world_pos.x), fwidth(world_pos.z));
	int k = int(clamp(ceil(px_per_frag * 0.8), 1.0, 6.0));
	bool border = false;
	if (id > 0) {
		border = province_at(texel + ivec2(k, 0)) != id
			|| province_at(texel - ivec2(k, 0)) != id
			|| province_at(texel + ivec2(0, k)) != id
			|| province_at(texel - ivec2(0, k)) != id;
	}
	if (border) {
		col = mix(col, border_color, 0.85);
	}

	ALBEDO = col;
	ROUGHNESS = 0.92;
	SPECULAR = 0.15;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
