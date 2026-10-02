class_name OutbuildingLayer
extends Node3D

## Lot TB3 (ADR 0153) : bâtiments hors les murs de la carte de campagne (ferme, moulin, vignoble,
## mine, saline, abbaye, marché, port), posés sur le terroir autour des colonies selon les
## bâtiments construits que le moteur expose (`CampaignSim.get_settlements_live`, sinon
## `settlement_detail`), au niveau 1 à 3 donné par `data/map/building_models.json`. Rendu
## seulement : aucune règle de jeu.
## - Maquettes : `game/assets/models/outbuildings/` (une surface `Building` par maquette, même
##   matériau atlas que les villes 1:1, `TownBuilder.material`).
## - Rendu : un `MultiMesh` par maillage pour tout le voisinage de la caméra (au plus un appel de
##   dessin par maquette distincte), reconstruit par petites étapes quand la caméra s'éloigne du
##   centre du voisinage ou que l'état de la simulation change. Hauteurs de base en mètres, posées
##   par le shader (`campaign_display_height`).
## - Échelle réelle (ADR 0138) ; `render.exaggeration` des données permet de grossir de loin.
## - Les autres pièces de la croissance (faubourgs, enceinte, lot TB3 point 4) passent par le même
##   rendu : `TownGrowth` fournit ses instances par colonie.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const DATA_FILE := "map/building_models.json"
const MODEL_DIR := "res://assets/models/outbuildings/"
const WORKSITE_FAMILY := "worksite"
const WORKSITE_MODEL := "worksite_1"
## Écarts d'angle essayés autour du secteur d'une famille (en tiers de secteur), du plus proche
## au plus lointain.
const ANGLE_STEPS: Array[int] = [0, -1, 1, -2, 2, -4, 4, -6, 6, -9, 9, -12, 12]
const SHORE_DIRECTIONS := 16
## Temps (µs) donné à la reconstruction du voisinage par image.
const STEP_BUDGET_USEC := 1500
## Images d'attente avant de recaler les hauteurs après un changement de surface du relief.
const REGROUND_DELAY := 20
## Côté (unités) des cases de la grille de voisinage des colonies (emprises voisines à éviter).
const GRID_UNITS := 8.0
## Demi-angle (rad) laissé libre autour de la route de chaque porte.
const GATE_CORRIDOR := 0.2

var config: Dictionary = {}
var manifest: Dictionary = {}
var stats: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false

var _layer: SettlementLayer
var _map: MapData
var _terrain: TerrainBuilder
var _data: SettlementData
var _mpu := 719.0
var _sim: Object = null
var _revision := -2
var _states: Dictionary = {}  # id de colonie → {buildings, fortification, building}
var _resources: Dictionary = {}  # province → Array d'ids de ressource
var _provinces: Dictionary = {}  # province → {population, devastation, besieged, constructing}
var _baseline: Dictionary = {}  # province → population de 1337 (données)
var _soot: Dictionary = {}  # indice de colonie → suie 0-1
var _anchors: Dictionary = {}  # "i|famille" → {px, yaw} ou {} (aucun site)
var _root: Node3D
var _batches: Dictionary = {}  # clé de maillage → MultiMeshInstance3D
var _instances: Array = []  # instances affichées
var _center := Vector2(INF, INF)
var _dirty := true
var _pending: Array = []  # indices de colonies à traiter (reconstruction en cours)
var _building: Array = []  # instances de la reconstruction en cours
var _build_center := Vector2.ZERO
var _reground_in := -1
var _scale := 1.0
var _vertical_scale := -1.0
var _shadows := true
var _meshes: Dictionary = {}
var _family_order: Array = []
var _grid: Dictionary = {}  # Vector2i → PackedInt32Array (indices de colonies)


## Dossier `data/` du jeu : celui de l'autoload `MapPaths`, sinon le dossier par défaut (scripts
## de test lancés par `--script`, où l'identifiant de l'autoload n'est pas compilé).
static func data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var paths := tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	return str(paths.get("data_dir")) if paths != null else MAP_PATHS.default_data_dir()


