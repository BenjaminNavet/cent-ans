class_name CountrysideLayer
extends Node3D

## Lot DN-PAYS : campagne vivante hors champs (rendu seulement, aucune règle de jeu) : haies et
## clôtures de bocage, puits, croix de chemin et calvaires, moulins à vent, salines, ruines,
## piloris, charrettes et caravanes au bord des routes. Les troupeaux restent à `FaunaLayer`.
##
## - Données : `data/map/map_countryside.json` (schéma `map_countryside.schema.json`) : modèles
##   (`props`, glb générés `dn/<dossier>/<nom>_lod{0,1,2}.glb`), régions (ellipses lon/lat) et règles
##   de placement. Aucune règle de placement dans le code : une règle = un mode (`scatter`,
##   `paddock`, `village`, `road`), des modèles pondérés, des régions, une densité et des filtres de
##   site (biome, prairie/forêt, altitude, pente, côte, marais, saison).
## - Semis par cellule de `cell_px` pixels, à la demande autour du point visé, déterministe
##   (graine = cellule, règle, région). Un `MultiMesh` par modèle et par cellule.
## - Taille tenue à l'écran (`size_k` × distance^`size_exponent`, jamais sous la taille réelle,
##   bornée par `max_mult` du modèle) ; les enclos s'écartent en proportion (`spread_exponent`).
##   Fondu de distance propre à chaque modèle. `--no-countryside` : état d'avant (A/B).
## - Réutilise `FaunaLayer` pour les lacs et les zones humides affichés (ne les duplique pas).

const SHADER := preload("res://shaders/countryside.gdshader")
const DATA_FILE := "map/map_countryside.json"
const DN_DIR := "res://assets/models/dn/"
const FLOATS_PER_INSTANCE := 16
const MAX_PROFILES := 24
const MAX_LODS := 3
const BIOMES_FILE := "biomes.png"
const BIOMES_SHRINK := 4
const GRID_PX := 8.0

var config: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false
var stats: Dictionary = {"cells": 0, "visible_cells": 0, "draw_calls": 0, "instances": 0, "visible": 0, "build_ms_max": 0.0, "state_ms_max": 0.0}

var _map_data: MapData
var _mpp := 719.0
var _rig: Node3D
var _fauna: FaunaLayer
var _rules: Array = []  # règles résolues : {rule, regions: Array[{center, radius}], props: Array, weights: PackedFloat32Array, profile}
var _profiles: Array[Vector4] = []
var _profile_index: Dictionary = {}
var _states: Dictionary = {}  # id de modèle → état de rendu
var _cells: Dictionary = {}  # Vector2i → {nodes, count, last_seen}
var _town_grid: Dictionary = {}  # Vector2i (case de GRID_PX) → PackedVector2Array
var _village_grid: Dictionary = {}  # Vector2i (cellule) → Array[{px, kind}]
var _road_grid: Dictionary = {}  # Vector2i (cellule) → Array[{a, b, type, id}]
var _biomes: Image
var _frame := 0
var _lod := -1
## Modèles à préparer (un par image) pour éviter l'à-coup de la première cellule.
var _warm_queue: Array = []


static func load_config() -> Dictionary:
	var parsed: Variant = DataFile.read_json(DATA_FILE)
	return parsed if parsed is Dictionary else {}


func _ready() -> void:
	if CmdArgs.has("--no-countryside"):
		enabled = false


## Branche la couche sur la carte. `towns` : colonies à éviter (px) ; `villages` : {px, kind}
## (colonies et hameaux) ; `roads` : {type, main, points} (px) ; `fauna` : lacs et zones humides.
func setup(map_data: MapData, rig: Node3D = null, towns: PackedVector2Array = PackedVector2Array(), villages: Array = [], roads: Array = [], fauna: FaunaLayer = null) -> void:
	clear()
	_map_data = map_data
	_mpp = map_data.meters_per_px if map_data != null else 719.0
	_rig = rig
	_fauna = fauna
	config = load_config()
	_rules.clear()
	_states.clear()
	_profiles.clear()
	_profile_index.clear()
	_town_grid.clear()
	_village_grid.clear()
	_road_grid.clear()
	_biomes = null
	if config.is_empty() or map_data == null:
		return
	for town in towns:
		var key := Vector2i(floori(town.x / GRID_PX), floori(town.y / GRID_PX))
		var list: PackedVector2Array = _town_grid.get(key, PackedVector2Array())
		list.append(town)
		_town_grid[key] = list
	_resolve_rules()
	_request_models()
	_index_villages(villages)
	_index_roads(roads)
	var needs_biomes := false
	for entry: Dictionary in _rules:
		if not ((entry["rule"] as Dictionary).get("filters", {}) as Dictionary).get("biomes", []).is_empty():
			needs_biomes = true
	if needs_biomes and FileAccess.file_exists(map_data.map_dir.path_join(BIOMES_FILE)):
		_biomes = Image.load_from_file(map_data.map_dir.path_join(BIOMES_FILE))
		if _biomes != null:
			_biomes.resize(maxi(_biomes.get_width() / BIOMES_SHRINK, 1), maxi(_biomes.get_height() / BIOMES_SHRINK, 1), Image.INTERPOLATE_NEAREST)


