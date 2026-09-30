class_name TownFarLayer
extends Node3D

## Lot VT-E (ADR 0138) : maillage lointain des villes à l'échelle 1:1 (paliers F1/F2), par tuiles.
## Rendu seulement.
## - Index de ville stable (masque `TownFarMask`) : ordre de `towns_1340.json`, puis les villes v2
##   (`LandmarkV2Library.all()`, triées par colonie). Une colonie qui a une ville v2 prend son
##   lointain v2 (F1 et F2) au lieu de F1/F2 (sauf `--no-landmarks-1to1`).
## - Génération au chargement dans `WorkerThreadPool` (tâche de groupe, un élément = une tuile :
##   `TownFarBuilder` puis `mesh_arrays`) ; le fil principal ne fait que créer les `ArrayMesh` et
##   les nœuds, par petites étapes (`build_budget_ms` et `FrameBudget.has_time()`, au moins une
##   tuile par image).
## - Tuiles : F1 par `f1_tile` unités (visible jusqu'à `f1_range`, fondu), F2 par `f2_tile`
##   unités (de `f1_range`, fondu, à `ZoomTiers.model_range`). Ombres : F1 seulement, rig sous
##   `shadow_rig_distance` ; F2 jamais. Calque masqué en vue stratégique.
## - Passage au plan complet : union des `built_ids()` des calques 1:1 (`TownLayer`,
##   `LandmarkCityLayer`) → `TownFarMask` ; le shader enfonce ces villes sous
##   `sink_factor` × portée des blocs (`TownRenderProfile.block_range` × qualité). Au-delà de
##   `max_rig_distance`, les calques 1:1 sont inactifs, `built_ids()` est vide : masque vidé.
## Options : `--no-town-far` (calque coupé).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SHADER := preload("res://shaders/town_far.gdshader")

## Côtés des tuiles (unités carte).
@export var f1_tile: float = 128.0
@export var f2_tile: float = 512.0
## Fin de F1 et début de F2 (distance caméra, unités), multipliée par le facteur de qualité ;
## fondu croisé sur `fade_fraction` de cette portée.
@export var f1_range: float = 300.0
@export var fade_fraction: float = 0.1
## Ombres de F1 sous cette distance du rig (hystérésis 10 %).
@export var shadow_rig_distance: float = 60.0
## Calque masqué à partir de ce poids de la vue stratégique (parchemin).
@export var hide_strategic_weight: float = 0.99
## Enfoncement d'une ville construite en 1:1 sous `sink_factor` × portée des blocs.
@export var sink_factor: float = 0.95
## Fils de travail de la génération (0 : moitié des cœurs, 4 au plus : au-delà, la génération
## GDScript ne va pas plus vite, 1,2 s mesurées à 4 comme à 8 fils).
@export var generation_threads: int = 0
## Budget de construction des tuiles par image (ms, fil principal ; une tuile ≤ ~1 ms de plus).
@export var build_budget_ms: float = 2.0
## Facteurs par niveau de `RenderQuality` : portée de F1, ombres de F1.
@export var quality: Dictionary = {
	"low": {"f1": 0.5, "shadows": false},
	"medium": {"f1": 0.8, "shadows": false},
	"high": {"f1": 1.0, "shadows": true},
	"ultra": {"f1": 1.2, "shadows": true},
}

var profile: TownRenderProfile
var data: TownData
var terrain: TerrainBuilder
var map_data: MapData
var tiers: ZoomTiers
## Calques 1:1 dont on lit `built_ids()` et `version` (TownLayer, LandmarkCityLayer ; tests :
## tout objet qui a ces deux membres).
var sources: Array = []
var mask := TownFarMask.new()
var material: ShaderMaterial
var stats: Dictionary = {}

var _index_of: Dictionary = {}  # id de colonie → index de ville (masque)
## Tuiles à générer : {level ("f1" | "f2"), key (Vector2i), items: [{i, town} | {i, city, anchor, ground}]}
var _tiles: Array = []
var _wall_params: Dictionary = {}
var _mpu := 719.0
var _group := -1
var _mutex := Mutex.new()
var _ready: Array = []  # résultats des fils (sous `_mutex`)
var _gen_start_usec := 0
var _gen_end_usec := 0
var _worker_usec := 0
var _built := 0
var _nodes_f1: Array[MeshInstance3D] = []
var _nodes_f2: Array[MeshInstance3D] = []
var _bounds: Dictionary = {}  # nœud → Vector2(y_min, y_max) (mètres)
var _quality: Dictionary = {}
var _masked: Dictionary = {}  # index → vrai
var _mask_key: Array = []
var _shadows_on := false
var _camera := Vector3(INF, INF, INF)
var _disabled := false
var _frame_max_usec := 0
var _step_max_usec := 0