## Charge la correspondance (`data/map/building_models.json`) et le manifeste des maquettes.
static func load_config(data_dir: String) -> Dictionary:
	var path := data_dir.path_join(DATA_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


static func load_manifest() -> Dictionary:
	var path := MODEL_DIR + "manifest.json"
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


## Vrai si la condition `when` d'un niveau est remplie : chaque groupe `require` compte au moins
## `min` bâtiments construits, et la province produit une des `resources` (si la liste existe).
static func level_reached(when: Dictionary, buildings: Array, resources: Array) -> bool:
	for group: Dictionary in when.get("require", []):
		var built := 0
		for id in group.get("of", []):
			if buildings.has(id):
				built += 1
		if built < int(group.get("min", 1)):
			return false
	var wanted: Array = when.get("resources", [])
	if wanted.is_empty():
		return true
	for id in wanted:
		if resources.has(id):
			return true
	return false


## Niveau (0 à 3) d'une famille pour des bâtiments construits et les ressources de la province.
static func family_level(family: Dictionary, buildings: Array, resources: Array) -> int:
	var best := 0
	for level: Dictionary in family.get("levels", []):
		if level_reached(level.get("when", {}), buildings, resources):
			best = maxi(best, int(level.get("level", 0)))
	return best


## Maquettes à poser autour d'une colonie : `[{family, level, model, site, slot}]`, une par
## famille au plus haut niveau atteint, au plus `render.max_per_settlement` (les plus hauts
## niveaux d'abord, puis l'ordre des familles dans les données).
static func models_for(cfg: Dictionary, buildings: Array, resources: Array) -> Array:
	var out: Array = []
	var families: Dictionary = cfg.get("families", {})
	var slot := 0
	for family_name: String in families:
		var family: Dictionary = families[family_name]
		var level := family_level(family, buildings, resources)
		if level > 0:
			var model := ""
			for entry: Dictionary in family["levels"]:
				if int(entry["level"]) == level:
					model = str(entry["model"])
			out.append({"family": family_name, "level": level, "model": model, "site": str(family.get("site", "field")), "slot": slot})
		slot += 1
	var cap := int((cfg.get("render", {}) as Dictionary).get("max_per_settlement", 4))
	if out.size() > cap:
		out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			if int(a["level"]) != int(b["level"]):
				return int(a["level"]) > int(b["level"])
			return int(a["slot"]) < int(b["slot"]))
		out.resize(cap)
		out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["slot"]) < int(b["slot"]))
	return out


## Grossissement à la distance du rig `rig_distance` : 1 sous `full_size_below`, `max` au-delà de
## `max_above`, fondu sur le logarithme de la distance entre les deux.
static func exaggeration(cfg: Dictionary, rig_distance: float) -> float:
	var e: Dictionary = (cfg.get("render", {}) as Dictionary).get("exaggeration", {})
	var top := float(e.get("max", 1.0))
	if top <= 1.0:
		return 1.0
	var lo := maxf(float(e.get("full_size_below", 12.0)), 1e-3)
	var hi := maxf(float(e.get("max_above", 60.0)), lo * 1.01)
	var t := smoothstep(log(lo), log(hi), log(maxf(rig_distance, 1e-3)))
	return pow(top, t)


func setup(layer: SettlementLayer, map: MapData, terrain: TerrainBuilder, data: SettlementData, meters_per_unit: float = 719.0) -> void:
	_layer = layer
	_map = map
	_terrain = terrain
	_data = data
	_mpu = meters_per_unit
	config = load_config(data_dir())
	manifest = load_manifest()
	_family_order = (config.get("families", {}) as Dictionary).keys()
	_grid.clear()
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		var cell := Vector2i(floori(px.x / GRID_UNITS), floori(px.y / GRID_UNITS))
		if not _grid.has(cell):
			_grid[cell] = PackedInt32Array()
		(_grid[cell] as PackedInt32Array).append(i)
	_root = Node3D.new()
	_root.name = "Batches"
	add_child(_root)
	enabled = enabled and not config.is_empty() and not OS.get_cmdline_user_args().has("--no-tb3")
	visible = false
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	stats = {"instances": 0, "nodes": 0, "settlements": 0, "build_ms": 0.0}


## Relit l'état de la simulation s'il a changé (`get_state_revision`) : la prochaine mise à jour
## de la vue reconstruit le voisinage. Rend vrai si l'état a été relu.
func refresh(sim: Object) -> bool:
	_sim = sim
	var revision := -1
	if sim != null and sim.has_method("get_state_revision"):
		revision = int(sim.call("get_state_revision"))
	elif sim != null and sim.has_method("get_turn"):
		revision = int(sim.call("get_turn"))
	if revision == _revision and revision >= 0:
		return false
	_revision = revision
	_states.clear()
	_provinces.clear()
	_read_states()
	_dirty = true
	return true


