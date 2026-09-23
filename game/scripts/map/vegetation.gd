class_name Vegetation
extends Node3D

## Végétation de la carte de campagne (lot V3, ADR 0004) : forêts, bosquets et haies en
## `MultiMeshInstance3D`, par tuile (même découpage 16 × 16 que `TerrainBuilder`).
##
## - Construction paresseuse : seules les tuiles proches de la caméra sont semées, dans des
##   tâches `WorkerThreadPool` (`VegetationTileJob`) ; les tuiles construites restent en cache
##   (au plus `max_cached_tiles`, les plus anciennes hors champ sont libérées).
## - Distance : deux maillages par essence (détaillé de près, ≈ 20 triangles au loin),
##   éclaircissement progressif (`foliage.gdshader`) et, au-delà de `max_camera_distance`, plus
##   aucun arbre (le terrain texturé suffit au dézoom).
## - Autonome : se branche seul sur la scène parente (`map_data`, `load_ok`) et sur le rig de
##   caméra ; `build(map_data)` peut aussi être appelé directement.
## Purement visuel : aucune règle de jeu.

const FOLIAGE_SHADER := preload("res://shaders/foliage.gdshader")

@export var camera_rig_path: NodePath = ^"../CameraRig"
## Pas (px de carte) de la grille de candidats ; plus petit = forêts plus denses.
@export var spacing: float = 1.25
@export var tree_scale: float = 1.0
## Au-delà de cette distance caméra → point visé, plus d'arbres.
@export var max_camera_distance: float = 700.0
## Portée de l'éclaircissement, en multiples de la distance caméra (bornes absolues en plus).
@export var fade_start_factor: float = 1.5
@export var fade_end_factor: float = 2.6
@export var fade_min_start: float = 110.0
@export var fade_min_end: float = 200.0
## Tuiles à moins de `detail_factor` × distance caméra : maillage détaillé.
@export var detail_factor: float = 1.3
@export var max_concurrent_jobs: int = 3
@export var max_cached_tiles: int = 64
@export var cast_shadows: bool = true

var map_data: MapData
var mask: VegetationMask
var chunk_px: int = 0
var enabled: bool = true
## Statistiques : tuiles construites, instances, temps de semis (ms, somme et max).
var stats: Dictionary = {"tiles": 0, "instances": 0, "build_ms_total": 0.0, "build_ms_max": 0.0, "source": ""}

var _material: ShaderMaterial
var _tiles: Dictionary = {}  # index → {"node": Node3D, "mmis": Array[MultiMeshInstance3D], "counts", "last_seen"}
var _jobs: Dictionary = {}  # index → {"task": int, "job": VegetationTileJob}
var _exclusions := PackedVector3Array()
var _rig: Node3D
var _frame: int = 0
var _log_bursts := false


func _ready() -> void:
	_rig = get_node_or_null(camera_rig_path) as Node3D
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot") or arg == "--vegetation-stats":
			_log_bursts = true


func _exit_tree() -> void:
	_wait_all_jobs()


func build(data: MapData) -> void:
	clear()
	map_data = data
	chunk_px = TerrainBuilder.chunk_px_for(data.size)
	mask = VegetationMask.new()
	mask.setup(data)
	stats["source"] = mask.source
	_material = ShaderMaterial.new()
	_material.shader = FOLIAGE_SHADER
	_exclusions.clear()
	for index in data.provinces:
		var capital: Vector2 = data.provinces[index].get("capital_px", Vector2(-1, -1))
		if capital.x >= 0.0:
			_exclusions.append(Vector3(capital.x, capital.y, 11.0))


func clear() -> void:
	_wait_all_jobs()
	for entry in _tiles.values():
		(entry["node"] as Node).queue_free()
	_tiles.clear()
	stats["tiles"] = 0
	stats["instances"] = 0


func tile_count() -> int:
	return _tiles.size()


func instance_count() -> int:
	return int(stats["instances"])


func pending_jobs() -> int:
	return _jobs.size()


func _process(_delta: float) -> void:
	if map_data == null:
		_try_autobind()
		if map_data == null:
			return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	update_view(camera.global_position, distance)


func _try_autobind() -> void:
	var parent := get_parent()
	if parent == null or not bool(parent.get("load_ok")):
		return
	var data: Variant = parent.get("map_data")
	if data is MapData:
		build(data)


## Met à jour tuiles, LOD et éclaircissement pour une caméra en `camera_position`, à
## `camera_distance` de son point visé.
func update_view(camera_position: Vector3, camera_distance: float) -> void:
	_frame += 1
	_collect_jobs()
	var active := enabled and camera_distance < max_camera_distance
	visible = active
	if not active or _material == null:
		return
	# Au voisinage de la distance maximale, la portée se referme : les arbres s'effacent.
	var closing := 1.0 - smoothstep(max_camera_distance * 0.7, max_camera_distance, camera_distance)
	var fade_start := maxf(camera_distance * fade_start_factor, fade_min_start) * closing
	var fade_end := maxf(camera_distance * fade_end_factor, fade_min_end) * closing
	_material.set_shader_parameter("view_origin", camera_position)
	_material.set_shader_parameter("fade_start", fade_start)
	_material.set_shader_parameter("fade_end", maxf(fade_end, fade_start + 1.0))
	var detail_distance := camera_distance * detail_factor
	var wanted: Array = []
	for cy in TerrainBuilder.CHUNKS:
		for cx in TerrainBuilder.CHUNKS:
			var index := cy * TerrainBuilder.CHUNKS + cx
			var rect := Rect2(cx * chunk_px, cy * chunk_px, chunk_px, chunk_px)
			var nearest := Vector2(clampf(camera_position.x, rect.position.x, rect.end.x), clampf(camera_position.z, rect.position.y, rect.end.y))
			var d := nearest.distance_to(Vector2(camera_position.x, camera_position.z))
			var in_range := d < fade_end
			if _tiles.has(index):
				var entry: Dictionary = _tiles[index]
				(entry["node"] as Node3D).visible = in_range
				if in_range:
					entry["last_seen"] = _frame
					_apply_lod(entry, d, detail_distance, fade_start, fade_end)
			elif in_range and not _jobs.has(index):
				wanted.append([d, index])
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for item in wanted:
		if _jobs.size() >= max_concurrent_jobs:
			break
		_start_job(item[1])
	_evict()


