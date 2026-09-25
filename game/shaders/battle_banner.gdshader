// Bannières des régiments (lot V4) : drapeau subdivisé qui ondule au vent (onde qui court de la
// hampe vers le bord libre, amplitude croissante), armoiries de la faction (centre de l'écu) ou
// couleur de livrée, blanc quand le régiment est en déroute. Légère émission pour rester
// lisible de loin.
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform vec3 livery : source_color = vec3(0.2, 0.3, 0.75);
uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform bool full_texture = false; // bannière peinte (texture entière, alpha découpé)
uniform bool routing = false;
uniform float phase = 0.0;
uniform float flag_length = 2.6;
// BV3 : vent de la météo (0 calme : drapeau qui pend ; 1 grand vent : tendu, claquant) et
// rafales (modulation lente de la force, propre à chaque drapeau par `phase`).
uniform float wind_strength = 0.6;
uniform float wind_gust = 0.2;

varying float v_fold;

void vertex() {
	float x = UV.x;
	float gust = sin(TIME * 0.7 + phase * 0.37) * 0.5 + sin(TIME * 1.9 + phase) * 0.25;
	float s = clamp(wind_strength + wind_gust * gust, 0.0, 1.0);
	float t = TIME * mix(2.5, 7.5, s) + phase;
	float amp = mix(0.08, 0.3, s);
	float wave = sin(x * 7.0 - t) * amp + sin(x * 13.0 - t * 1.7 + 1.3) * amp * 0.27;
	VERTEX.z += wave * x * flag_length * 0.35;
	// Sans vent, le bord libre retombe le long de la hampe.
	VERTEX.y -= x * x * mix(0.45, 0.06, s) * flag_length;
	VERTEX.x *= mix(0.8, 1.0, s);
	float slope = (cos(x * 7.0 - t) * 7.0 * amp + cos(x * 13.0 - t * 1.7 + 1.3) * 13.0 * amp * 0.27) * x * 0.35;
	NORMAL = normalize(vec3(-slope, 0.0, 1.0));
	v_fold = clamp(0.5 + wave * 2.0, 0.0, 1.0);
}

void fragment() {
	vec3 color = livery;
	if (full_texture && has_heraldry) {
		vec4 cloth = texture(heraldry, UV);
		color = cloth.rgb;
		ALPHA = cloth.a;
		ALPHA_SCISSOR_THRESHOLD = 0.5;
	} else if (has_heraldry) {
		vec4 arms = texture(heraldry, vec2(mix(0.2, 0.8, UV.x), mix(0.12, 0.58, UV.y)));
		color = mix(livery, arms.rgb, arms.a);
	}
	if (routing) {
		color = vec3(0.92, 0.92, 0.9);
	}
	// Frange (drapeaux tirés de l'écu) et ombrage du tissu : les creux de l'onde plus sombres.
	if (!full_texture) {
		float fringe = step(0.94, UV.x);
		color = mix(color, color * 0.55, fringe);
	}
	color *= 0.85 + 0.15 * v_fold;
	ALBEDO = color;
	EMISSION = color * 0.25;
	ROUGHNESS = 0.9;
}
