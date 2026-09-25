class_name LifeAmbient
extends Node3D

## Lot CV1 : vie ambiante de la carte (rendu seulement) :
## - vols d'oiseaux autour du point visé par la caméra (réserve de vols recentrés quand la vue se
##   déplace ; orbite et battement d'ailes dans `life_birds.gdshader`) ;
## - bateaux sur les grands fleuves (va-et-vient le long du tracé) et navires sur la Manche et
##   les côtes (lignes entre ports voisins dont le trajet reste en mer).
## Palier près (oiseaux) et près/moyen (bateaux) ; un `MultiMesh` par famille.

const BIRDS_SHADER := preload("res://shaders/life_birds.gdshader")
const FLOCKS := 8
const BIRDS_PER_FLOCK := 9
## Rayon (px carte) autour du point visé où vivent les vols.
const FLOCK_AREA := 70.0
## Importance minimale (Strahler) d'un fleuve navigable et longueur par bateau (px carte).
const RIVER_MIN_IMPORTANCE := 5
const RIVER_PX_PER_BOAT := 120.0
const MAX_RIVER_BOATS := 90
const MAX_SEA_LANES := 40
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
var _birds: MultiMeshInstance3D
var _boats: MultiMeshInstance3D
var _ships: MultiMeshInstance3D
var _flock_centers: Array[Vector2] = []
var _flock_timer := 0.0
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
	_build_birds()
	_river_routes = _build_river_routes()
	_sea_routes = _build_sea_routes(settlements)
	_boats = _route_instance("RiverBoats", _river_routes.size(), RIVER_BOAT_SCALE)
	_ships = _route_instance("SeaShips", _sea_routes.size(), SEA_SHIP_SCALE)
	stats = {"birds": FLOCKS * BIRDS_PER_FLOCK, "river_boats": _river_routes.size(), "sea_ships": _sea_routes.size()}


# --- Oiseaux -------------------------------------------------------------------------


static func _bird_mesh() -> ArrayMesh:
	# V aplati : deux ailes en triangles, corps au centre (bec vers -Z).
	var vertices := PackedVector3Array([
		Vector3(0, 0, -0.25), Vector3(-1.0, 0.08, 0.15), Vector3(0, 0, 0.2),
		Vector3(0, 0, -0.25), Vector3(0, 0, 0.2), Vector3(1.0, 0.08, 0.15),
	])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _build_birds() -> void:
	var material := ShaderMaterial.new()
	material.shader = BIRDS_SHADER
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = _bird_mesh()
	multimesh.instance_count = FLOCKS * BIRDS_PER_FLOCK
	_birds = MultiMeshInstance3D.new()
	_birds.name = "Birds"
	_birds.multimesh = multimesh
	_birds.material_override = material
	_birds.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_birds.extra_cull_margin = 4096.0
	add_child(_birds)
	_flock_centers.resize(FLOCKS)
	_flock_centers.fill(Vector2(-1e6, -1e6))


func _place_flock(f: int, center: Vector2) -> void:
	_flock_centers[f] = center
	var ground := _terrain.surface_height_at(center.x, center.y) if _terrain != null else 0.0
	var base_height := 5.0 + _rng.randf() * 6.0
	var direction := 1.0 if _rng.randf() < 0.5 else -1.0
	var radius := 4.0 + _rng.randf() * 8.0
	for b in BIRDS_PER_FLOCK:
		var n := f * BIRDS_PER_FLOCK + b
		var jitter := Vector3(_rng.randf_range(-1.2, 1.2), 0.0, _rng.randf_range(-1.2, 1.2))
		var origin := Vector3(center.x, ground + base_height, center.y) + jitter
		_birds.multimesh.set_instance_transform(n, Transform3D(Basis.from_scale(Vector3.ONE * 0.5), origin))
		# Même vitesse dans un vol, phases proches : ils volent ensemble.
		_birds.multimesh.set_instance_custom_data(n, Color(radius + _rng.randf() * 0.8, direction * (0.22 + 0.04 * float(f % 3)), 0.1 * f + 0.012 * b, _rng.randf() * 0.8))


func _update_birds(focus: Vector2, visible_birds: bool) -> void:
	_birds.visible = visible_birds
	if not visible_birds:
		return
	_flock_timer -= get_process_delta_time()
	if _flock_timer > 0.0:
		return
	_flock_timer = 1.5
	for f in FLOCKS:
		if _flock_centers[f].distance_to(focus) > FLOCK_AREA * 1.3:
			var angle := _rng.randf() * TAU
			var place := focus + Vector2(cos(angle), sin(angle)) * FLOCK_AREA * sqrt(_rng.randf())
			if _map_data == null or _map_data.is_land_px(int(place.x), int(place.y)):
				_place_flock(f, place)


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
	mmi.extra_cull_margin = 4096.0
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


func update_view(focus: Vector2, near_weight: float, medium_weight: float) -> void:
	_time += get_process_delta_time()
	_update_birds(focus, near_weight > 0.35)
	var show_boats := near_weight > 0.35 or medium_weight > 0.5
	_boats.visible = show_boats
	_ships.visible = show_boats
	if show_boats:
		var radius := 220.0 if near_weight > 0.35 else 480.0
		_update_routes(_boats, _river_routes, focus, radius, false)
		_update_routes(_ships, _sea_routes, focus, radius, true)
