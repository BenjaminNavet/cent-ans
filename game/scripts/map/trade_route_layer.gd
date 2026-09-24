class_name TradeRouteLayer
extends MeshInstance3D

## Lot C5 : routes commerciales tracées sur la carte — ruban fin façon parchemin posé sur le
## relief (`PolylineMesh`, comme `PathPreview`), épaisseur selon la valeur de la route. Couche
## activable (bouton de la barre de filtres ou touche `map_toggle_trade`) ; respecte le
## brouillard (C1) : une route dont aucun bout n'est dans une province visible de la faction
## joueuse est masquée.

const MIN_WIDTH := 0.5
const MAX_WIDTH := 3.2
## Teinte encre sépia, plus marquée si l'accord commercial est actif.
const COLOR := Color(0.42, 0.30, 0.14, 0.85)
const COLOR_AGREEMENT := Color(0.55, 0.14, 0.10, 0.9)
const COLOR_CUT := Color(0.35, 0.35, 0.35, 0.4)

var map_data: MapData
var settlement_layer: SettlementLayer
var settlement_data: SettlementData

## Routes affichées ce tour : id -> {points: PackedVector2Array (carte), route: Dictionary}.
var _routes_screen: Dictionary = {}


func setup(data: MapData, layer: SettlementLayer, settlements: SettlementData) -> void:
	map_data = data
	settlement_layer = layer
	settlement_data = settlements
	visible = false


## `routes` : tableau de dictionnaires `CampaignSim.get_trade_routes()`. `visible_provinces` :
## ids de provinces vues par la faction joueuse (brouillard C1) ; vide = pas de filtrage.
func refresh(routes: Array, camera_distance: float, visible_provinces: PackedStringArray) -> void:
	_routes_screen.clear()
	if map_data == null or settlement_layer == null:
		return
	var lines: Array = []
	var widths: Array = []
	var colors: Array = []
	var fog_on := not visible_provinces.is_empty()
	for route_variant in routes:
		var route: Dictionary = route_variant
		var path: PackedStringArray = route.get("path", PackedStringArray())
		if path.size() < 2:
			continue
		if fog_on and settlement_data != null:
			var from_province := str(settlement_data.get_settlement(str(route.get("from_settlement", ""))).get("province", ""))
			var to_province := str(settlement_data.get_settlement(str(route.get("to_settlement", ""))).get("province", ""))
			if not visible_provinces.has(from_province) and not visible_provinces.has(to_province):
				continue
		var points := PackedVector2Array()
		for id in path:
			var world: Vector3 = settlement_layer.world_position_of(str(id))
			if world == Vector3.ZERO:
				continue
			points.append(Vector2(world.x, world.z))
		if points.size() < 2:
			continue
		var value: float = float(route.get("total_value", 0))
		var width := clampf(MIN_WIDTH + value / 90.0, MIN_WIDTH, MAX_WIDTH)
		var cut := bool(route.get("cut", false))
		var color := COLOR_CUT if cut else (COLOR_AGREEMENT if bool(route.get("agreement", false)) else COLOR)
		lines.append(points)
		widths.append(width if not cut else MIN_WIDTH * 0.6)
		colors.append(color)
		_routes_screen[str(route.get("id", ""))] = {"points": points, "route": route}
	if lines.is_empty():
		mesh = null
		return
	# `PolylineMesh.build` colore tout le maillage d'un seul matériau : les routes coupées
	# (grisées, fines) restent lisibles à côté des routes actives sans nouveau shader.
	mesh = PolylineMesh.build(lines, widths, map_data, 0.55)
	material_override = PolylineMesh.flat_material(COLOR)
	material_override.render_priority = 1
	visible = true


func set_layer_visible(value: bool) -> void:
	visible = value


## Route la plus proche de `world_xz` (carte), en dessous de `max_distance` (unités carte,
## déjà mises à l'échelle par l'appelant selon le zoom) ; `{}` si aucune.
func nearest_route(world_xz: Vector2, max_distance: float) -> Dictionary:
	var best: Dictionary = {}
	var best_dist := max_distance
	for entry in _routes_screen.values():
		var points: PackedVector2Array = entry["points"]
		for i in points.size() - 1:
			var d := _distance_to_segment(world_xz, points[i], points[i + 1])
			if d < best_dist:
				best_dist = d
				best = entry["route"]
	return best


static func _distance_to_segment(p: Vector2, a: Vector2, b: Vector2) -> float:
	var ab := b - a
	var length_sq := ab.length_squared()
	if length_sq <= 0.0001:
		return p.distance_to(a)
	var t := clampf((p - a).dot(ab) / length_sq, 0.0, 1.0)
	return p.distance_to(a + ab * t)
