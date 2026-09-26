// Lot FR1 (ADR 0074) — frontières de faction lumineuses, façon Total War.
// Inclus par terrain.gdshader et terrain_parchment.gdshader APRÈS parchment_map.gdshaderinc
// (s'appuie sur province_at, border_d, map_size, pm_weight, pm_emission, pm_emission_gain) ;
// crochet d'une ligne en fin de fragment : `fr1_borders(p, uv, footprint, albedo, emission)`.
// Uniformes posés par FactionBorders (game/scripts/map/faction_borders.gd) sur le matériau
// partagé du terrain ; `fr1_alpha` vaut 0 par défaut (sans FactionBorders : aucun effet, un
// seul test uniforme).
//
// Frontière : zéro du champ de distance `province_border_dist.png` (trait sous-pixel lissé,
// drapé de fait puisque peint dans le fragment du relief, à tous les zooms). Chaque côté se peint
// de la couleur de SA faction : trait coloré + halo intérieur aux frontières de royaume
// (propriétaire ou contrôleur différent), trait fin et discret entre provinces d'une même
// faction, hachures de l'occupant le long d'une province occupée. Couleur en albédo + part
// émissive (halo lisible à l'ombre du relief) ; en vue parchemin, même partage albédo / émission
// que le papier (pm_output).

// Par province (index raster) : r = faction propriétaire, g = faction contrôleuse (index de
// palette / 255, 0 = aucune), b = 1 si le joueur est propriétaire ou contrôleur.
uniform sampler2D fr1_info : filter_nearest, repeat_disable, hint_default_black;
// Couleur (sRGB) par index de faction.
uniform sampler2D fr1_palette : filter_nearest, repeat_disable, hint_default_black;
uniform float fr1_alpha = 0.0;           // mode de carte × fondu de zoom (0 = rien)
uniform float fr1_width_scale = 1.0;     // dézoom
uniform bool fr1_neutral = false;        // filtre de carte aux couleurs propres : encre neutre
uniform vec3 fr1_neutral_color = vec3(0.012, 0.008, 0.005);
uniform bool fr1_glow = true;
uniform bool fr1_hatch = true;
uniform float fr1_emission = 0.45;       // part émissive de la couleur (3D)
uniform float fr1_pulse = 1.0;           // respiration du halo du joueur (posée chaque image)

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
uniform float fr1_hatch_period_px = 7.0;
uniform float fr1_hatch_duty = 0.5;
uniform float fr1_hatch_band_px = 10.0;
uniform float fr1_hatch_alpha = 0.75;
uniform float fr1_parchment_width_scale = 1.7;
uniform float fr1_parchment_alpha_scale = 1.15;
uniform float fr1_parchment_glow_scale = 0.6;
uniform float fr1_parchment_ink_mix = 0.35;

vec3 fr1_info_of(int id) {
	if (id <= 0) {
		return vec3(0.0);
	}
	vec4 c = texelFetch(fr1_info, ivec2(clamp(id, 0, textureSize(fr1_info, 0).x - 1), 0), 0);
	return vec3(round(c.r * 255.0), round(c.g * 255.0), c.b);
}

vec3 fr1_faction_rgb(float index) {
	vec3 c = texelFetch(fr1_palette, ivec2(clamp(int(index), 0, textureSize(fr1_palette, 0).x - 1), 0), 0).rgb;
	return pow(c, vec3(2.2));
}

// Composition « par-dessus » (alpha non prémultiplié).
void fr1_over(inout vec4 dst, vec3 rgb, float a) {
	float out_a = a + dst.a * (1.0 - a);
	if (out_a > 1e-5) {
		dst.rgb = (rgb * a + dst.rgb * dst.a * (1.0 - a)) / out_a;
	}
	dst.a = out_a;
}

