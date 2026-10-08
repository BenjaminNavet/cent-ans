shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : cartes de cumulus éclairées par le soleil de la carte (même direction que les ombres de
// nuages RV-C). MultiMesh de quads : position de base = translation de l'instance, taille = échelle
// de l'instance, INSTANCE_CUSTOM.x = tirage de la carte (forme, dérive, présence). Face à la caméra.

#include "res://shaders/map_atmosphere_common.gdshaderinc"

uniform float opacity = 0.55;
uniform float keep = 1.0;                 // part des cartes présentes (ciel chargé : réduite)
uniform float drift_share = 1.0;
uniform vec3 lit_color : source_color = vec3(1.0, 0.97, 0.92);
uniform vec3 shade_color : source_color = vec3(0.56, 0.62, 0.72);

varying vec3 world_pos;
varying vec2 card_uv;
varying float card_rand;
varying vec2 card_ground;

void vertex() {
	card_rand = INSTANCE_CUSTOM.x;
	vec3 base = MODEL_MATRIX[3].xyz;
	base.xz = mod(base.xz + atm_wind * TIME * drift_share, atm_map_size);
	card_ground = base.xz;
	float w = length(MODEL_MATRIX[0].xyz);
	float h = length(MODEL_MATRIX[1].xyz);
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	vec3 up = INV_VIEW_MATRIX[1].xyz;
	world_pos = base + right * VERTEX.x * w + up * VERTEX.y * h;
	card_uv = VERTEX.xy * 2.0;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world_pos, 1.0);
}

void fragment() {
	if (card_rand > keep || atm_fade <= 0.001) {
		discard;
	}
	vec2 p = card_uv;
	vec2 seed = vec2(card_rand * 91.7, card_rand * 37.3);
	// Chou-fleur : bords et sommet creusés par le bruit, base plate (le cumulus a un fond net).
	float n = atm_fbm(p * 1.7 + seed);
	vec2 q = vec2(p.x, p.y > -0.25 ? p.y : -0.25 + (p.y + 0.25) * 3.2);
	float shape = 1.0 - length(q * vec2(1.0, 1.15)) + (n - 0.5) * 0.9;
	float density = smoothstep(0.0, 0.42, shape);
	if (density <= 0.002) {
		discard;
	}
	// Éclairage : normale de sphère + relief du bruit, soleil exprimé dans le repère de la vue.
	vec3 sun_view = normalize((VIEW_MATRIX * vec4(atm_sun_dir, 0.0)).xyz);
	float height_here = sqrt(max(1.0 - dot(p, p), 0.0)) + n * 0.35;
	float toward = atm_fbm((p + sun_view.xy * 0.18) * 1.7 + seed);
	vec3 normal = normalize(vec3(p, max(height_here, 0.15)));
	float diffuse = dot(normal, sun_view) * 0.5 + 0.5;
	float lit = clamp(diffuse + (toward - n) * 2.4 - 0.1, 0.0, 1.0);
	lit *= mix(0.72, 1.0, smoothstep(-0.8, 0.1, p.y));
	vec3 col = mix(shade_color, lit_color * atm_sun_color, smoothstep(0.15, 0.85, lit));
	// Liseré clair des bords tournés vers le soleil.
	col += lit_color * pow(1.0 - clamp(shape * 2.4, 0.0, 1.0), 3.0) * max(sun_view.z * -0.5 + 0.2, 0.0);
	float depth_fade = smoothstep(40.0, 160.0, length(VERTEX));
	float a = density * opacity * atm_fade * depth_fade * atm_edge_fade(card_ground)
			* atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	ALBEDO = col;
	ALPHA = clamp(a, 0.0, 0.85);
}
