// Étendard des marqueurs d'armée (lot V3) : tissu qui flotte au vent, tourné vers la caméra
// autour de la hampe (axe Y). Trois modes (`mode`) :
//   0 = drapeau attaché à gauche, armoiries rognées dans l'écu (`game/assets/heraldry/<id>.png`)
//       ou couleur unie de la faction ;
//   1 = flamme / drapeau attaché à gauche, texture entière (texture plus large que haute) ;
//   2 = bannière verticale suspendue à une traverse, texture entière
//       (`game/assets/heraldry/banners/fac_<id>_banner.png`, couleurs plates : l'ombrage du tissu est ici).
// Maillage : QuadMesh subdivisé ; l'origine du nœud est sur la hampe (bord gauche en mode 0/1,
// milieu du bord haut en mode 2).
shader_type spatial;
render_mode cull_disabled, diffuse_lambert_wrap;

uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform int mode = 0;
uniform vec4 faction_color : source_color = vec4(0.6, 0.1, 0.1, 1.0);
// Mode 0 : rectangle (u, v, largeur, hauteur) de l'écu reproduit sur le drapeau.
uniform vec4 crop = vec4(0.17, 0.12, 0.66, 0.5);
uniform float wave_amplitude = 0.22;
uniform float wave_speed = 3.2;
// Décalage vers la caméra (mode 2) : la bannière passe devant la hampe.
uniform float front_offset = 0.18;

varying float v_fold;

void vertex() {
	float phase_seed = MODEL_MATRIX[3].x * 0.1 + MODEL_MATRIX[3].z * 0.07;
	if (mode == 2) {
		// Suspendue par le haut : l'ondulation croît vers le bas et court en travers.
		float along = UV.y;
		float phase = TIME * wave_speed * 0.8 - along * 3.5 + UV.x * 1.6 + phase_seed;
		VERTEX.z += sin(phase) * wave_amplitude * along + front_offset;
		VERTEX.x += sin(phase * 0.7 + 1.3) * wave_amplitude * 0.35 * along;
		v_fold = cos(phase) * along;
	} else {
		float u = UV.x;
		float phase = TIME * wave_speed - u * 5.5 + phase_seed;
		VERTEX.z += sin(phase) * wave_amplitude * u;
		VERTEX.y -= u * u * 0.12;
		v_fold = cos(phase) * u;
	}
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
	float alpha = 1.0;
	if (has_heraldry) {
		if (mode == 0) {
			vec4 tex = texture(heraldry, crop.xy + UV * crop.zw);
			color = mix(color, tex.rgb, tex.a);
		} else {
			vec4 tex = texture(heraldry, UV);
			color = tex.rgb;
			alpha = tex.a;
		}
	}
	float edge;
	if (mode == 2) {
		// Traverse sombre en haut, ourlets sur les côtés.
		edge = step(UV.y, 0.025) + 0.5 * (step(UV.x, 0.025) + step(0.975, UV.x));
	} else {
		edge = step(0.94, UV.x) + step(0.93, UV.y) + step(UV.y, 0.05);
	}
	color *= mix(1.0, 0.5, clamp(edge, 0.0, 1.0));
	ALBEDO = color * (0.86 + 0.18 * v_fold);
	ALPHA = alpha;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
	ROUGHNESS = 0.9;
	SPECULAR = 0.2;
	BACKLIGHT = color * 0.3;
}
