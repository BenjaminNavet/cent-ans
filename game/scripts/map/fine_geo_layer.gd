class_name FineGeoLayer
extends Node3D

## Rendu de près de l'hydrographie fine, des routes drapées et des ancrages (lot ZG5b, ADR 0036).
## Enfant de `RiversRenderer` (`Rivers/FineGeo`), actif seulement avec la pyramide de relief et
## les données ZG5a en cache (`FineGeoStore`), et en deçà du palier « comté » (poids du palier
## près de `ZoomTiers`) : aucun coût en vue stratégique.
##
## - Tuiles E2 (64 unités) autour du point visé : chargées dans des fils (`FineGeoStore`), puis
##   maillées dans des fils (`FineRibbonJob`) ; un `MeshInstance3D` fleuves + un routes par tuile,
##   cache LRU, remaillage quand les pages de relief de la tuile changent.
## - Lit : pages du quadtree creusées avant téléversement (`FineBedCarver`), l'ancien lit
##   (`river_bed.png`, tuiles de relief fin) n'est plus creusé.
## - Fondu avec l'ancien rendu : disque `fine_zone` (centre = point visé, rayon = tuiles prêtes)
##   × poids du palier ; les anciens rubans de fleuves, berges et rubans de routes s'effacent
##   dedans, les nouveaux n'existent que dedans.
## - Ancrages : colonies et hameaux (`SettlementLayer.apply_fine_anchors`), ponts historiques
##   et génériques (`RiverCrossings.set_fine_anchors`), ponts-portes recalculés sur le fleuve fin.
## - Échelle verticale : hauteurs en mètres dans les maillages, `height_scale` relu à chaque
##   image (`MapData.vertical_scale()`, dynamique au lot ZG4) : rien à remailler.

const RIVER_SHADER := preload("res://shaders/river_fine.gdshader")
const ROAD_SHADER := preload("res://shaders/road_fine.gdshader")
const WALLED := ["city", "town", "castle"]
const GATE_MIN_WIDTH := 0.3
## Lot ZG4b : hauteur (m) du tablier des ponts-portes au-dessus de l'eau (pas d'ancrage `z_deck`
## pour eux) ; ouvrages à l'échelle réelle comme les ponts ancrés (`RiverCrossings.FINE_SCALE`).
const GATE_DECK_RISE_M := 7.0

## Rayon (unités) des tuiles voulues autour du point visé : distance caméra × facteur, borné.
@export var radius_factor: float = 1.3
@export var min_radius: float = 40.0
@export var max_radius: float = 170.0
## Tuiles maillées gardées en cache (visibles ou non).
@export var max_built_tiles: int = 40
@export var max_build_jobs: int = 2
@export var max_installs_per_frame: int = 2
## Délai sans nouvelle page avant de remailler une tuile (ms).
@export var surface_settle_ms: int = 600
## Poids du palier près au-dessus duquel les ponts passent sur leurs ancrages fins.
@export var bridge_switch_weight: float = 0.5

var map_data: MapData
var terrain: TerrainBuilder
var rivers: RiversRenderer
var settlements: SettlementLayer
var roads: RoadRenderer
var tiers: ZoomTiers
var store: FineGeoStore
var carver: FineBedCarver
var enabled := false
var river_material: ShaderMaterial
var road_material: ShaderMaterial
var stats: Dictionary = {}

## Clé de tuile → {node: Node3D, river: MeshInstance3D, road: MeshInstance3D, used: int, dirty: int, gates: Array}
var _built: Dictionary = {}
## Clé → {task, job}
var _jobs: Dictionary = {}
var _ready_jobs: Array = []
var _wanted: Array[int] = []
var _frame := 0
var _weight := -1.0
var _scale := -1.0
var _zone := Vector4(0.0, 0.0, -1.0, 0.0)
var _towns := PackedVector3Array()
var _zones := PackedVector3Array()
## SZ2b : zones personnalisées des villes 1:1 (VH) et ids de leurs colonies.
var _open_zones := PackedVector3Array()
var _open_zone_cities: Array[PackedStringArray] = []
var _cover_open := -1.0
var _zone_open := -1.0
var _bridges_fine := false
var _update_us_total := 0
var _update_us_max := 0
var _updates := 0
var _build_ms_total := 0.0
var _install_ms_max := 0.0
var _install_mesh_ms_max := 0.0
var _install_gates_ms_max := 0.0
var _builds := 0
## Préréglage de qualité (PF1) : ordre minimal des cours d'eau, rayon, parcellaire (0-2).
var _min_order := 3
var _radius_scale := 1.0
var _parcels := 2


