class_name TownLayer
extends Node3D

## Lot ZG6 (ADR 0036) : villes ordinaires à l'échelle réelle aux paliers « vallée » et « site »
## (caméra rapprochée de ZG4, pyramide de relief en cache). Rendu seulement.
## - Données : `data/map/towns_1340.json` (`TownData`, outil `cent-ans geo towns`).
## - Streaming : les villes à moins de `TownRenderProfile` × distance du rig de la caméra sont
##   planifiées dans des fils de travail (`TownPlan.generate` sur un instantané des pages du
##   quadtree), puis construites par petites étapes (`TownBuilder.step`, ≤ `build_budget_ms` et
##   `FrameBudget.has_time()`, ADR 0051) ; déchargées au-delà de `unload_factor` × ce rayon.
## - Hauteurs en mètres, posées par le shader (`campaign_vertical_scale`) : les émissions de
##   `chunk_surface_changed` dues à l'exagération (`rescaling_vertical`) sont ignorées ; seules les
##   pages plus fines déclenchent un recalcul des hauteurs (fil de travail, puis reconstruction
##   échangée d'un bloc quand elle est prête).
## - Qualité : `RenderQuality` (PF1) règle portées, rayon de chargement et ombres.
## `SettlementLayer` masque la maquette d'une colonie dont la ville 1:1 est affichée.

signal towns_changed

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var profile: TownRenderProfile
var data: TownData
var terrain: TerrainBuilder
var map_data: MapData
var tiers: ZoomTiers
## Vrai quand le rendu 1:1 est actif (pyramide présente et palier vallée atteint).
var active := false
## Force l'activité sans pyramide (tests headless).
var force_active := false
## Incrémenté à chaque ville affichée ou retirée (`SettlementLayer` recalcule alors les maquettes).
var version := 0
var stats: Dictionary = {}

var _ids: Array[String] = []
var _anchor: Dictionary = {}  # id → Vector2 (unités)
var _extent: Dictionary = {}  # id → rayon (unités)
var _ids_by_chunk: Dictionary = {}
## id → {plan, builder (construit), pending (TownBuilder en cours), dirty_ms, state}
var _entries: Dictionary = {}
var _jobs: Dictionary = {}  # id → [task_id, kind ("plan" | "reground")]
var _results: Dictionary = {}  # id → plan (écrit par les fils)
var _mutex := Mutex.new()
var _plan_cache: Dictionary = {}
var _cache_order: Array[String] = []
var _quality: Dictionary = {}
var _stream_timer := 0
var _last_distance := INF
var _plan_usec: Array[int] = []
var _step_max_usec := 0
## Captures et mesures : `--town-lod=blocks` (blocs seuls), `--town-lod=detail` (kit partout),
## `--no-towns` (rendu d'avant ZG6).
var _detail_override := 1.0
var _disabled := false


func setup(p_map: MapData, p_terrain: TerrainBuilder, p_tiers: ZoomTiers, settlement_ids: Array, p_data: TownData = null) -> void:
	name = "Towns"
	map_data = p_map
	terrain = p_terrain
	tiers = p_tiers if p_tiers != null else ZoomTiers.new()
	profile = TownRenderProfile.load_default()
	data = p_data if p_data != null else TownData.load_from(MAP_PATHS.default_data_dir().path_join("map"))
	_ids.clear()
	for id in settlement_ids:
		var sid := str(id)
		if data.has_town(sid) and LandmarkLibrary.for_settlement(sid).is_empty():
			_ids.append(sid)
			_anchor[sid] = data.anchor_of(sid)
			_extent[sid] = data.extent_units(sid)
			if terrain != null and terrain.chunk_px > 0:
				_register_chunks(sid)
	if terrain != null:
		if not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
			terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
		if not terrain.vertical_scale_changed.is_connected(_on_vertical_scale_changed):
			terrain.vertical_scale_changed.connect(_on_vertical_scale_changed)
	for arg in OS.get_cmdline_user_args():
		if arg == "--town-lod=blocks":
			_detail_override = 0.001
		elif arg == "--town-lod=detail":
			_detail_override = 10.0
		elif arg == "--no-towns":
			_disabled = true
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	stats = {"towns": _ids.size()}


