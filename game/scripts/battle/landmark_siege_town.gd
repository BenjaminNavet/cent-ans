class_name LandmarkSiegeTown
extends Node3D

## Lot L3 (ADR 0026) — ville assiégée tirée du plan d'une ville emblématique : le cœur construit
## l'enceinte, les portes et les maisons (`SiegeWorks::from_layout`) ; ce nœud ajoute ce que le
## rendu générique de `BattleSiege` ne connaît pas : les grandes rues du plan pavées entre les
## maisons (Saint-Jacques, la Harpe à Paris ; Cheapside à Londres…). Rendu seulement.
## Données : `BattleSim.get_siege_landmark()`.

const STREET_WIDTH := 8.0
const STREET_TEXTURE := "res://assets/textures/battle/cobblestone_floor_01_diff.jpg"

var landmark: Dictionary = {}
var stats: Dictionary = {}


## Rues de la ville emblématique, null pour une ville générique.
static func create(info: Dictionary, height_at: Callable) -> LandmarkSiegeTown:
	if info.is_empty():
		return null
	var node := LandmarkSiegeTown.new()
	node.name = "LandmarkSiegeTown"
	node._build(info, height_at)
	return node


func _build(info: Dictionary, height_at: Callable) -> void:
	landmark = info
	var material := StandardMaterial3D.new()
	if ResourceLoader.exists(STREET_TEXTURE):
		material.albedo_texture = load(STREET_TEXTURE)
	material.albedo_color = Color(0.78, 0.74, 0.68)
	material.uv1_triplanar = true
	material.uv1_world_triplanar = true
	material.uv1_scale = Vector3.ONE * 0.25
	material.roughness = 0.95
	var surface := SurfaceTool.new()
	surface.begin(Mesh.PRIMITIVE_TRIANGLES)
	var quads := 0
	for street in info.get("streets", []):
		var points: PackedVector2Array = street
		for i in points.size() - 1:
			quads += _segment(surface, points[i], points[i + 1], height_at)
	if quads == 0:
		return
	surface.generate_normals()
	var mesh_instance := MeshInstance3D.new()
	mesh_instance.name = "Streets"
	mesh_instance.mesh = surface.commit()
	mesh_instance.material_override = material
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mesh_instance)
	stats = {"streets": info.get("streets", []).size(), "quads": quads}
	print("LandmarkSiegeTown: %s, gate %s, %d streets" % [info.get("name", ""), info.get("gate_name", ""), stats["streets"]])


## Ruban de la rue a → b, découpé tous les 4 m et posé sur le terrain.
func _segment(surface: SurfaceTool, a: Vector2, b: Vector2, height_at: Callable) -> int:
	var length := a.distance_to(b)
	if length < 0.5:
		return 0
	var direction := (b - a) / length
	var side := Vector2(-direction.y, direction.x) * STREET_WIDTH * 0.5
	var steps := maxi(1, int(ceil(length / 4.0)))
	var h := func(p: Vector2) -> float: return (float(height_at.call(p.x, p.y)) if height_at.is_valid() else 0.0) + 0.12
	for k in steps:
		var p0 := a.lerp(b, float(k) / steps)
		var p1 := a.lerp(b, float(k + 1) / steps)
		var corners := [p0 - side, p0 + side, p1 + side, p1 - side]
		var v: Array[Vector3] = []
		for c in corners:
			var p: Vector2 = c
			v.append(Vector3(p.x, h.call(p), p.y))
		for index in [0, 1, 2, 0, 2, 3]:
			surface.add_vertex(v[index])
	return steps