## Vrai si le rendu fin est actif (pyramide + données ZG5a en cache).
func setup(rivers_renderer: RiversRenderer, settlement_layer: SettlementLayer) -> bool:
	rivers = rivers_renderer
	map_data = rivers.map_data
	terrain = rivers.terrain
	settlements = settlement_layer
	tiers = ZoomTiers.load_default()
	enabled = false
	if terrain == null or terrain.quadtree == null:
		return false
	store = FineGeoStore.new()
	var relief_root: String = preload("res://scripts/map/map_paths.gd").relief_root_for(map_data.map_dir)  # ZG7b
	if not store.load_from(map_data.map_dir, relief_root) or not store.available(CafvTile.LAYER_RIVERS):
		store = null
		return false
	enabled = true
	carver = FineBedCarver.new(store, map_data.meters_per_px, terrain.pyramid.height_min_m, terrain.pyramid.height_range_m)
	# SZ2b : le lit est creusé aussi sous les emprises des colonies (au palier près, les
	# maquettes cèdent la place aux villes 1:1 de ZG6, dont le couloir de fleuve attend l'eau) ;
	# les rubans y sont effacés tant que les maquettes sont affichées (`cover_open`).
	carver.covers = PackedVector4Array()
	terrain.quadtree.page_filter = carver
	for zone in rivers.zones:
		var c: Vector2 = zone["px"]
		var circle := Vector3(c.x, c.y, float(zone["radius_px"]))
		var cities := _landmark_cities_in(circle)
		if not cities.is_empty():
			# SZ2b : ville 1:1 (VH, ADR 0078) : le fleuve fin y est l'eau affichée (son plan
			# ne dessine que les couloirs), lit creusé, rubans au fondu de la maquette L1/L2.
			_open_zones.append(circle)
			_open_zone_cities.append(cities)
			continue
		_zones.append(circle)
		# ZG7a : pas de lit creusé dans les zones personnalisées (villes emblématiques) : les
		# rubans n'y sont pas dessinés, le lit restait une tranchée vide (Tamise à -7,8 m sous
		# des rives à 2-4 m à Londres). Contrat de `river_styles.json` : le rendu générique
		# (eau, lit, berges, ponts) se retire dans ces cercles.
		carver.covers.append(Vector4(c.x, c.y, float(zone["radius_px"]), -1.0))
	_collect_towns()
	river_material = ShaderMaterial.new()
	river_material.shader = RIVER_SHADER
	river_material.render_priority = 1
	rivers.apply_fine_display(river_material)
	road_material = ShaderMaterial.new()
	road_material.shader = ROAD_SHADER
	if CmdArgs.has("--fine-debug"):
		river_material.set_shader_parameter("debug_flat", true)
		road_material.set_shader_parameter("debug_flat", true)
	if settlements != null:
		settlements.apply_fine_anchors(store)
	if rivers.crossings != null:
		rivers.crossings.set_fine_anchors(store.crossings)
	if not terrain.surface_rect_changed.is_connected(_on_surface_rect_changed):
		terrain.surface_rect_changed.connect(_on_surface_rect_changed)
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	stats = store.stats.duplicate()
	print("FineGeoLayer: %s" % JSON.stringify(stats))
	return true


## PF1 : préréglages de qualité, déduits du budget de nœuds du quadtree (`relief_items`) :
## Basse (≤ 350) : grands cours d'eau seulement (rang ≥ 5), rayon × 0,6, pas de parcellaire ;
## Moyenne (< 700) : rang ≥ 4, rayon × 0,8, parcellaire simple (sans enclos du bocage) ;
## Haute, Ultra : tout.
func apply_render_quality(p: Dictionary) -> void:
	var items := int(p.get("relief_items", 700))
	var level := 0 if items <= 350 else (1 if items < 700 else 2)
	var min_order: int = [5, 4, 3][level]
	_radius_scale = [0.6, 0.8, 1.0][level]
	_parcels = level
	if terrain != null and terrain.material != null:
		terrain.material.set_shader_parameter("fp_quality", _parcels)
	if min_order != _min_order:
		_min_order = min_order
		var now := Time.get_ticks_msec() - surface_settle_ms
		for entry: Dictionary in _built.values():
			entry["dirty"] = now


