class_name ForestDetail
extends Node3D

## Lot SZ4b (suites SZ4, ADR 0036 / 0062) : forêts denses autour du point visé aux paliers vallée et
## site. Rendu seulement.
##
## Les arbres de la carte (`Vegetation`) sont semés pour des arbres de ~1 km ; à leur taille réelle
## (`MapPropScale.tree_scale()`, 1:1 à toute distance depuis VT3), le semis est clairsemé. Cette couche sème, par cellules de
## `cell_size` unités autour du point visé, des arbres au pas fin (`Vegetation.spacing ×
## full_scale`) avec les grilles grossières de la tuile (mêmes masques de forêt, d'essences, de
## bosquets), sans haies, par le pool natif (`VegetationScatter`, crate `vegetation`).
## - Part affichée : `ForestDetailProfile.fraction_at(d)` (VT3 : pleine de près, plus claire vers
##   la portée des arbres, nulle au-delà), décroissante avec la distance au point visé, bornée par un budget d'instances
##   (le rayon se resserre).
## - Graines triées (rang normalisé) : `visible_instance_count` garde les premières, le paramètre
##   d'instance `instance_cut` du shader fait grandir celles qui apparaissent (pas de saut).
## - Une cellule n'est semée que pour la part voulue (`keep`, paliers) ; le flux aléatoire ne
##   dépend pas de `keep` : resemer plus dense ajoute des arbres sans déplacer les autres.
## - Recalage sur la surface affichée (pages du quadtree) quand une tuile change, groupé par tuile.

var profile: ForestDetailProfile
## Lot FC5 : parties à moins de `near_factor` × distance caméra du point visé en cartes de
## feuillage (les autres en imposteurs) ; borne le coût des arbres proches.
var near_factor: float = 0.4
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
var _ground_queue: Dictionary = {}  # clé de cellule → true
var _gain := 1.0
var _part_updates_left := 0
var _flushing := false
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
	_ground_queue.clear()
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
	var t_start := Time.get_ticks_usec()
	_update_view(focus, camera_distance, shadows)
	stats["update_ms_max"] = maxf(float(stats.get("update_ms_max", 0.0)), (Time.get_ticks_usec() - t_start) / 1000.0)


