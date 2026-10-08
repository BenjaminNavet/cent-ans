shader_type spatial;
render_mode depth_draw_opaque, cull_back, diffuse_lambert_wrap, specular_disabled, skip_vertex_transform;

// Lot DN-ME2 : troupeaux et faune de la carte de campagne (`FaunaLayer`). Aucune armature : errance
// lente, pas et rebond en deplacement de sommets.
//
// Donnees par instance (une bete) :
//  - MODEL_MATRIX : echelle uniforme (unites monde par unite de maillage), origine = point de
//    depart de la bete (x, altitude en METRES, z) ; l'altitude affichee vient de
//    `campaign_display_height` (source unique, comme les maquettes) ;
//  - INSTANCE_CUSTOM : x, y = ecart (unites monde) au centre du troupeau, z = profil de saison
//    (entier) + graine du troupeau (fraction), w = rang du troupeau (0-1, amincissement).
// Errance : Lissajous deterministe par troupeau (graine + `turn_phase`, change a chaque tour),
// temps deforme `tau = t - sin(w t + phi) / w` : la vitesse tombe a zero puis double (arrets pour
// brouter) ; les pattes suivent `tau`, donc s'arretent avec la bete.

#include "res://shaders/campaign_relief.gdshaderinc"

global uniform vec4 campaign_season;

uniform sampler2D albedo_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_texture = false;
uniform vec4 albedo : source_color = vec4(0.55, 0.45, 0.35, 1.0);
// Partage de la couche : fondu de distance (0-1), amincissement par plafond, grossissement.
uniform float fade : hint_range(0.0, 1.0) = 1.0;
uniform float thin : hint_range(0.0, 1.0) = 1.0;
uniform float size_mult = 1.0;
uniform float spread_mult = 1.0;
uniform float turn_phase = 0.0;
// Maillage : (centre x, pied y, centre z) ; longueur et hauteur ; axe long = X (sinon Z).
uniform vec3 mesh_origin = vec3(0.0);
uniform vec2 mesh_size = vec2(1.0, 0.6);
uniform bool long_x = false;
// Errance : rayon (unites monde, avant `spread_mult`) et pulsation (rad/s : vitesse / rayon).
uniform float wander_radius = 0.0003;
uniform float wander_omega = 0.007;
uniform float time_scale = 1.0;
// Allure : (cadence, debattement, levee, rebond) puis (baisse de la tete a l'arret).
uniform vec4 gait = vec4(1.25, 0.16, 0.06, 0.018);
uniform float graze = 0.22;
// Poids par saison des profils (printemps, ete, automne, hiver).
uniform vec4 season_profiles[24];

varying float v_tint;
varying float v_rank_ok;

float hash11(float n) {
	return fract(sin(n * 12.9898) * 43758.5453);
}

float hash12(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

// Position d'errance (sans unite, ~[-1, 1]) et sa derivee par rapport a tau.
vec2 wander_at(float tau, vec4 s, out vec2 d) {
	vec2 a = vec2(1.0, 0.85) * (0.8 + 0.4 * s.x);
	vec2 b = vec2(2.4, 2.1) * (0.8 + 0.4 * s.y);
	vec2 p = vec2(sin(a.x * tau + s.z * 6.283), cos(a.y * tau + s.w * 6.283)) + 0.5 * vec2(sin(b.x * tau + s.w * 17.0), cos(b.y * tau + s.z * 13.0));
	d = vec2(a.x * cos(a.x * tau + s.z * 6.283), -a.y * sin(a.y * tau + s.w * 6.283)) + 0.5 * vec2(b.x * cos(b.x * tau + s.w * 17.0), -b.y * sin(b.y * tau + s.z * 13.0));
	return p / 1.5;
}

void vertex() {
	float profile = floor(INSTANCE_CUSTOM.z + 0.0001);
	float herd = INSTANCE_CUSTOM.z - profile;
	vec4 weights = season_profiles[int(clamp(profile, 0.0, 23.0))];
	float presence = dot(campaign_season, weights);
	v_rank_ok = INSTANCE_CUSTOM.w < min(fade, thin) * presence ? 1.0 : 0.0;
	vec3 root = MODEL_MATRIX[3].xyz;
	float base_scale = length(MODEL_MATRIX[0].xyz);
	float member = hash12(root.xz * 9.1);
	v_tint = member;
	// Errance du troupeau (graines communes) + decalage propre a chaque bete.
	vec4 seeds = vec4(hash11(herd * 91.7 + 1.0), hash11(herd * 57.3 + 2.0), hash11(herd * 33.1 + 3.0 + turn_phase), hash11(herd * 71.9 + 4.0 + turn_phase * 1.7));
	float t = TIME * time_scale;
	float w = 0.09 + 0.05 * seeds.x;  // rythme des arrets (rad/s)
	float u = w * t + seeds.z * 6.283;
	float tau = t - sin(u) / w;  // temps deforme : la vitesse (1 - cos u) tombe a 0 puis double
	vec2 dir;
	vec2 wander = wander_at(tau * wander_omega, seeds, dir);
	float moving = smoothstep(0.08, 0.5, 1.0 - cos(u));
	float heading = length(dir) > 1e-5 ? atan(dir.x, dir.y) : 0.0;
	// Maillage : recentre, axe long en +Z.
	vec3 p = VERTEX - vec3(mesh_origin.x, mesh_origin.y, mesh_origin.z);
	if (long_x) {
		p = vec3(-p.z, p.y, p.x);
	}
	float len = mesh_size.x;
	float hgt = mesh_size.y;
	// Allure : pattes en diagonale (avant gauche + arriere droite), levee, rebond du corps, tete basse.
	float phase = tau * gait.x * 6.283 + member * 6.283;
	float low = 1.0 - smoothstep(0.22 * hgt, 0.6 * hgt, p.y);
	float front = step(0.0, p.z);
	float left = step(0.0, p.x);
	float leg_phase = phase + 3.14159 * (front + left);
	p.z += sin(leg_phase) * gait.y * len * low * moving;
	p.y += max(0.0, cos(leg_phase)) * gait.z * len * low * moving;
	p.y += abs(sin(phase)) * gait.w * len * moving;
	float head = smoothstep(0.2 * len, 0.5 * len, p.z) * smoothstep(0.5 * hgt, 0.95 * hgt, p.y);
	p.y -= head * graze * hgt * (1.0 - moving);
	p.z += head * graze * hgt * 0.3 * (1.0 - moving);
	// Cap : lacet vers la direction de marche.
	float c = cos(heading);
	float s = sin(heading);
	vec3 oriented = vec3(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
	oriented *= base_scale * size_mult;
	vec2 offset = INSTANCE_CUSTOM.xy * (spread_mult - 1.0) + wander * wander_radius * spread_mult;
	vec2 xz = root.xz + offset;
	float ground = campaign_display_height(root.y, xz);
	vec3 world = vec3(xz.x, ground, xz.y) + oriented;
	// Repere vue (skip_vertex_transform) ; une bete masquee se replie sur son origine.
	vec3 shown = v_rank_ok > 0.5 ? world : root;
	VERTEX = (VIEW_MATRIX * vec4(shown, 1.0)).xyz;
	vec3 n = vec3(NORMAL.x * c + NORMAL.z * s, NORMAL.y, -NORMAL.x * s + NORMAL.z * c);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}

void fragment() {
	vec3 col = albedo.rgb;
	if (has_texture) {
		col = texture(albedo_texture, UV).rgb * albedo.rgb;
	}
	col *= 0.9 + 0.2 * v_tint;
	ALBEDO = col;
	ROUGHNESS = 0.9;
}