func setup(p_map: MapData, p_terrain: TerrainBuilder, p_tiers: ZoomTiers, settlement_ids: Array, p_sources: Array = [], p_data: TownData = null) -> void:
	name = "TownFar"
	map_data = p_map
	terrain = p_terrain
	tiers = p_tiers if p_tiers != null else ZoomTiers.new()
	sources = p_sources
	profile = TownRenderProfile.load_default()
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-town-far":
			_disabled = true
	data = p_data if p_data != null else TownData.load_from(MAP_PATHS.default_data_dir().path_join("map"))
	_mpu = data.meters_per_unit
	_wall_params = data.params.get("walls", {})
	material = _make_material()
	if terrain != null and not terrain.vertical_scale_changed.is_connected(_on_vertical_scale_changed):
		terrain.vertical_scale_changed.connect(_on_vertical_scale_changed)
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	if _disabled:
		stats = {"disabled": true}
		return
	_plan_tiles(settlement_ids)
	_start_generation()


## Index de ville (masque) de la colonie `id`, -1 si elle n'a pas de lointain.
func index_of(id: String) -> int:
	return int(_index_of.get(id, -1))


## Vrai quand toutes les tuiles sont générées et construites.
func is_complete() -> bool:
	_mutex.lock()
	var pending := _ready.size()
	_mutex.unlock()
	return _group < 0 and pending == 0 and _built >= _tiles.size()


## Portée courante de F1 (unités), qualité comprise.
func f1_range_current() -> float:
	return f1_range * float(_quality.get("f1", 1.0))


## Distance d'enfoncement courante (0 : aucune ville construite en 1:1).
func sink_distance() -> float:
	return float(material.get_shader_parameter("sink_distance")) if material != null else 0.0


func tile_nodes(level: String) -> Array[MeshInstance3D]:
	return _nodes_f1 if level == "f1" else _nodes_f2


# --- Chargement ------------------------------------------------------------------------------


func _make_material() -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	var atlas := BuildingMaterials.material("Building", "far") as ShaderMaterial
	if atlas != null:
		for p in ["albedo_array", "layer_tint", "roof_first", "roof_last"]:
			mat.set_shader_parameter(p, atlas.get_shader_parameter(p))
	mat.set_shader_parameter("meters_per_unit", _mpu)
	mat.set_shader_parameter("roofscape", profile.roofscape_strength)
	mat.set_shader_parameter("roofscape_near", profile.roofscape_near)
	mat.set_shader_parameter("roofscape_far", profile.roofscape_far)
	mat.set_shader_parameter("roofscape_cell_m", profile.roofscape_cell_m)
	mat.set_shader_parameter("roofscape_gain", profile.roofscape_gain)
	mat.set_shader_parameter("built_mask", mask.texture())
	mat.set_shader_parameter("sink_distance", 0.0)
	return mat


