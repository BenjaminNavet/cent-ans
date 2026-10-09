class_name WaterPropsLayer
extends Node3D

## Lot DN-FLEUVE : décors de rive en glb générés (rendu seulement) : moulins à eau sur les berges
## des lieux qui en ont un (`bld_water_mill`), jetées, grues et navires amarrés des ports,
## chantiers navals, épaves. Règles et modèles : `data/art/dn_water_models.json` (`DnWaterModels`).
## Un `MultiMesh` par modèle et par tuile de `TILE` unités (portée de visibilité par tuile, comme
## les maquettes de lieux) ; hauteurs recalées quand une tuile de terrain change de niveau.
## Table vide ou glb non importés : couche vide. Les villes et bourgs gardent leurs propres glb
## (camp-bati / port) : aucun entrepôt ni port-ville ajouté ici.

const TILE := 320.0
const GRID_CELL := 8.0
const SHORE_STEP := 0.5
const SHORE_DIRECTIONS := 24
const REGROUND_PER_FRAME := 160

var stats: Dictionary = {}

var _map_data: MapData = null
var _terrain: TerrainBuilder = null
## Lieux posés : {id, kind, px, heading, scale, height ("land" | "water" | "shore"), y, group, slot}.
var _places: Array[Dictionary] = []
var _groups: Array[Dictionary] = []
var _reground_cursor := -1
var _river_grid: Dictionary = {}


func setup(map_data: MapData, terrain: TerrainBuilder, settlements: SettlementData) -> void:
	_map_data = map_data
	_terrain = terrain
	if DnWaterModels.is_empty() or settlements == null or map_data == null:
		return
	_collect_ports(settlements)
	_collect_mills(settlements)
	_build_groups()
	var counts := {}
	for place in _places:
		counts[place["kind"]] = int(counts.get(place["kind"], 0)) + 1
	stats = {"places": _places.size(), "kinds": counts, "groups": _groups.size()}
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_changed)
	set_process(false)


## Les décors sont-ils visibles (même seuils que les bateaux) ?
func update_view(near_weight: float, normal_weight: float) -> void:
	visible = near_weight > 0.35 or normal_weight > 0.5


func place_count(kind: String = "") -> int:
	if kind == "":
		return _places.size()
	var total := 0
	for place in _places:
		if place["kind"] == kind:
			total += 1
	return total


func places() -> Array[Dictionary]:
	return _places


# --- Rivage ------------------------------------------------------------------------------------


func _land(p: Vector2) -> bool:
	return _map_data.is_land_px(int(p.x), int(p.y))


## Rivage le plus proche de `center` (jusqu'à `max_px`) : {shore: point de la côte, dir: vecteur
## unitaire vers la mer} ; {} si l'eau est hors de portée.
func shore_near(center: Vector2, max_px: float) -> Dictionary:
	var center_land := _land(center)
	var best := INF
	var result := {}
	for k in SHORE_DIRECTIONS:
		var angle := TAU * float(k) / SHORE_DIRECTIONS
		var v := Vector2(cos(angle), sin(angle))
		var d := SHORE_STEP
		while d <= minf(max_px, best):
			var p := center + v * d
			if _land(p) != center_land:
				best = d
				result = {"shore": center + v * (d - SHORE_STEP * 0.5), "dir": v if center_land else -v}
				break
			d += SHORE_STEP
	return result


func _add(id: String, kind: String, px: Vector2, heading: Vector2, height: String, scale_multiplier: float = 1.0) -> void:
	if id == "" or DnWaterModels.info(id, _lod()).is_empty():
		return
	_places.append({"id": id, "kind": kind, "px": px, "heading": heading, "scale": scale_multiplier, "height": height, "y": 0.0})


static func _lod() -> int:
	return int(DnWaterModels.default_value("lod_main", 1.0))


# --- Ports, chantiers, épaves ----------------------------------------------------------------


func _collect_ports(settlements: SettlementData) -> void:
	var ports_rule := DnWaterModels.section("ports")
	var by_kind: Dictionary = ports_rule.get("by_kind", {})
	var search := float(ports_rule.get("search_px", 14.0))
	var arsenals: Array = ports_rule.get("arsenal_places", [])
	var yards: Array = DnWaterModels.section("shipyards").get("places", [])
	var wreck_rule := DnWaterModels.section("wrecks")
	var port_ids: Array[String] = []
	var shores := {}
	for entry in settlements.settlements:
		var id := str(entry["id"])
		var is_port := bool(entry.get("port", false)) or (entry.get("initial_buildings", []) as Array).has("bld_port")
		if not is_port and not yards.has(id):
			continue
		var shore := shore_near(entry["px"], search)
		if shore.is_empty():
			continue
		shores[id] = shore
		if is_port:
			port_ids.append(id)
		_add_port(entry, shore, by_kind, arsenals, yards.has(id))
	var every := int(wreck_rule.get("every", 0))
	if every > 0:
		port_ids.sort()
		for n in port_ids.size():
			if n % every == 0:
				_add_wreck(shores[port_ids[n]], port_ids[n], str(wreck_rule.get("model", "")), float(wreck_rule.get("distance_px", 6.0)))


