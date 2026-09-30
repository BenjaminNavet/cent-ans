class_name RoadRenderer
extends Node3D

## Routes de la carte (lot C6), purement visuelles :
## - palier moyen : routes principales (`type` = `main`) en traits deux tons (liseré sombre, cœur
##   clair) dont la largeur à l'écran suit la distance (`road_line.gdshader`, lot C7b), routes
##   secondaires et calculées en traits fins et pâles qui s'effacent avant le haut du palier ;
##   le tout en fondu selon le poids du palier ;
## - palier près : toutes les routes en rubans de chemin de terre (`road.gdshader`) drapés sur la
##   surface exacte du terrain affiché (`TerrainBuilder.surface_height_at`), un maillage par
##   tuile de terrain au niveau proche ou fin, reconstruit quand la tuile change de niveau.

## Palier moyen (lot C7b) : cœur et liseré des routes principales, largeur écran selon la distance.
## SS2 (ADR 0141) : terre battue claire et liseré discret (plus de trait crème cartographique) ;
## avec la carte de couleur, les routes y sont peintes de loin : `main_far_alpha_colormap`.
@export var main_fill: Color = Color(0.66, 0.56, 0.41, 1.0)
@export var main_casing: Color = Color(0.36, 0.28, 0.18, 0.45)
@export var main_width: float = 0.5
@export var main_near_px: float = 3.4
@export var main_far_px: float = 1.6
@export var main_far_alpha_colormap: float = 0.25
## Routes secondaires et calculées au palier moyen : discrètes, effacées au-delà de `minor_fade_distance`.
@export var minor_color: Color = Color(0.36, 0.25, 0.13, 0.55)
@export var minor_near_px: float = 1.5
@export var minor_far_px: float = 0.9
@export var minor_fade_distance: float = 420.0
@export var ribbon_main_width: float = 0.55
@export var ribbon_width: float = 0.38
## Soulèvement au-dessus de la surface (évite le z-fighting).
@export var lift: float = 0.06
## Pas d'échantillonnage des rubans le long de la route (unités monde).
@export var sample_step: float = 0.5
@export var max_ribbon_builds_per_frame: int = 3
## SZ6 : avec le relief quadtree, rubans construits dans des fils (`WorkerThreadPool`) sur un
## instantané des pages (une tuile coûtait jusqu'à 115 ms au fil principal) : au plus N
## constructions en cours, installées sur le fil principal dans le budget de l'image.
@export var max_ribbon_jobs: int = 4

const ROAD_LINE_SHADER := preload("res://shaders/road_line.gdshader")

var map_data: MapData
var terrain: TerrainBuilder
var stats: Dictionary = {"roads": 0, "main_roads": 0, "runs": 0, "ribbon_chunks": 0}

var _main_lines: MeshInstance3D
var _main_material: ShaderMaterial
var _minor_lines: MeshInstance3D
var _minor_material: ShaderMaterial
var _ribbon_material: ShaderMaterial
## index de tuile → Array de {points: PackedVector2Array, width: float}
var _runs_by_chunk: Dictionary = {}
## index de tuile → MeshInstance3D
var _ribbons: Dictionary = {}
var _dirty: Dictionary = {}
## SZ6 : emprise (carte) des tronçons de chaque tuile, élargie de la demi-largeur des rubans :
## rectangle de l'instantané des pages lu par le fil de travail.
var _run_bounds: Dictionary = {}
## index de tuile → {"task": id `WorkerThreadPool`, "job": RibbonJob}
var _jobs: Dictionary = {}
var _near_alpha := -1.0
var _medium_alpha := -1.0


func build(data: MapData, settlement_data: SettlementData, terrain_builder: TerrainBuilder) -> void:
	for child in get_children():
		child.queue_free()
	_wait_jobs(false)
	_ribbons.clear()
	_runs_by_chunk.clear()
	_run_bounds.clear()
	_dirty.clear()
	map_data = data
	terrain = terrain_builder
	var main_lines := []
	var main_widths := []
	var minor_lines := []
	var minor_widths := []
	for road in settlement_data.roads:
		if road["main"]:
			main_lines.append(road["points"])
			main_widths.append(main_width)
		else:
			minor_lines.append(road["points"])
			minor_widths.append(0.0)
		_split_by_chunk(road["points"], ribbon_main_width if road["main"] else ribbon_width)
	stats["roads"] = settlement_data.roads.size()
	stats["main_roads"] = main_lines.size()
	# Ordre (sans test de profondeur matériel) : routes secondaires (0) < principales et côte (1) < fleuves (2) ;
	# icônes et étiquettes au-dessus (`SettlementLayer`).
	_minor_material = _line_material(minor_color, minor_color, minor_near_px, minor_far_px, 0.0, 0)
	_minor_material.set_shader_parameter("far_distance", minor_fade_distance)
	_minor_material.set_shader_parameter("far_alpha", 0.0)
	_minor_lines = _lines_instance("MinorRoads", minor_lines, minor_widths, _minor_material)
	_main_material = _line_material(main_fill, main_casing, main_near_px, main_far_px, 0.8, 1)
	_main_lines = _lines_instance("MainRoads", main_lines, main_widths, _main_material)
	if data != null and ReliefLandcover.load_colormap_meta(data.map_dir):
		_main_material.set_shader_parameter("far_alpha", main_far_alpha_colormap)
	_ribbon_material = ShaderMaterial.new()
	_ribbon_material.shader = preload("res://shaders/road.gdshader")
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


