class_name DeploymentZone
extends Node3D

## Zone de déploiement du joueur (F5c), dessinée au sol : remplissage translucide qui épouse le
## relief et liseré d'or sur le contour. Rendu seulement : le rectangle vient de
## `BattleSim.get_deployment_zone(side)` ({x0, z0, x1, z1}, mètres du champ).

const FILL := Color(1.0, 0.82, 0.3, 0.24)
const EDGE := Color(1.0, 0.85, 0.35, 0.9)
const STEP := 10.0
const LIFT := 0.6
const EDGE_WIDTH := 4.0

var rect := Rect2()


func build(zone: Dictionary, height_at: Callable) -> void:
	for child in get_children():
		child.queue_free()
	var x0 := float(zone.get("x0", 0.0))
	var z0 := float(zone.get("z0", 0.0))
	rect = Rect2(x0, z0, float(zone.get("x1", x0)) - x0, float(zone.get("z1", z0)) - z0)
	if rect.size.x <= 0.0 or rect.size.y <= 0.0:
		return
	_add_mesh(_fill_mesh(height_at), FILL)
	_add_mesh(_edge_mesh(height_at), EDGE)


func _add_mesh(mesh: Mesh, color: Color) -> void:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	material.albedo_color = color
	material.no_depth_test = false
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
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


func _edge_mesh(height_at: Callable) -> Mesh:
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	var p := rect.position
	var q := rect.end
	var corners := [Vector2(p.x, p.y), Vector2(q.x, p.y), Vector2(q.x, q.y), Vector2(p.x, q.y)]
	for k in 4:
		var a: Vector2 = corners[k]
		var b: Vector2 = corners[(k + 1) % 4]
		var inward := (rect.get_center() - (a + b) * 0.5).normalized() * EDGE_WIDTH
		var steps := maxi(1, ceili(a.distance_to(b) / STEP))
		for s in steps:
			var u := a.lerp(b, float(s) / steps)
			var w := a.lerp(b, float(s + 1) / steps)
			var v0 := _point(height_at, u.x, u.y) + Vector3.UP * 0.1
			var v1 := _point(height_at, w.x, w.y) + Vector3.UP * 0.1
			var v2 := _point(height_at, w.x + inward.x, w.y + inward.y) + Vector3.UP * 0.1
			var v3 := _point(height_at, u.x + inward.x, u.y + inward.y) + Vector3.UP * 0.1
			for v in [v0, v1, v2, v0, v2, v3]:
				tool.add_vertex(v)
	return tool.commit()
