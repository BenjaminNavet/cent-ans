class_name FieldLayer
extends Node3D

## Lot DN-CHAMPS (ADR 0220) : champs, vergers, vignes et terrasses de la carte de campagne en
## modèles 3D générés (`crop_*`, `tree_*`, `econ_*`), posés par-dessus le sol peint. Rendu seulement.
## - Placement : `FieldPlan` (parcelles, cultures par région dominante, rangées), par cellules de
##   `cell_px` planifiées à la demande autour du point visé, sous un budget de ms par image, et mises
##   en cache (hauteur en mètres comprise : la hauteur affichée est relue à chaque reconstruction).
## - Rendu : un `MultiMesh` par modèle (un appel de dessin par modèle), reconstruit quand le point
##   visé s'éloigne du centre, que la distance, le LOD ou l'échelle verticale changent, ou que de
##   nouvelles cellules sont prêtes (au plus une fois toutes les `rebuild_period_frames` images).
## - Au-delà de `view_range_units` (palier parchemin, carte lointaine) rien n'est construit.
## - Les forêts, prés et haies ne sont pas à cette couche (cultures seulement).
## - `--no-fields` coupe la couche (A/B).

const CONFIG_FILE := "art/dn_fields.json"
const MODEL_ROOT := "res://assets/models/dn/"

var config: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false
var stats: Dictionary = {}

var plan: FieldPlan
var _terrain: TerrainBuilder
var _map: MapData
var _mpu := 719.0
var _root: Node3D
var _cells: Dictionary = {}  # Vector2i -> {"items": id -> PackedFloat32Array, "height": id -> PackedFloat32Array}
var _meshes: Dictionary = {}  # id -> Array de maillages (un par LOD)
var _native: Dictionary = {}  # id -> AABB du LOD le plus fin
var _batches: Dictionary = {}  # id -> MultiMeshInstance3D
var _rig_distance := 1000.0
var _center := Vector2(INF, INF)
var _placed_distance := -1.0
var _placed_lod := -1
var _signature := ""
var _dirty := true
var _frames_since := 1000
var _new_cells := 0
var _shown_cells: Array = []
var _order_key := Vector3i(-99999, 0, 0)
var _order: Array = []
var _height_scales: Dictionary = {}
var _epoch := 0  # change quand la hauteur affichée change (transformations à réécrire)
var _reground_in := -1


func setup(map: MapData, terrain: TerrainBuilder, towns: PackedVector2Array) -> void:
	name = "Fields"
	_map = map
	_terrain = terrain
	_mpu = map.meters_per_px if map != null else 719.0
	visible = false
	_root = Node3D.new()
	_root.name = "Batches"
	add_child(_root)
	var data_dir := OutbuildingLayer.data_dir()
	config = _read_json(data_dir.path_join(CONFIG_FILE))
	stats = {"instances": 0, "nodes": 0, "cells": 0, "plan_ms": 0.0, "build_ms": 0.0, "build_ms_max": 0.0, "plan_ms_max": 0.0}
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	if config.is_empty() or CmdArgs.has("--no-fields"):
		enabled = false
		return
	var mix := _read_json(data_dir.path_join(HbGround.MIX_FILE))
	var landscapes := _read_json(data_dir.path_join(HbGround.AGRI_FILE))
	var biomes := HbGround._load_biomes(data_dir.path_join(HbGround.BIOMES_FILE))
	var agri := HbGround._load_biomes(data_dir.path_join(HbGround.AGRI_MASK_FILE))
	plan = FieldPlan.new()
	plan.setup(config, map, mix, landscapes, biomes, agri, towns)
	enabled = plan.is_ready()
	if enabled:
		_prefetch_models()


## Demande le chargement des glb en arrière-plan : le premier affichage ne bloque pas sur les
## ~75 fichiers (3 niveaux de détail par modèle).
func _prefetch_models() -> void:
	for material: String in config.get("crops", {}):
		var crop: Dictionary = config["crops"][material]
		for entry: Dictionary in crop["variants"] + crop.get("accents", []):
			for level in 3:
				var path := "%s%s_lod%d.glb" % [MODEL_ROOT, entry["id"], level]
				if ResourceLoader.exists(path):
					ResourceLoader.load_threaded_request(path)