static func _line_material(fill: Color, casing: Color, near_px: float, far_px: float, casing_px: float, priority: int) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = ROAD_LINE_SHADER
	material.set_shader_parameter("fill_color", fill)
	material.set_shader_parameter("casing_color", casing)
	material.set_shader_parameter("near_px", near_px)
	material.set_shader_parameter("far_px", far_px)
	material.set_shader_parameter("casing_px", casing_px)
	material.render_priority = priority
	return material


func _lines_instance(node_name: String, lines: Array, widths: Array, material: ShaderMaterial) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = PolylineMesh.build_screen_lines(lines, widths, map_data, 0.1)
	instance.material_override = material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)
	return instance


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
	var bounds := Rect2(run[0], Vector2.ZERO)
	for p in run:
		bounds = bounds.expand(p)
	bounds = bounds.grow(width * 0.5 + 0.01)
	_run_bounds[index] = (_run_bounds[index] as Rect2).merge(bounds) if _run_bounds.has(index) else bounds
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
		_main_material.set_shader_parameter("alpha", medium)
		_minor_material.set_shader_parameter("alpha", medium)
		_main_lines.visible = medium > 0.01
		_minor_lines.visible = medium > 0.01
	if not is_equal_approx(near, _near_alpha):
		_near_alpha = near
		_ribbon_material.set_shader_parameter("alpha", near)
	var show_ribbons := near > 0.01
	var threaded := _threaded()
	var builds := 0
	if not _jobs.is_empty():
		_install_jobs(show_ribbons, false)
	for index in _runs_by_chunk:
		var level := terrain.chunk_level(index)
		var wanted := show_ribbons and level >= 1
		var ribbon: MeshInstance3D = _ribbons.get(index)
		if wanted:
			if ribbon == null or _dirty.has(index):
				if threaded:
					if not _jobs.has(index) and _jobs.size() < max_ribbon_jobs:
						_start_job(index)
					continue
				if builds >= max_ribbon_builds_per_frame or (builds > 0 and not FrameBudget.has_time()):
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
	FrameBudget.unlimited = true
	_wait_jobs(true)
	_sync = true
	_near_alpha = -1.0
	update_view(maxf(_medium_alpha, 0.0), near)
	_sync = false
	FrameBudget.unlimited = false
	max_ribbon_builds_per_frame = saved


func ribbon_count() -> int:
	return _ribbons.size()


## Constructions de rubans en cours dans des fils (tests, mesures).
func pending_jobs() -> int:
	return _jobs.size()


func _exit_tree() -> void:
	_wait_jobs(false)


# --- Construction dans des fils (SZ6) ----------------------------------------------------

## Vrai pendant `flush` : tout se construit sur le fil principal.
var _sync := false


## Les fils de travail lisent un instantané des pages du quadtree ; sans quadtree (repli E0),
## les grilles des tuiles changent de niveau : construction sur le fil principal comme avant.
func _threaded() -> bool:
	return not _sync and max_ribbon_jobs > 0 and terrain != null and terrain.quadtree != null


func _start_job(index: int) -> void:
	_dirty.erase(index)
	var job := RibbonJob.new()
	job.runs = _runs_by_chunk[index]
	job.sample_step = sample_step
	job.lift = lift
	job.snapshot = terrain.quadtree.surface_snapshot(_run_bounds[index], Vector2.ZERO)
	_jobs[index] = {"task": WorkerThreadPool.add_task(job.run, false, "road ribbon %d" % index), "job": job}


## Installe les rubans terminés (au moins un par image, puis dans le budget de l'image ; tous
## si `block`). Un ruban d'une tuile repassée au niveau lointain est abandonné.
func _install_jobs(show_ribbons: bool, block: bool) -> void:
	var installed := 0
	for index: int in _jobs.keys():
		var entry: Dictionary = _jobs[index]
		if not block:
			if not WorkerThreadPool.is_task_completed(entry["task"]):
				continue
			if installed > 0 and not FrameBudget.has_time():
				break
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(index)
		if not block and terrain.chunk_level(index) == 0:
			continue
		_install_ribbon(index, (entry["job"] as RibbonJob).arrays, show_ribbons or block)
		installed += 1


## Attend les constructions en cours ; les installe si `install` (sinon les abandonne).
func _wait_jobs(install: bool) -> void:
	if install:
		_install_jobs(true, true)
		return
	for index: int in _jobs:
		WorkerThreadPool.wait_for_task_completion(_jobs[index]["task"])
	_jobs.clear()