func _register_chunks(sid: String) -> void:
	var a: Vector2 = _anchor[sid]
	var e: float = _extent[sid]
	var size := float(terrain.chunk_px)
	for cy in range(int(floor((a.y - e) / size)), int(floor((a.y + e) / size)) + 1):
		for cx in range(int(floor((a.x - e) / size)), int(floor((a.x + e) / size)) + 1):
			if cx < 0 or cy < 0 or cx >= TerrainBuilder.CHUNKS or cy >= TerrainBuilder.CHUNKS:
				continue
			var index := cy * TerrainBuilder.CHUNKS + cx
			if not _ids_by_chunk.has(index):
				_ids_by_chunk[index] = []
			(_ids_by_chunk[index] as Array).append(sid)


func town_ids() -> Array[String]:
	return _ids


## Cercles de finage (x, y, rayon en unités monde) : parcellaire du lot ZG5b.
func finage_zones() -> PackedVector3Array:
	return data.finage_zones() if data != null else PackedVector3Array()


## Vrai si la ville 1:1 de la colonie `id` est construite et affichée.
func is_shown(id: String) -> bool:
	if not active or not _entries.has(id):
		return false
	var entry: Dictionary = _entries[id]
	return entry.get("builder") != null


func plan_of(id: String) -> Dictionary:
	return (_entries.get(id, {}) as Dictionary).get("plan", {})


## PF1 : préréglage de qualité (niveau lu par `RenderQuality.current()`).
func apply_render_quality(_preset: Dictionary) -> void:
	if profile == null:
		return
	_quality = profile.factors(RenderQuality.current())
	for entry in _entries.values():
		for key in ["builder", "pending"]:
			var b: TownBuilder = entry.get(key)
			if b != null:
				_configure(b)


func _configure(b: TownBuilder) -> void:
	var detail := profile.detail_range * float(_quality.get("detail", 1.0)) * _detail_override
	b.set_ranges(detail, profile.block_range * float(_quality.get("block", 1.0)), bool(_quality.get("detail_shadows", true)), bool(_quality.get("block_shadows", false)))


# --- Mise à jour par image ----------------------------------------------------------------


func update_view(rig_distance: float) -> void:
	if data == null or _ids.is_empty():
		return
	var usable := not _disabled and (force_active or (terrain != null and terrain.quadtree != null))
	var now_active := usable and tiers.valley_weight(rig_distance) >= profile.min_valley_weight
	if now_active != active:
		active = now_active
		visible = active
		version += 1
		towns_changed.emit()
	_last_distance = rig_distance
	_poll_jobs()
	if not active:
		return
	_stream_timer -= 1
	if _stream_timer <= 0:
		_stream_timer = 10
		_stream(rig_distance)
	_step_builders(int(profile.build_budget_ms * 1000.0))
	_check_reground()


func _camera_ground() -> Vector2:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return Vector2.ZERO
	return Vector2(camera.global_position.x, camera.global_position.z)


func stream_radius(rig_distance: float) -> float:
	return clampf(profile.stream_factor * rig_distance, profile.stream_min, profile.stream_max) * float(_quality.get("stream", 1.0))


## Demande les villes proches, décharge les lointaines.
## Rend le nombre de villes voulues pas encore chargées ni en cours.
func _stream(rig_distance: float, center: Variant = null) -> int:
	var here: Vector2 = center if center is Vector2 else _camera_ground()
	var radius := stream_radius(rig_distance)
	var wanted: Array = []
	for id in _ids:
		var d := here.distance_to(_anchor[id]) - float(_extent[id])
		if d <= radius:
			wanted.append([d, id])
		elif d > radius * profile.unload_factor and _entries.has(id):
			_unload(id)
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var missing := 0
	for w in wanted:
		var id: String = w[1]
		if _entries.has(id) or _jobs.has(id):
			continue
		if _plan_cache.has(id):
			_entries[id] = {"plan": _plan_cache[id], "dirty_ms": Time.get_ticks_msec()}
			_plan_cache.erase(id)
			_cache_order.erase(id)
			_start_build(id)
			continue
		if _jobs.size() >= profile.max_plan_jobs:
			missing += 1
			continue
		_start_plan(id)
	stats["loaded"] = _entries.size()
	stats["jobs"] = _jobs.size()
	stats["radius"] = radius
	return missing