## Index des villes et contenu des tuiles (fil principal, au chargement).
func _plan_tiles(settlement_ids: Array) -> void:
	var t0 := Time.get_ticks_usec()
	var wanted: Dictionary = {}
	for id in settlement_ids:
		wanted[str(id)] = true
	var v2_enabled := not "--no-landmarks-1to1" in OS.get_cmdline_user_args()
	# Index stable : ordre du fichier, puis villes v2 triées par colonie.
	var n := 0
	var town_index: Dictionary = {}
	for id: String in data.towns:
		town_index[id] = n
		n += 1
	var cities: Array = LandmarkV2Library.all() if v2_enabled else []  # chargée ici, avant les fils
	cities.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return str(a.get("settlement", "")) < str(b.get("settlement", "")))
	var f1: Dictionary = {}  # Vector2i → items
	var f2: Dictionary = {}
	var v2_ids: Dictionary = {}
	for k in cities.size():
		var city: Dictionary = cities[k]
		var sid := str(city.get("settlement", ""))
		var index := n + k
		if index >= TownFarMask.CAPACITY:
			push_warning("TownFarLayer: index %d beyond mask capacity" % index)
			continue
		v2_ids[sid] = true
		if not wanted.has(sid):
			continue
		var anchor := LandmarkV2Library.anchor_units(city)
		var item := {"i": index, "city": city, "anchor": anchor, "ground": _ground_at(anchor, sid, city)}
		_index_of[sid] = index
		_add_item(f1, _key(anchor, f1_tile), item)
		_add_item(f2, _key(anchor, f2_tile), item)
	for id: String in data.towns:
		if not wanted.has(id) or v2_ids.has(id):
			continue
		var index: int = town_index[id]
		if index >= TownFarMask.CAPACITY:
			continue
		var anchor := data.anchor_of(id)
		var item := {"i": index, "town": data.towns[id]}
		_index_of[id] = index
		_add_item(f1, _key(anchor, f1_tile), item)
		_add_item(f2, _key(anchor, f2_tile), item)
	# F2 d'abord (peu nombreuses, toute la vue lointaine), puis F1.
	for key: Vector2i in f2:
		_tiles.append({"level": "f2", "key": key, "items": f2[key]})
	for key: Vector2i in f1:
		_tiles.append({"level": "f1", "key": key, "items": f1[key]})
	stats["towns"] = _index_of.size()
	stats["plan_tiles_ms"] = (Time.get_ticks_usec() - t0) / 1000.0


static func _key(p: Vector2, size: float) -> Vector2i:
	return Vector2i(floori(p.x / size), floori(p.y / size))


static func _add_item(into: Dictionary, key: Vector2i, item: Dictionary) -> void:
	if not into.has(key):
		into[key] = []
	(into[key] as Array).append(item)


## Sol (m) au centre d'une ville v2 : pages du relief chargées, sinon heightmap, sinon `z_m` de la
## colonie dans `towns_1340.json`, sinon celui de la ville v2.
func _ground_at(anchor: Vector2, sid: String, city: Dictionary) -> float:
	var fallback := float(city.get("z_m", 0.0))
	if data.has_town(sid):
		fallback = float(data.towns[sid].get("z_m", fallback))
	var h := TownPlan.Heights.new()
	h.anchor = anchor
	h.meters_per_unit = _mpu
	h.map_data = map_data
	h.fallback_m = fallback
	if terrain != null and terrain.quadtree != null:
		var e := 1.0
		var snap := terrain.quadtree.surface_snapshot(Rect2(anchor - Vector2(e, e), Vector2(e, e) * 2.0), anchor)
		h.pages = snap.get("qt_pages", {})
		h.top_level = int(snap.get("max_level", 0))
		h.h_min = float(snap.get("h_min", 0.0))
		h.h_range = float(snap.get("h_range", 1.0))
	return h.height_m(0.0, 0.0)


func _start_generation() -> void:
	if _tiles.is_empty():
		return
	_gen_start_usec = Time.get_ticks_usec()
	var threads := generation_threads if generation_threads > 0 else clampi(OS.get_processor_count() / 2, 1, 4)
	_group = WorkerThreadPool.add_group_task(_run_tile, _tiles.size(), threads, false, "town far tiles")


## Fil de travail : maillage fusionné d'une tuile.
func _run_tile(t: int) -> void:
	var t0 := Time.get_ticks_usec()
	var spec: Dictionary = _tiles[t]
	var f1 := str(spec["level"]) == "f1"
	var merged: Dictionary = {}
	for item: Dictionary in spec["items"]:
		var part: Dictionary
		if item.has("city"):
			part = TownFarBuilder.build_v2_far(item["city"], int(item["i"]), _mpu, item["anchor"], float(item["ground"]))
		elif f1:
			part = TownFarBuilder.build_f1(item["town"], int(item["i"]), _mpu, _wall_params)
		else:
			part = TownFarBuilder.build_f2(item["town"], int(item["i"]), _mpu)
		TownFarBuilder.append(merged, part)
	var result := {"t": t, "arrays": TownFarBuilder.mesh_arrays(merged), "triangles": TownFarBuilder.triangle_count(merged)}
	if int(result["triangles"]) > 0:
		var verts: PackedVector3Array = merged["vertices"]
		var lo := Vector2(INF, INF)
		var hi := Vector2(-INF, -INF)
		for v in verts:
			lo = Vector2(minf(lo.x, v.x), minf(lo.y, v.z))
			hi = Vector2(maxf(hi.x, v.x), maxf(hi.y, v.z))
		result["rect"] = Rect2(lo, hi - lo)
		result["y"] = Vector2(float(merged["y_min"]), float(merged["y_max"]))
		result["vertices"] = verts.size()
		result["indices"] = (merged["indices"] as PackedInt32Array).size()
	var usec := Time.get_ticks_usec() - t0
	_mutex.lock()
	_ready.append(result)
	_worker_usec += usec
	_gen_end_usec = Time.get_ticks_usec()
	_mutex.unlock()


