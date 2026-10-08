class_name DecorPlanner
extends RefCounted

## Lots DN ME6/ME7/ME9 : planification du décor ponctuel hors les villes (croix, gibets, péages,
## phares, mines, forges, mégalithes, ruines romaines, champs de bataille, pèlerins, foires…).
## Données dans `data/map/map_landmarks_extra.json` (schéma `map_landmarks_extra.schema.json`) :
## les règles `rules` placent des instances de façon déterministe (graine = graine du fichier x
## graine de la règle), les `sites` posent des lieux historiques réels (lonlat cuits en pixels).
## Modes de règle :
## - `road`     : le long des routes du graphe de colonies (`settlement_edge_paths.json`) ;
## - `gate`     : aux portes des villes, sur une route sortante ;
## - `crossing` : aux ponts (`crossings_px.json`) ;
## - `coast`    : le long des côtes (recul dans les terres, distance à une colonie) ;
## - `province` : par province selon ressources, terrain, climat, surface ;
## - `route`    : le long du plus court chemin entre des colonies (pèlerins, caravanes).
## Pure planification : aucun nœud, aucun rendu. Une instance est un dictionnaire
## `{type, px, yaw, rule, province, from_year?, to_year?, seasons?, site?}`.

const DATA_FILE := "map/map_landmarks_extra.json"
const CROSSINGS_FILE := "map/crossings_px.json"
const SETTLEMENT_CELL := 8.0
const DEDUP_CELL := 1.0
const SNAP_RADIUS := 7
const SAMPLE_TRIES := 40

var config: Dictionary = {}
var instances: Array = []
var stats: Dictionary = {}

var _map: MapData
var _data: SettlementData
var _province_info: Dictionary = {}
var _crossings: Array = []
var _rng := RandomNumberGenerator.new()
var _settlement_cells: Dictionary = {}
var _taken: Dictionary = {}
var _adjacency: Dictionary = {}
var _rule: Dictionary = {}


## Charge `data/map/map_landmarks_extra.json` ({} s'il manque).
static func load_config(data_dir: String) -> Dictionary:
	var path := data_dir.path_join(DATA_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


static func load_crossings(data_dir: String) -> Array:
	var path := data_dir.path_join(CROSSINGS_FILE)
	if not FileAccess.file_exists(path):
		return []
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return (parsed as Dictionary).get("crossings", []) if parsed is Dictionary else []


## Province -> `{resources, coastal, climate, terrain}` d'après `data/provinces/*.json`.
static func load_province_info(data_dir: String) -> Dictionary:
	var out := {}
	var dir := data_dir.path_join("provinces")
	for file in DirAccess.get_files_at(dir):
		if file.get_extension() != "json":
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(dir.path_join(file)))
		if parsed is Dictionary:
			var entry: Dictionary = parsed
			out[str(entry.get("id", file.get_basename()))] = {
				"resources": entry.get("resources", []),
				"coastal": bool(entry.get("coastal", false)),
				"climate": str(entry.get("climate", "")),
				"terrain": str(entry.get("terrain", "")),
			}
	return out


func setup(cfg: Dictionary, map: MapData, settlements: SettlementData, province_info: Dictionary, crossings: Array) -> void:
	config = cfg
	_map = map
	_data = settlements
	_province_info = province_info
	_crossings = crossings


## Calcule toutes les instances (sites d'abord, puis règles dans l'ordre du fichier).
func plan() -> Array:
	var t0 := Time.get_ticks_usec()
	instances = []
	_taken = {}
	_adjacency = {}
	_build_settlement_cells()
	for site: Dictionary in config.get("sites", []):
		_plan_site(site)
	for rule: Dictionary in config.get("rules", []):
		_rule = rule
		_rng.seed = int(config.get("seed", 1)) * 1000003 + int(rule.get("seed", 0))
		var before := instances.size()
		match str(rule.get("mode", "")):
			"road":
				_plan_roads()
			"gate":
				_plan_gates()
			"crossing":
				_plan_crossings()
			"coast":
				_plan_coast()
			"province":
				_plan_provinces()
			"route":
				_plan_route()
		stats[str(rule.get("id", ""))] = instances.size() - before
	stats["total"] = instances.size()
	stats["plan_ms"] = float(Time.get_ticks_usec() - t0) / 1000.0
	return instances


# --- Sites ---------------------------------------------------------------------------------


