class_name FolkCaravans
extends RefCounted

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.2) : marchands en
## charrette sur les routes commerciales. Rendu seulement : lit `get_trade_routes` du pont (les
## routes de `TradeRouteLayer` : chemin de colonies, valeur, état coupé), une fois par tour.
##
## - Route **active** seulement (`cut` faux) : route coupée (guerre, embargo) → aucune charrette.
## - Tracé de chaque étape (deux colonies successives du chemin) : le tracé routier de l'arête
##   (`SettlementData.edge_path`, celui des rubans de `RoadRenderer`) s'il existe, sinon le
##   segment droit ; une étape surtout en mer est laissée aux navires de `LifeAmbient`.
## - Charrettes bâchées par 10 unités de route selon la valeur (`carts_per_value`), dans les deux
##   sens, phases décalées ; chacune avec son charretier, un marchand à pied et, au-delà de
##   `guard_value`, un ou deux gardes.
## - Indépendant de la couche des routes commerciales (touche V) : c'est de la vie.

const CELL := 32.0
## Longueur maximale d'un trajet rectiligne.
const PIECE := 6.0
## Charrettes par 10 unités de route et par unité de valeur ; bornes.
const CARTS_PER_VALUE := 0.02
const MIN_CARTS_PER_10 := 0.3
const MAX_CARTS_PER_10 := 3.0
## Valeur au-delà de laquelle la charrette a un garde (deux au double).
const GUARD_VALUE := 40.0
## Part du plafond restant accordée aux marchands.
const SHARE := 0.4
## Part d'eau (échantillons) au-delà de laquelle une étape droite est maritime.
const SEA_SHARE := 0.3

var stats: Dictionary = {}
var off: bool = false

var _map_data: MapData = null
var _settlements: SettlementData = null
## Trajets rectilignes : extrémités, densité (charrettes / 10 unités), valeur, route.
var _piece_a := PackedVector2Array()
var _piece_b := PackedVector2Array()
var _piece_rate := PackedFloat32Array()
var _piece_value := PackedFloat32Array()
var _piece_route := PackedStringArray()
var _cells: Dictionary = {}
var _pool: FolkPool = null
var _routes_key: String = ""


func setup(map_data: MapData, settlement_data: SettlementData) -> void:
	_map_data = map_data
	_settlements = settlement_data


func refresh(sim: Object) -> void:
	if sim == null:
		return
	if not sim.has_method("get_trade_routes"):
		set_routes([])
		return
	set_routes(sim.call("get_trade_routes"))


## Routes du pont (`get_trade_routes`) ; reconstruit les trajets si elles ont changé.
func set_routes(routes: Array) -> void:
	var parts := PackedStringArray()
	for route in routes:
		parts.append("%s:%d:%d" % [route.get("id", ""), int(bool(route.get("cut", false))), int(float(route.get("total_value", 0)))])
	var key := ",".join(parts)
	if key == _routes_key and not _piece_a.is_empty():
		return
	_routes_key = key
	_piece_a = PackedVector2Array()
	_piece_b = PackedVector2Array()
	_piece_rate = PackedFloat32Array()
	_piece_value = PackedFloat32Array()
	_piece_route = PackedStringArray()
	_cells = {}
	var active := 0
	var cut := 0
	var sea_legs := 0
	for route in routes:
		if bool(route.get("cut", false)):
			cut += 1
			continue
		var path: PackedStringArray = PackedStringArray(route.get("path", PackedStringArray()))
		if path.size() < 2:
			continue
		active += 1
		var value := float(route.get("total_value", 0))
		var rate := clampf(value * _carts_per_value(), MIN_CARTS_PER_10, MAX_CARTS_PER_10)
		for i in path.size() - 1:
			var leg := _leg(path[i], path[i + 1])
			if leg.is_empty():
				sea_legs += 1
				continue
			_add_leg(leg, rate, value, str(route.get("id", "")))
	stats = {"active_routes": active, "cut_routes": cut, "sea_legs": sea_legs, "pieces": _piece_a.size()}


func _carts_per_value() -> float:
	return float(_pool.settings.get("carts_per_value", CARTS_PER_VALUE)) if _pool != null else CARTS_PER_VALUE


## Tracé d'une étape (carte) : tracé routier, sinon segment droit ; vide si maritime ou inconnu.
func _leg(from_id: String, to_id: String) -> PackedVector2Array:
	if _settlements == null:
		return PackedVector2Array()
	var road := _settlements.edge_path(from_id, to_id)
	if road.size() >= 2:
		return road
	var a: Variant = _settlements.get_settlement(from_id).get("px")
	var b: Variant = _settlements.get_settlement(to_id).get("px")
	if not (a is Vector2) or not (b is Vector2):
		return PackedVector2Array()
	if _at_sea(a, b):
		return PackedVector2Array()
	return PackedVector2Array([a, b])


