shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_back, shadows_disabled;

// Lot FR1 (ADR 0070) — frontières de faction lumineuses, façon Total War.
// Passe suivante (`next_pass`) du matériau partagé du terrain (morceaux E0 et patchs du quadtree
// ZG2) : même maillage, même déplacement de sommets (crochet `qt_vertex`, relief exagéré ZG8,
// lit creusé V4) que terrain.gdshader, d'où des frontières drapées exactement sur le relief à
// tous les zooms, sans ruban flottant ni enfoui. Aucune ligne de terrain.gdshader n'est modifiée.
// Les uniformes partagés avec le terrain (heightmap, pages du quadtree, caméra, lits…) sont
// recopiés depuis son matériau par FactionBorders (game/scripts/map/faction_borders.gd) ; les
// uniformes d'instance du quadtree sont déclarés dans le même ordre (même include) : les patchs
// les partagent entre les deux passes.
//
// Frontière : zéro du champ de distance `province_border_dist.png` (trait sous-pixel lissé).
// Chaque côté se peint de la couleur de SA faction : trait coloré + halo intérieur aux frontières
// de royaume (propriétaire ou contrôleur différent), trait fin et discret entre provinces d'une
// même faction, hachures de l'occupant le long d'une province occupée.

uniform sampler2D heightmap : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform int height_bpp = 1;
uniform bool height_little_endian = false;
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;

#include "res://shaders/campaign_relief.gdshaderinc"

uniform vec2 map_size = vec2(4096.0, 4096.0);
uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D border_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_border_dist = false;
uniform float border_dist_scale = 6.0;

#include "res://shaders/river_banks.gdshaderinc"

global uniform float campaign_parchment;

// --- Propres à FR1 (posés par FactionBorders, jamais recopiés du terrain) ---
// Par province (index raster) : r = faction propriétaire, g = faction contrôleuse (index de
// palette / 255, 0 = aucune), b = 1 si le joueur est propriétaire ou contrôleur.
uniform sampler2D fr1_info : filter_nearest, repeat_disable, hint_default_black;
// Couleur (sRGB) par index de faction.
uniform sampler2D fr1_palette : filter_nearest, repeat_disable, hint_default_black;
uniform float fr1_alpha = 1.0;           // mode de carte × fondu de zoom
uniform float fr1_width_scale = 1.0;     // dézoom
uniform bool fr1_neutral = false;        // filtre de carte aux couleurs propres : encre neutre
uniform vec3 fr1_neutral_color = vec3(0.118, 0.090, 0.063);
uniform float fr1_depth_bias = 0.00003;  // relatif (profondeur inversée) : vers la caméra
uniform bool fr1_glow = true;
uniform bool fr1_hatch = true;

uniform float fr1_realm_core_px = 1.6;
uniform float fr1_realm_core_alpha = 0.95;
uniform float fr1_realm_core_brightness = 1.35;
uniform float fr1_realm_outline_px = 2.6;
uniform float fr1_realm_outline_alpha = 0.55;
uniform float fr1_realm_glow_px = 9.0;
uniform float fr1_realm_glow_alpha = 0.55;
uniform float fr1_realm_glow_exponent = 1.6;
uniform float fr1_province_core_px = 0.7;
uniform float fr1_province_core_alpha = 0.32;
uniform float fr1_province_core_brightness = 0.9;
uniform float fr1_player_width_scale = 1.25;
uniform float fr1_player_glow_scale = 1.35;
uniform float fr1_player_brightness = 1.25;
uniform float fr1_player_pulse_amount = 0.12;
uniform float fr1_player_pulse_period = 3.2;
uniform float fr1_hatch_period_px = 7.0;
uniform float fr1_hatch_duty = 0.5;
uniform float fr1_hatch_band_px = 10.0;
uniform float fr1_hatch_alpha = 0.75;
uniform float fr1_parchment_width_scale = 1.7;
uniform float fr1_parchment_alpha_scale = 1.15;
uniform float fr1_parchment_glow_scale = 0.6;
uniform float fr1_parchment_ink_mix = 0.35;

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

