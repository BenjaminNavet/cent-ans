// Imposteurs lointains des figurines (lot BV3, ADR 0024) : un quadrilatère par soldat, tourné
// vers la caméra autour de la verticale, texturé par l'atlas cuit depuis les figurines V2
// (`BattleImpostors`). Transformée d'instance = celle des figurines (position, cap, taille).
// Colonne = angle de la caméra vu depuis la figurine (8 angles) ; ligne = jeu d'images de
// l'état du régiment × image du cycle (phase propre à chaque soldat).
shader_type spatial;
render_mode world_vertex_coords, cull_disabled, specular_disabled, diffuse_lambert;

uniform sampler2D atlas : source_color, filter_linear_mipmap, repeat_disable;
uniform vec2 cell_m = vec2(1.3, 2.6);
uniform float foot = 0.06;
uniform int cols = 8;
uniform int rows = 16;
uniform int frames = 4;
uniform vec4 set_len = vec4(1.0);
uniform int imp_set = 0;
uniform float anim_time = 0.0;
uniform float highlight = 0.0;
// EP1 (ADR 0076) : palier très lointain. À 1, une figurine sur deux est effacée (quadrilatère
// réduit à un point) et les autres sont élargies d'autant : même masse lue, moitié moins de
// fragments et de mélange alpha.
uniform float thin_out = 0.0;
// NT10 : bandes de l'atlas (copies : variante de casque, habit) ; sang du régiment (0-1).
uniform int copies = 1;
uniform float blood = 0.0;

varying vec2 v_uv;
varying float v_shade;
varying vec3 v_normal_world;
varying vec2 v_local;
varying float v_seed;

float hash1(float n) {
	return fract(sin(n) * 43758.5453123);
}

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec3 ax = MODEL_MATRIX[0].xyz;
	vec3 az = MODEL_MATRIX[2].xyz;
	float scale = length(ax);
	vec3 to_cam = CAMERA_POSITION_WORLD - origin;
	vec2 tc = normalize(to_cam.xz + vec2(1e-4, 0.0));
	// Direction de la caméra dans le repère de la figurine : colonne c = figurine tournée de
	// c × 45° devant une caméra fixe (angle local = −c × 45°).
	float lx = dot(ax.xz, tc) / scale;
	float lz = dot(az.xz, tc) / scale;
	float local_angle = atan(lx, lz);
	float step_a = 6.2831853 / float(cols);
	int col = int(mod(round(-local_angle / step_a), float(cols)) + 0.5) % cols;
	float h = hash1(float(INSTANCE_ID) * 1.37 + 0.11);
	int s = clamp(imp_set, 0, 3);
	float len = max(set_len[s], 0.1);
	int f = int(fract(anim_time / len + h) * float(frames)) % frames;
	int row = s * frames + f;
	int copy = min(int(hash1(float(INSTANCE_ID) * 3.91 + 2.3) * float(copies)), copies - 1);
	col += copy * cols;
	// `world_vertex_coords` : VERTEX arrive déjà en coordonnées monde ; le coin vient des UV.
	vec2 corner = UV;
	vec2 local = vec2(UV.x - 0.5, 1.0 - UV.y);
	v_uv = (vec2(float(col), float(row)) + corner) / vec2(float(cols * copies), float(rows));
	v_local = corner;
	v_seed = hash1(float(INSTANCE_ID) * 5.17 + 0.9);
	vec3 right = vec3(tc.y, 0.0, -tc.x);
	vec3 up = vec3(0.0, 1.0, 0.0);
	float wide = cell_m.x * scale;
	float tall = cell_m.y * scale;
	if (thin_out > 0.5) {
		if (INSTANCE_ID % 2 == 1) {
			wide = 0.0;
			tall = 0.0;
		} else {
			wide *= 1.6;
		}
	}
	VERTEX = origin + right * local.x * wide + up * (local.y * tall - foot * tall);
	v_normal_world = normalize(vec3(tc.x, 1.2, tc.y));
	NORMAL = v_normal_world;
	v_shade = mix(0.9, 1.08, hash1(float(INSTANCE_ID) * 2.71 + 0.53));
}

void fragment() {
	vec4 t = texture(atlas, v_uv);
	// Normale fixée ici : sans face arrière retournée (quadrilatère vu de dos selon l'angle).
	NORMAL = normalize((VIEW_MATRIX * vec4(v_normal_world, 0.0)).xyz);
	// Atlas prémultiplié (fond transparent noir) : couleur vraie = rgb / alpha, mipmaps compris.
	ALBEDO = t.rgb / max(t.a, 0.004) * v_shade + vec3(highlight * 0.25);
	if (blood > 0.0) {
		// NT10 : taches de sang en gros grains (le shader skinné en fait autant de près), plus
		// denses en bas de la silhouette ; la part tachée croît avec le sang du régiment.
		vec2 cell = floor(v_local * vec2(4.0, 9.0));
		float spots = hash1(cell.x * 7.13 + cell.y * 3.71 + v_seed * 91.0);
		float low = smoothstep(0.1, 0.55, v_local.y);
		float amount = step(1.0 - blood * 0.55 * (0.4 + 0.6 * low), spots) * step(v_seed, blood * 1.6);
		ALBEDO = mix(ALBEDO, vec3(0.2, 0.015, 0.012), amount * 0.85);
	}
	// Couverture conservée au loin : dans les mipmaps, la silhouette se dilue dans le fond et
	// l'alpha moyen passerait sous le seuil (figurines qui disparaissent) ; on le relève selon
	// le niveau de mipmap lu.
	float lod = max(textureQueryLod(atlas, v_uv).x, 0.0);
	ALPHA = t.a * (1.0 + lod * 0.9);
	ALPHA_SCISSOR_THRESHOLD = 0.45;
	ROUGHNESS = 0.9;
}
