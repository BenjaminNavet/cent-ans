class_name LifeAmbient
extends Node3D

## Lot CV1 : vie ambiante de la carte (rendu seulement) :
## - oiseaux (lot ME3 : `MapBirdFlocks`, espèces/habitats/saisons dans `data/map/map_birds.json`) ;
## - bateaux sur les grands fleuves (va-et-vient le long du tracé) et navires sur la Manche et
##   les côtes (lignes entre ports voisins dont le trajet reste en mer).
## Palier près (oiseaux) et près/moyen (bateaux) ; un `MultiMesh` par famille.

## Importance minimale (Strahler) d'un fleuve navigable et longueur par bateau (px carte).
const RIVER_MIN_IMPORTANCE := 5
const RIVER_PX_PER_BOAT := 260.0
const MAX_RIVER_BOATS := 40
const MAX_SEA_LANES := 20
const SEA_LANE_MIN_PX := 35.0
const SEA_LANE_MAX_PX := 260.0
const RIVER_BOAT_SCALE := 0.8
const SEA_SHIP_SCALE := 1.5
## Vitesse (px carte par seconde).
const RIVER_SPEED := 0.9
const SEA_SPEED := 2.2

var stats: Dictionary = {}

var _map_data: MapData = null
var _terrain: TerrainBuilder = null
var _birds: MapBirdFlocks
var _boats: MultiMeshInstance3D
var _ships: MultiMeshInstance3D
## Trajets : {points: PackedVector2Array, length: float, cumulative: PackedFloat32Array,
## offset: float, speed: float}.
var _river_routes: Array = []
var _sea_routes: Array = []
var _time := 0.0
var _rng := RandomNumberGenerator.new()


func setup(map_data: MapData, terrain: TerrainBuilder, settlements: SettlementData) -> void:
	_map_data = map_data
	_terrain = terrain
	_rng.seed = 1337
	_birds = MapBirdFlocks.new()
	_birds.name = "Birds"
	add_child(_birds)
	_birds.setup(map_data, terrain, settlements)
	_river_routes = _build_river_routes()
	_sea_routes = _build_sea_routes(settlements)
	_boats = _route_instance("RiverBoats", _river_routes.size(), RIVER_BOAT_SCALE)
	_ships = _route_instance("SeaShips", _sea_routes.size(), SEA_SHIP_SCALE)
	stats = {"birds": int(_birds.stats.get("total", 0)), "river_boats": _river_routes.size(), "sea_ships": _sea_routes.size()}


# --- Bateaux -------------------------------------------------------------------------


static func _route(points: PackedVector2Array, offset: float, speed: float) -> Dictionary:
	var cumulative := PackedFloat32Array([0.0])
	for i in range(1, points.size()):
		cumulative.append(cumulative[i - 1] + points[i].distance_to(points[i - 1]))
	return {"points": points, "length": cumulative[-1], "cumulative": cumulative, "offset": offset, "speed": speed}


func _build_river_routes() -> Array:
	var routes: Array = []
	if _map_data == null:
		return routes
	for river in _map_data.rivers:
		if int(river["importance"]) < RIVER_MIN_IMPORTANCE:
			continue
		var points: PackedVector2Array = PackedVector2Array(river["points"])
		if points.size() < 2:
			continue
		var route := _route(points, 0.0, RIVER_SPEED)
		var boats := int(route["length"] / RIVER_PX_PER_BOAT)
		for k in boats:
			if routes.size() >= MAX_RIVER_BOATS:
				return routes
			routes.append(_route(points, _rng.randf() * 2.0 * route["length"], RIVER_SPEED * _rng.randf_range(0.7, 1.2)))
	return routes


## Lignes entre ports voisins dont le trajet reste en mer (hors 5 px autour des ports).
func _build_sea_routes(settlements: SettlementData) -> Array:
	var routes: Array = []
	if settlements == null or _map_data == null:
		return routes
	var ports: Array[Vector2] = []
	for entry in settlements.settlements:
		if bool(entry.get("port", false)):
			ports.append(entry["px"])
	for i in ports.size():
		for j in range(i + 1, ports.size()):
			if routes.size() >= MAX_SEA_LANES:
				return routes
			var a: Vector2 = ports[i]
			var b: Vector2 = ports[j]
			var d := a.distance_to(b)
			if d < SEA_LANE_MIN_PX or d > SEA_LANE_MAX_PX or not _at_sea(a, b):
				continue
			routes.append(_route(PackedVector2Array([a, b]), _rng.randf() * 2.0 * d, SEA_SPEED * _rng.randf_range(0.8, 1.2)))
	return routes