## Routes (lot C6) : leurs rubans de près s'effacent dans le disque fin.
func attach_roads(road_renderer: RoadRenderer) -> void:
	roads = road_renderer


func _exit_tree() -> void:
	for entry: Dictionary in _jobs.values():
		WorkerThreadPool.wait_for_task_completion(entry["task"])
	_jobs.clear()


func _collect_towns() -> void:
	_towns.clear()
	if settlements == null or settlements.data == null:
		return
	for i in settlements.data.settlements.size():
		var entry: Dictionary = settlements.data.settlements[i]
		var kind := str(entry.get("kind", ""))
		if kind != "city" and kind != "town":
			continue
		var p := settlements.model_px(i)
		_towns.append(Vector3(p.x, p.y, settlements.model_radius(i) * 1.3 + 0.05))


# --- Mise à jour par image -------------------------------------------------------------


## Appelé chaque image par `RiversRenderer.update_visibility`.
func update_view(camera_distance: float) -> void:
	if not enabled:
		return
	var t0 := Time.get_ticks_usec()
	_frame += 1
	store.poll()
	var scale := MapData.vertical_scale()
	if scale != _scale:
		# Rubans : `campaign_vertical_scale` (paramètre global, ZG4) ; ponts-portes recalés ici.
		_scale = scale
		_reground_gates()
	var weight := tiers.near_weight(camera_distance)
	if weight != _weight:
		_weight = weight
		visible = weight > 0.001
		river_material.set_shader_parameter("fine_weight", weight)
		road_material.set_shader_parameter("fine_weight", weight)
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if weight <= 0.001 or camera == null:
		_set_zone(Vector4(0.0, 0.0, -1.0, 0.0))
		_switch_bridges(false)
		_collect_jobs()
		_note_update(t0)
		return
	var focus := _focus(camera, camera_distance)
	var radius := clampf(camera_distance * radius_factor, min_radius, max_radius) * _radius_scale
	_wanted = _wanted_tiles(focus, radius)
	for key in _wanted:
		store.request(CafvTile.LAYER_RIVERS, key & 0xFFF, key >> 12)
		store.request(CafvTile.LAYER_ROADS, key & 0xFFF, key >> 12)
	_collect_jobs()
	_start_jobs()
	_install_ready()
	var covered := _covered_radius(focus, radius)
	_set_zone(Vector4(focus.x, focus.y, covered, weight))
	for key: int in _built:
		var entry: Dictionary = _built[key]
		var wanted := _wanted.has(key)
		(entry["node"] as Node3D).visible = wanted
		if wanted:
			entry["used"] = _frame
	_switch_bridges(weight >= bridge_switch_weight)
	_update_open()
	_evict()
	_note_update(t0)


## SZ2b : ouverture de l'eau dans les emprises (villes 1:1 de ZG6 actives : maquettes masquées)
## et dans les zones des villes 1:1 VH (complément de l'opacité de leur maquette).
func _update_open() -> void:
	var cover_open := 1.0 if settlements != null and settlements.towns != null and settlements.towns.active else 0.0
	var zone_open := 0.0
	var lc: LandmarkCityLayer = settlements.landmark_cities if settlements != null else null
	if lc != null:
		for ids in _open_zone_cities:
			for id in ids:
				zone_open = maxf(zone_open, 1.0 if lc.is_shown(id) else 0.0)
	if cover_open != _cover_open:
		_cover_open = cover_open
		river_material.set_shader_parameter("cover_open", cover_open)
		_show_gates()
	if absf(zone_open - _zone_open) > 0.004:
		_zone_open = zone_open
		river_material.set_shader_parameter("zone_open", zone_open)


