class_name LandmarkCityLayer
extends Node3D

## Lot VH4 (ADR 0078) : villes emblématiques à l'échelle 1:1 (format v2) aux paliers « vallée »
## et « site », sur le relief fin. Rendu seulement.
## - Données : `data/landmarks_v2/<id>.json` (`LandmarkV2Library`).
## - Plan dans un fil de travail (`LandmarkPlan.generate` sur un instantané des pages du
##   quadtree, puis `TownBuilder.prepare`), construction par étapes (`TownBuilder.step`, budget
##   `TownRenderProfile.build_budget_ms` et `FrameBudget`, ADR 0051). Mêmes portées, HLOD par
##   maison et hauteurs (mètres posés par le shader, hauteur affichée ZG8) que les villes
##   ordinaires ZG6 ; nœuds des maisons par cellule d'îlots (`plan.detail_cell_m`).
## - Transition (ADR 0078) : la ville se prépare dès que la caméra approche (`prepare_valley`
##   du palier vallée) ; `fade(id)` rend l'opacité de la maquette L1/L2 (1 → 0 entre les poids
##   vallée `render.fade_valley`), que `SettlementLayer` applique à `LandmarkModel`.
## - Pages de relief plus fines : hauteurs recalculées (`LandmarkPlan.reground`) puis bloc
##   reconstruit et échangé.

signal cities_changed

## Poids du palier vallée à partir duquel la ville est planifiée et construite (avant le fondu).
const PREPARE_VALLEY := 0.15
const DEFAULT_FADE := [0.35, 0.65]

var profile: TownRenderProfile
var terrain: TerrainBuilder
var map_data: MapData
var tiers: ZoomTiers
var year := 1340
## Force l'activité sans pyramide (tests headless).
var force_active := false
var version := 0
var stats: Dictionary = {}

var _cities: Dictionary = {}  # settlement id → ville v2
var _anchor: Dictionary = {}  # id → Vector2 (unités)
var _extent: Dictionary = {}  # id → rayon (unités)
var _ids_by_chunk: Dictionary = {}
var _entries: Dictionary = {}  # id → {plan, builder, pending, dirty_ms}
var _jobs: Dictionary = {}  # id → [task, kind]
var _results: Dictionary = {}
var _mutex := Mutex.new()
var _valley := 0.0
var _last_distance := INF
var _quality: Dictionary = {}
var _disabled := false
## Point visé imposé (tests sans caméra), unités carte.
var focus_override: Variant = null


func setup(p_map: MapData, p_terrain: TerrainBuilder, p_tiers: ZoomTiers, settlement_ids: Array) -> void:
	name = "LandmarkCities"
	map_data = p_map
	terrain = p_terrain
	tiers = p_tiers if p_tiers != null else ZoomTiers.new()
	profile = TownRenderProfile.load_default()
	TownBuilder.manifest()
	for id in settlement_ids:
		var sid := str(id)
		var city := LandmarkV2Library.for_settlement(sid)
		if city.is_empty():
			continue
		_cities[sid] = city
		_anchor[sid] = LandmarkV2Library.anchor_units(city)
		_extent[sid] = LandmarkV2Library.extent_units(city)
		if terrain != null and terrain.chunk_px > 0:
			_register_chunks(sid)
	if terrain != null:
		if not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
			terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
		if not terrain.vertical_scale_changed.is_connected(_on_vertical_scale_changed):
			terrain.vertical_scale_changed.connect(_on_vertical_scale_changed)
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-landmarks-1to1":
			_disabled = true
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	stats = {"cities": _cities.size()}


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


## Année de la partie : si un élément daté apparaît ou disparaît (beffroi de 1389, aître de
## 1348…), les villes chargées sont replanifiées.
func set_year(p_year: int) -> void:
	if p_year == year:
		return
	var changed := false
	for city in _cities.values():
		if _dated_signature(city, year) != _dated_signature(city, p_year):
			changed = true
	year = p_year
	if changed:
		for id in _entries.keys():
			_unload(id)