# --- Mise à jour par image -------------------------------------------------------------------


func update_view(rig_distance: float) -> void:
	if _disabled or _tiles.is_empty():
		return
	var t_start := Time.get_ticks_usec()
	var tp := t_start
	_build_step(int(build_budget_ms * 1000.0))
	tp = PerfProbe.lap("townfar/build", tp)
	var show := tiers.strategic_weight(rig_distance) < hide_strategic_weight
	if show != visible:
		visible = show
	if show:
		_update_mask()
		tp = PerfProbe.lap("townfar/mask", tp)
		_update_view_params(rig_distance)
		PerfProbe.lap("townfar/view", tp)
	_frame_max_usec = maxi(_frame_max_usec, Time.get_ticks_usec() - t_start)
	stats["frame_max_ms"] = _frame_max_usec / 1000.0


## Construit les tuiles prêtes (au moins une par image, puis tant que le budget le permet).
func _build_step(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	while true:
		_mutex.lock()
		var result: Dictionary = _ready.pop_back() if not _ready.is_empty() else {}
		_mutex.unlock()
		if result.is_empty():
			break
		var tb := Time.get_ticks_usec()
		var first := _built == 0
		_build_tile(result)
		var took := Time.get_ticks_usec() - tb
		if first:
			stats["build_first_tile_ms"] = took / 1000.0  # premier matériau, premier maillage
		else:
			_step_max_usec = maxi(_step_max_usec, took)
		if Time.get_ticks_usec() - t0 >= budget_usec or not FrameBudget.has_time():
			break
	_poll_group()


func _poll_group() -> void:
	if _group < 0 or not WorkerThreadPool.is_group_task_completed(_group):
		return
	WorkerThreadPool.wait_for_group_task_completion(_group)
	_group = -1
	stats["gen_ms"] = (_gen_end_usec - _gen_start_usec) / 1000.0
	stats["gen_cpu_ms"] = _worker_usec / 1000.0


func _build_tile(result: Dictionary) -> void:
	_built += 1
	var arrays: Array = result["arrays"]
	if arrays.is_empty():
		return
	var spec: Dictionary = _tiles[int(result["t"])]
	var f1 := str(spec["level"]) == "f1"
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var node := MeshInstance3D.new()
	node.name = "%s_%d_%d" % [spec["level"], spec["key"].x, spec["key"].y]
	node.mesh = mesh
	node.material_override = material
	node.set_meta("rect", result["rect"])
	_bounds[node] = result["y"]
	_apply_aabb(node, MapData.vertical_scale())
	if f1:
		_nodes_f1.append(node)
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if _shadows_on else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	else:
		_nodes_f2.append(node)
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_apply_range(node, f1)
	add_child(node)
	var key := "f1" if f1 else "f2"
	stats["tiles_" + key] = int(stats.get("tiles_" + key, 0)) + 1
	stats["triangles_" + key] = int(stats.get("triangles_" + key, 0)) + int(result["triangles"])
	stats["vertices"] = int(stats.get("vertices", 0)) + int(result["vertices"])
	stats["indices"] = int(stats.get("indices", 0)) + int(result["indices"])
	# Mémoire vidéo estimée : position 12 o, normale 4 o (octaédrique), couleur 4 o (RGBA8),
	# UV2 8 o ; indices 32 bits au-delà de 65 535 sommets par tuile, 16 sinon (majorant : 4 o).
	stats["vram_mb"] = (int(stats["vertices"]) * 28 + int(stats["indices"]) * 4) / 1048576.0
	stats["build_tile_max_ms"] = _step_max_usec / 1000.0


## Boîte englobante monde d'une tuile selon l'échelle verticale (le sol est posé par le shader).
func _apply_aabb(node: MeshInstance3D, s: float) -> void:
	var y: Vector2 = _bounds[node]
	var rect: Rect2 = node.get_meta("rect")
	var up := 1.0 + MapData.relief_gain_for_scale(s)  # ZG8 : y ≤ s·(1 + g)·h
	var down := 1.0 - MapData.relief_squash_max_for_scale(s)  # SZ1 : y ≥ s·(1 − c·k)·h
	var margin := 200.0 / _mpu  # bâti (m / m par unité, non exagéré) et enfoncement
	var y0 := y.x * s * (down if y.x > 0.0 else 1.0) - margin
	var y1 := y.y * s * (up if y.y > 0.0 else 1.0) + margin
	node.custom_aabb = AABB(Vector3(rect.position.x, y0, rect.position.y), Vector3(rect.size.x, y1 - y0, rect.size.y))


func _apply_range(node: MeshInstance3D, f1: bool) -> void:
	var r := f1_range_current()
	node.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	if f1:
		node.visibility_range_begin = 0.0
		node.visibility_range_end = r
		node.visibility_range_end_margin = r * fade_fraction
	else:
		node.visibility_range_begin = r
		node.visibility_range_begin_margin = r * fade_fraction
		node.visibility_range_end = tiers.model_range
		node.visibility_range_end_margin = tiers.model_range * 0.05


## Masque des villes 1:1 construites, recalculé quand un calque source change (`version`).
func _update_mask() -> void:
	var key: Array = []
	for s in sources:
		key.append(int(s.get("version")) if s != null else -1)
	if key == _mask_key:
		return
	_mask_key = key
	var want: Dictionary = {}
	for s in sources:
		if s == null:
			continue
		for id in s.built_ids():
			var index := index_of(str(id))
			if index >= 0:
				want[index] = true
	for index: int in _masked:
		if not want.has(index):
			mask.set_built(index, false)
	for index: int in want:
		mask.set_built(index, true)
	_masked = want
	material.set_shader_parameter("built_mask", mask.texture())
	material.set_shader_parameter("sink_distance", _sink_distance() if not want.is_empty() else 0.0)
	stats["masked"] = want.size()


func _sink_distance() -> float:
	var block := float(profile.factors(RenderQuality.current()).get("block", 1.0))
	return profile.block_range * block * sink_factor


## Caméra d'enfoncement (passe d'ombre comprise) et ombres de F1 selon la distance du rig.
func _update_view_params(rig_distance: float) -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera != null and not camera.global_position.is_equal_approx(_camera):
		_camera = camera.global_position
		material.set_shader_parameter("sink_camera", _camera)
	var limit := shadow_rig_distance * (1.1 if _shadows_on else 1.0)
	var want := bool(_quality.get("shadows", true)) and rig_distance < limit
	if want != _shadows_on:
		_shadows_on = want
		var mode := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if want else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		for node in _nodes_f1:
			node.cast_shadow = mode
	stats["shadows"] = _shadows_on


## PF1 : préréglage de qualité (niveau lu par `RenderQuality.current()`).
func apply_render_quality(_preset: Dictionary) -> void:
	_quality = quality.get(RenderQuality.current(), quality.get("high", {}))
	for node in _nodes_f1:
		_apply_range(node, true)
	for node in _nodes_f2:
		_apply_range(node, false)
	_mask_key = []  # portée des blocs changée : enfoncement recalculé
	if not bool(_quality.get("shadows", true)) and _shadows_on:
		_shadows_on = false
		for node in _nodes_f1:
			node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


func _on_vertical_scale_changed(_old: float, new_scale: float) -> void:
	for node in _nodes_f1:
		_apply_aabb(node, new_scale)
	for node in _nodes_f2:
		_apply_aabb(node, new_scale)


## Termine tout de suite génération et construction (captures, tests).
func flush() -> void:
	if _group >= 0:
		WorkerThreadPool.wait_for_group_task_completion(_group)
		_group = -1
		stats["gen_ms"] = (_gen_end_usec - _gen_start_usec) / 1000.0
		stats["gen_cpu_ms"] = _worker_usec / 1000.0
	FrameBudget.unlimited = true
	_build_step(1 << 30)
	FrameBudget.unlimited = false
	_mask_key = []


func _exit_tree() -> void:
	if _group >= 0:
		WorkerThreadPool.wait_for_group_task_completion(_group)
		_group = -1
