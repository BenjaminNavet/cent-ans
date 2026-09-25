// Traits fichés (lot BV1, UR2 : balles, javelots) : couche statique qui grandit pendant la
// bataille (au sol, dans les pavois, dans les pieux, dans les corps). Transformée d'instance =
// position réelle du trait (pointe à l'origine, fût le long de -Z) ; INSTANCE_CUSTOM.x = instant
// où il se fiche (caché avant), INSTANCE_CUSTOM.y = épaisseur relative (0 flèche, 1 carreau ou
// balle, courts et épais ; entre les deux pour un javelot, plus long).
shader_type spatial;
render_mode cull_disabled, specular_disabled;

uniform float time_now = 0.0;

void vertex() {
	float shown = time_now >= INSTANCE_CUSTOM.x ? 1.0 : 0.0;
	vec3 world_origin = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	// Plus fines de près, épaissies de loin pour rester lisibles (sans devenir des confettis).
	float d = distance(CAMERA_POSITION_WORLD, world_origin);
	float widen = clamp(d / 60.0, 1.0, 1.5);
	// Au-delà de 240 m, plus de flèches fichées (poussière de pixels, sommets inutiles).
	shown *= 1.0 - step(240.0, d);
	float bolt = INSTANCE_CUSTOM.y;
	VERTEX.xy *= widen * mix(1.0, 1.5, bolt);
	VERTEX.z *= mix(1.0, 0.5, bolt);
	VERTEX *= shown;
}

void fragment() {
	// Plus sombres que les traits en vol (A1-16 : « confettis blancs trop clairs »).
	ALBEDO = COLOR.rgb * 0.8;
	ROUGHNESS = 0.85;
}