func _add_port(entry: Dictionary, shore: Dictionary, by_kind: Dictionary, arsenals: Array, shipyard: bool) -> void:
	var id := str(entry["id"])
	var kind := str(entry.get("kind", "village"))
	var coast: Vector2 = shore["shore"]
	var dir: Vector2 = shore["dir"]
	var side := Vector2(-dir.y, dir.x)
	var hash_value := absi(id.hash())
	var rule: Dictionary = by_kind.get(kind, {})
	var props: Array = rule.get("props", [])
	for prop in props:
		match str(prop):
			"port_pier":
				_add(str(prop), "port", coast + dir * 1.0, dir, "water")
			"port_crane":
				_add(str(prop), "port", coast - dir * 0.3 + side * 2.2, dir, "land")
			"port_beach":
				_add(str(prop), "port", coast + dir * 0.2, side, "shore")
			_:
				_add(str(prop), "port", coast, dir, "shore")
	if arsenals.has(id):
		_add("port_arsenal", "arsenal", coast - dir * 2.2 - side * 3.2, dir, "land")
	if shipyard:
		_add("shipyard_slip", "shipyard", coast - dir * 0.4 - side * 2.6, dir, "shore")
	var moored := int(rule.get("moored", 0))
	if moored > 0 and int(entry.get("weight", 0)) >= 40:
		moored += 1
	var ports_rule := DnWaterModels.section("ports")
	var by_basin: Dictionary = ports_rule.get("moored_by_basin", {})
	var basin := SeaBasins.basin_at(coast + dir * 2.0)
	var ids: Array = by_basin.get(basin, ports_rule.get("moored_default", []))
	for i in moored:
		var lateral := (1.0 if i % 2 == 0 else -1.0) * (2.4 + 1.8 * float(i / 2))
		var out := 3.8
		var at := coast + dir * out + side * lateral
		var tries := 0
		while _land(at) and tries < 3:
			out += 1.2
			at = coast + dir * out + side * lateral
			tries += 1
		if _land(at):
			continue
		var heading := (side * (1.0 if i % 2 == 0 else -1.0) + dir * 0.15 * float((hash_value + i) % 5 - 2)).normalized()
		_add(DnWaterModels.pick(ids, hash_value + i), "moored", at, heading, "water", float(ports_rule.get("moored_scale", 1.0)))


func _add_wreck(shore: Dictionary, id: String, model: String, distance: float) -> void:
	var coast: Vector2 = shore["shore"]
	var dir: Vector2 = shore["dir"]
	var side := Vector2(-dir.y, dir.x)
	var sign_value := 1.0 if absi(id.hash()) % 2 == 0 else -1.0
	var at := coast + side * distance * sign_value + dir * 0.8
	if _land(at):
		return
	_add(model, "wreck", at, Vector2(cos(float(absi(id.hash()) % 628) * 0.01), sin(float(absi(id.hash()) % 628) * 0.01)), "water")


# --- Moulins ----------------------------------------------------------------------------------


func _collect_mills(settlements: SettlementData) -> void:
	var rule := DnWaterModels.section("mills")
	if rule.is_empty():
		return
	var building := str(rule.get("building", "bld_water_mill"))
	var river_max := float(rule.get("river_max_px", 8.0))
	var coast_max := float(rule.get("coast_max_px", 0.0))
	var bank := float(rule.get("bank_offset_px", 0.9))
	var floating_places: Array = rule.get("floating_places", [])
	_build_river_grid()
	for entry in settlements.settlements:
		if not (entry.get("initial_buildings", []) as Array).has(building):
			continue
		var id := str(entry["id"])
		var hash_value := absi(id.hash())
		var town: Vector2 = entry["px"]
		if coast_max > 0.0:
			var shore := shore_near(town, coast_max)
			if not shore.is_empty():
				var tidal_dir: Vector2 = shore["dir"]
				_add(DnWaterModels.pick(rule.get("tidal", []), hash_value), "mill", (shore["shore"] as Vector2) - tidal_dir * 0.4, tidal_dir, "shore")
				continue
		var near := _nearest_river_point(town, river_max)
		if near.is_empty():
			continue
		var tangent: Vector2 = near["tangent"]
		var at: Vector2 = near["point"]
		if floating_places.has(id) and int(near["importance"]) >= int(rule.get("floating_min_importance", 6)):
			_add(DnWaterModels.pick(rule.get("floating", []), hash_value), "mill", at, tangent, "water")
			continue
		var normal := Vector2(-tangent.y, tangent.x)
		if normal.dot(town - at) < 0.0:
			normal = -normal
		var family := TownMaquetteData.family_of_province(str(entry.get("province", "")))
		var ids: Array = (rule.get("by_family", {}) as Dictionary).get(family, rule.get("default", []))
		_add(DnWaterModels.pick(ids, hash_value), "mill", at + normal * bank, tangent, "shore")