## Colonies (ids) dont la ville 1:1 (VH) est dans le cercle.
func _landmark_cities_in(circle: Vector3) -> PackedStringArray:
	var ids := PackedStringArray()
	var lc: LandmarkCityLayer = settlements.landmark_cities if settlements != null else null
	if lc == null or not lc.is_enabled():
		return ids
	for id: String in lc.city_ids():
		var z := lc.zone_of(id)
		if Vector2(z.x, z.y).distance_to(Vector2(circle.x, circle.y)) < circle.z:
			ids.append(id)
	return ids


func _note_update(t0: int) -> void:
	var us := Time.get_ticks_usec() - t0
	_update_us_total += us
	_update_us_max = maxi(_update_us_max, us)
	_updates += 1


## Point visé : intersection de l'axe de la caméra avec le plan à la distance du rig.
static func _focus(camera: Camera3D, distance: float) -> Vector2:
	var forward := -camera.global_transform.basis.z
	var p := camera.global_position + forward * distance
	return Vector2(p.x, p.z)


## Tuiles E2 présentes (fleuves ou routes) coupant le disque, triées par distance.
func _wanted_tiles(focus: Vector2, radius: float) -> Array[int]:
	var result: Array[int] = []
	var order: Array = []
	var t := FineGeoStore.TILE_UNITS
	for row in range(int(floor((focus.y - radius) / t)), int(floor((focus.y + radius) / t)) + 1):
		for col in range(int(floor((focus.x - radius) / t)), int(floor((focus.x + radius) / t)) + 1):
			if col < 0 or row < 0:
				continue
			if not store.has_tile(CafvTile.LAYER_RIVERS, col, row) and not store.has_tile(CafvTile.LAYER_ROADS, col, row):
				continue
			var d := _rect_distance(FineGeoStore.tile_rect(col, row), focus)
			if d < radius:
				order.append([d, FineGeoStore.key_of(col, row)])
	order.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for item in order:
		result.append(item[1])
	return result


static func _rect_distance(rect: Rect2, p: Vector2) -> float:
	var q := Vector2(clampf(p.x, rect.position.x, rect.end.x), clampf(p.y, rect.position.y, rect.end.y))
	return q.distance_to(p)


## Rayon du disque autour du point visé entièrement couvert par des tuiles maillées.
func _covered_radius(focus: Vector2, radius: float) -> float:
	var covered := radius
	for key in _wanted:
		var entry: Dictionary = _built.get(key, {})
		if entry.is_empty() or int(entry.get("version", 0)) == 0:
			covered = minf(covered, _rect_distance(FineGeoStore.tile_rect(key & 0xFFF, key >> 12), focus))
	return covered


func _set_zone(zone: Vector4) -> void:
	if zone.is_equal_approx(_zone):
		return
	_zone = zone
	for m: ShaderMaterial in rivers.old_materials():
		m.set_shader_parameter("fine_zone", zone)
	if terrain.material != null:
		terrain.material.set_shader_parameter("fine_zone", zone)
	if roads != null:
		roads.set_fine_zone(zone)
	river_material.set_shader_parameter("fine_zone", zone)
	road_material.set_shader_parameter("fine_zone", zone)


# --- Maillage --------------------------------------------------------------------------


func _start_jobs() -> void:
	var now := Time.get_ticks_msec()
	for key in _wanted:
		if _jobs.size() >= max_build_jobs:
			return
		if _jobs.has(key):
			continue
		var entry: Dictionary = _built.get(key, {})
		var dirty := int(entry.get("dirty", -1))
		var needs := entry.is_empty() or (dirty >= 0 and now - dirty >= surface_settle_ms)
		if not needs:
			continue
		var col := key & 0xFFF
		var row := key >> 12
		var river_ready := not store.has_tile(CafvTile.LAYER_RIVERS, col, row) or store.is_loaded(CafvTile.LAYER_RIVERS, col, row)
		var road_ready := not store.has_tile(CafvTile.LAYER_ROADS, col, row) or store.is_loaded(CafvTile.LAYER_ROADS, col, row)
		if not river_ready or not road_ready:
			continue
		_start_job(key)