func clear() -> void:
	for entry: Dictionary in _cells.values():
		for node: Node in entry["nodes"]:
			node.queue_free()
	_cells.clear()
	stats["cells"] = 0
	stats["visible_cells"] = 0
	stats["draw_calls"] = 0
	stats["instances"] = 0
	stats["visible"] = 0


func _render(key: String, fallback: float) -> float:
	return float((config.get("render", {}) as Dictionary).get(key, fallback))


func cell_count() -> int:
	return _cells.size()


func rule_count() -> int:
	return _rules.size()


# --- Règles -----------------------------------------------------------------------------------


func _ellipse(center_lonlat: Array, radius_km: Array) -> Dictionary:
	var center := FaunaLayer.lonlat_to_px(float(center_lonlat[0]), float(center_lonlat[1]), _map_data)
	return {"center": center, "radius": Vector2(float(radius_km[0]), float(radius_km[1])) * 1000.0 / _mpp}


func _resolve_rules() -> void:
	var regions: Dictionary = config.get("regions", {})
	for rule: Dictionary in config.get("rules", []):
		var resolved: Array = []
		for region_id in rule.get("regions", []):
			var region: Dictionary = regions.get(str(region_id), {})
			if not region.is_empty():
				resolved.append(_ellipse(region["center"], region["radius_km"]))
		var from: Dictionary = rule.get("regions_from", {})
		if not from.is_empty():
			var source: Variant = DataFile.read_json(str(from["file"]))
			var ids: Array = from.get("ids", [])
			var scale := float(from.get("scale", 1.0))
			if source is Dictionary:
				for site: Dictionary in (source as Dictionary).get(str(from["key"]), []):
					if ids.is_empty() or ids.has(str(site.get("id", ""))):
						var size: Array = site["size_km"]
						resolved.append(_ellipse(site["center"], [float(size[0]) * 0.5 * scale, float(size[1]) * 0.5 * scale]))
		var is_global := (rule.get("regions", []) as Array).is_empty() and from.is_empty()
		if resolved.is_empty() and not is_global:
			continue
		var props: Array = []
		var weights := PackedFloat32Array()
		var total := 0.0
		for prop_id: String in (rule.get("props", {}) as Dictionary):
			if (config.get("props", {}) as Dictionary).has(prop_id):
				props.append(prop_id)
				weights.append(float(rule["props"][prop_id]))
				total += float(rule["props"][prop_id])
		if props.is_empty() or total <= 0.0:
			continue
		_rules.append({"rule": rule, "regions": resolved, "is_global": is_global, "props": props, "weights": weights, "total": total, "profile": _profile_of(_season_vector(rule.get("seasons", {})))})


func _season_vector(seasons: Dictionary) -> Vector4:
	var v := Vector4.ONE
	for n in FaunaLayer.SEASONS.size():
		if seasons.has(FaunaLayer.SEASONS[n]):
			v[n] = float(seasons[FaunaLayer.SEASONS[n]])
	return v


func _profile_of(weights: Vector4) -> int:
	var key := "%.3f|%.3f|%.3f|%.3f" % [weights.x, weights.y, weights.z, weights.w]
	if _profile_index.has(key):
		return int(_profile_index[key])
	if _profiles.size() >= MAX_PROFILES:
		push_warning("CountrysideLayer: plus de %d profils de saison" % MAX_PROFILES)
		return 0
	_profiles.append(weights)
	_profile_index[key] = _profiles.size() - 1
	return _profiles.size() - 1


func _cell_key_of(p: Vector2) -> Vector2i:
	var side := _cell_px()
	return Vector2i(floori(p.x / side), floori(p.y / side))


func _index_villages(villages: Array) -> void:
	for village: Dictionary in villages:
		var key := _cell_key_of(village["px"])
		var list: Array = _village_grid.get(key, [])
		list.append(village)
		_village_grid[key] = list


func _index_roads(roads: Array) -> void:
	var road_id := 0
	for road: Dictionary in roads:
		var points: PackedVector2Array = road["points"]
		for n in points.size() - 1:
			var a := points[n]
			var b := points[n + 1]
			if a.distance_squared_to(b) < 1e-6:
				continue
			var key := _cell_key_of((a + b) * 0.5)
			var list: Array = _road_grid.get(key, [])
			list.append({"a": a, "b": b, "type": str(road.get("type", "secondary")), "id": road_id * 1000 + n})
			_road_grid[key] = list
		road_id += 1


# --- Semis ------------------------------------------------------------------------------------


func _cell_px() -> float:
	return _render("cell_px", 96.0)


