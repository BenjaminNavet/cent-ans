class_name RoadRenderer
extends Node3D

## Routes de la carte (lot C6), purement visuelles :
## - palier moyen : routes principales (`type` = `main`) en traits fins à largeur minimale écran
##   (`terrain_line.gdshader`, comme les fleuves), en fondu selon le poids du palier ;
## - palier près : toutes les routes en rubans de chemin de terre (`road.gdshader`) drapés sur la
##   surface exacte du terrain affiché (`TerrainBuilder.surface_height_at`), un maillage par
##   tuile de terrain au niveau proche ou fin, reconstruit quand la tuile change de niveau.

@export var main_color: Color = Color(0.36, 0.25, 0.13, 0.85)
@export var main_width: float = 0.5
@export var main_min_px: float = 1.3
@export var ribbon_main_width: float = 0.55
@export var ribbon_width: float = 0.38
## Soulèvement au-dessus de la surface (évite le z-fighting).
@export var lift: float = 0.06
## Pas d'échantillonnage des rubans le long de la route (unités monde).
@export var sample_step: float = 0.5
@export var max_ribbon_builds_per_frame: int = 3

var map_data: MapData
var terrain: TerrainBuilder
var stats: Dictionary = {"roads": 0, "main_roads": 0, "runs": 0, "ribbon_chunks": 0}

var _main_lines: MeshInstance3D
var _main_material: ShaderMaterial
var _ribbon_material: ShaderMaterial
## index de tuile → Array de {points: PackedVector2Array, width: float}
var _runs_by_chunk: Dictionary = {}
## index de tuile → MeshInstance3D
var _ribbons: Dictionary = {}
var _dirty: Dictionary = {}
var _near_alpha := -1.0
var _medium_alpha := -1.0


func build(data: MapData, settlement_data: SettlementData, terrain_builder: TerrainBuilder) -> void:
	for child in get_children():
		child.queue_free()
	_ribbons.clear()
	_runs_by_chunk.clear()
	_dirty.clear()
	map_data = data
	terrain = terrain_builder
	var main_lines := []
	var main_widths := []
	for road in settlement_data.roads:
		if road["main"]:
			main_lines.append(road["points"])
			main_widths.append(main_width)
		_split_by_chunk(road["points"], ribbon_main_width if road["main"] else ribbon_width)
	stats["roads"] = settlement_data.roads.size()
	stats["main_roads"] = main_lines.size()
	_main_lines = MeshInstance3D.new()
	_main_lines.name = "MainRoads"
	_main_lines.mesh = PolylineMesh.build_screen_lines(main_lines, main_widths, data, 0.1)
	_main_material = PolylineMesh.line_material(main_color, main_min_px, 1)
	_main_lines.material_override = _main_material
	_main_lines.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_main_lines)
	_ribbon_material = ShaderMaterial.new()
	_ribbon_material.shader = preload("res://shaders/road.gdshader")
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


## Découpe une polyligne en tronçons par tuile de terrain (tuile du milieu de chaque segment).
func _split_by_chunk(points: PackedVector2Array, width: float) -> void:
	if terrain == null or terrain.chunk_px <= 0:
		return
	var current := -2
	var run := PackedVector2Array()
	for i in points.size() - 1:
		var a := points[i]
		var b := points[i + 1]
		var mid := (a + b) * 0.5
		var index := terrain.chunk_index_at(mid.x, mid.y)
		if index != current:
			_flush_run(current, run, width)
			run = PackedVector2Array([a])
			current = index
		run.append(b)
	_flush_run(current, run, width)


func _flush_run(index: int, run: PackedVector2Array, width: float) -> void:
	if index < 0 or run.size() < 2:
		return
	if not _runs_by_chunk.has(index):
		_runs_by_chunk[index] = []
	_runs_by_chunk[index].append({"points": run, "width": width})
	stats["runs"] = int(stats["runs"]) + 1


