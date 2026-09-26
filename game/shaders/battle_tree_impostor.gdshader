// Imposteurs des arbres de bataille (lot DA6) : un quadrilatère par arbre, tourné vers la caméra
// autour de la verticale (un peu incliné vers elle quand elle est haute), texturé par l'atlas
// cuit au lancement depuis les maillages des essences (`BattleTrees.bake_impostors`) : une ligne
// par essence, une colonne par angle de vue cuit (choisi par instance). Transformée d'instance =
// celle de l'arbre (position, lacet, échelle) ; couleur d'instance = teinte de l'arbre ;
// INSTANCE_CUSTOM.r = ligne de l'essence. Normale arrondie reconstituée (houppier sphérique).
shader_type spatial;
render_mode world_vertex_coords, cull_disabled, specular_disabled, diffuse_burley;

#include "res://shaders/battle_tree_lod.gdshaderinc"

uniform sampler2D atlas : source_color, filter_linear_mipmap, repeat_disable;
uniform int cols = 4;
uniform int rows = 6;
// Côté (m) d'une cellule pour une échelle d'instance de 1, par ligne ; marge sous le pied.
uniform float cell_m[8];
uniform float foot = 0.04;
uniform float decor_saturation = 0.6;
uniform vec3 backlight = vec3(0.25, 0.28, 0.12);

varying vec2 v_uv;
varying vec2 v_local;
varying vec3 v_right;
varying vec3 v_face;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec3 cam = MAIN_CAM_INV_VIEW_MATRIX[3].xyz;
	v_lod = tree_lod(origin, cam);
	if (v_lod.x <= 0.0 || v_lod.y <= 0.0) {
		VERTEX = vec3(0.0, -1000.0, 0.0);
	} else {
		float scale = length(MODEL_MATRIX[0].xyz);
		int row = clamp(int(INSTANCE_CUSTOM.r + 0.5), 0, rows - 1);
		float h = fract(sin(dot(origin.xz, vec2(12.9898, 78.233))) * 43758.5453);
		int col = int(h * float(cols)) % cols;
		vec3 to_cam = cam - origin;
		vec2 tc = normalize(to_cam.xz + vec2(1e-4, 0.0));
		vec3 right = vec3(tc.y, 0.0, -tc.x);
		// Caméra haute : quadrilatère incliné vers elle (à moitié), pied fixe.
		float elev = clamp(to_cam.y / max(length(to_cam), 1.0), 0.0, 0.9);
		vec3 face = vec3(tc.x, 0.0, tc.y);
		vec3 up = normalize(vec3(0.0, 1.0, 0.0) - face * elev * 0.5);
		float size = cell_m[row] * scale;
		vec2 local = vec2(UV.x - 0.5, 1.0 - UV.y);
		VERTEX = origin + right * local.x * size + up * (local.y - foot) * size;
		v_uv = (vec2(float(col), float(row)) + UV) / vec2(float(cols), float(rows));
		v_local = local;
		v_right = right;
		v_face = face;
	}
}

void fragment() {
	if (tree_lod_discard(FRAGCOORD.xy, v_lod)) {
		discard;
	}
	vec4 t = texture(atlas, v_uv);
	// Normale arrondie : houppier vu comme une sphère centrée aux deux tiers de la cellule.
	vec2 q = vec2(v_local.x * 2.0, (v_local.y - 0.55) * 2.0);
	float z = sqrt(max(1.0 - dot(q, q) * 0.6, 0.05));
	vec3 n = normalize(v_right * q.x * 0.8 + vec3(0.0, 1.0, 0.0) * (q.y * 0.7 + 0.35) + v_face * z);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
	// Atlas prémultiplié (fond transparent noir) : couleur vraie = rgb / alpha.
	vec3 col = t.rgb / max(t.a, 0.004) * COLOR.rgb;
	ALBEDO = mix(vec3(dot(col, vec3(0.3, 0.59, 0.11))), col, decor_saturation);
	float lod = max(textureQueryLod(atlas, v_uv).x, 0.0);
	ALPHA = t.a * (1.0 + lod * 0.6);
	ALPHA_SCISSOR_THRESHOLD = 0.45;
	ROUGHNESS = 0.9;
	BACKLIGHT = backlight;
}
