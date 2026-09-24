class_name DeploymentZone
extends Node3D

## Zone de déploiement du joueur (F5c), dessinée au sol en épousant le relief. A1-02 : un liseré
## d'or lumineux, un halo et des hachures en bordure (shader `deployment_zone.gdshader`), qui
## s'estompent avec la distance caméra, au lieu d'un aplat jaune. Rendu seulement : le rectangle vient de
## `BattleSim.get_deployment_zone(side)` ({x0, z0, x1, z1}, mètres du champ).

const SHADER := preload("res://shaders/deployment_zone.gdshader")
const EDGE := Color(1.0, 0.86, 0.42, 1.0)
const STEP := 10.0
const LIFT := 0.6

var rect := Rect2()


func build(zone: Dictionary, height_at: Callable) -> void:
	for child in get_children():
		child.queue_free()
	var x0 := float(zone.get("x0", 0.0))
	var z0 := float(zone.get("z0", 0.0))
	rect = Rect2(x0, z0, float(zone.get("x1", x0)) - x0, float(zone.get("z1", z0)) - z0)
	if rect.size.x <= 0.0 or rect.size.y <= 0.0:
		return
	var material := ShaderMaterial.new()
	material.shader = SHADER
	material.set_shader_parameter("rect_min", rect.position)
	material.set_shader_parameter("rect_max", rect.end)
	material.set_shader_parameter("edge_color", EDGE)
	var instance := MeshInstance3D.new()
	instance.mesh = _fill_mesh(height_at)
	instance.material_override = material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)


func _point(height_at: Callable, x: float, z: float) -> Vector3:
	return Vector3(x, float(height_at.call(x, z)) + LIFT, z)


func _fill_mesh(height_at: Callable) -> Mesh:
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	var nx := maxi(1, ceili(rect.size.x / STEP))
	var nz := maxi(1, ceili(rect.size.y / STEP))
	for i in nx:
		for j in nz:
			var xa := rect.position.x + rect.size.x * i / nx
			var xb := rect.position.x + rect.size.x * (i + 1) / nx
			var za := rect.position.y + rect.size.y * j / nz
			var zb := rect.position.y + rect.size.y * (j + 1) / nz
			var a := _point(height_at, xa, za)
			var b := _point(height_at, xb, za)
			var c := _point(height_at, xb, zb)
			var d := _point(height_at, xa, zb)
			for v in [a, b, c, a, c, d]:
				tool.add_vertex(v)
	return tool.commit()