static func _dated_signature(city: Dictionary, p_year: int) -> String:
	var sig := ""
	for key in ["walls", "streets", "bridges", "monuments", "districts", "open_spaces"]:
		for item in city.get(key, []):
			if (item.has("from_year") or item.has("until_year")) and LandmarkV2Library.present(item, p_year):
				sig += str(item.get("id", "")) + ","
	return sig


func has_city(id: String) -> bool:
	return _cities.has(id)


func city_ids() -> Array:
	return _cities.keys()


## Vrai si la ville 1:1 de la colonie `id` est construite et affichée.
func is_shown(id: String) -> bool:
	return _entries.has(id) and (_entries[id] as Dictionary).get("builder") != null and visible


func plan_of(id: String) -> Dictionary:
	return (_entries.get(id, {}) as Dictionary).get("plan", {})


## Cercle (x, z, rayon en unités) couvert par la ville 1:1 de `id`.
func zone_of(id: String) -> Vector3:
	var a: Vector2 = _anchor[id]
	return Vector3(a.x, a.y, float(_extent[id]))


## Opacité (0-1) de la maquette L1/L2 de `id` : 1 tant que la ville 1:1 n'est pas prête,
## puis fondu entre les poids vallée `render.fade_valley`.
func fade(id: String) -> float:
	if _disabled or not is_shown(id):
		return 1.0
	var range_v: Array = (_cities[id] as Dictionary).get("render", {}).get("fade_valley", DEFAULT_FADE)
	return 1.0 - smoothstep(float(range_v[0]), float(range_v[1]), _valley)


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
	b.set_ranges(profile.detail_range * float(_quality.get("detail", 1.0)), profile.block_range * float(_quality.get("block", 1.0)), bool(_quality.get("detail_shadows", true)), bool(_quality.get("block_shadows", false)))


# --- Mise à jour par image ----------------------------------------------------------------


func update_view(rig_distance: float) -> void:
	if _cities.is_empty():
		return
	_last_distance = rig_distance
	var usable := not _disabled and (force_active or (terrain != null and terrain.quadtree != null))
	_valley = tiers.valley_weight(rig_distance) if usable else 0.0
	var now_visible := usable and _valley >= PREPARE_VALLEY
	if now_visible != visible:
		visible = now_visible
		version += 1
		cities_changed.emit()
	_poll_jobs()
	if not now_visible:
		return
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera != null:
		TownBuilder.set_lod_view(camera.global_position, profile.detail_range * float(_quality.get("detail", 1.0)))
	_stream(rig_distance)
	_step_builders(int(profile.build_budget_ms * 1000.0))
	_check_reground()


func _camera_ground() -> Vector2:
	if focus_override is Vector2:
		return focus_override
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return Vector2.ZERO
	return Vector2(camera.global_position.x, camera.global_position.z)


func _stream(rig_distance: float, center: Variant = null) -> int:
	var here: Vector2 = center if center is Vector2 else _camera_ground()
	var radius := clampf(profile.stream_factor * rig_distance, profile.stream_min, profile.stream_max) * float(_quality.get("stream", 1.0))
	var missing := 0
	for id in _cities:
		var d := here.distance_to(_anchor[id]) - float(_extent[id])
		if d > radius * profile.unload_factor:
			if _entries.has(id):
				_unload(id)
			continue
		if d > radius or _entries.has(id) or _jobs.has(id):
			continue
		if _jobs.size() >= profile.max_plan_jobs:
			missing += 1
			continue
		_start_plan(id)
	stats["loaded"] = _entries.size()
	return missing


func _heights_for(id: String) -> TownPlan.Heights:
	var h := TownPlan.Heights.new()
	h.anchor = _anchor[id]
	h.meters_per_unit = LandmarkV2Library.meters_per_unit()
	h.map_data = map_data
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
	var task := WorkerThreadPool.add_task(_run_plan.bind(id, _cities[id], year, _heights_for(id)), false, "landmark plan " + id)
	_jobs[id] = [task, "plan"]