// Hachures diagonales fixées à la carte, période en pixels écran : deux octaves d'échelle fondues
// (pas de rampement pendant le zoom).
float fr1_hatch_at(vec2 p, float px) {
	float lv = log2(max(px, 1e-4));
	float l0 = floor(lv);
	float s0 = exp2(l0) * fr1_hatch_period_px;
	float c = (p.x + p.y) * 0.70710678;
	float a0 = fract(c / s0);
	float a1 = fract(c / (s0 * 2.0));
	float edge = 0.08;
	float h0 = smoothstep(0.0, edge, a0) * (1.0 - smoothstep(fr1_hatch_duty, fr1_hatch_duty + edge, a0));
	float h1 = smoothstep(0.0, edge, a1) * (1.0 - smoothstep(fr1_hatch_duty, fr1_hatch_duty + edge, a1));
	return mix(h0, h1, lv - l0);
}

// Distance au trait (pixels carte) par bilinéaire manuel en flottant : le filtrage matériel n'a
// que 8 bits de poids sous le texel (marches de ~3 m : trait pointillé au zoom vallée).
float fr1_border_d_precise(vec2 uv) {
	ivec2 size = textureSize(border_dist, 0);
	vec2 t = uv * vec2(size) - 0.5;
	vec2 f = floor(t);
	vec2 w = t - f;
	ivec2 i = ivec2(f);
	ivec2 hi = size - ivec2(1);
	vec3 a = texelFetch(border_dist, clamp(i, ivec2(0), hi), 0).rgb;
	vec3 b = texelFetch(border_dist, clamp(i + ivec2(1, 0), ivec2(0), hi), 0).rgb;
	vec3 c = texelFetch(border_dist, clamp(i + ivec2(0, 1), ivec2(0), hi), 0).rgb;
	vec3 e = texelFetch(border_dist, clamp(i + ivec2(1, 1), ivec2(0), hi), 0).rgb;
	vec3 s = (mix(mix(a, b, w.x), mix(c, e, w.x), w.y) * 255.0 - 128.0) / border_dist_scale;
	return min(abs(s.r), min(abs(s.g), abs(s.b)));
}