func _cell_rect(key: Vector2i) -> Rect2:
	var side := _cell_px()
	return Rect2(Vector2(key) * side, Vector2(side, side))


func _in_regions(entry: Dictionary, p: Vector2) -> bool:
	if entry["is_global"]:
		return true
	for region: Dictionary in entry["regions"]:
		var d: Vector2 = (p - (region["center"] as Vector2)) / (region["radius"] as Vector2)
		if d.length_squared() <= 1.0:
			return true
	return false


func _region_touches(entry: Dictionary, rect: Rect2) -> bool:
	if entry["is_global"]:
		return true
	for region: Dictionary in entry["regions"]:
		var center: Vector2 = region["center"]
		var radius: Vector2 = region["radius"]
		if Rect2(center - radius, radius * 2.0).intersects(rect):
			return true
	return false


func _pick_prop(entry: Dictionary, rng: RandomNumberGenerator) -> String:
	var roll := rng.randf() * float(entry["total"])
	var weights: PackedFloat32Array = entry["weights"]
	for n in weights.size():
		roll -= weights[n]
		if roll <= 0.0:
			return str((entry["props"] as Array)[n])
	return str((entry["props"] as Array)[0])


func _filter(rule: Dictionary, filter_name: String) -> float:
	var defaults: Dictionary = (config.get("render", {}) as Dictionary).get("defaults", {})
	return float((rule.get("filters", {}) as Dictionary).get(filter_name, defaults.get(filter_name, 0.0)))


func _splat_at(p: Vector2) -> Color:
	var image := _map_data.splat_image
	if image == null:
		return Color(0, 0, 0, 0)
	return image.get_pixel(clampi(int(p.x * image.get_width() / _map_data.size.x), 0, image.get_width() - 1), clampi(int(p.y * image.get_height() / _map_data.size.y), 0, image.get_height() - 1))


func _biome_at(p: Vector2) -> int:
	if _biomes == null:
		return -1
	return int(roundf(_biomes.get_pixel(clampi(int(p.x * _biomes.get_width() / _map_data.size.x), 0, _biomes.get_width() - 1), clampi(int(p.y * _biomes.get_height() / _map_data.size.y), 0, _biomes.get_height() - 1)).r * 255.0))


func near_town(p: Vector2, clearance: float) -> bool:
	var tk := Vector2i(floori(p.x / GRID_PX), floori(p.y / GRID_PX))
	var reach := maxi(1, ceili(clearance / GRID_PX))
	for dy in range(-reach, reach + 1):
		for dx in range(-reach, reach + 1):
			var list: PackedVector2Array = _town_grid.get(tk + Vector2i(dx, dy), PackedVector2Array())
			for town in list:
				if town.distance_squared_to(p) < clearance * clearance:
					return true
	return false


## Le site (px carte) convient-il à la règle ? Terre, hors fleuve et lac, filtres de la règle.
func site_ok(p: Vector2, rule: Dictionary) -> bool:
	var map := _map_data
	if p.x < 1.0 or p.y < 1.0 or p.x >= map.size.x - 1 or p.y >= map.size.y - 1:
		return false
	if not map.is_land_px(int(p.x), int(p.y)):
		return false
	var filters: Dictionary = rule.get("filters", {})
	if map.river_sd_at(p.x, p.y) < 0.3:
		return false
	if _fauna != null and _fauna.in_lake(p):
		return false
	var height := map.height_m_at(p.x, p.y)
	if height < _filter(rule, "alt_min_m") or height > _filter(rule, "alt_max_m"):
		return false
	var slope_max := _filter(rule, "slope_max_deg")
	if slope_max < 89.0:
		var gx := map.height_m_at(p.x + 1.0, p.y) - map.height_m_at(p.x - 1.0, p.y)
		var gy := map.height_m_at(p.x, p.y + 1.0) - map.height_m_at(p.x, p.y - 1.0)
		if rad_to_deg(atan(Vector2(gx, gy).length() / (2.0 * _mpp))) > slope_max:
			return false
	if _fauna != null and not filters.get("wet_ok", false):
		var wet := _fauna.wetness_at(p)
		if wet.r > _filter(rule, "marsh_max") or wet.g > 0.4:
			return false
	var splat := _splat_at(p)
	if splat.r < _filter(rule, "prairie_min") or splat.r + splat.g + splat.a * 0.5 < _filter(rule, "open_min") or splat.b > _filter(rule, "forest_max") or splat.g > _filter(rule, "crops_max"):
		return false
	var biomes: Array = filters.get("biomes", [])
	if not biomes.is_empty():
		var biome_names: Dictionary = config.get("biome_index", {})
		var biome := _biome_at(p)
		var allowed := false
		for biome_name in biomes:
			if int(biome_names.get(str(biome_name), -2)) == biome:
				allowed = true
		if not allowed:
			return false
	var shore := int(filters.get("coast_px", 0))
	if shore > 0:
		var touches_sea := false
		for k in 8:
			var a := TAU * k / 8.0
			if not map.is_land_px(int(p.x + cos(a) * shore), int(p.y + sin(a) * shore)):
				touches_sea = true
		if not touches_sea:
			return false
	return true


