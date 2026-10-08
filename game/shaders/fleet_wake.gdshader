shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded;

// ME1 : sillage d'un navire. UV.x = 0 à la poupe, 1 en bout de traîne ; UV.y = 0..1 en travers.
// Deux bras en V d'écume rompue et une traînée centrale, qui s'élargissent et s'effacent.
uniform vec3 wake_color = vec3(0.8, 0.86, 0.86);
uniform float wake_speed = 0.9;
instance uniform float wake_alpha : hint_range(0.0, 1.0) = 0.5;

float h21(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(h21(i), h21(i + vec2(1.0, 0.0)), u.x), mix(h21(i + vec2(0.0, 1.0)), h21(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	float t = UV.x;
	float across = (UV.y - 0.5) * 2.0;           // -1..1
	float arm = abs(abs(across) - mix(0.15, 0.85, t));
	float arms = 1.0 - smoothstep(0.0, mix(0.10, 0.28, t), arm);
	float centre = (1.0 - smoothstep(0.0, mix(0.18, 0.5, t), abs(across))) * (1.0 - t);
	float flow = TIME * wake_speed;
	float broken = smoothstep(0.35, 0.8, vnoise(vec2(t * 14.0 - flow * 2.0, across * 5.0)));
	float streaks = 0.55 + 0.45 * vnoise(vec2(t * 30.0 - flow * 3.0, across * 2.0 + 7.0));
	float body = max(arms * broken, centre * 0.6 * streaks);
	float fade = (1.0 - smoothstep(0.35, 1.0, t)) * smoothstep(0.0, 0.05, t);
	ALBEDO = wake_color;
	ALPHA = clamp(body * fade * wake_alpha, 0.0, 1.0);
}