func _run_plan(id: String, city: Dictionary, p_year: int, heights: TownPlan.Heights) -> void:
	var t0 := Time.get_ticks_usec()
	var plan := LandmarkPlan.generate(city, p_year, heights)
	TownBuilder.prepare(plan)
	plan["plan_usec"] = Time.get_ticks_usec() - t0
	_mutex.lock()
	_results[id] = plan
	_mutex.unlock()


func _run_reground(id: String, plan: Dictionary, heights: TownPlan.Heights) -> void:
	LandmarkPlan.reground(plan, heights)
	TownBuilder.prepare(plan)
	_mutex.lock()
	_results[id] = plan
	_mutex.unlock()


func _poll_jobs(force: bool = false) -> void:
	for id in _jobs.keys():
		var job: Array = _jobs[id]
		if not force and not WorkerThreadPool.is_task_completed(job[0]):
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
			stats["plan_ms"] = int(plan.get("plan_usec", 0)) / 1000.0
			stats["houses"] = int(plan.get("stats", {}).get("houses", 0))
			stats["monuments"] = int(plan.get("stats", {}).get("monuments", 0))
			_entries[id] = {"plan": plan, "dirty_ms": 0}
		elif _entries.has(id):
			_entries[id]["plan"] = plan
		else:
			continue
		_start_build(id)


func _start_build(id: String) -> void:
	var entry: Dictionary = _entries[id]
	var old: TownBuilder = entry.get("pending")
	if old != null:
		old.free_nodes()
	var b := TownBuilder.new(entry["plan"], _anchor[id], LandmarkV2Library.meters_per_unit(), self)
	b.root.name = "City_" + id
	_configure(b)
	b.root.visible = false
	entry["pending"] = b


func _step_builders(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	for id in _entries:
		var entry: Dictionary = _entries[id]
		var b: TownBuilder = entry.get("pending")
		if b == null:
			continue
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
			stats["build_task_max_ms"] = b.task_max_usec / 1000.0
			version += 1
			cities_changed.emit()
		if not FrameBudget.has_time():
			break


func _unload(id: String) -> void:
	var entry: Dictionary = _entries[id]
	for key in ["builder", "pending"]:
		var b: TownBuilder = entry.get(key)
		if b != null:
			b.free_nodes()
	_entries.erase(id)
	version += 1
	cities_changed.emit()


func _on_chunk_surface_changed(index: int) -> void:
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
		entry["dirty_ms"] = 0
		var task := WorkerThreadPool.add_task(_run_reground.bind(id, entry["plan"], _heights_for(id)), false, "landmark reground " + id)
		_jobs[id] = [task, "reground"]


func _on_vertical_scale_changed(_old: float, new_scale: float) -> void:
	for entry in _entries.values():
		for key in ["builder", "pending"]:
			var b: TownBuilder = entry.get(key)
			if b != null:
				b.refresh_aabbs(new_scale)


## Termine tout de suite plans et constructions autour de `center` (captures, tests), hauteurs
## recalculées sur les pages présentes.
func flush(center: Variant = null) -> void:
	if not visible or _cities.is_empty():
		return
	FrameBudget.unlimited = true
	for _round in 16:
		var missing := _stream(_last_distance if _last_distance < INF else 1.0, center)
		_poll_jobs(true)
		_step_builders(1 << 30)
		var busy := not _jobs.is_empty() or missing > 0
		for entry in _entries.values():
			if entry.get("pending") != null:
				busy = true
		if not busy:
			break
	for id in _entries:
		var entry: Dictionary = _entries[id]
		if entry.get("builder") == null:
			continue
		entry["dirty_ms"] = 0
		LandmarkPlan.reground(entry["plan"], _heights_for(id))
		TownBuilder.prepare(entry["plan"])
		_start_build(id)
	_step_builders(1 << 30)
	FrameBudget.unlimited = false
	print("LandmarkCityLayer: flush %s, shown %s" % [JSON.stringify(stats), str(_entries.keys())])


func _exit_tree() -> void:
	for id in _jobs.keys():
		WorkerThreadPool.wait_for_task_completion(_jobs[id][0])
	_jobs.clear()