## État des provinces à la dernière relecture : province → `{population, devastation, besieged,
## constructing}`.
func provinces_live() -> Dictionary:
	return _provinces


## TB3, point 5 : suie (0-1) des faubourgs, de l'enceinte et des bâtiments hors les murs de la
## colonie `id` (par instance de `MultiMesh`, `INSTANCE_CUSTOM.b`).
func set_soot(id: String, amount: float) -> void:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	if i < 0 or is_equal_approx(float(_soot.get(i, 0.0)), amount):
		return
	if amount > 0.0:
		_soot[i] = amount
	else:
		_soot.erase(i)
	for inst: Dictionary in _instances:
		if int(inst["settlement"]) == i:
			_write_batches()
			return


## État des provinces (population, dévastation, siège, chantier de la cité : `ProvinceSnapshot`)
## et de toutes les colonies en un appel groupé (`get_settlements_live`) ; sans ce dernier,
## lecture paresseuse par `settlement_detail` (`_state_of`).
func _read_states() -> void:
	if _sim == null or _data == null:
		return
	if _map != null and (_sim.has_method("get_provinces_snapshot") or _sim.has_method("get_province_state")):
		var snapshot := ProvinceSnapshot.of(_sim, _map)
		var has_sites := snapshot.constructing.size() == snapshot.ids.size()
		for p in snapshot.ids.size():
			if not snapshot.has(p):
				continue
			_provinces[snapshot.ids[p]] = {
				"population": float(snapshot.population_total[p]),
				"devastation": float(snapshot.devastation[p]),
				"besieged": snapshot.besieged[p] != 0,
				"constructing": has_sites and snapshot.constructing[p] != 0,
			}
	if not _sim.has_method("get_settlements_live"):
		return
	var live: Dictionary = _sim.call("get_settlements_live")
	var ids: PackedStringArray = live.get("id", PackedStringArray())
	var forts: PackedInt32Array = live.get("fortification_level", PackedInt32Array())
	var cities: PackedByteArray = live.get("is_city", PackedByteArray())
	var buildings: Array = live.get("buildings", [])
	for k in ids.size():
		var i: int = _data.index_by_id.get(ids[k], -1)
		var is_city := k < cities.size() and cities[k] != 0
		var province: Dictionary = _provinces.get(str(_data.settlements[i]["province"]), {}) if i >= 0 else {}
		_states[ids[k]] = {
			"buildings": Array(buildings[k]) if k < buildings.size() else [],
			"fortification": forts[k] if k < forts.size() else 0,
			"is_city": is_city,
			"building": is_city and bool(province.get("constructing", false)),
		}


## État d'une colonie : `{buildings: Array, fortification: int, building: bool}`.
func _state_of(i: int) -> Dictionary:
	var id := str(_data.settlements[i]["id"])
	if _states.has(id):
		return _states[id]
	var state := {"buildings": [], "fortification": int(_data.settlements[i].get("fortification_level", 0)), "is_city": false, "building": false}
	if _sim != null and _sim.has_method("settlement_detail"):
		var detail: Dictionary = _sim.call("settlement_detail", id)
		state["buildings"] = Array(detail.get("buildings", []))
		state["fortification"] = int(detail.get("fortification_level", state["fortification"]))
		state["is_city"] = bool(detail.get("is_city", false))
		state["building"] = not (detail.get("construction", {}) as Dictionary).is_empty()
	_states[id] = state
	return state


## Ressources de la province (statiques) : `get_province_city`, sinon `data/provinces/<id>.json`.
func _resources_of(province: String) -> Array:
	if _resources.has(province):
		return _resources[province]
	var out: Array = []
	if _sim != null and _sim.has_method("get_province_city"):
		out = Array((_sim.call("get_province_city", province) as Dictionary).get("resources", []))
	else:
		var path := data_dir().path_join("provinces").path_join(province + ".json")
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				out = Array((parsed as Dictionary).get("resources", []))
	_resources[province] = out
	return out


