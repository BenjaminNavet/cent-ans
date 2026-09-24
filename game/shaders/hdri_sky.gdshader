// Ciel HDRI (lot V3, A1-05) : panorama équirectangulaire Poly Haven (CC0) tourné pour que son
// soleil tombe sur la lumière directionnelle (LIGHT0), exposé, teinté et désaturé selon la météo ;
// sous l'horizon, fondu vers une couleur de sol (les ciels « pure sky » n'ont pas de sol) ; bande
// d'horizon voilée vers la couleur du brouillard. Statique (pas de TIME) : la carte de radiance
// n'est calculée qu'une fois.
shader_type sky;

uniform sampler2D panorama : filter_linear, repeat_enable, hint_default_black;
// Azimut (degrés, 0 = -Z, vers +X) du soleil dans le panorama ; aligné sur LIGHT0 si align_sun.
uniform float panorama_sun_azimuth = 0.0;
uniform bool align_sun = true;
uniform float extra_yaw = 0.0;
uniform float exposure = 1.0;
uniform vec3 tint : source_color = vec3(1.0);
uniform float saturation = 1.0;
uniform vec3 ground_color : source_color = vec3(0.3, 0.31, 0.28);
uniform vec3 haze_color : source_color = vec3(0.75, 0.78, 0.82);
uniform float haze_amount : hint_range(0.0, 1.0) = 0.35;
// Disque solaire ajouté (le soleil des HDRI est souvent écrêté) ; 0 par temps couvert.
uniform float sun_disc = 0.0;
// Plafond de luminance (après exposition) : le soleil peint dans le panorama (des dizaines de
// milliers de fois la moyenne) ne doit pas entrer dans la lumière ambiante ni dans les reflets,
// la lumière directionnelle joue déjà ce rôle (sinon les ombres portées disparaissent).
uniform float max_luminance = 2.5;


vec3 sample_panorama(vec3 dir, float yaw) {
	float azimuth = atan(dir.x, -dir.z) - yaw;
	float u = fract(azimuth / (2.0 * PI) + 0.5);
	float v = acos(clamp(dir.y, -1.0, 1.0)) / PI;
	return texture(panorama, vec2(u, v)).rgb;
}

void sky() {
	vec3 dir = EYEDIR;
	float yaw = radians(extra_yaw);
	if (align_sun && LIGHT0_ENABLED) {
		vec3 l = LIGHT0_DIRECTION;
		yaw += atan(l.x, -l.z) - radians(panorama_sun_azimuth);
	}
	float h = dir.y;
	// Au-dessus d'un léger seuil : le panorama ; en dessous, sa ligne d'horizon étirée puis le sol.
	vec3 sky_dir = normalize(vec3(dir.x, max(h, 0.015), dir.z));
	vec3 col = sample_panorama(sky_dir, yaw);
	float peak = max(col.r, max(col.g, col.b)) * exposure;
	if (peak > max_luminance) {
		col *= max_luminance / peak;
	}
	float grey = dot(col, vec3(0.2126, 0.7152, 0.0722));
	col = mix(vec3(grey), col, saturation) * tint;
	// Voile d'horizon (perspective aérienne vers le brouillard de la scène).
	float band = pow(1.0 - clamp(h, 0.0, 1.0), 8.0);
	col = mix(col, haze_color * max(grey, 0.2) / max(dot(haze_color, vec3(0.333)), 0.05), band * haze_amount);
	if (h < 0.0) {
		col = mix(col, ground_color * max(grey, 0.3), smoothstep(0.0, -0.12, h));
	}
	if (!AT_CUBEMAP_PASS && LIGHT0_ENABLED && sun_disc > 0.0) {
		float d = dot(dir, LIGHT0_DIRECTION);
		col += LIGHT0_COLOR * smoothstep(0.99965, 0.99985, d) * sun_disc * 6.0;
	}
	COLOR = col * exposure;
}