func _update_view(focus: Vector2, camera_distance: float, shadows: bool) -> void:
	_frame += 1
	var usable := vegetation != null and terrain != null and terrain.quadtree != null and vegetation.has_native()
	var fraction := profile.fraction_at(camera_distance) * vegetation.quality_density if usable else 0.0
	stats["fraction"] = fraction
	_active = fraction >= profile.min_fraction
	visible = _active
	if not _active:
		stats["visible"] = 0
		_evict()
		return
	# VT3 : pas au-delà de la portée des arbres ; le gain du budget s'applique après les bornes.
	var reach := minf(profile.radius_factor * camera_distance, minf(profile.radius_max, MapPropScale.shared().tree_view_range))
	var radius := maxf(reach * _gain, minf(profile.radius_min, reach))
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
	_part_updates_left = (1 << 20) if _flushing else profile.max_part_updates
	var t_parts := Time.get_ticks_usec()
	for key in _cells:
		var entry: Dictionary = _cells[key]
		var node := entry["node"] as Node3D
		var show := shown_keys.has(key)
		node.visible = show
		if show:
			entry["last_seen"] = _frame
			if entry.get("stale_ground", false):
				entry.erase("stale_ground")
				_ground_queue[key] = true
			total_visible += _apply_parts(entry, focus, fraction, radius, camera_distance, shadows)
	var t_jobs := Time.get_ticks_usec()
	stats["parts_ms_max"] = maxf(float(stats.get("parts_ms_max", 0.0)), (t_jobs - t_parts) / 1000.0)
	_start_jobs(wanted)
	var t_ground := Time.get_ticks_usec()
	_start_ground_jobs()
	var t_evict := Time.get_ticks_usec()
	stats["jobs_ms_max"] = maxf(float(stats.get("jobs_ms_max", 0.0)), (t_ground - t_jobs) / 1000.0)
	stats["ground_ms_max"] = maxf(float(stats.get("ground_ms_max", 0.0)), (t_evict - t_ground) / 1000.0)
	# Budget : rayon resserré si trop d'arbres affichés, relâché lentement sinon.
	if total_visible > profile.instance_budget:
		_gain = maxf(_gain * 0.92, profile.min_gain)
	elif total_visible < profile.instance_budget * 0.75:
		_gain = minf(_gain * 1.03, 1.0)
	stats["visible"] = total_visible
	stats["gain"] = _gain
	stats["jobs"] = _jobs.size()
	var t_ev := Time.get_ticks_usec()
	_evict()
	stats["evict_ms_max"] = maxf(float(stats.get("evict_ms_max", 0.0)), (Time.get_ticks_usec() - t_ev) / 1000.0)


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
	var near_limit := near_factor * camera_distance if vegetation.near_cards_active() else -1.0
	for part: Dictionary in entry["parts"]:
		var d := _rect_distance(part["rect"], focus)
		var share := snappedf(clampf(fraction * _falloff(d, radius) / keep, 0.0, 1.0), 0.004)
		var cast := shadows and d < detail_limit
		var near := d < near_limit
		var state := Vector3(share, 1.0 if near else 0.0, 1.0 if cast else 0.0)
		# PB : rien à écrire si l'état n'a pas changé ; sinon au plus `max_part_updates` parties
		# réécrites par image (les autres gardent leur état, réessayées à l'image suivante).
		if part.get("state", Vector3(-1, -1, -1)) == state or _part_updates_left <= 0:
			total += int(part.get("count", 0))
			continue
		_part_updates_left -= 1
		part["state"] = state
		var part_node := part["node"] as Node3D
		part_node.visible = share > 0.0
		if share <= 0.0:
			part["count"] = 0
			continue
		# Maillage bas (≈ 20 triangles) partout : changer le maillage d'un MultiMesh coûte ~1 ms
		# de fil principal (mesuré) et les arbres à taille réelle ne font que quelques pixels ;
		# la proximité ne règle que les ombres.
		var cast_setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cast else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		# Bande de croissance proportionnelle à la part : les arbres qui apparaissent grandissent,
		# et la part moyenne affichée reste `share` (bande centrée sur le seuil).
		var band := clampf(share * 0.5, 0.02, 0.18)
		var cut := maxf(1.0 - share - band * 0.5, 0.0)
		var shown := minf(share + band * 0.5, 1.0)
		var count_part := 0
		if part.get("near", false) != near:
			part["near"] = near
			_swap_meshes(entry, part, near)
		for mmi in part["mmis"]:
			if mmi == null:
				continue
			var instance := mmi as MultiMeshInstance3D
			var count := ceili(instance.multimesh.instance_count * shown)
			instance.multimesh.visible_instance_count = count
			count_part += count
			instance.cast_shadow = cast_setting
			instance.set_instance_shader_parameter("instance_cut", cut)
			instance.set_instance_shader_parameter("instance_band", band)
		part["count"] = count_part
		total += count_part
	return total


## Lot FC5 : maillages d'une partie (cartes de près, imposteurs sinon) ; MultiMesh recréé depuis
## la copie processeur du tampon (comme `Vegetation._with_mesh`), boîte fixe gardée.
func _swap_meshes(entry: Dictionary, part: Dictionary, near: bool) -> void:
	var meshes := _meshes(near)
	var buffers: Array = entry["buffers"]
	var mmis: Array = part["mmis"]
	for kind in mmis.size():
		var mmi: MultiMeshInstance3D = mmis[kind]
		if mmi == null or mmi.multimesh.mesh == meshes[kind]:
			continue
		var old := mmi.multimesh
		var multimesh := MultiMesh.new()
		multimesh.transform_format = old.transform_format
		multimesh.use_custom_data = old.use_custom_data
		multimesh.mesh = meshes[kind]
		multimesh.instance_count = old.instance_count
		multimesh.buffer = buffers[int(part["slot0"]) + kind]
		multimesh.custom_aabb = old.custom_aabb
		multimesh.visible_instance_count = old.visible_instance_count
		mmi.multimesh = multimesh
		mmi.material_override = vegetation.forest_material(kind, near)


## Lot FC5 : cartes de feuillage au plus près, imposteurs ailleurs (`Vegetation.forest_meshes`).
func _meshes(near: bool) -> Array:
	return vegetation.forest_meshes(near)


# --- Semis -----------------------------------------------------------------------------------