## Population de la province en 1337 (somme des classes de `data/provinces/<id>.json`), 0 si
## inconnue.
func baseline_population(province: String) -> float:
	if _baseline.has(province):
		return _baseline[province]
	var total := 0.0
	var path := data_dir().path_join("provinces").path_join(province + ".json")
	if FileAccess.file_exists(path):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
		if parsed is Dictionary:
			var classes: Dictionary = ((parsed as Dictionary).get("population", {}) as Dictionary).get("classes", {})
			for name: String in classes:
				total += float((classes[name] as Dictionary).get("count", 0.0))
	_baseline[province] = total
	return total


## Population simulée de la province rapportée à celle de 1337 (1 si l'une des deux manque).
func population_ratio(province: String) -> float:
	var now := float((_provinces.get(province, {}) as Dictionary).get("population", 0.0))
	var then := baseline_population(province)
	return now / then if now > 0.0 and then > 0.0 else 1.0


## Maquettes hors les murs de la colonie `id` pour l'état courant : `[{family, level, model…}]`,
## chantier compris (`family` = `worksite`).
func models_of(id: String) -> Array:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	return _models_of_index(i) if i >= 0 else []


## Croissance affichée de la colonie `id` : maquettes hors les murs, nombre de maisons de
## faubourg ajoutées et enceinte ajoutée ("" : aucune).
func growth_of(id: String) -> Dictionary:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	if i < 0:
		return {}
	var suburb := 0
	var enclosure := ""
	for inst: Dictionary in _growth_instances(i, _state_of(i)):
		if str(inst["family"]) == "suburb":
			suburb += 1
		elif str(inst.get("part", "")) == "wall":
			enclosure = TownGrowth.KINDS[int(inst["level"])]
	return {"outbuildings": _models_of_index(i), "suburb_houses": suburb, "enclosure": enclosure}


func _models_of_index(i: int) -> Array:
	var state := _state_of(i)
	var models := models_for(config, state["buildings"], _resources_of(str(_data.settlements[i]["province"])))
	if bool(state["building"]) and manifest.has(WORKSITE_MODEL):
		models.append({"family": WORKSITE_FAMILY, "level": 1, "model": WORKSITE_MODEL, "site": "road", "slot": _family_order.size()})
	return models


# --- Mise à jour par image ---------------------------------------------------------------


func view_range() -> float:
	return float((config.get("render", {}) as Dictionary).get("view_range_units", 45.0))


func update_view(rig_distance: float) -> void:
	if not enabled or _data == null:
		visible = false
		return
	var shown := force_active or rig_distance < view_range()
	if shown != visible:
		visible = shown
	if not shown:
		return
	var scale := exaggeration(config, rig_distance)
	if absf(scale - _scale) > 0.04 * _scale:
		_scale = scale
		_write_batches()
	var shadows := rig_distance < float((config.get("render", {}) as Dictionary).get("shadow_range_units", 12.0))
	if shadows != _shadows:
		_shadows = shadows
		for mmi: MultiMeshInstance3D in _batches.values():
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if MapData.vertical_scale() != _vertical_scale:
		_vertical_scale = MapData.vertical_scale()
		_refresh_aabbs()
	var here := _camera_ground()
	if _pending.is_empty() and (_dirty or here.distance_to(_center) > load_radius() * 0.3):
		_begin_rebuild(here)
	if not _pending.is_empty():
		_step(STEP_BUDGET_USEC if FrameBudget.in_frame() else 1 << 30)
	elif _reground_in >= 0:
		_reground_in -= 1
		if _reground_in < 0:
			_reground()


## Rayon (unités) du voisinage chargé autour du point visé.
func load_radius() -> float:
	return view_range() * 1.5


## Point du sol visé par la caméra (unités carte) ; position de la caméra en repli.
func _camera_ground() -> Vector2:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return _center if _center.x != INF else Vector2.ZERO
	var origin := camera.global_position
	var forward := -camera.global_transform.basis.z
	if forward.y < -0.05:
		origin += forward * (origin.y / -forward.y)
	return Vector2(origin.x, origin.z)


## Termine tout de suite la reconstruction autour de `center` (captures, tests).
func flush(center: Variant = null) -> void:
	if not enabled or _data == null:
		return
	if center is Vector2 or _dirty or _center.x == INF:
		_begin_rebuild(center if center is Vector2 else _camera_ground())
	if not _pending.is_empty():
		_step(1 << 30)
	if _reground_in >= 0:
		_reground_in = -1
		_reground()