func _plan_site(site: Dictionary) -> void:
	var px := Vector2.ZERO
	if site.has("near"):
		var settlement := _data.get_settlement(str(site["near"]))
		if settlement.is_empty():
			return
		var offset: Array = site.get("offset_px", [0.0, 0.0])
		px = (settlement["px"] as Vector2) + Vector2(float(offset[0]), float(offset[1]))
	else:
		var at: Array = site.get("px", [])
		if at.size() < 2:
			return
		px = Vector2(float(at[0]), float(at[1]))
	px = _snap_to_land(px)
	if px.x < 0.0:
		return
	var yaw := deg_to_rad(float(site["yaw_deg"])) if site.has("yaw_deg") else float(hash(str(site["id"])) % 628) / 100.0
	var instance := _instance(str(site["type"]), px, yaw, str(site["id"]))
	instance["site"] = str(site["id"])
	for key in ["from_year", "to_year", "seasons"]:
		if site.has(key):
			instance[key] = site[key]
	_take(px)
	instances.append(instance)


## Pixel de terre le plus proche de `px` (rayon `SNAP_RADIUS`), Vector2(-1, -1) sinon.
func _snap_to_land(px: Vector2) -> Vector2:
	if _is_land(px):
		return px
	var best := Vector2(-1.0, -1.0)
	var best_distance := INF
	for dy in range(-SNAP_RADIUS, SNAP_RADIUS + 1):
		for dx in range(-SNAP_RADIUS, SNAP_RADIUS + 1):
			var candidate := px + Vector2(dx, dy)
			var d := candidate.distance_squared_to(px)
			if d < best_distance and _is_land(candidate):
				best_distance = d
				best = candidate
	return best


# --- Modes ---------------------------------------------------------------------------------


func _plan_roads() -> void:
	var seen := {}
	for key: String in _data.edge_paths:
		var ends := key.split("|")
		var reverse := "%s|%s" % [ends[1], ends[0]]
		if seen.has(reverse):
			continue
		seen[key] = true
		_along(_data.edge_paths[key])


## Pose sur une ligne brisée tous les `spacing_px` (tirage de départ aléatoire).
func _along(points: PackedVector2Array) -> void:
	var spacing := float(_rule.get("spacing_px", 12.0))
	var chance := float(_rule.get("chance", 1.0))
	var lateral: Array = _rule.get("lateral_px", [0.0, 0.0])
	var next_at := _rng.randf_range(0.2, 1.0) * spacing
	var walked := 0.0
	for i in range(points.size() - 1):
		var a := points[i]
		var b := points[i + 1]
		var length := a.distance_to(b)
		if length <= 1e-4:
			continue
		var tangent := (b - a) / length
		while next_at <= walked + length:
			var along := next_at - walked
			next_at += spacing * _rng.randf_range(0.7, 1.3)
			if _rng.randf() > chance:
				continue
			var normal := Vector2(-tangent.y, tangent.x)
			var side := -1.0 if _rng.randf() < 0.5 else 1.0
			var px := a + tangent * along + normal * side * _rng.randf_range(float(lateral[0]), float(lateral[1]))
			_try_emit(px, _yaw_along(tangent))
		walked += length


func _plan_gates() -> void:
	var kinds: Array = _rule.get("kinds", [])
	var distance: Array = _rule.get("distance_px", [1.0, 2.0])
	var chance := float(_rule.get("chance", 1.0))
	_build_adjacency()
	for settlement: Dictionary in _data.settlements:
		if not kinds.is_empty() and not kinds.has(str(settlement.get("kind", ""))):
			continue
		if _rng.randf() > chance:
			continue
		var id := str(settlement["id"])
		var start: Vector2 = settlement["px"]
		var wanted := _rng.randf_range(float(distance[0]), float(distance[1]))
		var edges: Array = _adjacency.get(id, [])
		var px := start
		var yaw := _rng.randf() * TAU
		if not edges.is_empty():
			var edge: Array = edges[_rng.randi() % edges.size()]
			var path := _oriented_path(edge[1], edge[3])
			var at := _point_at_distance(path, wanted)
			var tangent: Vector2 = at[1]
			px = (at[0] as Vector2) + Vector2(-tangent.y, tangent.x) * _rng.randf_range(0.05, 0.14)
			yaw = _yaw_along(tangent)
		else:
			px += Vector2.from_angle(yaw) * wanted
		_try_emit(px, yaw)