## Facteur de hauteur d'un modèle (`height_scale` de sa culture : champs plus bas et plus denses).
func _height_scale(id: String) -> float:
	if _height_scales.is_empty():
		for material: String in config.get("crops", {}):
			var crop: Dictionary = config["crops"][material]
			for entry: Dictionary in crop["variants"]:
				_height_scales[str(entry["id"])] = float(crop.get("height_scale", 1.0))
	return float(_height_scales.get(id, 1.0))


func _on_chunk_surface_changed(_index: int) -> void:
	if _terrain != null and _terrain.rescaling_vertical:
		return
	if visible and not _shown_cells.is_empty():
		_reground_in = 20


static func _read_json(path: String) -> Dictionary:
	var text := FileAccess.get_file_as_string(path)
	var parsed: Variant = JSON.parse_string(text) if not text.is_empty() else null
	return parsed as Dictionary if parsed is Dictionary else {}


func _render(key: String, fallback: Variant) -> Variant:
	return (config.get("render", {}) as Dictionary).get(key, fallback)


func view_range() -> float:
	return float(_render("view_range_units", 38.0))


func load_radius() -> float:
	return clampf(float(_render("load_factor", 1.6)) * _rig_distance, float(_render("load_min_units", 6.0)), float(_render("load_max_units", 60.0)))


func lod_of(rig_distance: float) -> int:
	var distances: Array = _render("lod_distances", [7.0, 18.0])
	if rig_distance < float(distances[0]):
		return 0
	return 1 if rig_distance < float(distances[1]) else 2


# --- Maillages -------------------------------------------------------------------------------


func _mesh_of(id: String, lod: int) -> Mesh:
	if not _meshes.has(id):
		var list: Array = []
		for level in 3:
			var mesh := OutbuildingLayer._load_glb_mesh("%s%s_lod%d.glb" % [MODEL_ROOT, id, level])
			if mesh != null:
				DnCampaignModels.brighten(mesh, {"albedo_gain": float(_render("albedo_gain", 2.2))})
				list.append(mesh)
		_meshes[id] = list
		if not list.is_empty():
			_native[id] = (list[0] as Mesh).get_aabb()
	var list: Array = _meshes[id]
	return null if list.is_empty() else list[mini(lod, list.size() - 1)]


## Modèles disponibles parmi `ids` (les autres sont ignorés sans bruit).
func has_model(id: String) -> bool:
	return _mesh_of(id, 0) != null


# --- Vue -------------------------------------------------------------------------------------


func update_view(rig_distance: float) -> void:
	if not enabled:
		visible = false
		return
	_rig_distance = rig_distance
	var shown := force_active or rig_distance < view_range()
	if shown != visible:
		visible = shown
	if not shown:
		return
	var focus := _camera_ground()
	var radius := load_radius()
	_plan_cells(focus, radius)
	_frames_since += 1
	var lod := lod_of(rig_distance)
	var signature := "%.4f/%.4f" % [MapData.vertical_scale(), MapData.relief_gain()]
	var moved := _center.x == INF or focus.distance_to(_center) > 0.3 * radius
	var zoomed := absf(rig_distance - _placed_distance) > 0.06 * maxf(_placed_distance, 0.1)
	if signature != _signature and _placed_lod >= 0:
		_epoch += 1
		_dirty = true
	if _reground_in >= 0:
		_reground_in -= 1
		if _reground_in < 0:
			_epoch += 1
			_dirty = true
	if moved or zoomed or lod != _placed_lod:
		_dirty = true
	if _new_cells > 0:
		_dirty = true
	if _dirty and _frames_since >= int(_render("rebuild_period_frames", 12)) and _cells_pending(focus, radius) == 0:
		rebuild(focus)
	elif _dirty and _frames_since >= 4 * int(_render("rebuild_period_frames", 12)):
		rebuild(focus)


