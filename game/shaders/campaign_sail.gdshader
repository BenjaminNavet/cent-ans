// Voile carrée des cogues et nefs de la carte de campagne (lot CV2) : toile teinte aux couleurs
// de la faction, laizes verticales cousues, armes de la faction peintes au centre, bord bas
// sali ; gonflée par le vent (creux parabolique qui respire lentement). UV de la voile : (0, 0)
// en haut à gauche, (1, 1) en bas à droite (`tools/blender_scripts/campaign_fleet.py`).
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform vec3 faction_color : source_color = vec3(0.7, 0.1, 0.1);
uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
// Creux de la voile (unités du modèle) et respiration.
uniform float belly = 0.09;
uniform float breathe = 0.2;
// Axe local de gonflement (la voile est dans le plan YZ, le vent pousse vers +X).
uniform vec3 wind_axis = vec3(1.0, 0.0, 0.0);

void vertex() {
	float across = UV.x * 2.0 - 1.0;
	float down = UV.y;
	float shape = (1.0 - across * across) * (0.35 + 0.65 * sin(3.14159 * clamp(down, 0.0, 1.0) * 0.9 + 0.15));
	float pulse = 1.0 + breathe * sin(TIME * 0.9 + NODE_POSITION_WORLD.x * 0.05);
	VERTEX += wind_axis * belly * shape * pulse;
}

void fragment() {
	vec2 uv = UV;
	// Laizes : bandes verticales de toile, coutures sombres.
	float strips = 7.0;
	float seam = abs(fract(uv.x * strips) - 0.5);
	float seam_line = 1.0 - smoothstep(0.0, 0.04, 0.5 - seam);
	float strip_id = floor(uv.x * strips);
	float tone = 0.92 + 0.08 * fract(sin(strip_id * 12.9898) * 43758.5453);
	vec3 cloth = mix(faction_color, vec3(dot(faction_color, vec3(0.3, 0.59, 0.11))), 0.15) * tone;
	cloth *= 1.0 - 0.25 * seam_line;
	// Armes au centre de la voile.
	vec2 arms_uv = (uv - vec2(0.2, 0.18)) / vec2(0.6, 0.62);
	if (has_heraldry && arms_uv.x >= 0.0 && arms_uv.x <= 1.0 && arms_uv.y >= 0.0 && arms_uv.y <= 1.0) {
		vec4 arms = texture(heraldry, arms_uv);
		cloth = mix(cloth, arms.rgb * 0.9, arms.a);
	}
	// Bord bas sali par les embruns, ombre du creux.
	cloth *= mix(1.0, 0.72, smoothstep(0.8, 1.0, uv.y));
	cloth *= mix(0.85, 1.0, sin(3.14159 * uv.x));
	ALBEDO = cloth;
	ROUGHNESS = 0.92;
	SPECULAR = 0.2;
	BACKLIGHT = cloth * 0.35;
}