func _build_river_grid() -> void:
	_river_grid.clear()
	for river_index in _map_data.rivers.size():
		var points: PackedVector2Array = _map_data.rivers[river_index]["points"]
		for point_index in points.size():
			var key := Vector2i(int(points[point_index].x / GRID_CELL), int(points[point_index].y / GRID_CELL))
			if not _river_grid.has(key):
				_river_grid[key] = []
			(_river_grid[key] as Array).append(Vector2i(river_index, point_index))


## Point de fleuve le plus proche de `center` dans `max_px` : {point, tangent, importance}.
func _nearest_river_point(center: Vector2, max_px: float) -> Dictionary:
	var cell := Vector2i(int(center.x / GRID_CELL), int(center.y / GRID_CELL))
	var reach := ceili(max_px / GRID_CELL)
	var best := max_px * max_px
	var result := {}
	for dx in range(-reach, reach + 1):
		for dy in range(-reach, reach + 1):
			for ref: Vector2i in _river_grid.get(cell + Vector2i(dx, dy), []):
				var river: Dictionary = _map_data.rivers[ref.x]
				var points: PackedVector2Array = river["points"]
				var d := points[ref.y].distance_squared_to(center)
				if d < best:
					best = d
					var before := points[maxi(ref.y - 1, 0)]
					var after := points[mini(ref.y + 1, points.size() - 1)]
					var tangent := (after - before).normalized()
					result = {"point": points[ref.y], "tangent": tangent if tangent != Vector2.ZERO else Vector2.RIGHT, "importance": int(river["importance"])}
	return result


# --- MultiMesh -----------------------------------------------------------------------------------


func _build_groups() -> void:
	var by_key: Dictionary = {}
	for n in _places.size():
		var place := _places[n]
		var px: Vector2 = place["px"]
		var tile := Vector2i(int(floorf(px.x / TILE)), int(floorf(px.y / TILE)))
		var key := "%s|%d|%d" % [place["id"], tile.x, tile.y]
		if not by_key.has(key):
			by_key[key] = {"id": place["id"], "tile": tile, "members": []}
		(by_key[key]["members"] as Array).append(n)
	var range_end := DnWaterModels.default_value("prop_range", 420.0) + TILE * 0.7072
	for key: String in by_key:
		var spec: Dictionary = by_key[key]
		var info := DnWaterModels.info(str(spec["id"]), _lod())
		var members: Array = spec["members"]
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.mesh = info["mesh"]
		multimesh.instance_count = members.size()
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "Water_" + key.replace("|", "_").replace("-", "m")
		var tile: Vector2i = spec["tile"]
		mmi.position = Vector3((tile.x + 0.5) * TILE, 0.0, (tile.y + 0.5) * TILE)
		mmi.multimesh = multimesh
		mmi.visibility_range_end = range_end
		add_child(mmi)
		var group_index := _groups.size()
		_groups.append({"mmi": mmi, "info": info})
		for slot in members.size():
			_places[members[slot]]["group"] = group_index
			_places[members[slot]]["slot"] = slot
	for n in _places.size():
		_write(n)


func _height_of(place: Dictionary) -> float:
	var px: Vector2 = place["px"]
	var water := _map_data.surface_world_at(px.x, px.y)
	match str(place["height"]):
		"water":
			return water
		"land":
			return _terrain.surface_height_at(px.x, px.y) if _terrain != null else water
	return maxf(water, _terrain.surface_height_at(px.x, px.y)) if _terrain != null else water


func _write(n: int) -> void:
	var place := _places[n]
	var group: Dictionary = _groups[place["group"]]
	var mmi: MultiMeshInstance3D = group["mmi"]
	var px: Vector2 = place["px"]
	var y := _height_of(place)
	place["y"] = y
	var world := DnWaterModels.pose(group["info"], Vector3(px.x, y, px.y), place["heading"], float(place["scale"]))
	mmi.multimesh.set_instance_transform(int(place["slot"]), Transform3D(world.basis, world.origin - mmi.position))


func _on_chunk_changed(_index: int) -> void:
	if _places.is_empty():
		return
	_reground_cursor = 0
	set_process(true)


func _process(_delta: float) -> void:
	if _reground_cursor < 0:
		set_process(false)
		return
	var stop := mini(_reground_cursor + REGROUND_PER_FRAME, _places.size())
	for n in range(_reground_cursor, stop):
		_write(n)
	_reground_cursor = -1 if stop >= _places.size() else stop