func _camera_ground() -> Vector2:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return _center if _center.x != INF else Vector2.ZERO
	var origin := camera.global_position
	var forward := -camera.global_transform.basis.z
	if forward.y < -0.05:
		origin += forward * (origin.y / -forward.y)
	return Vector2(origin.x, origin.z)


## Cellules du disque (centre, rayon), des plus proches aux plus lointaines : [distance, clé].
func _cells_in(focus: Vector2, radius: float) -> Array:
	var cell := float(_render("cell_px", 2.0))
	var key := Vector3i(floori(focus.x / cell), floori(focus.y / cell), roundi(radius * 4.0))
	if key == _order_key:
		return _order
	var found: Array = []
	for cy in range(floori((focus.y - radius) / cell), floori((focus.y + radius) / cell) + 1):
		for cx in range(floori((focus.x - radius) / cell), floori((focus.x + radius) / cell) + 1):
			var rect := Rect2(cx * cell, cy * cell, cell, cell)
			var nearest := Vector2(clampf(focus.x, rect.position.x, rect.end.x), clampf(focus.y, rect.position.y, rect.end.y))
			var d := nearest.distance_to(focus)
			if d <= radius:
				found.append([d, Vector2i(cx, cy)])
	found.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	_order_key = key
	_order = found
	return found


func _cells_pending(focus: Vector2, radius: float) -> int:
	var pending := 0
	for entry: Array in _cells_in(focus, radius):
		if not _cells.has(entry[1]):
			pending += 1
	return pending


