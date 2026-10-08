shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_back, specular_schlick_ggx;

// Lot DN-ME4 : mares et lagunes peu profondes (disque plat) : eau sombre et reflet de ciel selon
// l'incidence ; couleur et bancs de sable par instance (INSTANCE_CUSTOM.rgb couleur, .a bord).
uniform float fw_scale = 1.0;
uniform float fw_alpha = 1.0;
uniform vec3 sky_color : source_color = vec3(0.30, 0.42, 0.48);
uniform float reflect_amount = 0.25;
uniform float bias_factor = 0.004;

varying vec4 v_data;

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

void vertex() {
	v_data = INSTANCE_CUSTOM;
	VERTEX.xz *= fw_scale;
	// Rapproché de la caméra : la surface affichée (maillage plus grossier au loin) ne recouvre pas la mare.
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 to_cam = CAMERA_POSITION_WORLD - world;
	VERTEX += (inverse(mat3(MODEL_MATRIX)) * normalize(to_cam)) * length(to_cam) * bias_factor;
}

void fragment() {
	float r = length(UV - vec2(0.5)) * 2.0;
	float edge = 1.0 - smoothstep(0.55, 1.0, r + (hash12(floor(UV * 9.0) + v_data.a * 7.0) - 0.5) * 0.35);
	float fres = pow(1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0), 3.0);
	vec3 col = mix(v_data.rgb, sky_color, clamp(reflect_amount * 0.3 + fres * reflect_amount, 0.0, 0.6));
	// Banc de sable clair vers le bord.
	// Banc de sable : tache décalée (pas d'anneau), une mare sur trois.
	float bar = smoothstep(0.35, 0.0, length(UV - vec2(0.62, 0.42) * 0.8 - vec2(0.1)) * 2.0) * step(0.66, hash12(vec2(v_data.a * 91.0, 3.0)));
	col = mix(col, vec3(0.55, 0.52, 0.42), bar * 0.7);
	ALBEDO = col;
	ROUGHNESS = 0.12;
	SPECULAR = 0.6;
	ALPHA = edge * fw_alpha * 0.8;
}