func _heights_for(id: String) -> TownPlan.Heights:
	var h := TownPlan.Heights.new()
	h.anchor = _anchor[id]
	h.meters_per_unit = data.meters_per_unit
	h.map_data = map_data
	h.fallback_m = float(data.towns[id].get("z_m", 0.0))
	if terrain != null and terrain.quadtree != null:
		var e: float = _extent[id]
		var a: Vector2 = _anchor[id]
		var snap := terrain.quadtree.surface_snapshot(Rect2(a - Vector2(e, e), Vector2(e, e) * 2.0), a)
		h.pages = snap.get("qt_pages", {})
		h.top_level = int(snap.get("max_level", 0))
		h.h_min = float(snap.get("h_min", 0.0))
		h.h_range = float(snap.get("h_range", 1.0))
	return h


func _start_plan(id: String) -> void:
	var heights := _heights_for(id)
	var town: Dictionary = data.towns[id]
	var task := WorkerThreadPool.add_task(_run_plan.bind(id, town, data.params, heights), false, "town plan " + id)
	_jobs[id] = [task, "plan"]


func _run_plan(id: String, town: Dictionary, params: Dictionary, heights: TownPlan.Heights) -> void:
	var t0 := Time.get_ticks_usec()
	var plan := TownPlan.generate(town, params, heights)
	plan["plan_usec"] = Time.get_ticks_usec() - t0
	_mutex.lock()
	_results[id] = plan
	_mutex.unlock()


func _run_reground(id: String, plan: Dictionary, heights: TownPlan.Heights) -> void:
	TownPlan.reground(plan, heights)
	_mutex.lock()
	_results[id] = plan
	_mutex.unlock()


func _poll_jobs() -> void:
	for id in _jobs.keys():
		var job: Array = _jobs[id]
		if not WorkerThreadPool.is_task_completed(job[0]):
			continue
		WorkerThreadPool.wait_for_task_completion(job[0])
		_jobs.erase(id)
		_mutex.lock()
		var plan: Dictionary = _results.get(id, {})
		_results.erase(id)
		_mutex.unlock()
		if plan.is_empty():
			continue
		if str(job[1]) == "plan":
			_plan_usec.append(int(plan.get("plan_usec", 0)))
			if not active:
				_cache_plan(id, plan)
				continue
			_entries[id] = {"plan": plan, "dirty_ms": 0}
			_start_build(id)
		elif _entries.has(id):
			_entries[id]["plan"] = plan
			_start_build(id)


func _start_build(id: String) -> void:
	var entry: Dictionary = _entries[id]
	var old: TownBuilder = entry.get("pending")
	if old != null:
		old.free_nodes()
	var b := TownBuilder.new(entry["plan"], _anchor[id], data.meters_per_unit, self)
	_configure(b)
	b.root.visible = false
	entry["pending"] = b