## Instances de la cellule : Array de {prop, pos: Vector2, yaw, sx (longueur du segment en m, 0 =
## taille native), offset: Vector2 (px, écart au centre du groupe), rank, profile, rule}. Déterministe.
func cell_instances(key: Vector2i) -> Array:
	var result: Array = []
	if _map_data == null:
		return result
	var rect := _cell_rect(key)
	var clearance := _render("town_clearance_px", 0.8)
	var rule_index := -1
	for entry: Dictionary in _rules:
		rule_index += 1
		if not _region_touches(entry, rect):
			continue
		var rule: Dictionary = entry["rule"]
		var rng := RandomNumberGenerator.new()
		rng.seed = hash(Vector4i(key.x, key.y, str(rule["id"]).hash() & 0xffff, rule_index))
		match str(rule["mode"]):
			"scatter":
				_scatter(entry, rule, rect, clearance, rng, result)
			"paddock":
				_paddocks(entry, rule, rect, clearance, rng, result)
			"village":
				_villages(entry, rule, key, rng, result)
			"road":
				_roadside(entry, rule, key, clearance, rng, result)
	return result


func _count_for(expected: float, rng: RandomNumberGenerator) -> int:
	return int(expected) + (1 if rng.randf() < expected - floorf(expected) else 0)


## Rectangles où semer : la cellule (règle globale) ou ses intersections avec l'emprise de chaque
## région (la densité s'applique à l'aire semée, pas à la cellule entière).
func _sample_rects(entry: Dictionary, rect: Rect2) -> Array[Rect2]:
	var rects: Array[Rect2] = []
	if entry["is_global"]:
		rects.append(rect)
		return rects
	for region: Dictionary in entry["regions"]:
		var center: Vector2 = region["center"]
		var radius: Vector2 = region["radius"]
		var part := Rect2(center - radius, radius * 2.0).intersection(rect)
		if part.has_area():
			rects.append(part)
	return rects


func _area_km2(rect: Rect2) -> float:
	return rect.size.x * rect.size.y * _mpp * _mpp / 1.0e6


func _scatter(entry: Dictionary, rule: Dictionary, rect: Rect2, clearance: float, rng: RandomNumberGenerator, out: Array) -> void:
	for part in _sample_rects(entry, rect):
		var count := _count_for(float(rule["density_per_1000km2"]) * _area_km2(part) / 1000.0, rng)
		for n in count:
			var p := part.position + Vector2(rng.randf(), rng.randf()) * part.size
			var prop := _pick_prop(entry, rng)
			var yaw := rng.randf() * TAU
			var rank := rng.randf()
			if not _in_regions(entry, p) or near_town(p, clearance) or not site_ok(p, rule):
				continue
			out.append({"prop": prop, "pos": p, "yaw": yaw, "len_m": 0.0, "offset": Vector2.ZERO, "rank": rank, "profile": int(entry["profile"]), "rule": str(rule["id"])})


## Enclos de clôtures (rectangles tournés) : segments le long du périmètre, orientés selon le côté.
func _paddocks(entry: Dictionary, rule: Dictionary, rect: Rect2, clearance: float, rng: RandomNumberGenerator, out: Array) -> void:
	var settings: Dictionary = rule.get("paddock", {})
	var size_range: Array = settings.get("size_m", [90.0, 220.0])
	var segment_m := float(settings.get("segment_m", 14.0))
	var gap := float(settings.get("gap", 0.1))
	var centers: Array[Vector2] = []
	for part in _sample_rects(entry, rect):
		for n in _count_for(float(rule["density_per_1000km2"]) * _area_km2(part) / 1000.0, rng):
			centers.append(part.position + Vector2(rng.randf(), rng.randf()) * part.size)
	for center in centers:
		var prop := _pick_prop(entry, rng)
		var width := rng.randf_range(float(size_range[0]), float(size_range[1])) / _mpp
		var height := rng.randf_range(float(size_range[0]), float(size_range[1])) / _mpp
		var angle := rng.randf() * PI
		var rank := rng.randf()
		var gate_side := rng.randi() % 4
		if not _in_regions(entry, center) or near_town(center, clearance) or not site_ok(center, rule):
			continue
		var axis_u := Vector2(cos(angle), sin(angle))
		var axis_v := Vector2(-axis_u.y, axis_u.x)
		var corners: Array[Vector2] = [
			center - axis_u * width * 0.5 - axis_v * height * 0.5, center + axis_u * width * 0.5 - axis_v * height * 0.5,
			center + axis_u * width * 0.5 + axis_v * height * 0.5, center - axis_u * width * 0.5 + axis_v * height * 0.5,
		]
		for side in 4:
			var from := corners[side]
			var to := corners[(side + 1) % 4]
			var length_px := from.distance_to(to)
			var segments := maxi(1, roundi(length_px * _mpp / segment_m))
			var direction := (to - from).normalized()
			var yaw := atan2(-direction.y, direction.x)
			for s in segments:
				if side == gate_side and absf(float(s) + 0.5 - segments * 0.5) < maxf(1.0, segments * gap):
					continue
				var at := from + direction * length_px * (float(s) + 0.5) / segments
				if not _map_data.is_land_px(int(at.x), int(at.y)) or _map_data.river_sd_at(at.x, at.y) < 0.3 or (_fauna != null and _fauna.in_lake(at)):
					continue
				out.append({"prop": prop, "pos": at, "yaw": yaw, "len_m": length_px * _mpp / segments, "offset": at - center, "rank": rank, "profile": int(entry["profile"]), "rule": str(rule["id"])})