func _on_chunk_surface_changed(index: int) -> void:
	if _runs_by_chunk.has(index):
		_dirty[index] = true


## Fondus des deux représentations : `medium` (traits des routes principales) et `near`
## (rubans drapés, construits pour les tuiles au niveau proche ou fin).
func update_view(medium: float, near: float) -> void:
	if _main_lines == null:
		return
	if not is_equal_approx(medium, _medium_alpha):
		_medium_alpha = medium
		var color := main_color
		color.a *= medium
		_main_material.set_shader_parameter("color", color)
		_main_lines.visible = medium > 0.01
	if not is_equal_approx(near, _near_alpha):
		_near_alpha = near
		_ribbon_material.set_shader_parameter("alpha", near)
	var show_ribbons := near > 0.01
	var builds := 0
	for index in _runs_by_chunk:
		var level := terrain.chunk_level(index)
		var wanted := show_ribbons and level >= 1
		var ribbon: MeshInstance3D = _ribbons.get(index)
		if wanted:
			if ribbon == null or _dirty.has(index):
				if builds >= max_ribbon_builds_per_frame:
					continue
				builds += 1
				_build_ribbon(index)
			else:
				ribbon.visible = true
		elif ribbon != null:
			if level == 0:
				ribbon.queue_free()
				_ribbons.erase(index)
			else:
				ribbon.visible = false
	stats["ribbon_chunks"] = _ribbons.size()


## Construit tout de suite les rubans des tuiles voulues (captures, tests).
func flush(near: float) -> void:
	var saved := max_ribbon_builds_per_frame
	max_ribbon_builds_per_frame = 1 << 20
	_near_alpha = -1.0
	update_view(maxf(_medium_alpha, 0.0), near)
	max_ribbon_builds_per_frame = saved


func ribbon_count() -> int:
	return _ribbons.size()


func _build_ribbon(index: int) -> void:
	_dirty.erase(index)
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	for run in _runs_by_chunk[index]:
		var dense := _densify(run["points"])
		var half: float = run["width"] * 0.5
		var base := vertices.size()
		var count := dense.size()
		var along := 0.0
		for i in count:
			var p := dense[i]
			var prev := dense[maxi(i - 1, 0)]
			var next := dense[mini(i + 1, count - 1)]
			var dir := (next - prev).normalized()
			if dir == Vector2.ZERO:
				dir = Vector2.RIGHT
			var perp := Vector2(-dir.y, dir.x) * half
			if i > 0:
				along += p.distance_to(dense[i - 1])
			var left := p + perp
			var right := p - perp
			vertices.append(Vector3(left.x, terrain.surface_height_at(left.x, left.y) + lift, left.y))
			vertices.append(Vector3(right.x, terrain.surface_height_at(right.x, right.y) + lift, right.y))
			normals.append(Vector3.UP)
			normals.append(Vector3.UP)
			uvs.append(Vector2(along, 1.0))
			uvs.append(Vector2(along, -1.0))
		for i in count - 1:
			var a := base + i * 2
			indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])
	var ribbon: MeshInstance3D = _ribbons.get(index)
	if ribbon == null:
		ribbon = MeshInstance3D.new()
		ribbon.name = "Ribbon_%d" % index
		ribbon.material_override = _ribbon_material
		ribbon.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(ribbon)
		_ribbons[index] = ribbon
	ribbon.visible = true
	if vertices.is_empty():
		ribbon.mesh = null
		return
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	ribbon.mesh = mesh


## Points intermédiaires tous les `sample_step` (le ruban suit les facettes du terrain).
func _densify(points: PackedVector2Array) -> PackedVector2Array:
	var result := PackedVector2Array([points[0]])
	for i in range(1, points.size()):
		var a := points[i - 1]
		var b := points[i]
		var n := maxi(1, ceili(a.distance_to(b) / sample_step))
		for k in range(1, n + 1):
			result.append(a.lerp(b, float(k) / n))
	return result
