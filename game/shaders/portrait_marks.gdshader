// DA2 (ADR 0063) : marques de portrait appliquées à l'image seule (cadre et coiffes à part).
// grey : grisaille d'un défunt ; pallor : pâleur cireuse d'un malade ; dim : captivité (ombre).
shader_type canvas_item;

uniform float grey : hint_range(0.0, 1.0) = 0.0;
uniform float pallor : hint_range(0.0, 1.0) = 0.0;
uniform float dim : hint_range(0.0, 1.0) = 0.0;

void fragment() {
	vec4 c = texture(TEXTURE, UV) * COLOR;
	float luma = dot(c.rgb, vec3(0.299, 0.587, 0.114));
	// Pâleur : moitié désaturée, éclaircie vers un blanc verdâtre de cire.
	vec3 pale = mix(c.rgb, vec3(luma), 0.5) * 0.82 + vec3(0.15, 0.17, 0.12);
	vec3 col = mix(c.rgb, pale, pallor);
	// Grisaille (manuscrit en camaïeu) : gris chaud, légèrement éclairci.
	vec3 grisaille = vec3(luma) * vec3(0.93, 0.91, 0.87) + vec3(0.06);
	col = mix(col, grisaille, grey);
	col *= 1.0 - 0.25 * dim;
	COLOR = vec4(col, c.a);
}
