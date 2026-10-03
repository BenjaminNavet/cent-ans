// Lot RV-F : panaches régionaux de la carte de campagne (vue moyenne, caméra haute). Une fumée
// fine et couchée par le vent au-dessus de chaque colonie (relais des cheminées 1:1, trop petites
// au-delà du palier près), panache sombre et épais au-dessus des villes assiégées et des provinces
// ravagées. Instanciés par MultiMesh (`LifeEffects`, un appel de rendu). Taille fixée en part de la
// hauteur d'écran au point visé : le panache garde quelques pixels de large à tout zoom, jamais
// sous le pixel au centre de la vue (pas de scintillement au dézoom).
// Quad vertical (x ∈ [-0.5, 0.5], y ∈ [0, 1]).
// INSTANCE_CUSTOM : r graine, g noirceur (0 foyer, 1 incendie), b rang de densité (0-1 : le
// panache n'est dessiné que si rang < `density`), a vitesse.
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled, fog_disabled;

// Direction du vent (plan de carte x, z), alignée sur le vent météo de `campaign_weather`
// (`weather_wind`, d'ouest) ; couchage du panache (part de sa hauteur déportée au sommet).
uniform vec2 wind_dir = vec2(0.98, 0.2);
uniform float wind_lean = 0.35;
// Taille à l'écran (part de la hauteur de l'écran) : largeur, hauteur d'un foyer ; facteur des
// panaches d'incendie (plus larges et plus hauts).
uniform vec2 screen_size = vec2(0.014, 0.045);
uniform float fire_gain = 1.8;
// Distance caméra → point visé (unités monde), pilotée par le script.
uniform float focus_distance = 300.0;
// Fondu global (palier moyen, saison, parchemin), piloté par le script.
uniform float fade = 0.0;
// Part des foyers qui fument (rang < densité) : plus l'hiver, moins au grand dézoom.
uniform float density = 0.5;
// Opacité de pointe d'un foyer et d'un incendie.
uniform float smoke_alpha = 0.5;
uniform float fire_alpha = 0.85;
uniform vec3 light_smoke : source_color = vec3(0.86, 0.85, 0.83);
uniform vec3 dark_smoke : source_color = vec3(0.12, 0.11, 0.10);
// Lumière ambiante approximative (unshaded) : teinte chaude de la carte.
uniform vec3 sun_tint = vec3(1.0, 0.96, 0.9);

varying vec4 v_custom;
varying vec2 v_uv;
varying float v_keep;

float hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise2(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), f.x), mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), f.x), f.y);
}

void vertex() {
	v_custom = INSTANCE_CUSTOM;
	v_uv = vec2(VERTEX.x + 0.5, VERTEX.y);
	float dark = INSTANCE_CUSTOM.g;
	// Foyers au-delà de la densité : quad replié (aucun pixel à ombrer). Incendies toujours.
	v_keep = dark > 0.5 ? 1.0 : step(INSTANCE_CUSTOM.b, density);
	vec3 origin = MODEL_MATRIX[3].xyz;
	origin.y += MODEL_MATRIX[2].y;  // levée : faîte de la maquette (colonne z de la base)
	// Taille monde : part de la hauteur d'écran mesurée à la distance du point visé (même taille
	// monde pour tous les panaches d'une image : perspective naturelle, l'horizon reste léger),
	// bornée à la distance propre de l'instance (pas de panache géant au pied de la caméra).
	float dist = min(focus_distance, length(INV_VIEW_MATRIX[3].xyz - origin) * 1.5);
	// |P[1][1]| = 1 / tan(fov / 2) (signe selon l'API graphique : retournement de Y).
	float px_world = 2.0 * dist / max(abs(PROJECTION_MATRIX[1][1]), 1e-3);
	float gain = mix(1.0, fire_gain, dark) * mix(0.85, 1.15, INSTANCE_CUSTOM.r);
	float sx = screen_size.x * px_world * gain * v_keep;
	float sy = screen_size.y * px_world * gain * v_keep;
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	right = normalize(vec3(right.x, 0.0, right.z) + vec3(1e-4, 0.0, 0.0));
	vec3 up = normalize(mix(vec3(0.0, 1.0, 0.0), INV_VIEW_MATRIX[1].xyz, 0.5));
	float t = VERTEX.y;
	// Monte droit puis se couche avec le vent ; s'élargit en montant ; lente oscillation.
	float widen = mix(0.35, 1.0, sqrt(t));
	float sway = sin(TIME * 0.35 + INSTANCE_CUSTOM.r * 40.0 + t * 2.5) * 0.06 * t;
	vec3 wind3 = normalize(vec3(wind_dir.x, 0.0, wind_dir.y) + vec3(1e-4, 0.0, 0.0));
	vec3 world = origin + right * (VERTEX.x * sx * widen + sway * sy)
		+ up * t * sy * mix(1.0, 0.75, wind_lean)
		+ wind3 * (pow(t, 1.6) * sy * wind_lean);
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
}

void fragment() {
	if (v_keep < 0.5) {
		discard;
	}
	float seed = v_custom.r;
	float dark = v_custom.g;
	float speed = mix(0.12, 0.22, v_custom.a);
	vec2 uv = v_uv;
	float t = uv.y;
	// Bruit qui monte, basse fréquence (le panache fait quelques pixels de large : pas de grain fin).
	vec2 q = vec2(uv.x * 1.1, t * 1.8 - TIME * speed) + seed * 17.0;
	float n = noise2(q * 1.5) * 0.65 + noise2(q * 3.1 + 5.0) * 0.35;
	float dx = abs(uv.x - 0.5) * 2.0;
	// Filet serré au pied, bouffée qui s'ouvre et se déchire en montant.
	float core = 1.0 - smoothstep(mix(0.1, 0.35, t), 1.0, dx + (n - 0.5) * mix(0.3, 0.9, t));
	float body = smoothstep(0.0, 0.06, t) * (1.0 - smoothstep(0.25, 0.95, t));
	float density_px = clamp((n - 0.25 + 0.45 * (1.0 - t)) * 1.7, 0.0, 1.0) * core * body;
	float alpha = density_px * mix(smoke_alpha, fire_alpha, dark) * mix(0.75, 1.0, v_custom.a) * fade;
	if (alpha < 0.01) {
		discard;
	}
	vec3 col = mix(light_smoke, dark_smoke, dark * (1.0 - 0.4 * t)) * sun_tint * mix(0.9, 1.05, n);
	ALBEDO = col;
	ALPHA = alpha;
}