## Rubans d'une tuile construits hors du fil principal.
class RibbonJob:
	extends RefCounted

	var runs: Array = []
	var sample_step: float = 0.5
	var lift: float = 0.06
	var snapshot: Dictionary = {}
	var arrays: Array = []

	func run() -> void:
		arrays = RoadRenderer.ribbon_arrays(runs, sample_step, lift, snapshot, null)


func _build_ribbon(index: int) -> void:
	_dirty.erase(index)
	_install_ribbon(index, ribbon_arrays(_runs_by_chunk[index], sample_step, lift, {}, terrain), true)


func _install_ribbon(index: int, arrays: Array, show: bool) -> void:
	var ribbon: MeshInstance3D = _ribbons.get(index)
	if ribbon == null:
		ribbon = MeshInstance3D.new()
		ribbon.name = "Ribbon_%d" % index
		ribbon.material_override = _ribbon_material
		ribbon.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(ribbon)
		_ribbons[index] = ribbon
	ribbon.visible = show
	if arrays.is_empty():
		ribbon.mesh = null
		return
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	ribbon.mesh = mesh


## Tableaux du maillage des rubans de `runs` (vide s'il n'y a aucun sommet). Hauteurs : surface
## affichée du terrain (`terrain.surface_heights_at`), ou, sans `terrain`, l'instantané des pages
## `snapshot` (origine (0, 0)), même résultat : fil de travail.
static func ribbon_arrays(runs: Array, sample_step: float, lift: float, snapshot: Dictionary, terrain: TerrainBuilder) -> Array:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	for run: Dictionary in runs:
		var dense := _densify(run["points"], sample_step)
		var half: float = run["width"] * 0.5
		var base := vertices.size()
		var count := dense.size()
		# Bords gauche et droit alternés, hauteurs en un seul lot (PB1).
		var edges := PackedVector2Array()
		edges.resize(count * 2)
		for i in count:
			var p := dense[i]
			var prev := dense[maxi(i - 1, 0)]
			var next := dense[mini(i + 1, count - 1)]
			var dir := (next - prev).normalized()
			if dir == Vector2.ZERO:
				dir = Vector2.RIGHT
			var perp := Vector2(-dir.y, dir.x) * half
			edges[i * 2] = p + perp
			edges[i * 2 + 1] = p - perp
		var edge_heights := terrain.surface_heights_at(edges) if terrain != null else snapshot_heights(snapshot, edges)
		vertices.resize(base + count * 2)
		normals.resize(base + count * 2)
		uvs.resize(base + count * 2)
		var along := 0.0
		for i in count:
			if i > 0:
				along += dense[i].distance_to(dense[i - 1])
			var left := edges[i * 2]
			var right := edges[i * 2 + 1]
			vertices[base + i * 2] = Vector3(left.x, edge_heights[i * 2] + lift, left.y)
			vertices[base + i * 2 + 1] = Vector3(right.x, edge_heights[i * 2 + 1] + lift, right.y)
			normals[base + i * 2] = Vector3.UP
			normals[base + i * 2 + 1] = Vector3.UP
			uvs[base + i * 2] = Vector2(along, 1.0)
			uvs[base + i * 2 + 1] = Vector2(along, -1.0)
		var first := indices.size()
		indices.resize(first + (count - 1) * 6)
		for i in count - 1:
			var a := base + i * 2
			var k := first + i * 6
			indices[k] = a
			indices[k + 1] = a + 1
			indices[k + 2] = a + 3
			indices[k + 3] = a
			indices[k + 4] = a + 3
			indices[k + 5] = a + 2
	if vertices.is_empty():
		return []
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	return arrays


## Hauteurs de la surface affichée en une série de points dans un instantané des pages
## (`ReliefQuadtree.surface_snapshot`, origine (0, 0)) : même résultat que
## `TerrainBuilder.surface_heights_at` avec le quadtree (repli `MapData`, jamais sous la mer).
static func snapshot_heights(snapshot: Dictionary, points: PackedVector2Array) -> PackedFloat32Array:
	var result := PackedFloat32Array()
	result.resize(points.size())
	for n in points.size():
		result[n] = maxf(ReliefQuadtree.sample_snapshot(snapshot, points[n].x, points[n].y), 0.0)
	return result


## Points intermédiaires tous les `sample_step` (le ruban suit les facettes du terrain).
static func _densify(points: PackedVector2Array, sample_step: float) -> PackedVector2Array:
	var result := PackedVector2Array([points[0]])
	for i in range(1, points.size()):
		var a := points[i - 1]
		var b := points[i]
		var n := maxi(1, ceili(a.distance_to(b) / sample_step))
		for k in range(1, n + 1):
			result.append(a.lerp(b, float(k) / n))
	return result


## Lot ZG5b : disque (centre x, z ; rayon ; poids) où les routes drapées fines remplacent les
## rubans de chemin de terre (`FineGeoLayer`).
func set_fine_zone(zone: Vector4) -> void:
	if _ribbon_material != null:
		_ribbon_material.set_shader_parameter("fine_zone", zone)