func _step_builders(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	var here := _camera_ground()
	var pending: Array = []
	for id in _entries:
		var b: TownBuilder = _entries[id].get("pending")
		if b != null:
			pending.append([here.distance_to(_anchor[id]), id])
	pending.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for p in pending:
		var id: String = p[1]
		var entry: Dictionary = _entries[id]
		var b: TownBuilder = entry["pending"]
		var left := budget_usec - (Time.get_ticks_usec() - t0)
		if left <= 0:
			break
		if b.step(left):
			var previous: TownBuilder = entry.get("builder")
			if previous != null:
				previous.free_nodes()
			b.root.visible = true
			entry["builder"] = b
			entry["pending"] = null
			version += 1
			towns_changed.emit()
		if not FrameBudget.has_time():
			break
	_step_max_usec = maxi(_step_max_usec, Time.get_ticks_usec() - t0)
	stats["build_step_max_ms"] = _step_max_usec / 1000.0


func _unload(id: String) -> void:
	var entry: Dictionary = _entries[id]
	for key in ["builder", "pending"]:
		var b: TownBuilder = entry.get(key)
		if b != null:
			b.free_nodes()
	_entries.erase(id)
	_cache_plan(id, entry["plan"])
	version += 1
	towns_changed.emit()


func _cache_plan(id: String, plan: Dictionary) -> void:
	_plan_cache[id] = plan
	_cache_order.erase(id)
	_cache_order.append(id)
	while _cache_order.size() > profile.plan_cache:
		_plan_cache.erase(_cache_order.pop_front())


func _on_chunk_surface_changed(index: int) -> void:
	# Hauteurs en mètres : les recalages dus à l'exagération ne changent rien.
	if terrain != null and terrain.rescaling_vertical:
		return
	for id in _ids_by_chunk.get(index, []):
		if _entries.has(id):
			_entries[id]["dirty_ms"] = Time.get_ticks_msec()


func _check_reground() -> void:
	var now := Time.get_ticks_msec()
	for id in _entries:
		var entry: Dictionary = _entries[id]
		var dirty: int = entry.get("dirty_ms", 0)
		if dirty <= 0 or now - dirty < profile.reground_settle_ms or _jobs.has(id) or entry.get("pending") != null:
			continue
		if _jobs.size() >= profile.max_plan_jobs:
			return
		entry["dirty_ms"] = 0
		var task := WorkerThreadPool.add_task(_run_reground.bind(id, entry["plan"], _heights_for(id)), false, "town reground " + id)
		_jobs[id] = [task, "reground"]


func _on_vertical_scale_changed(_old: float, new_scale: float) -> void:
	for entry in _entries.values():
		for key in ["builder", "pending"]:
			var b: TownBuilder = entry.get(key)
			if b != null:
				b.refresh_aabbs(new_scale)


## Termine tout de suite plans et constructions autour de `center` (captures, tests).
func flush(center: Variant = null) -> void:
	if not active or data == null:
		return
	FrameBudget.unlimited = true
	for _round in 64:
		var missing := _stream(_last_distance if _last_distance < INF else 1.0, center)
		for id in _jobs.keys():
			WorkerThreadPool.wait_for_task_completion(_jobs[id][0])
		_force_poll()
		_step_builders(1 << 30)
		var busy := not _jobs.is_empty() or missing > 0
		for entry in _entries.values():
			if entry.get("pending") != null or int(entry.get("dirty_ms", 0)) > 0:
				busy = true
		if not busy:
			break
		for entry in _entries.values():
			entry["dirty_ms"] = 0
	# Hauteurs à jour (pages arrivées depuis le plan) : recalcul et reconstruction immédiats.
	for id in _entries:
		var entry: Dictionary = _entries[id]
		if entry.get("builder") == null:
			continue
		TownPlan.reground(entry["plan"], _heights_for(id))
		_start_build(id)
	_step_builders(1 << 30)
	FrameBudget.unlimited = false
	_update_stats()
	print("TownLayer: flush %s, shown %s" % [JSON.stringify(stats), str(_entries.keys())])


## Collecte des tâches déjà attendues (`flush`) : `is_task_completed` peut rester faux jusqu'à
## l'appel de `wait_for_task_completion`, déjà fait.
func _force_poll() -> void:
	for id in _jobs.keys():
		var job: Array = _jobs[id]
		_jobs.erase(id)
		_mutex.lock()
		var plan: Dictionary = _results.get(id, {})
		_results.erase(id)
		_mutex.unlock()
		if plan.is_empty():
			continue
		if str(job[1]) == "plan":
			_plan_usec.append(int(plan.get("plan_usec", 0)))
			_entries[id] = {"plan": plan, "dirty_ms": 0}
		else:
			_entries[id]["plan"] = plan
		_start_build(id)


func _update_stats() -> void:
	var houses := 0
	var built := 0
	for entry in _entries.values():
		if entry.get("builder") != null:
			built += 1
			houses += int((entry["plan"] as Dictionary).get("stats", {}).get("houses", 0))
	stats["built"] = built
	stats["houses"] = houses
	if not _plan_usec.is_empty():
		var total := 0
		var worst := 0
		for u in _plan_usec:
			total += u
			worst = maxi(worst, u)
		stats["plans"] = _plan_usec.size()
		stats["plan_ms_avg"] = total / 1000.0 / _plan_usec.size()
		stats["plan_ms_max"] = worst / 1000.0


func _exit_tree() -> void:
	for id in _jobs.keys():
		WorkerThreadPool.wait_for_task_completion(_jobs[id][0])
	_jobs.clear()