func _begin_rebuild(center: Vector2) -> void:
	_dirty = false
	_build_center = center
	_building = []
	_pending = []
	var radius := load_radius()
	for i in _data.settlements.size():
		var px: Vector2 = _data.settlements[i]["px"]
		if absf(px.x - center.x) <= radius and absf(px.y - center.y) <= radius and px.distance_to(center) <= radius:
			_pending.append(i)
	stats["settlements"] = _pending.size()
	stats["build_start_usec"] = Time.get_ticks_usec()
	if _pending.is_empty():
		_finish_rebuild()


func _step(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	while not _pending.is_empty():
		_building.append_array(_instances_of(_pending.pop_back()))
		if Time.get_ticks_usec() - t0 > budget_usec:
			break
	if _pending.is_empty():
		_finish_rebuild()


func _finish_rebuild() -> void:
	_instances = _building
	_building = []
	_center = _build_center
	_reground_in = -1
	_write_batches()
	stats["build_ms"] = float(Time.get_ticks_usec() - int(stats.get("build_start_usec", Time.get_ticks_usec()))) / 1000.0


## Instances d'une colonie : maquettes hors les murs, chantier, puis croissance de la ville.
func _instances_of(i: int) -> Array:
	var out: Array = []
	var state := _state_of(i)
	for model: Dictionary in _models_of_index(i):
		var entry: Dictionary = manifest.get(str(model["model"]), {})
		if entry.is_empty():
			continue
		var family := str(model["family"])
		# L'ancrage vaut pour les trois niveaux : il réserve l'emprise du niveau 3.
		var reserve := float((manifest.get("%s_3" % family, entry) as Dictionary).get("radius", entry["radius"]))
		var anchor := _anchor(i, family, int(model["slot"]), str(model["site"]), reserve)
		if anchor.is_empty():
			continue
		var px: Vector2 = anchor["px"]
		out.append({
			"key": "out:" + str(model["model"]),
			"settlement": i,
			"family": family,
			"level": int(model["level"]),
			"model": str(model["model"]),
			"px": px,
			"yaw": float(anchor["yaw"]),
			"base_m": _base_m(px),
			"scale": Vector3.ONE,
			"grow": true,
			"top": float(entry.get("height", 20.0)),
			"reach": float(entry.get("radius", 30.0)),
		})
	out.append_array(_growth_instances(i, state))
	return out


## TB3, point 4 : faubourgs (population) et enceinte (fortification) de la ville 1:1.
func _growth_instances(i: int, state: Dictionary) -> Array:
	var town := _town_of(i)
	var growth: Dictionary = config.get("growth", {})
	if town.is_empty() or growth.is_empty():
		return []
	var entry: Dictionary = _data.settlements[i]
	var center := _layer.towns.data.anchor_of(str(entry["id"]))
	var out := TownGrowth.suburb_instances(growth, town, i, center, _mpu, TownGrowth.quarter_count(growth, population_ratio(str(entry["province"]))), _base_m)
	var kind := TownGrowth.enclosure_to_add(growth, state["buildings"], entry.get("initial_buildings", []), str(town.get("walls", "none")))
	if kind != "":
		out.append_array(TownGrowth.enclosure_instances(growth, _layer.towns.data.params.get("walls", {}), town, i, center, _mpu, kind, _base_m))
	return out


## Entrée de `towns_1340.json` de la colonie `i` (vide : ville emblématique ou colonie absente).
func _town_of(i: int) -> Dictionary:
	if _layer == null or _layer.towns == null or _layer.towns.data == null:
		return {}
	return _layer.towns.data.towns.get(str(_data.settlements[i]["id"]), {})


# --- Sites ---------------------------------------------------------------------------------


func _height_m(p: Vector2) -> float:
	if _terrain != null:
		return MapData.height_from_display(_terrain.surface_height_at(p.x, p.y), p.x, p.y)
	return _map.height_m_at(p.x, p.y) if _map != null else 0.0


## Altitude de pose (m), jamais sous la grève.
func _base_m(p: Vector2) -> float:
	return maxf(_height_m(p), 0.3)


func _is_water(p: Vector2) -> bool:
	return _height_m(p) <= 0.05 or (_map != null and _map.river_sd_at(p.x, p.y) < 0.0)


## Ancrage `{px, yaw}` de la famille `family` autour de la colonie `i`, calculé une fois ; vide
## si aucun site ne convient. `radius_m` : emprise à réserver.
func _anchor(i: int, family: String, slot: int, site: String, radius_m: float) -> Dictionary:
	var key := "%d|%s" % [i, family]
	if not _anchors.has(key):
		_anchors[key] = _find_anchor(i, slot, site, radius_m)
	return _anchors[key]


func _find_anchor(i: int, slot: int, site: String, radius_m: float) -> Dictionary:
	var render: Dictionary = config.get("render", {})
	var center := _px_of(i)
	var built := _built_radius(i)
	var reach := radius_m / _mpu
	var ring_min := built + float(render.get("ring_min_m", 90.0)) / _mpu + reach
	var ring_max := maxf(built + float(render.get("ring_max_m", 650.0)) / _mpu, ring_min + reach)
	var seed_value := absi(str(_data.settlements[i]["id"]).hash())
	var theta0 := float(seed_value % 3600) / 3600.0 * TAU
	if site == "shore" or site == "coast":
		var shore := _shore_anchor(i, center, ring_min - reach, built + float(render.get("shore_reach_m", 2500.0)) / _mpu, theta0)
		if not shore.is_empty() or site == "shore":
			return shore
	var slots := float(_family_order.size() + 1)
	var sector := TAU / slots
	var max_slope := float(render.get("max_slope", 0.22))
	var gates := PackedFloat32Array()
	for gate: Dictionary in _town_of(i).get("gates", []):
		gates.append(deg_to_rad(float(gate["bearing"])))
	var best := {}
	var best_score := INF
	for step in ANGLE_STEPS:
		var angle := theta0 + (float(slot) + float(step) / 3.0) * sector
		# Les routes des portes restent libres (faubourgs à venir).
		var on_road := false
		for gate_angle in gates:
			if absf(angle_difference(angle, gate_angle)) < GATE_CORRIDOR:
				on_road = true
		if on_road:
			continue
		var dir := Vector2(cos(angle), sin(angle))
		for t: float in [0.1, 0.45]:
			var p := center + dir * lerpf(ring_min, ring_max, t)
			var slope := _site_slope(i, p, reach, max_slope)
			if slope < 0.0:
				continue
			var score := float(absi(step)) * 10.0 + t
			match site:
				"slope":
					score -= slope * 4.0
				"water":
					score += clampf(_map.river_sd_at(p.x, p.y), 0.0, 8.0) if _map != null else 0.0
				"road":
					score += t * 2.0
				_:
					score += slope * 4.0
			if score < best_score:
				best_score = score
				# Façade (+Z de la maquette) tournée vers la colonie.
				best = {"px": p, "yaw": atan2(-dir.x, -dir.y)}
		if not best.is_empty() and absi(step) >= 2:
			break
	return best


## Pente (m/m) du site `p` d'emprise `reach` (unités), ou -1 s'il ne convient pas : mer, lit de
## fleuve, emprise d'une autre colonie, pente trop forte.
func _site_slope(i: int, p: Vector2, reach: float, max_slope: float) -> float:
	if _map != null and (p.x < 1.0 or p.y < 1.0 or p.x > _map.size.x - 1.0 or p.y > _map.size.y - 1.0):
		return -1.0
	var h := _height_m(p)
	if h <= 0.5 or (_map != null and _map.river_sd_at(p.x, p.y) < 0.03):
		return -1.0
	if _on_other_settlement(i, p, reach):
		return -1.0
	if _layer != null and not _layer.is_landmark(i) and _layer.covered_by_landmark(p):
		return -1.0
	var steepest := 0.0
	for offset: Vector2 in [Vector2(reach, 0.0), Vector2(-reach, 0.0), Vector2(0.0, reach), Vector2(0.0, -reach)]:
		var other := _height_m(p + offset)
		if other <= 0.05:
			return -1.0
		steepest = maxf(steepest, absf(other - h) / (reach * _mpu))
	return steepest if steepest <= max_slope else -1.0


func _on_other_settlement(i: int, p: Vector2, reach: float) -> bool:
	var cell := Vector2i(floori(p.x / GRID_UNITS), floori(p.y / GRID_UNITS))
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for j: int in _grid.get(cell + Vector2i(dx, dy), PackedInt32Array()):
				if j != i and p.distance_to(_px_of(j)) < _built_radius(j) + reach:
					return true
	return false


func _px_of(i: int) -> Vector2:
	return _layer.model_px(i) if _layer != null else (_data.settlements[i]["px"] as Vector2)


## Rayon bâti (unités) de la colonie : emprise réelle de la ville 1:1.
func _built_radius(i: int) -> float:
	return _layer.built_radius(i) if _layer != null else 150.0 / _mpu


## Site de rive : la grève la plus proche de la colonie (mer ou fleuve), façade vers l'eau.
func _shore_anchor(i: int, center: Vector2, from: float, to: float, theta0: float) -> Dictionary:
	var best := {}
	var best_r := INF
	var step := 60.0 / _mpu
	for k in SHORE_DIRECTIONS:
		var angle := theta0 + TAU * float(k) / float(SHORE_DIRECTIONS)
		var dir := Vector2(cos(angle), sin(angle))
		var r := from
		if _is_water(center + dir * r):
			continue
		while r < to and r < best_r:
			if _is_water(center + dir * (r + step)):
				var lo := r
				var hi := r + step
				for n in 4:
					var mid := (lo + hi) * 0.5
					if _is_water(center + dir * mid):
						hi = mid
					else:
						lo = mid
				var p := center + dir * maxf(lo - 10.0 / _mpu, from)
				if not _on_other_settlement(i, p, 30.0 / _mpu):
					best_r = lo
					best = {"px": p, "yaw": atan2(dir.x, dir.y)}
				break
			r += step
	return best


# --- Rendu ---------------------------------------------------------------------------------


func _mesh_of(key: String) -> Mesh:
	if _meshes.has(key):
		return _meshes[key]
	var mesh: Mesh = null
	if key.begins_with("out:"):
		mesh = _load_glb_mesh(MODEL_DIR + key.trim_prefix("out:") + ".glb")
	elif key.begins_with("kit:"):
		mesh = TownBuilder.kit_mesh(key.trim_prefix("kit:"))
	elif key.begins_with("box:"):
		mesh = TownBuilder.box_mesh(key.trim_prefix("box:"))
	elif key == "tower":
		mesh = TownBuilder.tower_mesh()
	_meshes[key] = mesh
	return mesh


static func _load_glb_mesh(path: String) -> Mesh:
	if not ResourceLoader.exists(path):
		return null
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var node := scene.instantiate()
	var mesh: Mesh = null
	for child in node.find_children("*", "MeshInstance3D", true, false):
		mesh = (child as MeshInstance3D).mesh
		break
	node.free()
	return mesh


## Tampon d'instances (`TownBuilder.pack_instances`) avec la suie de chaque instance dans la
## composante b des données d'instance (`INSTANCE_CUSTOM.b` de `town_building.gdshader`).
static func with_soot(buffer: PackedFloat32Array, soots: Array) -> PackedFloat32Array:
	for k in soots.size():
		buffer[k * 16 + 14] = float(soots[k])
	return buffer


## Suie (0-1) portée par les instances de la colonie `id`.
func soot_of(id: String) -> float:
	return float(_soot.get(int(_data.index_by_id.get(id, -1)), 0.0)) if _data != null else 0.0


## Réécrit les `MultiMesh` depuis `_instances` (repère en mètres centré sur `_center`).
func _write_batches() -> void:
	if _root == null or _center.x == INF:
		return
	var s := 1.0 / _mpu
	_root.transform = Transform3D(Basis().scaled(Vector3(s, s, s)), Vector3(_center.x, 0.0, _center.y))
	var groups := {}  # clé → [xforms, bases, tints, top, suies]
	for inst: Dictionary in _instances:
		var key := str(inst["key"])
		if not groups.has(key):
			groups[key] = [[], [], [], 0.0, []]
		var g: Array = groups[key]
		var px: Vector2 = inst["px"]
		var grow := _scale if bool(inst.get("grow", false)) else 1.0
		var basis := Basis(Vector3.UP, float(inst["yaw"])) * Basis.from_scale((inst["scale"] as Vector3) * grow)
		(g[0] as Array).append(Transform3D(basis, Vector3((px.x - _center.x) * _mpu, float(inst.get("lift", 0.0)), (px.y - _center.y) * _mpu)))
		(g[1] as Array).append(float(inst["base_m"]))
		(g[2] as Array).append(float(inst.get("tint", 0.5)))
		g[3] = maxf(float(g[3]), (float(inst.get("top", 20.0)) + float(inst.get("reach", 30.0))) * grow)
		(g[4] as Array).append(float(_soot.get(int(inst["settlement"]), 0.0)))
	for key: String in _batches.keys():
		if not groups.has(key):
			(_batches[key] as Node).free()
			_batches.erase(key)
	var total := 0
	for key: String in groups:
		var mesh := _mesh_of(key)
		if mesh == null:
			continue
		var g: Array = groups[key]
		var packed := TownBuilder.pack_instances(g[0], g[1], g[2])
		var buffer := with_soot(packed["buffer"], g[4])
		var mmi: MultiMeshInstance3D = _batches.get(key)
		if mmi == null:
			mmi = MultiMeshInstance3D.new()
			mmi.name = key.replace(":", "_")
			mmi.multimesh = MultiMesh.new()
			mmi.multimesh.transform_format = MultiMesh.TRANSFORM_3D
			mmi.multimesh.use_custom_data = true
			mmi.multimesh.mesh = mesh
			mmi.material_override = TownBuilder.material(0, not (key.begins_with("out:") or key.begins_with("kit:")), 0.0, _mpu)
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if _shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			_root.add_child(mmi)
			_batches[key] = mmi
		mmi.multimesh.instance_count = int(packed["count"])
		mmi.multimesh.buffer = buffer
		mmi.set_meta("bounds", [float(packed["lo"]), float(packed["hi"]), float(g[3]), (packed["rect"] as Rect2).grow(float(g[3]))])
		total += int(packed["count"])
	_refresh_aabbs()
	stats["instances"] = total
	stats["nodes"] = _batches.size()


## Boîtes englobantes (mètres, repère du voisinage) : la base est ajoutée par le shader, Godot ne
## la voit pas (même calcul que `TownBuilder.refresh_aabbs`).
func _refresh_aabbs() -> void:
	var vertical := MapData.vertical_scale()
	var k := vertical * _mpu
	var up := 1.0 + MapData.relief_gain_for_scale(vertical)
	var down := 1.0 - MapData.relief_squash_max_for_scale(vertical)
	for mmi: MultiMeshInstance3D in _batches.values():
		var b: Array = mmi.get_meta("bounds", [])
		if b.size() < 4:
			continue
		var rect: Rect2 = b[3]
		var y0 := float(b[0]) * k * (down if float(b[0]) > 0.0 else 1.0) - 10.0
		var y1 := float(b[1]) * k * (up if float(b[1]) > 0.0 else 1.0) + float(b[2])
		mmi.custom_aabb = AABB(Vector3(rect.position.x, y0, rect.position.y), Vector3(rect.size.x, y1 - y0, rect.size.y))


func _on_chunk_surface_changed(_index: int) -> void:
	if _terrain != null and _terrain.rescaling_vertical:
		return
	if visible and not _instances.is_empty():
		_reground_in = REGROUND_DELAY


## Hauteurs de base relues sur la surface affichée (pages plus fines arrivées).
func _reground() -> void:
	var changed := false
	for inst: Dictionary in _instances:
		var base := _base_m(inst["px"]) if not inst.has("base_px") else _base_m(inst["base_px"])
		if absf(base - float(inst["base_m"])) > 0.05:
			inst["base_m"] = base
			changed = true
	if changed:
		_write_batches()


# --- Lecture pour les tests et les mesures -------------------------------------------------


## Instances affichées (dictionnaires : `key`, `settlement`, `family`, `level`, `px`, `yaw`…).
func instances() -> Array:
	return _instances


## Instances affichées de la colonie `id` (toutes, ou d'une famille).
func instances_of(id: String, family: String = "") -> Array:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	var out: Array = []
	for inst: Dictionary in _instances:
		if int(inst["settlement"]) == i and (family == "" or str(inst.get("family", "")) == family):
			out.append(inst)
	return out


func instance_count() -> int:
	return int(stats.get("instances", 0))


## Nombre de nœuds de rendu (un appel de dessin par nœud et par passe).
func node_count() -> int:
	return _batches.size()
