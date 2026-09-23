// Étendard des marqueurs d'armée (lot V3) : drapeau qui flotte au vent, tourné vers la caméra
// autour de la hampe (axe Y), aux armoiries de la faction (`game/assets/heraldry/<id>.png`,
// rognées à l'intérieur de l'écu) ou à sa couleur unie à défaut.
// Maillage : QuadMesh subdivisé dont le bord gauche (UV.x = 0) est contre la hampe.
shader_type spatial;
render_mode cull_disabled, diffuse_lambert_wrap;

uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform vec4 faction_color : source_color = vec4(0.6, 0.1, 0.1, 1.0);
// Rectangle (u, v, largeur, hauteur) de l'écu reproduit sur le drapeau.
uniform vec4 crop = vec4(0.17, 0.12, 0.66, 0.5);
uniform float wave_amplitude = 0.22;
uniform float wave_speed = 3.2;

varying float v_fold;

void vertex() {
	float u = UV.x;
	float phase = TIME * wave_speed - u * 5.5 + MODEL_MATRIX[3].x * 0.1;
	VERTEX.z += sin(phase) * wave_amplitude * u;
	VERTEX.y -= u * u * 0.12;
	v_fold = cos(phase) * u;
	// Billboard vertical (comme BILLBOARD_FIXED_Y), échelle du nœud conservée.
	MODELVIEW_MATRIX = VIEW_MATRIX * mat4(
		vec4(normalize(cross(vec3(0.0, 1.0, 0.0), INV_VIEW_MATRIX[2].xyz)), 0.0),
		vec4(0.0, 1.0, 0.0, 0.0),
		vec4(normalize(cross(INV_VIEW_MATRIX[0].xyz, vec3(0.0, 1.0, 0.0))), 0.0),
		MODEL_MATRIX[3]);
	MODELVIEW_MATRIX = MODELVIEW_MATRIX * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_NORMAL_MATRIX = mat3(MODELVIEW_MATRIX);
}

void fragment() {
	vec3 color = faction_color.rgb;
	if (has_heraldry) {
		vec4 tex = texture(heraldry, crop.xy + UV * crop.zw);
		color = mix(color, tex.rgb, tex.a);
	}
	// Liseré sombre et plis du tissu.
	float border = step(0.94, UV.x) + step(0.93, UV.y) + step(UV.y, 0.05);
	color *= mix(1.0, 0.55, clamp(border, 0.0, 1.0));
	ALBEDO = color * (0.88 + 0.16 * v_fold);
	ROUGHNESS = 0.9;
	SPECULAR = 0.2;
	BACKLIGHT = color * 0.3;
}
