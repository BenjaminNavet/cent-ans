class_name ForestDetail
extends Node3D

## Lot SZ4b (suites SZ4, ADR 0036 / 0062) : forêts denses autour du point visé aux paliers vallée et
## site. Rendu seulement.
##
## Les arbres de la carte (`Vegetation`) sont semés pour des arbres de ~1 km ; à leur taille réelle
## (`MapPropScale.tree_scale`), le semis est clairsemé. Cette couche sème, par cellules de
## `cell_size` unités autour du point visé, des arbres au pas fin (`Vegetation.spacing ×
## full_scale`) avec les grilles grossières de la tuile (mêmes masques de forêt, d'essences, de
## bosquets), sans haies, par le pool natif (`VegetationScatter`, crate `vegetation`).
## - Part affichée : `ForestDetailProfile.fraction_for(s)` (couvert constant quand les arbres
##   rétrécissent), décroissante avec la distance au point visé, bornée par un budget d'instances
##   (le rayon se resserre).
## - Graines triées (rang normalisé) : `visible_instance_count` garde les premières, le paramètre
##   d'instance `instance_cut` du shader fait grandir celles qui apparaissent (pas de saut).
## - Une cellule n'est semée que pour la part voulue (`keep`, paliers) ; le flux aléatoire ne
##   dépend pas de `keep` : resemer plus dense ajoute des arbres sans déplacer les autres.
## - Recalage sur la surface affichée (pages du quadtree) quand une tuile change, groupé par tuile.

var profile: ForestDetailProfile
var vegetation: Vegetation
var terrain: TerrainBuilder
var stats: Dictionary = {"cells": 0, "instances": 0, "visible": 0, "fraction": 0.0, "radius": 0.0, "jobs": 0, "scatter_ms_max": 0.0, "regrounds": 0}

## clé de cellule → {node, parts: [{node, mmis, rect}], keep, counts, buffers, slots, tile, last_seen}
var _cells: Dictionary = {}
## clé → {"id", "keep"} ; id natif → clé
var _jobs: Dictionary = {}
var _job_ids: Dictionary = {}
## Recalages : id natif → {tile, keys: Array, sizes: Array (tampons par cellule)}
var _ground_ids: Dictionary = {}
var _ground_dirty: Dictionary = {}  # tuile → true
var _gain := 1.0
var _frame := 0
var _active := false


func setup(p_vegetation: Vegetation, p_terrain: TerrainBuilder) -> void:
	name = "ForestDetail"
	vegetation = p_vegetation
	profile = ForestDetailProfile.shared()
	terrain = p_terrain
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


func clear() -> void:
	for entry: Dictionary in _cells.values():
		(entry["node"] as Node).queue_free()
	_cells.clear()
	_jobs.clear()
	_job_ids.clear()
	_ground_ids.clear()
	_ground_dirty.clear()
	stats["cells"] = 0


## Vrai si l'identifiant natif `id` est une requête de cette couche.
func owns(id: int) -> bool:
	return _job_ids.has(id) or _ground_ids.has(id)


func pending() -> int:
	return _job_ids.size() + _ground_ids.size()


func instance_count() -> int:
	return int(stats["instances"])


func visible_count() -> int:
	return int(stats["visible"])


# --- Mise à jour par image ----------------------------------------------------------------


