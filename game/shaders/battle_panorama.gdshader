// Panorama peint de l'horizon des batailles (lot EP2) : cylindre de rayon `radius` centré sur la
// caméra (en plan), au-delà de l'anneau d'horizon. L'angle d'élévation d'un fragment est mesuré
// depuis `ref_y` (hauteur de l'œil de référence du profil cuit). Chaque colonne de la peinture est
// étirée pour que sa crête moyenne (`paint_crest`) tombe sur la ligne d'horizon réelle de
// l'azimut (profil `skyline_real`, exagéré et borné) ; au-dessus : ciel (fragment rejeté). Dans
// les secteurs de mer (part de mer du profil), horizon marin plat. Brume et teinte propres.
shader_type spatial;
render_mode unshaded, fog_disabled, cull_disabled, shadows_disabled, world_vertex_coords;

uniform sampler2D painting : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D skyline_real : filter_linear, repeat_enable;
uniform float has_painting = 0.0;
uniform float repeats = 4.0;
uniform float paint_offset = 0.0;
uniform float paint_crest = 0.2;
// Crête peinte par colonne (0 = haut de la bande) ; `paint_detail` = part du relief peint gardée
// (0 : la silhouette est la crête réelle, 1 : la peinture garde toutes ses cimes).
uniform sampler2D paint_skyline : filter_linear, repeat_enable;
uniform float paint_detail = 0.3;
uniform float rotation_turns = 0.0;
uniform float ref_y = 25.0;
uniform float radius = 14600.0;
uniform float bottom_deg = -1.2;
uniform float min_angle_deg = 0.55;
uniform float max_angle_deg = 7.5;
uniform float exaggeration = 1.35;
uniform float sea_threshold = 0.55;
uniform float haze = 0.5;
uniform float exposure = 1.0;
uniform float fog_density = 0.0002;
uniform vec3 fog_color : source_color = vec3(0.7, 0.76, 0.84);
uniform vec3 light_tint : source_color = vec3(1.0);
uniform vec3 sea_color : source_color = vec3(0.1, 0.18, 0.22);
uniform vec3 land_fallback : source_color = vec3(0.3, 0.34, 0.26);

varying vec3 world_pos;

void vertex() {
	vec3 cam = INV_VIEW_MATRIX[3].xyz;
	VERTEX.xz += cam.xz;
	world_pos = VERTEX;
}

void fragment() {
	vec3 cam = INV_VIEW_MATRIX[3].xyz;
	vec2 plan = world_pos.xz - cam.xz;
	float bearing = fract(atan(plan.x, -plan.y) / TAU + 1.0);  // tours, 0 = nord, sens horaire
	float elevation = degrees(atan((world_pos.y - ref_y) / radius));
	vec4 real = texture(skyline_real, vec2(fract(bearing - rotation_turns + 1.0), 0.5));
	float crest = clamp(max(real.r * exaggeration, min_angle_deg), min_angle_deg, max_angle_deg);
	float sea = smoothstep(sea_threshold - 0.15, sea_threshold + 0.15, real.b) * (1.0 - smoothstep(0.15, 0.5, real.r));
	// Terre peinte : bande de `paint_crest` (crête moyenne) à 1 (bas), recalée sur [bottom, crest].
	float u = paint_offset + bearing * repeats;
	u = abs(fract(u * 0.5) * 2.0 - 1.0);  // aller-retour : bande sans raccord visible
	float column_crest = texture(paint_skyline, vec2(u, 0.5)).r;
	float anchor = mix(column_crest, paint_crest, paint_detail);
	float s = (elevation - bottom_deg) / max(crest - bottom_deg, 0.01);
	float v = 1.0 - s * (1.0 - anchor);
	vec4 paint = vec4(land_fallback, step(elevation, crest));
	if (has_painting > 0.5) {
		paint = v < 0.0 ? vec4(0.0) : texture(painting, vec2(u, clamp(v, 0.002, 0.998)));
	}
	vec3 land = paint.rgb * exposure * light_tint;
	float land_alpha = paint.a;
	// Mer : horizon plat (angle apparent de l'horizon marin ≈ 0 depuis la hauteur de référence).
	float sea_alpha = step(elevation, -0.02);
	vec3 col = mix(land, sea_color * light_tint, sea);
	float alpha = mix(land_alpha, sea_alpha, sea);
	// Brume : celle de la scène jusqu'au cylindre pour la mer, réglage par météo pour la terre
	// (plus lointaine : moins nette quand la crête réelle est loin).
	float far = smoothstep(20.0, 120.0, real.g);
	float land_haze = clamp(haze * mix(0.85, 1.12, far), 0.0, 1.0);
	float sea_haze = 1.0 - exp(-fog_density * radius);
	col = mix(col, fog_color, mix(land_haze, sea_haze, sea));
	ALBEDO = col;
	ALPHA = alpha;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