func _plan_crossings() -> void:
	var wanted: Array = _rule.get("crossing_types", ["bridge"])
	var offset := float(_rule.get("offset_px", 0.2))
	var chance := float(_rule.get("chance", 1.0))
	for crossing: Dictionary in _crossings:
		if not wanted.has(str(crossing.get("type", ""))):
			continue
		if _rng.randf() > chance:
			continue
		var at: Array = crossing.get("px", [])
		var dir: Array = crossing.get("dir", [1.0, 0.0])
		if at.size() < 2:
			continue
		var tangent := Vector2(float(dir[0]), float(dir[1])).normalized()
		var side := -1.0 if _rng.randf() < 0.5 else 1.0
		_try_emit(Vector2(float(at[0]), float(at[1])) + tangent * offset * side, _yaw_along(tangent))


func _plan_coast() -> void:
	var spacing := float(_rule.get("spacing_px", 8.0))
	var chance := float(_rule.get("chance", 1.0))
	var inland := float(_rule.get("inland_px", 0.3))
	for line in _map.coastlines:
		if line.size() < 2:
			continue
		var next_at := _rng.randf_range(0.2, 1.0) * spacing
		var walked := 0.0
		for i in range(line.size() - 1):
			var a := line[i]
			var b := line[i + 1]
			var length := a.distance_to(b)
			if length <= 1e-4:
				continue
			var tangent := (b - a) / length
			while next_at <= walked + length:
				var p := a + tangent * (next_at - walked)
				next_at += spacing * _rng.randf_range(0.7, 1.3)
				if _rng.randf() > chance:
					continue
				var normal := Vector2(-tangent.y, tangent.x)
				var px := p + normal * inland
				if not _is_land(px):
					px = p - normal * inland
				if _settlement_range_ok(px):
					_try_emit(px, _yaw_along(tangent))
			walked += length


## Distance à la colonie la plus proche (genres `settlement_kinds`) dans `settlement_px` ?
func _settlement_range_ok(px: Vector2) -> bool:
	var range_px: Array = _rule.get("settlement_px", [])
	if range_px.size() < 2:
		return true
	var kinds: Array = _rule.get("settlement_kinds", [])
	var nearest := INF
	var reach := int(ceilf(float(range_px[1]) / SETTLEMENT_CELL))
	var cell := Vector2i(floori(px.x / SETTLEMENT_CELL), floori(px.y / SETTLEMENT_CELL))
	for dy in range(-reach, reach + 1):
		for dx in range(-reach, reach + 1):
			for i: int in _settlement_cells.get(cell + Vector2i(dx, dy), []):
				var settlement: Dictionary = _data.settlements[i]
				if kinds.is_empty() or kinds.has(str(settlement.get("kind", ""))):
					nearest = minf(nearest, px.distance_to(settlement["px"]))
	return nearest >= float(range_px[0]) and nearest <= float(range_px[1])


func _plan_provinces() -> void:
	var per_province := float(_rule.get("per_province", 0.0))
	var per_area := float(_rule.get("per_area_px", 0.0))
	for index: int in _map.provinces:
		var province: Dictionary = _map.provinces[index]
		var info: Dictionary = _province_info.get(str(province["id"]), {})
		if info.is_empty() or not _province_matches(str(province["id"]), info):
			continue
		var expected := per_province + float(province.get("area_px", 0.0)) * per_area
		var count := int(expected) + (1 if _rng.randf() < expected - floorf(expected) else 0)
		if count <= 0:
			continue
		var bounds := _province_bounds(province)
		if bounds.size == Vector2.ZERO:
			continue
		var placed := 0
		for _try in count * SAMPLE_TRIES:
			if placed >= count:
				break
			var px := bounds.position + Vector2(_rng.randf() * bounds.size.x, _rng.randf() * bounds.size.y)
			if _map.province_index_at(px.x, px.y) != index:
				continue
			if _try_emit(px, _rng.randf() * TAU):
				placed += 1


func _province_bounds(province: Dictionary) -> Rect2:
	var rect := Rect2()
	var first := true
	for ring in province.get("rings", []):
		for point: Vector2 in ring:
			if first:
				rect = Rect2(point, Vector2.ZERO)
				first = false
			else:
				rect = rect.expand(point)
	return rect