## `focus` : point visé (x, z) ; `camera_distance` : distance du rig ; `shadows` : ombres des
## arbres permises (zoom et qualité, décidé par `Vegetation`).
func update_view(focus: Vector2, camera_distance: float, shadows: bool) -> void:
	_frame += 1
	var usable := vegetation != null and terrain != null and terrain.quadtree != null and vegetation.has_native()
	var tree_scale := MapPropScale.shared().tree_scale(camera_distance)
	var fraction := profile.fraction_for(tree_scale) * vegetation.quality_density if usable else 0.0
	stats["fraction"] = fraction
	_active = fraction >= profile.min_fraction
	visible = _active
	if not _active:
		stats["visible"] = 0
		_evict()
		return
	var radius := clampf(profile.radius_factor * camera_distance * _gain, profile.radius_min, profile.radius_max)
	stats["radius"] = radius
	var size := profile.cell_size
	var wanted: Array = []
	var shown_keys := {}
	for cy in range(int(floor((focus.y - radius) / size)), int(floor((focus.y + radius) / size)) + 1):
		for cx in range(int(floor((focus.x - radius) / size)), int(floor((focus.x + radius) / size)) + 1):
			var rect := Rect2(cx * size, cy * size, size, size)
			var d := _rect_distance(rect, focus)
			if d > radius:
				continue
			var key := _key(cx, cy)
			var need := fraction * _falloff(d, radius)
			if need <= 0.0:
				continue
			shown_keys[key] = true
			var entry: Dictionary = _cells.get(key, {})
			var keep := float(entry.get("keep", 0.0))
			var job_keep := float((_jobs.get(key, {}) as Dictionary).get("keep", 0.0))
			if keep < minf(need, 1.0) - 1e-6 and job_keep < minf(need, 1.0) - 1e-6:
				wanted.append([d, key, rect, profile.keep_for(need)])
	var total_visible := 0
	for key in _cells:
		var entry: Dictionary = _cells[key]
		var node := entry["node"] as Node3D
		var show := shown_keys.has(key)
		node.visible = show
		if show:
			entry["last_seen"] = _frame
			total_visible += _apply_parts(entry, focus, fraction, radius, camera_distance, shadows)
	_start_jobs(wanted)
	_start_ground_jobs()
	# Budget : rayon resserré si trop d'arbres affichés, relâché lentement sinon.
	if total_visible > profile.instance_budget:
		_gain = maxf(_gain * 0.92, profile.min_gain)
	elif total_visible < profile.instance_budget * 0.75:
		_gain = minf(_gain * 1.03, 1.0)
	stats["visible"] = total_visible
	stats["gain"] = _gain
	stats["jobs"] = _jobs.size()
	_evict()


static func _key(cx: int, cy: int) -> int:
	return (cy + 4096) * 16384 + (cx + 4096)


static func _rect_distance(rect: Rect2, point: Vector2) -> float:
	var nearest := Vector2(clampf(point.x, rect.position.x, rect.end.x), clampf(point.y, rect.position.y, rect.end.y))
	return nearest.distance_to(point)


func _falloff(d: float, radius: float) -> float:
	return 1.0 - smoothstep(radius * profile.fade_from, radius, d)


## Parties d'une cellule : nombre d'instances visibles, croissance des dernières (shader), maillage
## et ombres selon la distance. Rend le nombre d'instances visibles.
func _apply_parts(entry: Dictionary, focus: Vector2, fraction: float, radius: float, camera_distance: float, shadows: bool) -> int:
	var keep: float = entry["keep"]
	var total := 0
	var detail_limit := profile.detail_factor * camera_distance
	for part: Dictionary in entry["parts"]:
		var d := _rect_distance(part["rect"], focus)
		var share := clampf(fraction * _falloff(d, radius) / keep, 0.0, 1.0)
		var detailed := d < detail_limit
		var part_node := part["node"] as Node3D
		part_node.visible = share > 0.0
		if share <= 0.0:
			continue
		if part.get("detailed", null) != detailed:
			part["detailed"] = detailed
			var meshes := _meshes(detailed)
			for kind in (part["mmis"] as Array).size():
				var mmi: MultiMeshInstance3D = part["mmis"][kind]
				if mmi != null:
					mmi.multimesh.mesh = meshes[kind]
		var cast := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows and detailed else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		# Bande de croissance proportionnelle à la part : les arbres qui apparaissent grandissent,
		# et la part moyenne affichée reste `share` (bande centrée sur le seuil).
		var band := clampf(share * 0.5, 0.02, 0.18)
		var cut := maxf(1.0 - share - band * 0.5, 0.0)
		var shown := minf(share + band * 0.5, 1.0)
		var cut_key := snappedf(share, 0.005)
		for mmi in part["mmis"]:
			if mmi == null:
				continue
			var instance := mmi as MultiMeshInstance3D
			var count := ceili(instance.multimesh.instance_count * shown)
			instance.multimesh.visible_instance_count = count
			total += count
			instance.cast_shadow = cast
			if part.get("cut", -1.0) != cut_key:
				instance.set_instance_shader_parameter("instance_cut", cut)
				instance.set_instance_shader_parameter("instance_band", band)
		part["cut"] = cut_key
	return total