func _villages(entry: Dictionary, rule: Dictionary, key: Vector2i, rng: RandomNumberGenerator, out: Array) -> void:
	var settings: Dictionary = rule.get("village", {})
	var kinds: Array = settings.get("kinds", ["village"])
	var offset_m: Array = settings.get("offset_m", [200.0, 380.0])
	var chance := float(settings.get("chance", 0.4))
	for village: Dictionary in _village_grid.get(key, []):
		if not kinds.has(str(village["kind"])):
			continue
		var p: Vector2 = village["px"]
		var roll := rng.randf()
		var prop := _pick_prop(entry, rng)
		var angle := rng.randf() * TAU
		var distance := rng.randf_range(float(offset_m[0]), float(offset_m[1])) / _mpp
		var yaw := rng.randf() * TAU
		var rank := rng.randf()
		if roll > chance or not _in_regions(entry, p):
			continue
		var at := p + Vector2(cos(angle), sin(angle)) * distance
		if not _map_data.is_land_px(int(at.x), int(at.y)) or _map_data.river_sd_at(at.x, at.y) < 0.3 or (_fauna != null and _fauna.in_lake(at)):
			continue
		out.append({"prop": prop, "pos": at, "yaw": yaw, "len_m": 0.0, "offset": at - p, "rank": rank, "profile": int(entry["profile"]), "rule": str(rule["id"])})


func _roadside(entry: Dictionary, rule: Dictionary, key: Vector2i, clearance: float, rng: RandomNumberGenerator, out: Array) -> void:
	var settings: Dictionary = rule.get("road", {})
	var types: Array = settings.get("types", [])
	var per_100km := float(settings.get("per_100km", 1.0))
	var offset_m: Array = settings.get("offset_m", [6.0, 14.0])
	var parallel := bool(settings.get("parallel", true))
	for segment: Dictionary in _road_grid.get(key, []):
		if not types.is_empty() and not types.has(segment["type"]):
			continue
		var a: Vector2 = segment["a"]
		var b: Vector2 = segment["b"]
		var seg_rng := RandomNumberGenerator.new()
		seg_rng.seed = hash(Vector3i(int(segment["id"]), str(rule["id"]).hash() & 0xffff, 7))
		var expected := a.distance_to(b) * _mpp / 100000.0 * per_100km
		var count := _count_for(expected, seg_rng)
		var direction := (b - a).normalized()
		var normal := Vector2(-direction.y, direction.x)
		for n in count:
			var t := seg_rng.randf()
			var side := 1.0 if seg_rng.randf() < 0.5 else -1.0
			var lateral := seg_rng.randf_range(float(offset_m[0]), float(offset_m[1])) / _mpp
			var prop := _pick_prop(entry, seg_rng)
			var rank := seg_rng.randf()
			var yaw_free := seg_rng.randf() * TAU
			var at := a.lerp(b, t) + normal * lateral * side
			if not _in_regions(entry, at) or near_town(at, clearance * 0.6) or not site_ok(at, rule):
				continue
			var yaw := atan2(-direction.y, direction.x) if parallel else yaw_free
			out.append({"prop": prop, "pos": at, "yaw": yaw, "len_m": 0.0, "offset": Vector2.ZERO, "rank": rank, "profile": int(entry["profile"]), "rule": str(rule["id"])})


# --- Modèles ----------------------------------------------------------------------------------