func _start_job(key: int) -> void:
	var col := key & 0xFFF
	var row := key >> 12
	var rect := FineGeoStore.tile_rect(col, row)
	var job := FineRibbonJob.new()
	job.key = key
	job.river_tile = store.get_tile(CafvTile.LAYER_RIVERS, col, row)
	job.road_tile = store.get_tile(CafvTile.LAYER_ROADS, col, row)
	job.snapshot = terrain.quadtree.surface_snapshot(rect.grow(1.0), rect.position)
	job.finest = terrain.quadtree.finest_levels([rect] as Array[Rect2])[0]
	job.snapshot_scale = MapData.vertical_scale()
	job.meters_per_unit = map_data.meters_per_px
	job.min_order = _min_order
	for c in rivers.covers:
		if rect.grow(c.z + 1.0).has_point(Vector2(c.x, c.y)):
			job.covers.append(c)
	for z in _zones:
		if rect.grow(z.z).has_point(Vector2(z.x, z.y)):
			job.zones.append(z)
	for z in _open_zones:
		if rect.grow(z.z).has_point(Vector2(z.x, z.y)):
			job.open_zones.append(z)
	for t in _towns:
		if rect.grow(t.z).has_point(Vector2(t.x, t.y)):
			job.towns.append(t)
	if settlements != null and settlements.data != null:
		for c in job.covers:
			var index := int(c.w)
			if index >= 0 and index < settlements.data.settlements.size():
				job.cover_kinds[index] = str(settlements.data.settlements[index].get("kind", ""))
	if _built.has(key):
		(_built[key] as Dictionary)["dirty"] = -1
	_jobs[key] = {"task": WorkerThreadPool.add_task(job.run, false, "fine ribbons"), "job": job}