## Maillage selon la distance et nombre d'instances visibles : les graines (triées) inférieures
## au seuil d'éclaircissement du point le plus proche de la tuile sont invisibles partout.
func _apply_lod(entry: Dictionary, d: float, detail_distance: float, fade_start: float, fade_end: float) -> void:
	var detailed := d < detail_distance
	if entry.get("detailed", null) != detailed:
		entry["detailed"] = detailed
		var mmis: Array = entry["mmis"]
		var meshes := _meshes(detailed)
		for kind in mmis.size():
			var mmi: MultiMeshInstance3D = mmis[kind]
			if mmi != null:
				mmi.multimesh.mesh = meshes[kind]
				mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cast_shadows and detailed else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var t := clampf((d - fade_start) / maxf(fade_end - fade_start, 1.0), 0.0, 1.0)
	var fraction := clampf(1.0 - t + 0.02, 0.0, 1.0)
	for mmi in entry["mmis"]:
		if mmi != null:
			var multimesh: MultiMesh = (mmi as MultiMeshInstance3D).multimesh
			multimesh.visible_instance_count = ceili(multimesh.instance_count * fraction)


func _meshes(detailed: bool) -> Array:
	if detailed:
		return [VegetationMeshes.deciduous(), VegetationMeshes.conifer(), VegetationMeshes.hedge()]
	return [VegetationMeshes.deciduous_low(), VegetationMeshes.conifer_low(), VegetationMeshes.hedge()]


func _start_job(index: int) -> void:
	var job := VegetationTileJob.new()
	job.mask = mask
	job.tile_index = index
	job.origin_px = Vector2i((index % TerrainBuilder.CHUNKS) * chunk_px, (index / TerrainBuilder.CHUNKS) * chunk_px)
	job.size_px = chunk_px
	job.spacing = spacing * float(chunk_px) / 256.0 if chunk_px < 256 else spacing
	job.tree_scale = tree_scale
	job.exclusions = _exclusions
	var task := WorkerThreadPool.add_task(job.run, false, "vegetation tile %d" % index)
	_jobs[index] = {"task": task, "job": job}


func _collect_jobs() -> void:
	for index in _jobs.keys():
		var item: Dictionary = _jobs[index]
		if not WorkerThreadPool.is_task_completed(item["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(item["task"])
		_jobs.erase(index)
		_install_tile(index, item["job"])
		if _jobs.is_empty() and _log_bursts:
			print("Vegetation: %s" % JSON.stringify(stats))


func _wait_all_jobs() -> void:
	for item in _jobs.values():
		WorkerThreadPool.wait_for_task_completion(item["task"])
	_jobs.clear()


func _install_tile(index: int, job: VegetationTileJob) -> void:
	var node := Node3D.new()
	node.name = "Tile_%d" % index
	var mmis: Array = []
	var meshes := _meshes(false)
	for kind in VegetationTileJob.KIND_COUNT:
		var count: int = job.counts[kind]
		if count == 0:
			mmis.append(null)
			continue
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.use_custom_data = true
		multimesh.mesh = meshes[kind]
		multimesh.instance_count = count
		multimesh.buffer = job.buffers[kind]
		var mmi := MultiMeshInstance3D.new()
		mmi.name = ["Deciduous", "Conifer", "Hedge"][kind]
		mmi.multimesh = multimesh
		mmi.material_override = _material
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.add_child(mmi)
		mmis.append(mmi)
	add_child(node)
	_tiles[index] = {"node": node, "mmis": mmis, "counts": job.counts, "last_seen": _frame}
	stats["tiles"] = _tiles.size()
	stats["instances"] = int(stats["instances"]) + job.instance_total()
	stats["build_ms_total"] = float(stats["build_ms_total"]) + job.build_ms
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), job.build_ms)


## Libère les tuiles hors champ les plus anciennes au-delà de `max_cached_tiles`.
func _evict() -> void:
	if _tiles.size() <= max_cached_tiles:
		return
	var idle: Array = []
	for index in _tiles:
		var entry: Dictionary = _tiles[index]
		if not (entry["node"] as Node3D).visible:
			idle.append([int(entry["last_seen"]), index])
	idle.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var excess := _tiles.size() - max_cached_tiles
	for i in mini(excess, idle.size()):
		var index: int = idle[i][1]
		var entry: Dictionary = _tiles[index]
		var total := 0
		for count in entry["counts"]:
			total += count
		stats["instances"] = int(stats["instances"]) - total
		(entry["node"] as Node).queue_free()
		_tiles.erase(index)
	stats["tiles"] = _tiles.size()