func _province_matches(id: String, info: Dictionary) -> bool:
	var ids: Array = _rule.get("provinces", [])
	if not ids.is_empty() and not ids.has(id):
		return false
	var terrains: Array = _rule.get("terrains", [])
	if not terrains.is_empty() and not terrains.has(str(info.get("terrain", ""))):
		return false
	var climates: Array = _rule.get("climates", [])
	if not climates.is_empty() and not climates.has(str(info.get("climate", ""))):
		return false
	var resources: Array = info.get("resources", [])
	var any: Array = _rule.get("resources_any", [])
	if not any.is_empty() and not any.any(func(r: Variant) -> bool: return resources.has(r)):
		return false
	var every: Array = _rule.get("resources_all", [])
	if not every.is_empty() and not every.all(func(r: Variant) -> bool: return resources.has(r)):
		return false
	var by_resource: Array = _rule.get("provinces_resource", [])
	if not by_resource.is_empty() and not by_resource.any(func(r: Variant) -> bool: return resources.has(r)):
		return false
	return true


func _plan_route() -> void:
	var nodes: Array = _rule.get("nodes", [])
	if nodes.size() < 2:
		return
	_build_adjacency()
	var line := PackedVector2Array()
	for i in range(nodes.size() - 1):
		var steps := _shortest_path(str(nodes[i]), str(nodes[i + 1]))
		if steps.is_empty():  # colonies sans route commune : segment droit
			var from_px: Vector2 = _data.get_settlement(str(nodes[i])).get("px", Vector2.ZERO)
			var to_px: Vector2 = _data.get_settlement(str(nodes[i + 1])).get("px", Vector2.ZERO)
			line.append_array(PackedVector2Array([from_px, to_px]))
		for step: Array in steps:
			var path := _oriented_path(step[0], step[1])
			if line.size() > 0 and path.size() > 0 and line[line.size() - 1].distance_to(path[0]) < 0.01:
				path = path.slice(1)
			line.append_array(path)
	if line.size() >= 2:
		_along(line)


# --- Graphe des routes ---------------------------------------------------------------------


func _build_adjacency() -> void:
	if not _adjacency.is_empty():
		return
	for key: String in _data.edge_paths:
		var ends := key.split("|")
		var length := _length_of(_data.edge_paths[key])
		for pair in [[ends[0], ends[1], false], [ends[1], ends[0], true]]:
			if not _adjacency.has(pair[0]):
				_adjacency[pair[0]] = []
			(_adjacency[pair[0]] as Array).append([pair[1], key, length, pair[2]])


static func _length_of(points: PackedVector2Array) -> float:
	var total := 0.0
	for i in range(points.size() - 1):
		total += points[i].distance_to(points[i + 1])
	return total


func _oriented_path(key: String, reversed: bool) -> PackedVector2Array:
	var path: PackedVector2Array = _data.edge_paths.get(key, PackedVector2Array())
	if not reversed:
		return path
	var out := PackedVector2Array()
	for i in range(path.size() - 1, -1, -1):
		out.append(path[i])
	return out


## Chemin le plus court (liste de `[clé d'arête, inversée]`) ; Dijkstra à frontière.
func _shortest_path(from_id: String, to_id: String) -> Array:
	if from_id == to_id or not _adjacency.has(from_id):
		return []
	var best := {from_id: 0.0}
	var came := {}
	var frontier := {from_id: 0.0}
	var done := {}
	while not frontier.is_empty():
		var current := ""
		var current_cost := INF
		for id: String in frontier:
			if float(frontier[id]) < current_cost:
				current_cost = float(frontier[id])
				current = id
		frontier.erase(current)
		if current == to_id:
			break
		done[current] = true
		for edge: Array in _adjacency.get(current, []):
			var other: String = edge[0]
			if done.has(other):
				continue
			var cost := current_cost + float(edge[2])
			if cost < float(best.get(other, INF)):
				best[other] = cost
				came[other] = [current, edge[1], edge[3]]
				frontier[other] = cost
	var steps: Array = []
	var at := to_id
	while came.has(at):
		var entry: Array = came[at]
		steps.push_front([entry[1], entry[2]])
		at = entry[0]
	return steps if at == from_id else []