func _at_sea(a: Vector2, b: Vector2) -> bool:
	var d := a.distance_to(b)
	var steps := int(d / 2.0)
	var water := 0
	for s in range(1, steps):
		var p := a.lerp(b, float(s) / steps)
		if p.distance_to(a) < 5.0 or p.distance_to(b) < 5.0:
			continue
		if _map_data.is_land_px(int(p.x), int(p.y)):
			return false
		water += 1
	return water > 4


func _route_instance(node_name: String, count: int, model_scale: float) -> MultiMeshInstance3D:
	var mmi := MultiMeshInstance3D.new()
	mmi.name = node_name
	mmi.extra_cull_margin = _world_margin()
	add_child(mmi)
	var mesh := LifeEffects._first_mesh("ship")
	if mesh == null or count == 0:
		return mmi
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.mesh = mesh
	multimesh.instance_count = count
	mmi.multimesh = multimesh
	mmi.set_meta("model_scale", model_scale)
	return mmi


## Position et cap le long d'un trajet en va-et-vient.
static func route_pose(route: Dictionary, time: float) -> Array:
	var length: float = route["length"]
	if length <= 0.0:
		return [route["points"][0], Vector2.RIGHT]
	var travelled := fposmod(float(route["offset"]) + time * float(route["speed"]), 2.0 * length)
	var forward := travelled <= length
	var s := travelled if forward else 2.0 * length - travelled
	var cumulative: PackedFloat32Array = route["cumulative"]
	var points: PackedVector2Array = route["points"]
	var i := clampi(cumulative.bsearch(s) - 1, 0, points.size() - 2)
	var seg := maxf(cumulative[i + 1] - cumulative[i], 1e-4)
	var t := clampf((s - cumulative[i]) / seg, 0.0, 1.0)
	var direction := (points[i + 1] - points[i]).normalized()
	return [points[i].lerp(points[i + 1], t), direction if forward else -direction]


func _update_routes(mmi: MultiMeshInstance3D, routes: Array, focus: Vector2, radius: float, on_water: bool) -> void:
	if mmi.multimesh == null:
		return
	var model_scale: float = mmi.get_meta("model_scale", 1.0)
	for n in routes.size():
		var pose := route_pose(routes[n], _time)
		var p: Vector2 = pose[0]
		if p.distance_squared_to(focus) > radius * radius:
			# Hors de vue : rangé sous le sol, pas de calcul de hauteur.
			mmi.multimesh.set_instance_transform(n, Transform3D(Basis.from_scale(Vector3.ONE * 0.001), Vector3(p.x, -50.0, p.y)))
			continue
		var heading: Vector2 = pose[1]
		var y := _map_data.surface_world_at(p.x, p.y) if on_water else (_terrain.surface_height_at(p.x, p.y) if _terrain != null else 0.0)
		# Le modèle de cogue regarde +X (voir ModelLibrary, marqueurs d'armée embarquée).
		var basis := Basis(Vector3.UP, -atan2(heading.y, heading.x)).scaled(Vector3.ONE * model_scale)
		mmi.multimesh.set_instance_transform(n, Transform3D(basis, Vector3(p.x, y + 0.05, p.y)))


## DV : `normal_weight` = poids de la vue normale (1 − `ZoomTiers.strategic_weight`).
func update_view(focus: Vector2, near_weight: float, normal_weight: float, season_weights: Vector4 = Vector4(0, 1, 0, 0)) -> void:
	_time += get_process_delta_time()
	_birds.update_view(focus, near_weight > 0.35, season_weights)
	var show_boats := near_weight > 0.35 or normal_weight > 0.5
	_boats.visible = show_boats
	_ships.visible = show_boats
	if show_boats:
		var radius := 220.0 if near_weight > 0.35 else 480.0
		_update_routes(_boats, _river_routes, focus, radius, false)
		_update_routes(_ships, _sea_routes, focus, radius, true)


## Marge d'élagage couvrant tout le monde (instances réparties sur la carte, ADR 0115).
func _world_margin() -> float:
	if _map_data == null:
		return 16384.0
	return float(maxi(_map_data.size.x, _map_data.size.y))