## État de rendu d'un modèle : maillages par niveau de détail (surfaces recouvertes de
## `countryside.gdshader`), mesures natives (mètres).
func _prop_state(prop_id: String) -> Dictionary:
	if _states.has(prop_id):
		return _states[prop_id]
	var prop: Dictionary = (config["props"] as Dictionary)[prop_id]
	var lods: Array = []
	for level in MAX_LODS:
		var path := DN_DIR + "%s_lod%d.glb" % [str(prop["model"]), level]
		if not ResourceLoader.exists(path):
			break
		var mesh := mesh_of(path)
		if mesh == null:
			break
		lods.append(mesh)
	var from_dn := not lods.is_empty()
	if lods.is_empty():
		lods.append(_box_mesh())
	var aabb := (lods[0] as ArrayMesh).get_aabb()
	var extent_m := maxf(aabb.size.x, aabb.size.z)
	var state := {
		"id": prop_id, "lods": [], "materials": [], "from_dn": from_dn,
		"extent_m": extent_m, "length_x_m": maxf(aabb.size.x, 1e-3),
		"real_units": extent_m / _mpp,
		"max_mult": float(prop.get("max_mult", 30.0)),
		"size_scale": float(prop.get("size_scale", 1.0)),
		"max_distance": float(prop.get("max_distance", _render("max_distance", 110.0))),
		"fade_from": float(prop.get("fade_from", _render("fade_from", 70.0))),
		"stretched": bool(prop.get("stretched", false)),
	}
	var shared_texture: Texture2D = null
	for mesh: ArrayMesh in lods:
		var covered := mesh.duplicate() as ArrayMesh
		for s in covered.get_surface_count():
			var material := ShaderMaterial.new()
			material.shader = SHADER
			var source := mesh.surface_get_material(s) as BaseMaterial3D
			if source != null:
				material.set_shader_parameter("albedo", source.albedo_color)
				if source.albedo_texture != null:
					# Les niveaux de détail d'un modèle partagent le même albédo (fichiers identiques) :
					# une seule texture réduite par modèle, les autres sont libérées.
					if shared_texture == null:
						shared_texture = _small_texture(source.albedo_texture)
					material.set_shader_parameter("albedo_texture", shared_texture)
					material.set_shader_parameter("has_texture", true)
			material.set_shader_parameter("albedo_gain", float(prop.get("albedo_gain", 1.0)))
			material.set_shader_parameter("mesh_origin", Vector3(aabb.get_center().x, aabb.position.y, aabb.get_center().z))
			var table := PackedVector4Array()
			for n in MAX_PROFILES:
				table.append(Vector4(_profiles[n]) if n < _profiles.size() else Vector4.ONE)
			material.set_shader_parameter("season_profiles", table)
			covered.surface_set_material(s, material)
			(state["materials"] as Array).append(material)
		(state["lods"] as Array).append(covered)
	_states[prop_id] = state
	return state


## Maillage d'un glb DN. Chemin rapide : une seule instance de maillage sans transformation (cas des
## glb générés) → on reprend le maillage tel quel, sans repasser par les sommets en GDScript ;
## sinon `FaunaLayer.flat_mesh_of` (nœuds aplatis).
static func mesh_of(path: String) -> ArrayMesh:
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var root := scene.instantiate()
	var meshes := root.find_children("*", "MeshInstance3D", true, false)
	var direct: ArrayMesh = null
	if meshes.size() == 1:
		var instance := meshes[0] as MeshInstance3D
		var xf := instance.transform
		var parent := instance.get_parent()
		while parent != null and parent != root and parent is Node3D:
			xf = (parent as Node3D).transform * xf
			parent = parent.get_parent()
		if root is Node3D:
			xf = (root as Node3D).transform * xf
		if xf.is_equal_approx(Transform3D.IDENTITY) and instance.mesh is ArrayMesh:
			direct = (instance.mesh as ArrayMesh).duplicate() as ArrayMesh
	root.free()
	return direct if direct != null else FaunaLayer.flat_mesh_of(path)


## Demande le chargement en tâche de fond de tous les niveaux de détail des modèles (décodage des
## textures hors du fil principal).
func _request_models() -> void:
	_warm_queue = (config.get("props", {}) as Dictionary).keys()
	for prop: Dictionary in (config.get("props", {}) as Dictionary).values():
		for path in _model_paths(prop):
			ResourceLoader.load_threaded_request(path)


func _model_paths(prop: Dictionary) -> Array[String]:
	var paths: Array[String] = []
	for level in MAX_LODS:
		var path := DN_DIR + "%s_lod%d.glb" % [str(prop["model"]), level]
		if ResourceLoader.exists(path):
			paths.append(path)
	return paths


## Prépare au plus un modèle par image, dès que tous ses niveaux sont chargés (jamais d'attente).
func _warm_step() -> void:
	for n in range(_warm_queue.size() - 1, -1, -1):
		var prop_id := str(_warm_queue[n])
		var ready := true
		var paths := _model_paths((config["props"] as Dictionary)[prop_id])
		for path in paths:
			if ResourceLoader.load_threaded_get_status(path) == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
				ready = false
		if not ready:
			continue
		_warm_queue.remove_at(n)
		var t_warm := Time.get_ticks_usec()
		for path in paths:
			ResourceLoader.load_threaded_get(path)
		_prop_state(prop_id)
		var took := (Time.get_ticks_usec() - t_warm) / 1000.0
		if took > float(stats["state_ms_max"]):
			stats["state_ms_max"] = took
			stats["state_slowest"] = prop_id
		return