## `[point, tangente]` à la distance `distance` du début de `path` (borné à la fin).
static func _point_at_distance(path: PackedVector2Array, distance: float) -> Array:
	var walked := 0.0
	for i in range(path.size() - 1):
		var length := path[i].distance_to(path[i + 1])
		if length > 1e-4 and walked + length >= distance:
			var tangent := (path[i + 1] - path[i]) / length
			return [path[i] + tangent * (distance - walked), tangent]
		walked += length
	if path.size() >= 2:
		var tail := (path[path.size() - 1] - path[path.size() - 2]).normalized()
		return [path[path.size() - 1], tail]
	return [Vector2.ZERO, Vector2.RIGHT]


# --- Filtres et émission -------------------------------------------------------------------


## Rotation autour de Y qui aligne l'axe X du modèle sur la tangente (x, z carte).
static func _yaw_along(tangent: Vector2) -> float:
	return atan2(-tangent.y, tangent.x)


func _is_land(px: Vector2) -> bool:
	return px.x >= 1.0 and px.y >= 1.0 and px.x < _map.size.x - 1 and px.y < _map.size.y - 1 and _map.is_land_px(int(px.x), int(px.y))


func _build_settlement_cells() -> void:
	_settlement_cells = {}
	for i in _data.settlements.size():
		var px: Vector2 = _data.settlements[i]["px"]
		var cell := Vector2i(floori(px.x / SETTLEMENT_CELL), floori(px.y / SETTLEMENT_CELL))
		if not _settlement_cells.has(cell):
			_settlement_cells[cell] = []
		(_settlement_cells[cell] as Array).append(i)


func _settlement_distance(px: Vector2, limit: float) -> float:
	var nearest := INF
	var reach := int(ceilf(limit / SETTLEMENT_CELL))
	var cell := Vector2i(floori(px.x / SETTLEMENT_CELL), floori(px.y / SETTLEMENT_CELL))
	for dy in range(-reach, reach + 1):
		for dx in range(-reach, reach + 1):
			for i: int in _settlement_cells.get(cell + Vector2i(dx, dy), []):
				nearest = minf(nearest, px.distance_to(_data.settlements[i]["px"]))
	return nearest


## Filtres communs de la règle courante (terre, fleuve, altitude, colonies, province).
func _accepts(px: Vector2) -> bool:
	if not _is_land(px):
		return false
	var river := _map.river_sd_at(px.x, px.y)
	if river < 0.4 and not bool(_rule.get("allow_river", false)):
		return false
	var height := _map.height_m_at(px.x, px.y)
	if height < float(_rule.get("min_height_m", -INF)) or height > float(_rule.get("max_height_m", INF)):
		return false
	var minimum := float(_rule.get("min_settlement_px", 0.6))
	if minimum > 0.0 and _settlement_distance(px, minimum) < minimum:
		return false
	if _rule.has("near_river_px") and river > float(_rule["near_river_px"]):
		return false
	if _rule.has("provinces") or _rule.has("terrains") or _rule.has("climates") or _rule.has("provinces_resource"):
		var province := _map.get_province(_map.province_index_at(px.x, px.y))
		if province.is_empty():
			return false
		var info: Dictionary = _province_info.get(str(province["id"]), {})
		if info.is_empty() or not _province_matches(str(province["id"]), info):
			return false
	return true


func _take(px: Vector2) -> void:
	_taken[Vector2i(floori(px.x / DEDUP_CELL), floori(px.y / DEDUP_CELL))] = true


func _is_taken(px: Vector2) -> bool:
	return _taken.has(Vector2i(floori(px.x / DEDUP_CELL), floori(px.y / DEDUP_CELL)))


func _try_emit(px: Vector2, yaw: float) -> bool:
	if not _accepts(px) or _is_taken(px):
		return false
	var instance := _instance(_pick_type(), px, yaw, str(_rule.get("id", "")))
	for key in ["from_year", "to_year", "seasons"]:
		if _rule.has(key):
			instance[key] = _rule[key]
	_take(px)
	instances.append(instance)
	return true


func _instance(type: String, px: Vector2, yaw: float, rule_id: String) -> Dictionary:
	var province := _map.get_province(_map.province_index_at(px.x, px.y))
	return {"type": type, "px": px, "yaw": yaw, "rule": rule_id, "province": str(province.get("id", ""))}


func _pick_type() -> String:
	var weights: Dictionary = _rule.get("types", {})
	var total := 0.0
	for type: String in weights:
		total += float(weights[type])
	var roll := _rng.randf() * total
	for type: String in weights:
		roll -= float(weights[type])
		if roll <= 0.0:
			return type
	return str(weights.keys()[0])