func _start_jobs(wanted: Array) -> void:
	if wanted.is_empty():
		return
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var submitted := 0
	for item in wanted:
		# Une requête par image hors captures (~2-3 ms de fil principal : grilles, pages, couloirs).
		if _jobs.size() >= profile.max_jobs or (submitted >= 1 and not _flushing):
			return
		var key: int = item[1]
		if _jobs.has(key):
			continue
		var t0 := Time.get_ticks_usec()
		var rect: Rect2 = item[2]
		var tile := terrain.chunk_index_at(rect.position.x + 0.5, rect.position.y + 0.5)
		var coarse := vegetation.tile_coarse(tile)
		if coarse.is_empty():
			continue  # tuile de base pas encore semée : grilles grossières indisponibles
		if not _flushing and not _corridor_tiles_ready(rect):
			continue  # tuiles CAFV demandées en tâche de fond : cellule semée à leur arrivée
		var params := coarse.duplicate()
		params["tile_index"] = 1_000_000 + key  # graine propre à la cellule, indépendante de `keep`
		params["spacing"] = vegetation.spacing * profile.full_scale
		params["tree_scale"] = vegetation.tree_scale
		params["vertical_scale"] = MapData.vertical_scale()
		params["relief_gain"] = MapData.relief_gain()
		params["relief_squash"] = MapData.relief_squash()  # SZ1 : écrasement des montagnes
		params["exclusions"] = vegetation.exclusions_for(rect)
		params["ground_grid"] = terrain.quadtree.surface_snapshot(rect, rect.position)
		params["corridors"] = _corridors(rect)
		params["detail_rect"] = rect
		params["keep"] = float(item[3])
		params["parts_side"] = profile.parts_side
		var id := vegetation.native_submit(params)
		if id < 0:
			return
		submitted += 1
		_jobs[key] = {"id": id, "keep": float(item[3]), "rect": rect, "tile": tile}
		_job_ids[id] = key
		# Fil principal : grilles, instantané des pages, couloirs, conversion Rust.
		stats["request_ms_max"] = maxf(float(stats.get("request_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)


## Couloirs sans arbres d'une cellule : fleuves fins (rang affiché, lot ZG5b) et routes drapées,
## segments `x0, y0, x1, y1, demi-largeur` (unités monde) ; la trame du lit des fleuves 4096 ne
## connaît pas ces tracés fins. Tuiles CAFV lues à la demande (cache LRU du magasin).
func _corridors(rect: Rect2) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	var store := _fine_store()
	if store == null:
		return out
	var fine: Object = _fine_layer()
	var min_order := int(fine.get("_min_order")) if fine != null else 3
	var mpu := vegetation.map_data.meters_per_px if vegetation.map_data != null else 719.0
	var bounds := rect.grow(0.5)
	var size := FineGeoStore.TILE_UNITS
	for layer in [CafvTile.LAYER_RIVERS, CafvTile.LAYER_ROADS]:
		var clearance: float = profile.river_clearance_m if layer == CafvTile.LAYER_RIVERS else profile.road_clearance_m
		for row in range(int(floor(bounds.position.y / size)), int(floor(bounds.end.y / size)) + 1):
			for col in range(int(floor(bounds.position.x / size)), int(floor(bounds.end.x / size)) + 1):
				var tile := store.load_sync(layer, col, row)
				if tile == null:
					continue
				for li in tile.lines():
					var lb := tile.line_bounds[li]
					if lb.z < bounds.position.x or lb.x > bounds.end.x or lb.w < bounds.position.y or lb.y > bounds.end.y:
						continue
					if layer == CafvTile.LAYER_RIVERS and tile.line_rank[li] < min_order:
						continue
					var first := tile.line_start[li]
					for k in range(first, first + tile.line_count[li] - 1):
						var half := (tile.w[k] * 0.5 + clearance) / mpu
						out.append_array([tile.x[k], tile.y[k], tile.x[k + 1], tile.y[k + 1], half])
	stats["corridor_segments_max"] = maxi(int(stats.get("corridor_segments_max", 0)), out.size() / 5)
	return out


## Vrai si les tuiles CAFV (fleuves, routes) couvrant `rect` sont en cache ; sinon les demande au
## magasin (lecture dans `WorkerThreadPool`, relevée par `FineGeoLayer`) : pas de lecture disque
## sur le fil principal.
func _corridor_tiles_ready(rect: Rect2) -> bool:
	var store := _fine_store()
	if store == null:
		return true
	var bounds := rect.grow(0.5)
	var size := FineGeoStore.TILE_UNITS
	var ready := true
	for layer in [CafvTile.LAYER_RIVERS, CafvTile.LAYER_ROADS]:
		for row in range(int(floor(bounds.position.y / size)), int(floor(bounds.end.y / size)) + 1):
			for col in range(int(floor(bounds.position.x / size)), int(floor(bounds.end.x / size)) + 1):
				if store.has_tile(layer, col, row) and not store.is_loaded(layer, col, row):
					store.request(layer, col, row)
					ready = false
	return ready


func _fine_layer() -> Object:
	var parent := vegetation.get_parent() if vegetation != null else null
	var rivers: Variant = parent.get("rivers") if parent != null else null
	return (rivers as Object).get("fine") if rivers is Object else null


func _fine_store() -> FineGeoStore:
	var fine: Object = _fine_layer()
	if fine == null or not bool(fine.get("enabled")):
		return null
	return fine.get("store") as FineGeoStore


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
			multimesh.visible_instance_count = 0  # part affichée posée par `_apply_parts`
			# Boîte fixe : sans elle, chaque changement de part affichée recalcule la boîte sur
			# toutes les instances (fil principal, ~1 ms par MultiMesh de 30 000 arbres).
			var part_rect := Rect2(rect.position + Vector2(part_index % side, part_index / side) * (rect.size / float(side)), rect.size / float(side)).grow(1.0)
			multimesh.custom_aabb = AABB(Vector3(part_rect.position.x, -5.0, part_rect.position.y), Vector3(part_rect.size.x, 60.0, part_rect.size.y))
			var mmi := MultiMeshInstance3D.new()
			mmi.multimesh = multimesh
			mmi.material_override = vegetation.forest_material(kind, false)
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			part_node.add_child(mmi)
			mmis.append(mmi)
			slots.append(mmi)
		node.add_child(part_node)
		var cell := Vector2(part_index % side, part_index / side)
		parts.append({"node": part_node, "mmis": mmis, "rect": Rect2(rect.position + cell * part_size, part_size), "slot0": part_index * VegetationTileJob.KIND_COUNT})
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


## Recalage cellule par cellule (instantané des pages sur la cellule seulement : les copies
## des pages et des tampons restent petites sur le fil principal), au plus `max_ground_jobs` en vol.
func _start_ground_jobs() -> void:
	for tile in _ground_dirty.keys():
		var pending_scatter := _jobs.values().any(func(j: Dictionary) -> bool: return int(j["tile"]) == tile)
		for key in _cells:
			var entry: Dictionary = _cells[key]
			if int(entry["tile"]) == tile:
				_ground_queue[key] = true
		if not pending_scatter:
			_ground_dirty.erase(tile)
	if _ground_queue.is_empty():
		return
	for key in _ground_queue.keys():
		if _ground_ids.size() >= profile.max_ground_jobs:
			return
		_ground_queue.erase(key)
		var entry: Dictionary = _cells.get(key, {})
		if entry.is_empty():
			continue
		if not (entry["node"] as Node3D).visible:
			entry["stale_ground"] = true  # recalée quand elle redevient visible
			continue
		var t0 := Time.get_ticks_usec()
		var rect: Rect2 = entry["rect"]
		var grid := terrain.quadtree.surface_snapshot(rect, rect.position)
		var id := vegetation.native_submit_reground(entry["buffers"], grid, rect.position)
		stats["ground_request_ms_max"] = maxf(float(stats.get("ground_request_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)
		if id < 0:
			return
		_ground_ids[id] = {"keys": [key], "sizes": [(entry["buffers"] as Array).size()], "generations": [(entry["node"] as Node).get_instance_id()]}


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
	_flushing = true
	_flush(focus, camera_distance)
	_flushing = false


func _flush(focus: Vector2, camera_distance: float) -> void:
	for _i in 64:
		update_view(focus, camera_distance, false)
		vegetation.poll_native_blocking()
		if _jobs.is_empty() and _ground_ids.is_empty() and _ground_dirty.is_empty() and _ground_queue.is_empty():
			var before := _cells.size()
			update_view(focus, camera_distance, false)
			if _jobs.is_empty() and _cells.size() == before:
				return


## `--forest-stats` : statistiques imprimées en quittant (bancs, `map_bench.gd`).
func _exit_tree() -> void:
	if CmdArgs.has("--forest-stats"):
		print("ForestDetail: %s" % JSON.stringify(stats))
