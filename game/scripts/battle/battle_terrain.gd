class_name BattleTerrain
extends Node3D

## Champ de bataille maillé depuis `BattleSim.get_terrain()` : grille de hauteurs colorée par
## sommet (herbe selon l'altitude, sous-bois, boue, berges, gués), eau de la rivière en ruban,
## arbres low-poly instanciés (MultiMesh) dans les zones de forêt, et un sol lointain pour
## que l'horizon ne soit pas vide. Rendu seulement : les zones viennent de la simulation.

const GRASS_LOW := Color(0.47, 0.60, 0.29)
const GRASS_HIGH := Color(0.66, 0.66, 0.38)
const FOREST_FLOOR := Color(0.22, 0.33, 0.15)
const MUD := Color(0.36, 0.28, 0.18)
const BANK := Color(0.48, 0.42, 0.28)
const FORD := Color(0.62, 0.56, 0.40)
const SNOW := Color(0.90, 0.92, 0.95)

var terrain: Dictionary = {}
var tree_count: int = 0
var _heights: PackedFloat32Array
var _nx: int = 0
var _nz: int = 0
var _resolution: float = 10.0


func build(p_terrain: Dictionary, weather: String) -> void:
	terrain = p_terrain
	_heights = terrain.get("heights", PackedFloat32Array())
	_nx = int(terrain.get("nx", 0))
	_nz = int(terrain.get("nz", 0))
	_resolution = float(terrain.get("resolution", 10.0))
	for child in get_children():
		child.queue_free()
	if _nx < 2 or _nz < 2:
		return
	_build_ground(weather)
	_build_skirt(weather)
	if terrain.has("river"):
		_build_river(terrain["river"])
	_build_trees()


## Hauteur bilinéaire (même formule que la simulation).
func height_at(x: float, z: float) -> float:
	if _nx < 2:
		return 0.0
	var fx := clampf(x / _resolution, 0.0, float(_nx - 1))
	var fz := clampf(z / _resolution, 0.0, float(_nz - 1))
	var ix := int(floor(fx))
	var iz := int(floor(fz))
	var ix1 := mini(ix + 1, _nx - 1)
	var iz1 := mini(iz + 1, _nz - 1)
	var tx := fx - ix
	var tz := fz - iz
	var top := _heights[iz * _nx + ix] * (1.0 - tx) + _heights[iz * _nx + ix1] * tx
	var bottom := _heights[iz1 * _nx + ix] * (1.0 - tx) + _heights[iz1 * _nx + ix1] * tx
	return top * (1.0 - tz) + bottom * tz


func _in_zones(zones: Array, x: float, z: float, margin: float = 0.0) -> bool:
	for zone in zones:
		var dx: float = x - float(zone["x"])
		var dz: float = z - float(zone["z"])
		var r: float = float(zone["radius"]) + margin
		if dx * dx + dz * dz <= r * r:
			return true
	return false


func _river_distance(x: float, z: float) -> float:
	if not terrain.has("river"):
		return INF
	var points: PackedVector2Array = terrain["river"]["points"]
	var best := INF
	# Points tous les 10 m en x croissant : seuls les segments voisins comptent.
	var center := int(x / 10.0)
	for i in range(maxi(center - 3, 0), mini(center + 3, points.size() - 1)):
		var a := points[i]
		var b := points[i + 1]
		var p := Geometry2D.get_closest_point_to_segment(Vector2(x, z), a, b)
		best = minf(best, p.distance_to(Vector2(x, z)))
	return best


func _in_ford(x: float) -> bool:
	if not terrain.has("river"):
		return false
	for ford in terrain["river"]["fords"]:
		if absf(x - float(ford["x"])) <= float(ford["half_width"]):
			return true
	return false


func _ground_color(x: float, z: float, h: float, min_h: float, max_h: float, weather: String) -> Color:
	var t := clampf((h - min_h) / maxf(max_h - min_h, 1.0), 0.0, 1.0)
	var noise := sin(x * 0.043 + z * 0.021) * 0.5 + sin(x * 0.011 - z * 0.037) * 0.5
	var color := GRASS_LOW.lerp(GRASS_HIGH, t).lerp(Color(0.38, 0.5, 0.22), 0.2 + 0.2 * noise)
	if _in_zones(terrain.get("forests", []), x, z, 4.0):
		color = FOREST_FLOOR
	if _in_zones(terrain.get("mud", []), x, z):
		color = MUD.lerp(color, 0.15)
	var river_width: float = float(terrain["river"]["width"]) if terrain.has("river") else 0.0
	if river_width > 0.0:
		var d := _river_distance(x, z)
		if d < river_width * 1.4:
			color = FORD if _in_ford(x) else BANK
	if weather == "snow":
		color = color.lerp(SNOW, 0.7)
	elif weather == "rain":
		color = color.darkened(0.12)
	return color