#include "res://shaders/relief_quadtree.gdshaderinc"

int province_at(vec2 p) {
	ivec2 sz = textureSize(province_ids, 0);
	ivec2 texel = clamp(ivec2(floor(p)), ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

vec3 info_of(int id) {
	if (id <= 0) {
		return vec3(0.0);
	}
	vec4 c = texelFetch(fr1_info, ivec2(clamp(id, 0, textureSize(fr1_info, 0).x - 1), 0), 0);
	return vec3(round(c.r * 255.0), round(c.g * 255.0), c.b);
}

vec3 faction_rgb(float index) {
	vec3 c = texelFetch(fr1_palette, ivec2(clamp(int(index), 0, textureSize(fr1_palette, 0).x - 1), 0), 0).rgb;
	return pow(c, vec3(2.2));
}

float border_d(vec2 uv) {
	vec3 s = (texture(border_dist, uv).rgb * 255.0 - 128.0) / border_dist_scale;
	return min(abs(s.r), min(abs(s.g), abs(s.b)));
}

// Composition « par-dessus » (alpha non prémultiplié).
void over(inout vec4 dst, vec3 rgb, float a) {
	float out_a = a + dst.a * (1.0 - a);
	if (out_a > 1e-5) {
		dst.rgb = (rgb * a + dst.rgb * dst.a * (1.0 - a)) / out_a;
	}
	dst.a = out_a;
}

// Hachures diagonales stables sur la carte : période en pixels écran, calculée à deux octaves
// d'échelle et fondue (pas de rampement pendant le zoom).
float hatch_at(vec2 p, float px) {
	float lv = log2(max(px, 1e-4));
	float l0 = floor(lv);
	float t = lv - l0;
	float s0 = exp2(l0) * fr1_hatch_period_px;
	float s1 = s0 * 2.0;
	float c = (p.x + p.y) * 0.70710678;
	float a0 = fract(c / s0);
	float a1 = fract(c / s1);
	float edge = 0.08;
	float h0 = smoothstep(0.0, edge, a0) * (1.0 - smoothstep(fr1_hatch_duty, fr1_hatch_duty + edge, a0));
	float h1 = smoothstep(0.0, edge, a1) * (1.0 - smoothstep(fr1_hatch_duty, fr1_hatch_duty + edge, a1));
	return mix(h0, h1, t);
}

void vertex() {
	// Même chemin que le terrain : 3D (quadtree + lit creusé) ou parchemin seul (plat, shader
	// substitué par StrategicView au poids 1).
	if (campaign_parchment < 0.999) {
		VERTEX = qt_vertex(VERTEX);
		world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
		float carve = river_carve(world_pos.xz, map_size);
		VERTEX.y -= carve;
		world_pos.y -= carve;
	} else {
		world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	}
	POSITION = PROJECTION_MATRIX * (MODELVIEW_MATRIX * vec4(VERTEX, 1.0));
	POSITION.z *= 1.0 + fr1_depth_bias;  // profondeur inversée : z plus grand = plus proche
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = (p + vec2(0.5)) / map_size;
	float footprint = max(length(dFdx(p)), length(dFdy(p)));
	float d = has_border_dist ? border_d(uv) : 1e3;
	float px = clamp(fwidth(d), footprint * 0.5, footprint * 2.0);
	px = max(px, 1e-4);

	float pw = smoothstep(0.0, 1.0, campaign_parchment);
	float wscale = fr1_width_scale * mix(1.0, fr1_parchment_width_scale, pw);
	float gscale = wscale * mix(1.0, fr1_parchment_glow_scale, pw);
	float reach = max(max(fr1_realm_glow_px * fr1_player_glow_scale * gscale, fr1_hatch_band_px * wscale),
			fr1_realm_outline_px * fr1_player_width_scale * wscale) + 1.0;
	if (fr1_alpha <= 0.001 || d > reach * px) {
		discard;
	}

	// Côtés : gradient du champ de distance (comme terrain.gdshader).
	vec2 e = vec2(1.0 / map_size.x, 0.0);
	vec2 f = vec2(0.0, 1.0 / map_size.y);
	vec2 gd = vec2(border_d(uv + e) - border_d(uv - e), border_d(uv + f) - border_d(uv - f));
	vec2 away = gd / max(length(gd), 1e-4);
	int id = province_at(p + away * max(0.0, 1.3 - d));
	int other_id = province_at(p - away * (d + 1.3));
	if (id <= 0 || other_id <= 0 || other_id == id) {
		discard;
	}
	vec3 me = info_of(id);
	vec3 them = info_of(other_id);
	bool realm = me.x != them.x || me.y != them.y;
	bool occupied = me.y > 0.0 && me.y != me.x;
	bool player = me.z > 0.5;
	float ds = d / px;  // distance au trait, pixels écran

	vec3 own = me.x > 0.0 ? faction_rgb(me.x) : fr1_neutral_color;
	vec3 occupier = faction_rgb(me.y);
	vec3 ink = fr1_neutral_color;
	if (fr1_neutral) {
		own = ink;
		occupier = ink * 1.6;
	}
	own = mix(own, ink, fr1_parchment_ink_mix * pw);
	occupier = mix(occupier, ink, fr1_parchment_ink_mix * pw);

	vec4 acc = vec4(0.0);
	if (realm) {
		float ws = wscale * (player ? fr1_player_width_scale : 1.0);
		float gs = gscale * (player ? fr1_player_glow_scale : 1.0);
		float bright = fr1_realm_core_brightness * (player ? fr1_player_brightness : 1.0);
		if (fr1_glow && me.x > 0.0 && !fr1_neutral) {
			float g = pow(clamp(1.0 - ds / max(fr1_realm_glow_px * gs, 0.5), 0.0, 1.0), fr1_realm_glow_exponent);
			float pulse = player ? 1.0 + fr1_player_pulse_amount * sin(TIME * 6.2831853 / max(fr1_player_pulse_period, 0.1)) : 1.0;
			over(acc, own * bright, g * fr1_realm_glow_alpha * pulse * (1.0 - 0.5 * pw));
		}
		if (occupied) {
			float band = 1.0 - smoothstep(fr1_hatch_band_px * ws * 0.6, fr1_hatch_band_px * ws, ds);
			float h = fr1_hatch ? hatch_at(p, px) : 1.0;
			over(acc, occupier * bright, band * h * fr1_hatch_alpha);
		}
		float outline = 1.0 - smoothstep(fr1_realm_outline_px * ws - 0.6, fr1_realm_outline_px * ws + 0.6, ds);
		over(acc, ink * 0.35, outline * fr1_realm_outline_alpha);
		float core = 1.0 - smoothstep(fr1_realm_core_px * ws - 0.6, fr1_realm_core_px * ws + 0.6, ds);
		over(acc, (fr1_neutral ? ink : own * bright), core * fr1_realm_core_alpha);
	} else {
		float core = 1.0 - smoothstep(fr1_province_core_px * wscale - 0.6, fr1_province_core_px * wscale + 0.6, ds);
		over(acc, own * fr1_province_core_brightness, core * fr1_province_core_alpha);
		if (occupied) {
			float band = 1.0 - smoothstep(fr1_hatch_band_px * wscale * 0.6, fr1_hatch_band_px * wscale, ds);
			float h = fr1_hatch ? hatch_at(p, px) : 1.0;
			over(acc, occupier, band * h * fr1_hatch_alpha);
		}
	}
	float a = acc.a * fr1_alpha * mix(1.0, fr1_parchment_alpha_scale, pw);
	if (a < 0.004) {
		discard;
	}
	ALBEDO = acc.rgb;
	ALPHA = clamp(a, 0.0, 1.0);
}
