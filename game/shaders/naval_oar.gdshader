// Avirons des galères et des barges (lot NV1) : chaque aviron tourne autour de son tolet
// (UV.x = abscisse du tolet en mètres le long de la coque, UV.y = 1 bâbord / 0 tribord,
// `tools/blender_scripts/naval_ships.py`). Coup d'aviron : balayage horizontal vers l'arrière
// dans l'eau puis retour pale levée ; cadence `stroke_rate` (coups par seconde, 0 = avirons
// levés à l'arrêt), phase décalée d'un banc à l'autre pour la vague des rameurs.
shader_type spatial;
render_mode cull_disabled, diffuse_burley;

uniform vec3 wood : source_color = vec3(0.33, 0.25, 0.15);
uniform float stroke_rate = 0.0;
uniform float stroke_phase = 0.0;
uniform float pivot_y = 1.8;
uniform float half_width = 3.8;
uniform float sweep = 0.45;
uniform float lift = 0.18;
// Avirons rentrés (navire pris, abandonné) : rabattus le long du bordé.
uniform float shipped = 0.0;

void vertex() {
	bool port = UV.y > 0.5;
	float side = port ? -1.0 : 1.0;
	vec3 pivot = vec3(UV.x, pivot_y, side * half_width);
	vec3 p = VERTEX - pivot;
	float phase = stroke_phase - UV.x * 0.05;
	float a = sin(phase) * sweep * min(stroke_rate * 3.0, 1.0);
	float b = (cos(phase) * 0.5 + 0.5) * lift * min(stroke_rate * 3.0, 1.0);
	// Rabattu : balayage vers l'arrière presque parallèle au bordé.
	a = mix(a, -1.25, shipped);
	b = mix(b, 0.25, shipped);
	// Balayage autour de la verticale du tolet (sens miroir d'un bord à l'autre).
	float ca = cos(a * side);
	float sa = sin(a * side);
	p = vec3(p.x * ca + p.z * sa, p.y, -p.x * sa + p.z * ca);
	// Levée de la pale : rotation autour de l'axe longitudinal.
	float cb = cos(b * side);
	float sb = sin(b * side);
	p = vec3(p.x, p.y * cb + p.z * sb, -p.y * sb + p.z * cb);
	VERTEX = pivot + p;
}

void fragment() {
	ALBEDO = wood;
	ROUGHNESS = 0.85;
}