func _build_ground(weather: String) -> void:
	var min_h := INF
	var max_h := -INF
	for h in _heights:
		min_h = minf(min_h, h)
		max_h = maxf(max_h, h)
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	vertices.resize(_nx * _nz)
	normals.resize(_nx * _nz)
	colors.resize(_nx * _nz)
	for iz in _nz:
		for ix in _nx:
			var i := iz * _nx + ix
			var x := ix * _resolution
			var z := iz * _resolution
			var h := _heights[i]
			vertices[i] = Vector3(x, h, z)
			var hl := _heights[iz * _nx + maxi(ix - 1, 0)]
			var hr := _heights[iz * _nx + mini(ix + 1, _nx - 1)]
			var hd := _heights[maxi(iz - 1, 0) * _nx + ix]
			var hu := _heights[mini(iz + 1, _nz - 1) * _nx + ix]
			normals[i] = Vector3(hl - hr, 2.0 * _resolution, hd - hu).normalized()
			colors[i] = _ground_color(x, z, h, min_h, max_h, weather)
	var indices := PackedInt32Array()
	for iz in range(_nz - 1):
		for ix in range(_nx - 1):
			var a := iz * _nx + ix
			var b := a + 1
			var c := a + _nx
			var d := c + 1
			indices.append_array([a, c, b, b, c, d])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 1.0
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh.surface_set_material(0, mat)
	var instance := MeshInstance3D.new()
	instance.name = "Ground"
	instance.mesh = mesh
	add_child(instance)


## Sol lointain autour du champ (un peu plus bas que les bords).
func _build_skirt(weather: String) -> void:
	var plane := PlaneMesh.new()
	plane.size = Vector2(6000, 6000)
	var mat := StandardMaterial3D.new()
	var color := GRASS_LOW.darkened(0.08)
	if weather == "snow":
		color = color.lerp(SNOW, 0.7)
	mat.albedo_color = color
	mat.roughness = 1.0
	plane.material = mat
	var instance := MeshInstance3D.new()
	instance.name = "Skirt"
	instance.mesh = plane
	var min_h := INF
	for h in _heights:
		min_h = minf(min_h, h)
	instance.position = Vector3(float(terrain.get("width", 1200.0)) * 0.5, min_h - 0.6, float(terrain.get("depth", 800.0)) * 0.5)
	add_child(instance)


func _build_river(river: Dictionary) -> void:
	var points: PackedVector2Array = river["points"]
	var half := float(river["width"]) * 0.5 + 1.5
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for i in range(points.size() - 1):
		var a := points[i]
		var b := points[i + 1]
		var dir := (b - a).normalized()
		var n := Vector2(-dir.y, dir.x) * half
		var ya := height_at(a.x, a.y) + 1.1
		var yb := height_at(b.x, b.y) + 1.1
		var a0 := Vector3(a.x + n.x, ya, a.y + n.y)
		var a1 := Vector3(a.x - n.x, ya, a.y - n.y)
		var b0 := Vector3(b.x + n.x, yb, b.y + n.y)
		var b1 := Vector3(b.x - n.x, yb, b.y - n.y)
		for v in [a0, b0, a1, a1, b0, b1]:
			st.set_normal(Vector3.UP)
			st.add_vertex(v)
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.22, 0.38, 0.52, 0.85)
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.roughness = 0.15
	mat.metallic = 0.1
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh.surface_set_material(0, mat)
	var instance := MeshInstance3D.new()
	instance.name = "River"
	instance.mesh = mesh
	add_child(instance)


func _build_trees() -> void:
	var forests: Array = terrain.get("forests", [])
	var transforms: Array[Transform3D] = []
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	for zone in forests:
		var r := float(zone["radius"])
		var count := clampi(int(PI * r * r / 90.0), 10, 500)
		for _i in count:
			var angle := rng.randf() * TAU
			var dist := sqrt(rng.randf()) * r
			var x := float(zone["x"]) + cos(angle) * dist
			var z := float(zone["z"]) + sin(angle) * dist
			var s := rng.randf_range(0.8, 1.35)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(0.9, 1.2), s))
			transforms.append(Transform3D(basis, Vector3(x, height_at(x, z) - 0.2, z)))
	tree_count = transforms.size()
	if transforms.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleMeshes.tree()
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Trees"
	instance.multimesh = mm
	add_child(instance)