func _at_sea(a: Vector2, b: Vector2) -> bool:
	if _map_data == null:
		return false
	var steps := maxi(int(a.distance_to(b) / 2.0), 2)
	var water := 0
	for s in range(1, steps):
		var p := a.lerp(b, float(s) / steps)
		if not _map_data.is_land_px(int(p.x), int(p.y)):
			water += 1
	return float(water) / float(steps - 1) > SEA_SHARE


func _add_leg(points: PackedVector2Array, rate: float, value: float, route_id: String) -> void:
	for i in points.size() - 1:
		var a := points[i]
		var b := points[i + 1]
		var pieces := maxi(ceili(a.distance_to(b) / PIECE), 1)
		for k in pieces:
			var pa := a.lerp(b, float(k) / pieces)
			var pb := a.lerp(b, float(k + 1) / pieces)
			var index := _piece_a.size()
			_piece_a.append(pa)
			_piece_b.append(pb)
			_piece_rate.append(rate)
			_piece_value.append(value)
			_piece_route.append(route_id)
			var key := Vector2i(int(floorf((pa.x + pb.x) * 0.5 / CELL)), int(floorf((pa.y + pb.y) * 0.5 / CELL)))
			if not _cells.has(key):
				_cells[key] = PackedInt32Array()
			var list: PackedInt32Array = _cells[key]
			list.append(index)
			_cells[key] = list


## Nombre de trajets (tests).
func piece_count() -> int:
	return _piece_a.size()


## Identifiants des routes qui ont au moins un trajet (tests : aucune route coupée).
func routes_with_pieces() -> PackedStringArray:
	var seen := {}
	for id in _piece_route:
		seen[id] = true
	return PackedStringArray(seen.keys())


static func _h(a: int, b: int, c: int = 0) -> float:
	return VegetationFields.hash01(a * 73856093 ^ b * 19349663 ^ c * 83492791)


func populate(pool: FolkPool, focus: Vector2, radius: float) -> void:
	_pool = pool
	if off or _piece_a.is_empty():
		stats["carts"] = 0
		return
	var budget := int(pool.remaining() * SHARE)
	var found: Array = []
	var lo := Vector2i(int(floorf((focus.x - radius) / CELL)), int(floorf((focus.y - radius) / CELL)))
	var hi := Vector2i(int(floorf((focus.x + radius) / CELL)), int(floorf((focus.y + radius) / CELL)))
	var r2 := radius * radius
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var list: Variant = _cells.get(Vector2i(cx, cy))
			if list == null:
				continue
			for index in list:
				var d2 := ((_piece_a[index] + _piece_b[index]) * 0.5).distance_squared_to(focus)
				if d2 <= r2:
					found.append([d2, index])
	found.sort_custom(func(x: Array, y: Array) -> bool: return x[0] < y[0])
	var carts := 0
	var figures := 0
	for entry in found:
		if figures >= budget:
			break
		var index: int = entry[1]
		var a := _piece_a[index]
		var b := _piece_b[index]
		var expected := a.distance_to(b) / 10.0 * _piece_rate[index]
		var n := int(expected) + (1 if _h(index, 0, 1) < fposmod(expected, 1.0) else 0)
		for k in n:
			if figures >= budget:
				break
			var placed := _caravan(index, k, a, b, _piece_value[index])
			if placed > 0:
				carts += 1
				figures += placed
	stats["carts"] = carts
	stats["figures"] = figures


## Une charrette et sa suite ; renvoie le nombre de figurines posées (0 si la charrette ne
## trouve pas de place).
func _caravan(index: int, k: int, a: Vector2, b: Vector2, value: float) -> int:
	# Deux sens, phases décalées ; chacun tient sa droite.
	var forward := (k % 2 == 0) == (_h(index, k, 2) < 0.5)
	var from := a if forward else b
	var to := b if forward else a
	var phase := _h(index, k, 3)
	var side := 0.9
	if not _pool.add("merchant_cart", "roll", from, to, phase, side):
		return 0
	# Places de la suite : `slots` du manifeste FK2 (charretier près du cheval, marchand au flanc,
	# garde en queue) ; (latéral, avance) en mètres, repli pour la maquette.
	var carter := FolkModels.slot("merchant_cart", "carter", Vector2(1.1, 1.0))
	var merchant := FolkModels.slot("merchant_cart", "merchant", Vector2(0.0, -3.2))
	var guard := FolkModels.slot("merchant_cart", "guard", Vector2(-0.2, -5.0))
	var placed := 0
	placed += 1 if _pool.add("peasant", "walk", from, to, phase, side + carter.x, -carter.y) else 0
	placed += 1 if _pool.add("merchant", "walk", from, to, phase, side + merchant.x, -merchant.y) else 0
	var guard_value := float(_pool.settings.get("guard_value", GUARD_VALUE))
	var guards := 0 if value < guard_value else (1 if value < guard_value * 2.0 else 2)
	for g in guards:
		placed += 1 if _pool.add("guard", "guard_walk", from, to, phase, side + guard.x + 0.9 * g, -guard.y + 1.2 * g) else 0
	return placed