## Planifie les cellules manquantes, les plus proches d'abord, sous le budget de ms.
func _plan_cells(focus: Vector2, radius: float, budget_ms: float = -1.0) -> void:
	var budget := float(_render("plan_budget_ms", 3.0)) if budget_ms < 0.0 else budget_ms
	var start := Time.get_ticks_usec()
	for entry: Array in _cells_in(focus, radius):
		var key: Vector2i = entry[1]
		if _cells.has(key):
			continue
		var t0 := Time.get_ticks_usec()
		_cells[key] = _make_cell(plan.plan_cell(key))
		_new_cells += 1
		stats["plan_ms_max"] = maxf(float(stats["plan_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)
		if budget >= 0.0 and (Time.get_ticks_usec() - start) / 1000.0 > budget:
			break
	stats["plan_ms"] = (Time.get_ticks_usec() - start) / 1000.0
	stats["cells"] = _cells.size()


func _make_cell(items: Dictionary) -> Dictionary:
	var heights := {}
	for id: String in items:
		var buffer: PackedFloat32Array = items[id]
		var h := PackedFloat32Array()
		h.resize(buffer.size() / FieldPlan.STRIDE)
		for n in h.size():
			h[n] = _map.height_m_at(buffer[n * FieldPlan.STRIDE], buffer[n * FieldPlan.STRIDE + 1])
		heights[id] = h
	var cell := {"items": items, "height": heights, "tf": {}, "epoch": -1}
	for id: String in items:
		if _mesh_of(id, 0) != null:
			_transforms(cell, id)
	return cell


## Tampons de transformations (12 floats par instance) de la cellule, recalculés quand la hauteur
## affichée a changé (échelle verticale, relief) depuis leur écriture.
func _transforms(cell: Dictionary, id: String) -> PackedFloat32Array:
	var cached: Dictionary = cell["tf"]
	if int(cell["epoch"]) == _epoch and cached.has(id):
		return cached[id]
	if int(cell["epoch"]) != _epoch:
		cached.clear()
		cell["epoch"] = _epoch
	var items: PackedFloat32Array = cell["items"][id]
	var heights: PackedFloat32Array = cell["height"][id]
	var aabb: AABB = _native[id]
	var native_width := maxf(maxf(aabb.size.x, aabb.size.z), 0.01)
	var offset := Transform3D(Basis.IDENTITY, Vector3(-(aabb.position.x + aabb.size.x * 0.5), -aabb.position.y, -(aabb.position.z + aabb.size.z * 0.5)))
	var hscale := _height_scale(id)
	var sink := float(_render("sink_m", 6.0)) / _mpu * MapData.vertical_scale()
	var buffer := PackedFloat32Array()
	buffer.resize(heights.size() * MapInstancing.TRANSFORM_FLOATS)
	for n in heights.size():
		var base := n * FieldPlan.STRIDE
		var x := items[base]
		var z := items[base + 1]
		var scale := items[base + 3] / native_width / _mpu
		var y := maxf(MapData.display_height(heights[n], x, z), 0.0) - sink
		var basis := Basis(Vector3.UP, items[base + 2]).scaled(Vector3(scale, scale * hscale, scale))
		MapInstancing.write_transform(buffer, n * MapInstancing.TRANSFORM_FLOATS, Transform3D(basis, Vector3(x, y, z)) * offset)
	cached[id] = buffer
	return buffer


## Termine tout de suite la planification et la construction autour de `center` (tests, captures).
func flush(center: Variant = null) -> void:
	if not enabled or not (visible or force_active):
		return
	var focus: Vector2 = center if center is Vector2 else _camera_ground()
	_plan_cells(focus, load_radius(), 1.0e9)
	rebuild(focus)


## Reconstruit les MultiMesh autour de `focus`.
func rebuild(focus: Vector2) -> void:
	var t0 := Time.get_ticks_usec()
	_dirty = false
	_new_cells = 0
	_frames_since = 0
	_center = focus
	_placed_distance = _rig_distance
	_signature = "%.4f/%.4f" % [MapData.vertical_scale(), MapData.relief_gain()]
	var lod := lod_of(_rig_distance)
	_placed_lod = lod
	var cap := int(_render("max_instances", 16000))
	var total := 0
	var selected: Array = []
	for entry: Array in _cells_in(focus, load_radius()):
		var cell: Variant = _cells.get(entry[1])
		if cell == null:
			continue
		var count := 0
		for id: String in (cell as Dictionary)["items"]:
			count += (cell["items"][id] as PackedFloat32Array).size() / FieldPlan.STRIDE
		if count == 0:
			continue
		if total + count > cap:
			break
		total += count
		selected.append(cell)
	_shown_cells = selected
	var ids := {}
	for cell: Dictionary in selected:
		for id: String in cell["items"]:
			ids[id] = (ids.get(id, 0) as int) + (cell["items"][id] as PackedFloat32Array).size() / FieldPlan.STRIDE
	for id: String in _batches.keys():
		if not ids.has(id):
			(_batches[id] as Node).queue_free()
			_batches.erase(id)
	var shadows := _rig_distance < float(_render("shadow_range_units", 9.0))
	for id: String in ids:
		var mesh := _mesh_of(id, lod)
		if mesh == null:
			continue
		var buffer := PackedFloat32Array()
		var count: int = ids[id]
		for cell: Dictionary in selected:
			if (cell["items"] as Dictionary).has(id):
				buffer.append_array(_transforms(cell, id))
		var mmi: MultiMeshInstance3D = _batches.get(id)
		if mmi == null:
			mmi = MultiMeshInstance3D.new()
			mmi.name = id.replace("/", "_")
			_root.add_child(mmi)
			_batches[id] = mmi
		mmi.multimesh = MapInstancing.make(mesh, count, false, false, buffer)
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stats["instances"] = total
	stats["nodes"] = _batches.size()
	stats["build_ms"] = (Time.get_ticks_usec() - t0) / 1000.0
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), float(stats["build_ms"]))


# --- Lecture pour les tests ------------------------------------------------------------------


func instance_count() -> int:
	return int(stats.get("instances", 0))


func node_count() -> int:
	return _batches.size()


## Identifiants de modèles affichés.
func shown_models() -> Array:
	return _batches.keys()
