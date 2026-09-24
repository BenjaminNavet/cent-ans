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
@export var spacing: float = 1.35
@export var tree_scale: float = 1.0
## Au-delà de cette distance caméra → point visé, plus d'arbres.
@export var max_camera_distance: float = 700.0
## Portée de l'éclaircissement, en multiples de la distance caméra (bornes absolues en plus).
@export var fade_start_factor: float = 1.2
@export var fade_end_factor: float = 2.1
@export var fade_min_start: float = 110.0
@export var fade_min_end: float = 200.0
## Tuiles (point le plus proche) à moins de cette distance de la caméra : maillage détaillé ;
## au-delà, les arbres font moins de ≈ 10 pixels et la variante ≈ 20 triangles suffit.
@export var detail_distance: float = 170.0
## Densité globale selon la distance caméra : 1 jusqu'à `density_full_distance`, puis
## décroissance linéaire jusqu'à `density_min` à `max_camera_distance`.
@export var density_full_distance: float = 200.0
@export var density_min: float = 0.3
@export var max_concurrent_jobs: int = 5
## Tuiles semées d'un coup (et attendues) au premier affichage. `WorkerThreadPool` ne sert les
## tâches basse priorité que sur ~4 fils quel que soit le nombre de cœurs (mesuré) : demander
## plus que `max_concurrent_jobs` d'un coup ne fait qu'ajouter des salves d'attente bloquante en
## série sans plus de parallélisme réel. Les tuiles restantes arrivent ensuite normalement (même
## budget que le chargement en tâche de fond), en général en une poignée de frames.
@export var warm_start_tiles: int = 5
@export var max_cached_tiles: int = 64
@export var cast_shadows: bool = true
## Au-delà de cette distance caméra (zoom global, pas la distance d'une tuile), plus aucune
## ombre de végétation : à cette échelle les ombres portées des arbres/haies ne sont plus
## discernables individuellement mais restent payées en pleine géométrie d'ombre côté GPU (V6,
## perf ; zoom moyen d≈300-500).
@export var shadow_camera_distance: float = 300.0

var map_data: MapData
var mask: VegetationMask
## Lot C6 : cercles d'exclusion supplémentaires (colonies, hameaux) : Vector3(x, y, rayon) px carte.
var extra_exclusions: PackedVector3Array = PackedVector3Array()
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
var _warm := false


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
	# Sans colonies (C6), clairière autour de chaque capitale de province.
	for index in data.provinces if extra_exclusions.is_empty() else {}:
		var capital: Vector2 = data.provinces[index].get("capital_px", Vector2(-1, -1))
		if capital.x >= 0.0:
			_exclusions.append(Vector3(capital.x, capital.y, 11.0))
	_exclusions.append_array(extra_exclusions)


func clear() -> void:
	_wait_all_jobs()
	for entry in _tiles.values():
		(entry["node"] as Node).queue_free()
	_tiles.clear()
	_warm = false
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
	if parent == null or parent.get("load_ok") != true:
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
	var density := lerpf(1.0, density_min, clampf((camera_distance - density_full_distance) / maxf(max_camera_distance - density_full_distance, 1.0), 0.0, 1.0))
	_material.set_shader_parameter("density", density)
	var wanted: Array = []
	var camera_xz := Vector2(camera_position.x, camera_position.z)
	# Même métrique que le shader : distance horizontale + moitié de la hauteur de la caméra.
	var lift := absf(camera_position.y) * 0.5
	for cy in TerrainBuilder.CHUNKS:
		for cx in TerrainBuilder.CHUNKS:
			var index := cy * TerrainBuilder.CHUNKS + cx
			var d := _rect_distance(Rect2(cx * chunk_px, cy * chunk_px, chunk_px, chunk_px), camera_xz) + lift
			var in_range := d < fade_end
			if _tiles.has(index):
				var entry: Dictionary = _tiles[index]
				(entry["node"] as Node3D).visible = in_range
				if in_range:
					entry["last_seen"] = _frame
					for part: Dictionary in entry["parts"]:
						var part_d := _rect_distance(part["rect"], camera_xz) + lift
						(part["node"] as Node3D).visible = part_d < fade_end
						if part_d < fade_end:
							_apply_lod(part, part_d, fade_start, fade_end, density, camera_distance)
			elif in_range and not _jobs.has(index):
				wanted.append([d, index])
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	# Premier affichage : les tuiles les plus proches sont semées en parallèle et attendues
	# (pas d'apparition progressive au lancement ni dans les captures).
	var budget := max_concurrent_jobs if _warm else maxi(max_concurrent_jobs, warm_start_tiles)
	for item in wanted:
		if _jobs.size() >= budget:
			break
		_start_job(item[1])
	if not _warm and not wanted.is_empty():
		_warm = true
		var t0 := Time.get_ticks_msec()
		for item in _jobs.values():
			WorkerThreadPool.wait_for_task_completion(item["task"])
		var ready_jobs := _jobs.duplicate()
		_jobs.clear()
		for index in ready_jobs:
			_install_tile(index, ready_jobs[index]["job"])
		stats["warm_start_ms"] = Time.get_ticks_msec() - t0
		if _log_bursts:
			print("Vegetation (warm start): %s" % JSON.stringify(stats))
	_evict()


