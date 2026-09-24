// Flèches, carreaux et boulets des batailles (lot B4) : un MultiMesh par sorte de trait,
// trajectoire calculée sur la carte graphique (cf. `battle_projectile.gdshaderinc`). Le
// maillage est modelé le long de +Z, pointe en z = 0.
shader_type spatial;
render_mode skip_vertex_transform, diffuse_burley, specular_disabled;

#include "res://shaders/battle_projectile.gdshaderinc"

void vertex() {
	Flight f = projectile(MODEL_MATRIX);
	// Lisibilité de loin (comme dans Total War) : le trait s'épaissit avec la distance pour
	// rester visible (environ un pixel), sans s'allonger.
	float widen = clamp(distance(CAMERA_POSITION_WORLD, f.pos) / 30.0, 1.0, 7.0);
	vec3 local = VERTEX * f.scale * vec3(widen, widen, 1.0);
	vec3 world = f.pos + f.side * local.x + f.up * local.y + f.dir * local.z;
	VERTEX = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	vec3 wn = f.side * NORMAL.x + f.up * NORMAL.y + f.dir * NORMAL.z;
	NORMAL = normalize((VIEW_MATRIX * vec4(wn, 0.0)).xyz);
}

void fragment() {
	ALBEDO = COLOR.rgb;
	ROUGHNESS = 0.8;
}
