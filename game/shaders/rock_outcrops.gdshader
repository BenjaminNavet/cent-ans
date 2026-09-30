shader_type spatial;
render_mode depth_draw_opaque, diffuse_lambert_wrap, specular_schlick_ggx;

// Lot HB5 (ADR 0143) : affleurements rocheux à l'échelle du paysage (`RockOutcrops`, modèles
// TRELLIS de `data/art/rock_outcrops.yaml`, longueur 1 dans le maillage, taille réelle dans la
// transformation d'instance). Hauteur suivant en partie l'exagération verticale du relief
// (`campaign_vertical_scale` × `meters_per_unit`)^`vertical_follow`, puis grossissement
// `outcrop_scale` choisi par le script selon la distance caméra (1 de près). Neige sur les faces
// tournées vers le ciel en hiver. Données par instance (`INSTANCE_CUSTOM`) : x = teinte.

global uniform vec4 campaign_season;
global uniform float campaign_vertical_scale;

uniform float outcrop_scale = 1.0;
uniform float meters_per_unit = 719.0;
uniform float vertical_follow = 0.5;
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap;
uniform vec3 tint = vec3(1.0);
uniform vec3 snow_color : source_color = vec3(0.82, 0.84, 0.88);

varying float v_tint;
varying float v_up;

void vertex() {
	v_tint = INSTANCE_CUSTOM.x;
	float k = pow(max(campaign_vertical_scale * meters_per_unit, 0.05), vertical_follow);
	VERTEX.y *= k;
	VERTEX *= outcrop_scale;
	// Normale d'un étirement vertical k : (nx, ny / k, nz).
	NORMAL = normalize(vec3(NORMAL.x, NORMAL.y / k, NORMAL.z));
	v_up = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz).y;
}

void fragment() {
	vec3 base = texture(albedo_texture, UV).rgb * tint * mix(0.85, 1.1, v_tint);
	float snow = campaign_season.w * smoothstep(0.45, 0.8, v_up) * 0.85;
	ALBEDO = mix(base, snow_color, snow);
	ROUGHNESS = 0.9;
	SPECULAR = 0.25;
}
