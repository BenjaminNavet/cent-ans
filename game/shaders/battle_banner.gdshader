// Bannières des régiments (lot V4) : drapeau subdivisé qui ondule au vent (onde qui court de la
// hampe vers le bord libre, amplitude croissante), armoiries de la faction (centre de l'écu) ou
// couleur de livrée, blanc quand le régiment est en déroute. Légère émission pour rester
// lisible de loin.
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform vec3 livery : source_color = vec3(0.2, 0.3, 0.75);
uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform bool routing = false;
uniform float phase = 0.0;
uniform float flag_length = 2.6;

void vertex() {
	float x = UV.x;
	float t = TIME * 5.5 + phase;
	float wave = sin(x * 7.0 - t) * 0.22 + sin(x * 13.0 - t * 1.7 + 1.3) * 0.06;
	VERTEX.z += wave * x * flag_length * 0.35;
	VERTEX.y -= x * x * 0.12 * flag_length;
	float slope = (cos(x * 7.0 - t) * 7.0 * 0.22 + cos(x * 13.0 - t * 1.7 + 1.3) * 13.0 * 0.06) * x * 0.35;
	NORMAL = normalize(vec3(-slope, 0.0, 1.0));
}

void fragment() {
	vec3 color = livery;
	if (has_heraldry) {
		vec4 arms = texture(heraldry, vec2(mix(0.2, 0.8, UV.x), mix(0.12, 0.58, UV.y)));
		color = mix(livery, arms.rgb, arms.a);
	}
	if (routing) {
		color = vec3(0.92, 0.92, 0.9);
	}
	// Frange et léger ombrage du tissu.
	float fringe = step(0.94, UV.x);
	color = mix(color, color * 0.55, fringe);
	ALBEDO = color;
	EMISSION = color * 0.25;
	ROUGHNESS = 0.9;
}