## Albédo réduit à `texture_px` (les props font quelques dizaines de pixels à l'écran : 1024 px × 108
## niveaux de détail pèseraient ~450 Mo), avec mipmaps.
func _small_texture(source: Texture2D) -> Texture2D:
	var limit := int(_render("texture_px", 512.0))
	var image := source.get_image()
	if image == null:
		return source
	if image.is_compressed():
		image.decompress()
	if maxi(image.get_width(), image.get_height()) > limit:
		var ratio := float(limit) / float(maxi(image.get_width(), image.get_height()))
		image.resize(maxi(int(image.get_width() * ratio), 1), maxi(int(image.get_height() * ratio), 1), Image.INTERPOLATE_LANCZOS)
	image.generate_mipmaps()
	return ImageTexture.create_from_image(image)


func _set_uniform(state: Dictionary, uniform_name: String, value: Variant) -> void:
	for material: ShaderMaterial in state["materials"]:
		material.set_shader_parameter(uniform_name, value)


## Repli sans glb : boîte posée au sol (2 m).
static func _box_mesh() -> ArrayMesh:
	var box := BoxMesh.new()
	box.size = Vector3(2.0, 2.0, 2.0)
	var arrays := box.get_mesh_arrays()
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	for n in vertices.size():
		vertices[n] += Vector3(0.0, 1.0, 0.0)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.5, 0.42, 0.34)
	mesh.surface_set_material(0, material)
	return mesh


## Tampon de MultiMesh d'un modèle pour une liste d'instances (16 flottants par instance).
func pack_instances(instances: Array, prop_id: String) -> PackedFloat32Array:
	var state := _prop_state(prop_id)
	var unit := 1.0 / _mpp
	var count := 0
	for inst: Dictionary in instances:
		if inst["prop"] == prop_id:
			count += 1
	var buffer := PackedFloat32Array()
	buffer.resize(count * FLOATS_PER_INSTANCE)
	var o := 0
	for inst: Dictionary in instances:
		if inst["prop"] != prop_id:
			continue
		var at: Vector2 = inst["pos"]
		var yaw := float(inst["yaw"])
		var sx := unit
		if float(inst["len_m"]) > 0.0:
			sx = unit * float(inst["len_m"]) / float(state["length_x_m"])
		var c := cos(yaw)
		var s := sin(yaw)
		buffer[o + 0] = c * sx
		buffer[o + 2] = s * unit
		buffer[o + 3] = at.x
		buffer[o + 5] = unit
		buffer[o + 7] = maxf(_map_data.height_m_at(at.x, at.y), 0.0)
		buffer[o + 8] = -s * sx
		buffer[o + 10] = c * unit
		buffer[o + 11] = at.y
		var offset: Vector2 = inst["offset"]
		buffer[o + 12] = offset.x
		buffer[o + 13] = offset.y
		buffer[o + 14] = float(inst["profile"])
		buffer[o + 15] = float(inst["rank"])
		o += FLOATS_PER_INSTANCE
	return buffer


# --- Mise à jour ------------------------------------------------------------------------------


func _process(_delta: float) -> void:
	if _map_data == null or config.is_empty():
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	var focus_value: Variant = _rig.get("focus") if _rig != null else null
	var at := Vector2(focus_value.x, focus_value.z) if focus_value is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
	update_view(at, distance)


func _outer_distance() -> float:
	var outer := 0.0
	for prop: Dictionary in (config.get("props", {}) as Dictionary).values():
		outer = maxf(outer, float(prop.get("max_distance", _render("max_distance", 110.0))))
	return outer


## Cellules du disque autour de `at` construites (`cells_per_frame` par appel), visibilité, taille.
func update_view(at: Vector2, rig_distance: float) -> void:
	_frame += 1
	var active := (enabled and rig_distance < _outer_distance() and not _rules.is_empty()) or force_active
	if not active:
		if visible:
			visible = false
			stats["visible"] = 0
			stats["visible_cells"] = 0
			stats["draw_calls"] = 0
		return
	visible = true
	_warm_step()
	var radius := clampf(rig_distance * _render("load_factor", 2.4), _render("min_radius_px", 24.0), _render("max_radius_px", 200.0))
	var side := _cell_px()
	var lo := Vector2i(floori((at.x - radius) / side), floori((at.y - radius) / side))
	var hi := Vector2i(floori((at.x + radius) / side), floori((at.y + radius) / side))
	var wanted: Dictionary = {}
	var missing: Array[Vector2i] = []
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var key := Vector2i(cx, cy)
			var rect := _cell_rect(key)
			var nearest := Vector2(clampf(at.x, rect.position.x, rect.end.x), clampf(at.y, rect.position.y, rect.end.y))
			if nearest.distance_squared_to(at) > radius * radius or key.x < 0 or key.y < 0 or rect.position.x >= _map_data.size.x or rect.position.y >= _map_data.size.y:
				continue
			wanted[key] = true
			if not _cells.has(key):
				missing.append(key)
	missing.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return _cell_rect(a).get_center().distance_squared_to(at) < _cell_rect(b).get_center().distance_squared_to(at))
	for n in mini(missing.size(), int(_render("cells_per_frame", 1.0))):
		_install_cell(missing[n])
	var total := 0
	var shown_cells := 0
	var draws := 0
	for key: Vector2i in _cells:
		var entry: Dictionary = _cells[key]
		var shown := wanted.has(key)
		for node: MultiMeshInstance3D in entry["nodes"]:
			node.visible = shown
		if shown:
			entry["last_seen"] = _frame
			total += int(entry["count"])
			shown_cells += 1
			draws += (entry["nodes"] as Array).size()
	_evict()
	var budget := _render("max_visible_instances", 2500.0)
	var thin := 1.0 if total <= budget else budget / float(total)
	_apply_view(rig_distance, thin)
	stats["cells"] = _cells.size()
	stats["visible_cells"] = shown_cells
	stats["draw_calls"] = draws
	stats["visible"] = int(minf(float(total), budget))
	var sum := 0
	for entry: Dictionary in _cells.values():
		sum += int(entry["count"])
	stats["instances"] = sum