func _meshes(detailed: bool) -> Array:
	return [VegetationMeshes.essence("oak", detailed), VegetationMeshes.essence("beech", detailed),
		VegetationMeshes.essence("fir", detailed), VegetationMeshes.hedge() if detailed else VegetationMeshes.hedge_low()]


# --- Semis -----------------------------------------------------------------------------------


func _start_jobs(wanted: Array) -> void:
	if wanted.is_empty():
		return
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for item in wanted:
		if _jobs.size() >= profile.max_jobs:
			return
		var key: int = item[1]
		if _jobs.has(key):
			continue
		var rect: Rect2 = item[2]
		var tile := terrain.chunk_index_at(rect.position.x + 0.5, rect.position.y + 0.5)
		var coarse := vegetation.tile_coarse(tile)
		if coarse.is_empty():
			continue  # tuile de base pas encore semée : grilles grossières indisponibles
		var params := coarse.duplicate()
		params["tile_index"] = 1_000_000 + key  # graine propre à la cellule, indépendante de `keep`
		params["spacing"] = vegetation.spacing * profile.full_scale
		params["tree_scale"] = vegetation.tree_scale
		params["vertical_scale"] = MapData.vertical_scale()
		params["relief_gain"] = MapData.relief_gain()
		params["exclusions"] = vegetation.exclusions_for(rect)
		params["ground_grid"] = terrain.quadtree.surface_snapshot(rect, rect.position)
		params["detail_rect"] = rect
		params["keep"] = float(item[3])
		params["parts_side"] = profile.parts_side
		var id := vegetation.native_submit(params)
		if id < 0:
			return
		_jobs[key] = {"id": id, "keep": float(item[3]), "rect": rect, "tile": tile}
		_job_ids[id] = key


## Résultat natif (appelé par `Vegetation._poll_native`).
func on_result(result: Dictionary) -> void:
	var id: int = result["id"]
	if _ground_ids.has(id):
		_apply_ground(_ground_ids[id], result)
		_ground_ids.erase(id)
		return
	var key: int = _job_ids.get(id, -1)
	_job_ids.erase(id)
	var job: Dictionary = _jobs.get(key, {})
	if job.is_empty() or int(job["id"]) != id:
		return
	_jobs.erase(key)
	stats["scatter_ms_max"] = maxf(float(stats["scatter_ms_max"]), float(result["ms"]))
	_install(key, job, result)


func _install(key: int, job: Dictionary, result: Dictionary) -> void:
	var old: Dictionary = _cells.get(key, {})
	if not old.is_empty():
		stats["instances"] = int(stats["instances"]) - _sum(old["counts"])
		(old["node"] as Node).queue_free()
	var rect: Rect2 = job["rect"]
	var side := profile.parts_side
	var counts: PackedInt32Array = result["counts"]
	var buffers: Array = result["buffers"]
	var node := Node3D.new()
	node.name = "Cell_%d" % key
	var meshes := _meshes(false)
	var parts: Array = []
	var slots: Array = []
	var part_size := rect.size / float(side)
	for part_index in side * side:
		var part_node := Node3D.new()
		var mmis: Array = []
		for kind in VegetationTileJob.KIND_COUNT:
			var slot := part_index * VegetationTileJob.KIND_COUNT + kind
			var count: int = counts[slot] if slot < counts.size() else 0
			if count == 0:
				mmis.append(null)
				slots.append(null)
				continue
			var multimesh := MultiMesh.new()
			multimesh.transform_format = MultiMesh.TRANSFORM_3D
			multimesh.use_custom_data = true
			multimesh.mesh = meshes[kind]
			multimesh.instance_count = count
			multimesh.buffer = buffers[slot]
			var mmi := MultiMeshInstance3D.new()
			mmi.multimesh = multimesh
			mmi.material_override = vegetation.foliage_material()
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			part_node.add_child(mmi)
			mmis.append(mmi)
			slots.append(mmi)
		node.add_child(part_node)
		var cell := Vector2(part_index % side, part_index / side)
		parts.append({"node": part_node, "mmis": mmis, "rect": Rect2(rect.position + cell * part_size, part_size)})
	add_child(node)
	_cells[key] = {"node": node, "parts": parts, "keep": job["keep"], "counts": counts, "buffers": buffers,
		"slots": slots, "tile": job["tile"], "rect": rect, "last_seen": _frame}
	stats["cells"] = _cells.size()
	stats["instances"] = int(stats["instances"]) + _sum(counts)


