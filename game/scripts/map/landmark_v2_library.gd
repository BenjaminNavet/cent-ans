class_name LandmarkV2Library
extends RefCounted

## Lots VH0/VH4 (ADR 0078) : villes emblématiques à l'échelle 1:1, format v2 géoréférencé
## (`data/landmarks_v2/<id>.json`, schéma `landmark_v2.schema.json`). Lecture et conversions :
## EPSG:3035 (`origin_3035` + décalages [dE, dN] en mètres) → unités carte (grille de
## `data/map/map.json`, +Z vers le sud) et repère local de `TownPlan` / `TownBuilder` (mètres, x vers +X monde,
## y vers +Z monde, soit [dE, -dN]). Rendu seulement.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Échelle de repli (ADR 0082, inchangée par ADR 0115) si `data/map/map.json` est illisible ; les
## bornes n'ont alors pas de repli (villes 1:1 mal placées, avertissement).
const FALLBACK_METERS_PER_UNIT := 718.9765625

static var _by_settlement: Dictionary = {}
static var _loaded := false
static var _bounds: Array = []
static var _meters_per_unit: float = FALLBACK_METERS_PER_UNIT


static func clear_cache() -> void:
	_by_settlement.clear()
	_bounds.clear()
	_loaded = false


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _load() -> void:
	_loaded = true
	var dir_path := _data_dir().path_join("landmarks_v2")
	var dir := DirAccess.open(dir_path)
	if dir != null:
		for file_name in dir.get_files():
			if not file_name.ends_with(".json"):
				continue
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(dir_path.path_join(file_name)))
			if parsed is Dictionary and int((parsed as Dictionary).get("format", 0)) == 2:
				_by_settlement[str(parsed["settlement"])] = parsed
	var meta_path := _data_dir().path_join("map").path_join("map.json")
	if FileAccess.file_exists(meta_path):
		var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(meta_path))
		if meta is Dictionary and (meta as Dictionary).has("bounds_projected"):
			_bounds = meta["bounds_projected"]
			_meters_per_unit = float((meta as Dictionary).get("meters_per_px", FALLBACK_METERS_PER_UNIT))
	if _bounds.size() != 4:
		push_warning("LandmarkV2Library: no bounds_projected in %s" % meta_path)
		_bounds = [0.0, 0.0, 0.0, 0.0]
		_meters_per_unit = FALLBACK_METERS_PER_UNIT


## Ville v2 d'une colonie, {} sinon.
static func for_settlement(settlement_id: String) -> Dictionary:
	if not _loaded:
		_load()
	return _by_settlement.get(settlement_id, {})


static func all() -> Array:
	if not _loaded:
		_load()
	return _by_settlement.values()


static func bounds() -> Array:
	if not _loaded:
		_load()
	return _bounds


## Mètres par unité carte (≈ 719).
static func meters_per_unit() -> float:
	if not _loaded:
		_load()
	return _meters_per_unit


## Origine de la ville en unités carte.
static func anchor_units(city: Dictionary) -> Vector2:
	var b := bounds()
	var o: Array = city.get("origin_3035", [0.0, 0.0])
	var mpu := meters_per_unit()
	return Vector2((float(o[0]) - float(b[0])) / mpu, (float(b[3]) - float(o[1])) / mpu)


## Point EPSG:3035 absolu [E, N] → unités carte.
static func projected_to_units(e: float, n: float) -> Vector2:
	var b := bounds()
	var mpu := meters_per_unit()
	return Vector2((e - float(b[0])) / mpu, (float(b[3]) - n) / mpu)


## Décalage v2 [dE, dN] → repère local du plan (x est, y sud), en mètres.
static func local(p: Array) -> Vector2:
	return Vector2(float(p[0]), -float(p[1]))


static func local_line(points: Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	for p in points:
		out.append(local(p))
	return out


## Angle v2 (degrés, antihoraire depuis l'est de la grille, y au nord) → lacet du repère local
## (radians, depuis +x vers +y = sud) : le signe s'inverse.
static func yaw_of(angle_deg: float) -> float:
	return -deg_to_rad(angle_deg)


## Élément présent l'année `year` (`from_year` / `until_year` inclus).
static func present(item: Dictionary, year: int) -> bool:
	if item.has("from_year") and year < int(item["from_year"]):
		return false
	if item.has("until_year") and year > int(item["until_year"]):
		return false
	return true


## Rayon (unités carte) couvert par la ville.
static func extent_units(city: Dictionary) -> float:
	return float(city.get("extent_m", 1500.0)) / meters_per_unit()
