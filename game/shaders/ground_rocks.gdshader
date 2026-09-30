shader_type spatial;
render_mode depth_draw_opaque, diffuse_lambert_wrap, specular_schlick_ggx;

// Lot GA3-L2 : rochers proches de la carte de campagne (`GroundClutter`, maillages TRELLIS de
// `Ga3Vegetation.ROCKS`, 120 / 60 / 18 triangles). VT3 : taille réelle dans les instances ;
// même fondu par rang (distance caméra, bord du disque visible) que les touffes
// (`ground_clutter.gdshader`) ; pied enfoncé de `sink` (unités du maillage) ;
// neige sur les faces tournées vers le ciel en hiver. Données par instance (`INSTANCE_CUSTOM`) :
// x = variation de teinte, w = rang (coupe du fondu). Aucune ombre (réglée par le script).

global uniform vec4 campaign_season;

uniform float fade : hint_range(0.0, 1.0) = 1.0;
uniform vec2 focus = vec2(0.0);
uniform float radius = 40.0;
uniform float sink = 0.1;
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap;
uniform vec3 tint = vec3(1.0);
uniform vec3 snow_color : source_color = vec3(0.80, 0.82, 0.86);

varying float v_tint;
varying float v_up;

void vertex() {
	v_tint = INSTANCE_CUSTOM.x;
	VERTEX.y -= sink;
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_up = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz).y;
	float edge = 1.0 - smoothstep(radius * 0.75, radius, distance(world.xz, focus));
	if (INSTANCE_CUSTOM.w > fade * edge) {
		VERTEX = vec3(0.0);  // triangle dégénéré : ni fragment, ni coût de remplissage
	}
}

void fragment() {
	vec3 base = texture(albedo_texture, UV).rgb * tint * mix(0.82, 1.12, v_tint);
	float snow = campaign_season.w * smoothstep(0.5, 0.85, v_up) * 0.85;
	ALBEDO = mix(base, snow_color, snow);
	ROUGHNESS = 0.92;
	SPECULAR = 0.2;
}