// Crochet de fin de fragment. `footprint` : pixels carte par pixel écran.
void fr1_borders(vec2 p, vec2 uv, float footprint, inout vec3 albedo, inout vec3 emission) {
	float d = has_border_dist ? border_d(uv) : 1e3;
	vec2 dpdx = dFdx(p);
	vec2 dpdy = dFdy(p);
	float px = max(footprint, 1e-5);  // estimation prudente pour le test de portée
	if (fr1_alpha <= 0.001) {
		return;
	}
	float pw = pm_weight();
	float wscale = fr1_width_scale * mix(1.0, fr1_parchment_width_scale, pw);
	float gscale = wscale * mix(1.0, fr1_parchment_glow_scale, pw);
	float reach = max(max(fr1_realm_glow_px * fr1_player_glow_scale * gscale, fr1_hatch_band_px * wscale),
			fr1_realm_outline_px * fr1_player_width_scale * wscale) + 1.0;
	if (d > reach * px) {
		return;
	}
	// Côtés : gradient du champ de distance (même méthode que le terrain et le parchemin).
	vec2 e = vec2(1.0 / map_size.x, 0.0);
	vec2 f = vec2(0.0, 1.0 / map_size.y);
	vec2 gd = vec2(border_d(uv + e) - border_d(uv - e), border_d(uv + f) - border_d(uv - f));
	vec2 away = gd / max(length(gd), 1e-4);
	// Pixels carte par pixel écran le long de la normale au trait (écart mesuré à l'écran, net
	// même en vue rasante) ; de près, distance en bilinéaire précis.
	px = max(length(vec2(dot(away, dpdx), dot(away, dpdy))), 1e-5);
	if (footprint < 0.25) {
		d = fr1_border_d_precise(uv);
	}
	int id = province_at(p + away * max(0.0, 1.3 - d));
	int other_id = province_at(p - away * (d + 1.3));
	if (id <= 0 || other_id <= 0 || other_id == id) {
		return;
	}
	vec3 me = fr1_info_of(id);
	vec3 them = fr1_info_of(other_id);
	bool realm = me.x != them.x || me.y != them.y;
	bool occupied = me.y > 0.0 && me.y != me.x;
	bool player = me.z > 0.5;
	float ds = d / px;  // distance au trait, pixels écran

	vec3 ink = fr1_neutral_color;
	vec3 own = me.x > 0.0 ? fr1_faction_rgb(me.x) : ink;
	vec3 occupier = fr1_faction_rgb(me.y);
	if (fr1_neutral) {
		own = ink;
		occupier = ink * 3.0;
	}
	own = mix(own, ink, fr1_parchment_ink_mix * pw);
	occupier = mix(occupier, ink, fr1_parchment_ink_mix * pw);

	vec4 acc = vec4(0.0);
	float glow_emit = 0.0;
	if (realm) {
		float ws = wscale * (player ? fr1_player_width_scale : 1.0);
		float gs = gscale * (player ? fr1_player_glow_scale : 1.0);
		float bright = fr1_realm_core_brightness * (player ? fr1_player_brightness : 1.0);
		if (fr1_glow && me.x > 0.0 && !fr1_neutral) {
			float g = pow(clamp(1.0 - ds / max(fr1_realm_glow_px * gs, 0.5), 0.0, 1.0), fr1_realm_glow_exponent);
			float ga = g * fr1_realm_glow_alpha * (player ? fr1_pulse : 1.0) * (1.0 - 0.5 * pw);
			fr1_over(acc, own * bright, ga);
			glow_emit = ga;
		}
		if (occupied) {
			float band = 1.0 - smoothstep(fr1_hatch_band_px * ws * 0.6, fr1_hatch_band_px * ws, ds);
			float h = fr1_hatch ? fr1_hatch_at(p, px) : 1.0;
			fr1_over(acc, occupier * bright, band * h * fr1_hatch_alpha);
		}
		float outline = 1.0 - smoothstep(fr1_realm_outline_px * ws - 0.6, fr1_realm_outline_px * ws + 0.6, ds);
		fr1_over(acc, ink, outline * fr1_realm_outline_alpha);
		float core = 1.0 - smoothstep(fr1_realm_core_px * ws - 0.6, fr1_realm_core_px * ws + 0.6, ds);
		fr1_over(acc, fr1_neutral ? ink : own * bright, core * fr1_realm_core_alpha);
		glow_emit = max(glow_emit, fr1_neutral ? 0.0 : core);
	} else {
		float core = 1.0 - smoothstep(fr1_province_core_px * wscale - 0.6, fr1_province_core_px * wscale + 0.6, ds);
		fr1_over(acc, own * fr1_province_core_brightness, core * fr1_province_core_alpha);
		if (occupied) {
			float band = 1.0 - smoothstep(fr1_hatch_band_px * wscale * 0.6, fr1_hatch_band_px * wscale, ds);
			float h = fr1_hatch ? fr1_hatch_at(p, px) : 1.0;
			fr1_over(acc, occupier, band * h * fr1_hatch_alpha);
		}
	}
	float a = clamp(acc.a * fr1_alpha * mix(1.0, fr1_parchment_alpha_scale, pw), 0.0, 1.0);
	if (a < 0.004) {
		return;
	}
	// Albédo et émission : partage du parchemin (pm_output) au poids pw, part émissive propre en 3D.
	float k = pm_emission * pw;
	albedo = mix(albedo, acc.rgb * (1.0 - k), a);
	vec3 emit3d = acc.rgb * fr1_emission * clamp(glow_emit / max(acc.a, 1e-3), 0.0, 1.0);
	emission = mix(emission, mix(emit3d, acc.rgb * k * pm_emission_gain, pw), a);
}
