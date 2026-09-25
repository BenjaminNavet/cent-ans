// EP8 : oiseaux du champ de bataille (`BattleBirds`, un MultiMesh). Silhouette plate (corps et
// deux ailes) dont les ailes battent selon la distance au corps ; INSTANCE_CUSTOM.x = phase,
// .y = fréquence du battement (plané quand faible), .z = 1 si l'oiseau est posé (ailes repliées).
shader_type spatial;
render_mode cull_disabled, specular_disabled, shadows_disabled;

uniform vec3 color : source_color = vec3(0.08, 0.075, 0.07);

void vertex() {
	float span = abs(VERTEX.x);
	float perched = INSTANCE_CUSTOM.z;
	float flap = sin(TIME * INSTANCE_CUSTOM.y + INSTANCE_CUSTOM.x);
	VERTEX.y += flap * span * 0.9 * (1.0 - perched);
	VERTEX.x *= mix(1.0, 0.25, perched);
}

void fragment() {
	ALBEDO = color;
	ROUGHNESS = 1.0;
}