## Taille tenue à l'écran, fondu par modèle, amincissement et niveau de détail pour le rig.
func _apply_view(rig_distance: float, thin: float) -> void:
	var target := _render("size_k", 0.0045) * pow(maxf(rig_distance, 0.01), _render("size_exponent", 0.85))
	var spread_exponent := _render("spread_exponent", 0.5)
	var lod_distances: Array = (config["render"] as Dictionary)["lod_distances"]
	var lod := 0 if rig_distance < float(lod_distances[0]) else (1 if rig_distance < float(lod_distances[1]) else 2)
	for state: Dictionary in _states.values():
		var mult := clampf(target * float(state["size_scale"]) / maxf(float(state["real_units"]), 1e-9), 1.0, float(state["max_mult"]))
		var spread := pow(mult, spread_exponent)
		var fade := 1.0 - smoothstep(float(state["fade_from"]), float(state["max_distance"]), rig_distance)
		if force_active:
			fade = 1.0
		_set_uniform(state, "size_mult", mult)
		_set_uniform(state, "spread_mult", spread)
		_set_uniform(state, "x_mult", spread if state["stretched"] else mult)
		_set_uniform(state, "fade", fade)
		_set_uniform(state, "thin", thin)
		state["last_mult"] = mult
	if lod != _lod:
		_lod = lod
		for entry: Dictionary in _cells.values():
			for node: MultiMeshInstance3D in entry["nodes"]:
				var state: Dictionary = _states[str(node.get_meta("prop"))]
				node.multimesh.mesh = (state["lods"] as Array)[mini(lod, (state["lods"] as Array).size() - 1)]


## Construit les MultiMesh d'une cellule (un par modèle présent).
func _install_cell(key: Vector2i) -> void:
	var t0 := Time.get_ticks_usec()
	var instances := cell_instances(key)
	var entry := {"nodes": [], "count": 0, "last_seen": _frame}
	var by_prop: Dictionary = {}
	for inst: Dictionary in instances:
		by_prop[inst["prop"]] = true
	var side := _cell_px()
	var rect := _cell_rect(key)
	for prop_id: String in by_prop:
		var state := _prop_state(prop_id)
		var buffer := pack_instances(instances, prop_id)
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.use_custom_data = true
		var lods: Array = state["lods"]
		multimesh.mesh = lods[mini(maxi(_lod, 0), lods.size() - 1)]
		multimesh.instance_count = buffer.size() / FLOATS_PER_INSTANCE
		multimesh.buffer = buffer
		var margin := 8.0
		multimesh.custom_aabb = AABB(Vector3(rect.position.x - margin, -2.0, rect.position.y - margin), Vector3(side + 2.0 * margin, 120.0, side + 2.0 * margin))
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "Countryside_%s_%d_%d" % [prop_id, key.x, key.y]
		mmi.multimesh = multimesh
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		mmi.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		mmi.visible = false
		mmi.set_meta("prop", prop_id)
		add_child(mmi)
		(entry["nodes"] as Array).append(mmi)
		entry["count"] = int(entry["count"]) + multimesh.instance_count
	_cells[key] = entry
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)


func _evict() -> void:
	var limit := int(_render("max_cached_cells", 64.0))
	if _cells.size() <= limit:
		return
	var keys: Array = _cells.keys()
	keys.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return int(_cells[a]["last_seen"]) < int(_cells[b]["last_seen"]))
	for n in _cells.size() - limit:
		var entry: Dictionary = _cells[keys[n]]
		if int(entry["last_seen"]) == _frame:
			break
		for node: Node in entry["nodes"]:
			node.queue_free()
		_cells.erase(keys[n])