static func _rect_distance(rect: Rect2, point: Vector2) -> float:
	var nearest := Vector2(clampf(point.x, rect.position.x, rect.end.x), clampf(point.y, rect.position.y, rect.end.y))
	return nearest.distance_to(point)


## Maillage selon la distance et nombre d'instances visibles : les graines (triées) inférieures
## au seuil d'éclaircissement du point le plus proche de la tuile sont invisibles partout.
func _apply_lod(entry: Dictionary, d: float, fade_start: float, fade_end: float, density: float, camera_distance: float) -> void:
	var detailed := d < detail_distance
	var mmis: Array = entry["mmis"]
	if entry.get("detailed", null) != detailed:
		entry["detailed"] = detailed
		var meshes := _meshes(detailed)
		for kind in mmis.size():
			var mmi: MultiMeshInstance3D = mmis[kind]
			if mmi != null:
				mmi.multimesh.mesh = meshes[kind]
	# Ombres portées des tuiles proches seulement (au loin elles ne se voient plus) et seulement
	# au zoom global le plus rapproché (`shadow_camera_distance`) : au zoom moyen, des ombres
	# d'arbres individuelles ne se distinguent déjà plus mais coûtent toujours plein tarif côté
	# GPU. Réévalué chaque image (pas seulement au changement de LOD) : ne dépend pas de `detailed`
	# seul, mais aussi du zoom global qui peut varier sans que `detailed` change.
	var shadow_on := cast_shadows and detailed and camera_distance < shadow_camera_distance
	var shadow_setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadow_on else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	for mmi in mmis:
		if mmi != null:
			(mmi as MultiMeshInstance3D).cast_shadow = shadow_setting
	var t := clampf((d - fade_start) / maxf(fade_end - fade_start, 1.0), 0.0, 1.0)
	var fraction := clampf(minf(1.0 - t, density) + 0.02, 0.0, 1.0)
	for mmi in entry["mmis"]:
		if mmi != null:
			var multimesh: MultiMesh = (mmi as MultiMeshInstance3D).multimesh
			multimesh.visible_instance_count = ceili(multimesh.instance_count * fraction)


func _meshes(detailed: bool) -> Array:
	if detailed:
		return [VegetationMeshes.deciduous(), VegetationMeshes.conifer(), VegetationMeshes.hedge()]
	return [VegetationMeshes.deciduous_low(), VegetationMeshes.conifer_low(), VegetationMeshes.hedge_low()]


func _start_job(index: int) -> void:
	var job := VegetationTileJob.new()
	job.mask = mask
	job.tile_index = index
	job.origin_px = Vector2i((index % TerrainBuilder.CHUNKS) * chunk_px, (index / TerrainBuilder.CHUNKS) * chunk_px)
	job.size_px = chunk_px
	job.spacing = spacing * float(chunk_px) / 256.0 if chunk_px < 256 else spacing
	job.tree_scale = tree_scale
	job.exclusions = _exclusions_for(Rect2(Vector2(job.origin_px), Vector2(chunk_px, chunk_px)))
	var task := WorkerThreadPool.add_task(job.run, false, "vegetation tile %d" % index)
	_jobs[index] = {"task": task, "job": job}


## Exclusions qui touchent une tuile (le semis teste chaque candidat contre toute la liste).
func _exclusions_for(rect: Rect2) -> PackedVector3Array:
	var result := PackedVector3Array()
	for e in _exclusions:
		if rect.grow(e.z).has_point(Vector2(e.x, e.y)):
			result.append(e)
	return result


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
	var parts: Array = []
	var meshes := _meshes(false)
	var part_px := float(chunk_px) / VegetationTileJob.PARTS_SIDE
	for part_index in VegetationTileJob.PARTS:
		var part_node := Node3D.new()
		part_node.name = "Part_%d" % part_index
		var mmis: Array = []
		for kind in VegetationTileJob.KIND_COUNT:
			var slot := part_index * VegetationTileJob.KIND_COUNT + kind
			var count: int = job.counts[slot]
			if count == 0:
				mmis.append(null)
				continue
			var multimesh := MultiMesh.new()
			multimesh.transform_format = MultiMesh.TRANSFORM_3D
			multimesh.use_custom_data = true
			multimesh.mesh = meshes[kind]
			multimesh.instance_count = count
			multimesh.buffer = job.buffers[slot]
			var mmi := MultiMeshInstance3D.new()
			mmi.name = ["Deciduous", "Conifer", "Hedge"][kind]
			mmi.multimesh = multimesh
			mmi.material_override = _material
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			part_node.add_child(mmi)
			mmis.append(mmi)
		node.add_child(part_node)
		var cell := Vector2(part_index % VegetationTileJob.PARTS_SIDE, part_index / VegetationTileJob.PARTS_SIDE)
		parts.append({"node": part_node, "mmis": mmis, "rect": Rect2(Vector2(job.origin_px) + cell * part_px, Vector2(part_px, part_px))})
	add_child(node)
	_tiles[index] = {"node": node, "parts": parts, "counts": job.counts, "last_seen": _frame}
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