func _collect_jobs(block: bool = false) -> void:
	for key: int in _jobs.keys():
		var entry: Dictionary = _jobs[key]
		if not block and not WorkerThreadPool.is_task_completed(entry["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(key)
		_ready_jobs.append(entry["job"])


## Installe les maillages prêts : au moins un par image, puis tant que le budget commun de
## l'image (PB1, `FrameBudget`) le permet, au plus `max_installs_per_frame`.
func _install_ready(limit: int = -1) -> void:
	var count := 0
	var cap := max_installs_per_frame if limit < 0 else limit
	while not _ready_jobs.is_empty() and count < cap and (count == 0 or limit >= 0 or FrameBudget.has_time()):
		var job: FineRibbonJob = _ready_jobs.pop_front()
		_install(job)
		count += 1


func _install(job: FineRibbonJob) -> void:
	var t0 := Time.get_ticks_usec()
	var entry: Dictionary = _built.get(job.key, {})
	if entry.is_empty():
		var node := Node3D.new()
		node.name = "Tile_%d_%d" % [job.key & 0xFFF, job.key >> 12]
		add_child(node)
		var river := MeshInstance3D.new()
		river.name = "Rivers"
		river.material_override = river_material
		river.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.add_child(river)
		var road := MeshInstance3D.new()
		road.name = "Roads"
		road.material_override = road_material
		road.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.add_child(road)
		entry = {"node": node, "river": river, "road": road, "used": _frame, "dirty": -1, "version": 0, "gates": []}
		_built[job.key] = entry
	var t_mesh := Time.get_ticks_usec()
	(entry["river"] as MeshInstance3D).mesh = _mesh(job.river_arrays, job.river_aabb)
	(entry["road"] as MeshInstance3D).mesh = _mesh(job.road_arrays, job.road_aabb)
	_install_mesh_ms_max = maxf(_install_mesh_ms_max, (Time.get_ticks_usec() - t_mesh) / 1000.0)
	entry["version"] = int(entry["version"]) + 1
	entry["finest"] = job.finest
	entry["points"] = job.river_points + job.road_points
	var t_gates := Time.get_ticks_usec()
	_build_gates(entry, job.gates)
	_install_gates_ms_max = maxf(_install_gates_ms_max, (Time.get_ticks_usec() - t_gates) / 1000.0)
	_build_ms_total += job.build_ms
	_builds += 1
	_install_ms_max = maxf(_install_ms_max, (Time.get_ticks_usec() - t0) / 1000.0)


static func _mesh(arrays: Array, aabb: AABB) -> ArrayMesh:
	if arrays.is_empty():
		return null
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.custom_aabb = aabb
	return mesh


func _evict() -> void:
	if _built.size() <= max_built_tiles:
		return
	var keys := _built.keys()
	keys.sort_custom(func(a: int, b: int) -> bool: return int(_built[a]["used"]) < int(_built[b]["used"]))
	for key: int in keys:
		if _built.size() <= max_built_tiles:
			return
		if int(_built[key]["used"]) >= _frame:
			return
		(_built[key]["node"] as Node3D).queue_free()
		_built.erase(key)


## Pages arrivées ou évincées : remaillage d'une tuile seulement si l'étage de page le plus fin
## qui la touche a changé (pas à chaque page du même étage : le LRU des pages tourne sans cesse
## pendant un panoramique).
func _on_surface_rect_changed(rect: Rect2) -> void:
	var t_probe := Time.get_ticks_usec()
	_mark_dirty_tiles(rect)
	PerfProbe.add("qt/l_fine", Time.get_ticks_usec() - t_probe)  # RS-K : écouteur, dans `qt/*`


func _mark_dirty_tiles(rect: Rect2) -> void:
	var now := Time.get_ticks_msec()
	# ZG7a : étages les plus fins de toutes les tuiles touchées en un parcours des pages (un
	# instantané par tuile coûtait jusqu'à 9 ms par page arrivée sous charge).
	var keys: Array[int] = []
	var rects: Array[Rect2] = []
	for key: int in _built:
		var tile_rect := FineGeoStore.tile_rect(key & 0xFFF, key >> 12)
		if tile_rect.intersects(rect):
			keys.append(key)
			rects.append(tile_rect)
	if keys.is_empty():
		return
	var levels := terrain.quadtree.finest_levels(rects)
	for i in keys.size():
		var entry: Dictionary = _built[keys[i]]
		var finest := levels[i]
		var built_at := int(entry.get("finest", -1))
		# E5-E7 (zones de détail) : lit déjà creusé dans leurs pages, écart de surface faible ; on
		# ne remaille que jusqu'à E4.
		if finest != built_at and mini(finest, 4) != mini(built_at, 4):
			entry["dirty"] = now


# --- Ponts ----------------------------------------------------------------------------


## Ponts-portes aux bords des emprises des colonies, sur le fleuve fin.
func _build_gates(entry: Dictionary, gates: Array[Dictionary]) -> void:
	for node: Node3D in entry["gates"]:
		node.queue_free()
	var nodes: Array[Node3D] = []
	for gate in gates:
		var index: int = gate["cover"]
		var instance := MeshInstance3D.new()
		instance.name = "Gate_%d" % index
		# ZG4b : échelle réelle (ZG5b les laissait à l'échelle exagérée de la carte, ×2 en hauteur et
		# en largeur de tablier, culées de 50-100 m) : maillage d'une portée `width / FINE_SCALE`
		# réduit de `FINE_SCALE`, hauteur recalculée à chaque échelle verticale (`_ground_gate`).
		# ZG7a : type, largeur et tableaux préparés dans le fil du maillage (`FineRibbonJob`).
		var mesh_width: float = gate["mesh_width"]
		var key: String = gate["mesh_key"]
		instance.mesh = BridgeMeshes.cached(key)
		if instance.mesh == null:
			instance.mesh = BridgeMeshes.build_from(key, gate["surfaces"])
		var dir: Vector2 = gate["dir"]
		var across := Vector3(-dir.y, 0.0, dir.x)
		var along := across.cross(Vector3.UP)
		var p: Vector2 = gate["px"]
		instance.position = Vector3(p.x, 0.0, p.y)
		instance.set_meta("z_m", float(gate["z"]))
		instance.set_meta("across", across)
		instance.set_meta("along", along)
		instance.set_meta("mesh_width", mesh_width)
		instance.set_meta("deck_scale", BridgeMeshes.fine_deck_scale(str(gate["structure"]), mesh_width, map_data.meters_per_px, RiverCrossings.FINE_SCALE))
		instance.visibility_range_end = RiverCrossings.VISIBILITY_RANGE
		(entry["node"] as Node3D).add_child(instance)
		instance.visible = _bridges_fine and _cover_open < 0.5
		nodes.append(instance)
		_ground_gate(instance)
	entry["gates"] = nodes


func _ground_gate(instance: MeshInstance3D) -> void:
	var vs := MapData.vertical_scale()
	var k := RiverCrossings.FINE_SCALE
	var deck_top := (0.05 + 0.02 * float(instance.get_meta("mesh_width", 1.0))) * k
	var k_h := clampf(GATE_DECK_RISE_M * vs / maxf(deck_top, 1e-4), 0.3, 12.0)
	# ZG7a : tablier à sa largeur réelle (`BridgeMeshes.fine_deck_scale`), pas × `FINE_SCALE`.
	var k_along := float(instance.get_meta("deck_scale", k))
	instance.transform.basis = Basis(instance.get_meta("across", Vector3.RIGHT) * k, Vector3.UP * k * k_h, instance.get_meta("along", Vector3.BACK) * k_along)
	instance.position.y = maxf(MapData.display_height(float(instance.get_meta("z_m", 0.0)), instance.position.x, instance.position.z), 0.0)


func _reground_gates() -> void:
	for entry: Dictionary in _built.values():
		for node: MeshInstance3D in entry["gates"]:
			_ground_gate(node)


func _switch_bridges(fine: bool) -> void:
	if fine == _bridges_fine:
		return
	_bridges_fine = fine
	if rivers.crossings != null:
		rivers.crossings.set_fine_mode(fine)
		rivers.crossings.set_gates_hidden(fine)
	_show_gates()


## Ponts-portes : au palier près, tant que les maquettes (et donc leurs murs) sont affichées.
func _show_gates() -> void:
	var shown := _bridges_fine and _cover_open < 0.5
	for entry: Dictionary in _built.values():
		for node: Node3D in entry["gates"]:
			node.visible = shown


# --- Captures, tests, mesures -----------------------------------------------------------


## Charge et maille tout de suite les tuiles voulues (captures, tests).
func flush(camera_distance: float) -> void:
	if not enabled:
		return
	for _pass in 3:
		update_view(camera_distance)
		store.wait_all()
		for key in _wanted:
			if not _built.has(key) and not _jobs.has(key):
				_start_job(key)
		_collect_jobs(true)
		_install_ready(1 << 20)
	update_view(camera_distance)
	if rivers.crossings != null:
		rivers.crossings.pump_reshape(true)  # ZG4b : bascule étalée des ponts terminée
	print("FineGeoLayer: flush %s" % JSON.stringify({"tiles": _built.size(), "wanted": _wanted.size(), "zone": [_zone.x, _zone.y, _zone.z, _zone.w], "river_vertices": river_vertex_count(), "road_vertices": road_vertex_count()}.merged(perf_stats())))


func built_tile_count() -> int:
	return _built.size()


func river_vertex_count() -> int:
	var total := 0
	for entry: Dictionary in _built.values():
		var mesh: ArrayMesh = (entry["river"] as MeshInstance3D).mesh
		if mesh != null:
			total += mesh.surface_get_array_len(0)
	return total


func road_vertex_count() -> int:
	var total := 0
	for entry: Dictionary in _built.values():
		var mesh: ArrayMesh = (entry["road"] as MeshInstance3D).mesh
		if mesh != null:
			total += mesh.surface_get_array_len(0)
	return total


func zone() -> Vector4:
	return _zone


func perf_stats() -> Dictionary:
	return {
		"fine_update_ms_avg": snappedf(_update_us_total / 1000.0 / maxf(_updates, 1), 0.001),
		"fine_update_ms_max": snappedf(_update_us_max / 1000.0, 0.01),
		"fine_tiles_built": _built.size(),
		"fine_builds": _builds,
		"fine_build_ms_avg": snappedf(_build_ms_total / maxf(_builds, 1), 0.1),
		"fine_install_ms_max": snappedf(_install_ms_max, 0.01),
		"fine_install_mesh_ms_max": snappedf(_install_mesh_ms_max, 0.01),
		"fine_install_gates_ms_max": snappedf(_install_gates_ms_max, 0.01),
		"carved_pages": int(carver.stats.get("pages", 0)) if carver != null else 0,
		"bridge_reshape_ms_max": snappedf(rivers.crossings.reshape_ms_max, 0.01) if rivers != null and rivers.crossings != null else 0.0,
	}