static func _sum(counts: PackedInt32Array) -> int:
	var total := 0
	for c in counts:
		total += c
	return total


## Libère les cellules invisibles les plus anciennes au-delà de `max_cells` (toutes quand la
## couche est éteinte depuis longtemps).
func _evict() -> void:
	if _cells.size() <= profile.max_cells and int(stats["instances"]) <= profile.max_stored_instances:
		return
	var idle: Array = []
	for key in _cells:
		var entry: Dictionary = _cells[key]
		if not (entry["node"] as Node3D).visible or not _active:
			idle.append([int(entry["last_seen"]), key])
	idle.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for item in idle:
		if _cells.size() <= profile.max_cells and int(stats["instances"]) <= profile.max_stored_instances:
			break
		var key: int = item[1]
		var entry: Dictionary = _cells[key]
		stats["instances"] = int(stats["instances"]) - _sum(entry["counts"])
		(entry["node"] as Node).queue_free()
		_cells.erase(key)
	stats["cells"] = _cells.size()


# --- Recalage sur la surface affichée ---------------------------------------------------------


func _on_chunk_surface_changed(index: int) -> void:
	for entry: Dictionary in _cells.values():
		if int(entry["tile"]) == index:
			_ground_dirty[index] = true
			return
	# Semis en cours sur une surface périmée : résultat recalé à l'arrivée.
	for job: Dictionary in _jobs.values():
		if int(job["tile"]) == index:
			_ground_dirty[index] = true
			return


## Un recalage par tuile (toutes ses cellules visibles dans une requête, instantané des pages sur
## leur enveloppe).
func _start_ground_jobs() -> void:
	if _ground_dirty.is_empty() or _ground_ids.size() >= profile.max_ground_jobs:
		return
	for tile in _ground_dirty.keys():
		if _ground_ids.size() >= profile.max_ground_jobs:
			return
		var keys: Array = []
		var sizes: Array = []
		var buffers: Array = []
		var bounds := Rect2()
		for key in _cells:
			var entry: Dictionary = _cells[key]
			if int(entry["tile"]) != tile or not (entry["node"] as Node3D).visible:
				continue
			keys.append(key)
			sizes.append((entry["buffers"] as Array).size())
			buffers.append_array(entry["buffers"])
			bounds = entry["rect"] if keys.size() == 1 else bounds.merge(entry["rect"])
		if keys.is_empty():
			if not _jobs.values().any(func(j: Dictionary) -> bool: return int(j["tile"]) == tile):
				_ground_dirty.erase(tile)
			continue
		_ground_dirty.erase(tile)
		var grid := terrain.quadtree.surface_snapshot(bounds, bounds.position)
		var id := vegetation.native_submit_reground(buffers, grid, bounds.position)
		if id < 0:
			return
		_ground_ids[id] = {"tile": tile, "keys": keys, "sizes": sizes, "generations": keys.map(func(k: int) -> int: return (_cells[k]["node"] as Node).get_instance_id())}


func _apply_ground(job: Dictionary, result: Dictionary) -> void:
	var buffers: Array = result["buffers"]
	var offset := 0
	for n in (job["keys"] as Array).size():
		var key: int = job["keys"][n]
		var size: int = job["sizes"][n]
		var entry: Dictionary = _cells.get(key, {})
		if not entry.is_empty() and (entry["node"] as Node).get_instance_id() == int(job["generations"][n]):
			var fresh := buffers.slice(offset, offset + size)
			var slots: Array = entry["slots"]
			for slot in slots.size():
				var mmi: MultiMeshInstance3D = slots[slot]
				if mmi != null:
					mmi.multimesh.buffer = fresh[slot]
			entry["buffers"] = fresh
		offset += size
	stats["regrounds"] = int(stats["regrounds"]) + 1


## Sème et recale tout de suite ce qui est voulu (captures, tests).
func flush(focus: Vector2, camera_distance: float) -> void:
	for _i in 64:
		update_view(focus, camera_distance, false)
		vegetation.poll_native_blocking()
		if _jobs.is_empty() and _ground_ids.is_empty() and _ground_dirty.is_empty():
			var before := _cells.size()
			update_view(focus, camera_distance, false)
			if _jobs.is_empty() and _cells.size() == before:
				return
