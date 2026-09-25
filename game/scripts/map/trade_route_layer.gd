class_name TradeRouteLayer
extends MeshInstance3D

## Lot C5 : routes commerciales tracées sur la carte — ruban fin façon parchemin posé sur le
## relief (`PolylineMesh`, comme `PathPreview`), épaisseur selon la valeur de la route. Couche
## activable (bouton de la barre de filtres ou touche V, `map_toggle_trade`) ; respecte le
## brouillard (C1) : une route dont aucun bout n'est dans une province visible de la faction
## joueuse est masquée.

const MIN_WIDTH := 1.2
const MAX_WIDTH := 5.0
## Teinte encre sépia des routes actives (accord ou non : `PolylineMesh` n'a qu'un matériau
## par maillage, la ligne coupée passe par un second `MeshInstance3D` grisé).
const COLOR := Color(0.45, 0.18, 0.08, 0.9)
const COLOR_CUT := Color(0.4, 0.4, 0.4, 0.35)

var map_data: MapData
var settlement_layer: SettlementLayer
var settlement_data: SettlementData
var _cut_mesh: MeshInstance3D

## Routes affichées ce tour : id -> {points: PackedVector2Array (carte), route: Dictionary}.
var _routes_screen: Dictionary = {}
## Lot ZG4 : rubans larges d'un kilomètre ou plus (lisibles en vue stratégique), masqués aux
## paliers vallée / site ; `_wanted_visible` garde l'état voulu par la couche.
var _wanted_visible: bool = false
var _close_hidden: bool = false


func setup(data: MapData, layer: SettlementLayer, settlements: SettlementData) -> void:
	map_data = data
	settlement_layer = layer
	settlement_data = settlements
	_cut_mesh = MeshInstance3D.new()
	_cut_mesh.name = "CutRoutes"
	add_child(_cut_mesh)
	_wanted_visible = false
	_apply_visible()


## `routes` : tableau de dictionnaires `CampaignSim.get_trade_routes()`. `visible_provinces` :
## ids de provinces vues par la faction joueuse (brouillard C1) ; vide = pas de filtrage.
func refresh(routes: Array, camera_distance: float, visible_provinces: PackedStringArray) -> void:
	_routes_screen.clear()
	if map_data == null or settlement_layer == null:
		return
	var active_lines: Array = []
	var active_widths: Array = []
	var cut_lines: Array = []
	var cut_widths: Array = []
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
		var width := clampf(MIN_WIDTH + value / 40.0, MIN_WIDTH, MAX_WIDTH)
		if bool(route.get("cut", false)):
			cut_lines.append(points)
			cut_widths.append(MIN_WIDTH * 0.6)
		else:
			active_lines.append(points)
			active_widths.append(width)
		_routes_screen[str(route.get("id", ""))] = {"points": points, "route": route}
	if active_lines.is_empty() and cut_lines.is_empty():
		mesh = null
		_cut_mesh.mesh = null
		return
	if active_lines.is_empty():
		mesh = null
	else:
		mesh = PolylineMesh.build(active_lines, active_widths, map_data, 0.55)
		material_override = PolylineMesh.flat_material(COLOR)
		material_override.render_priority = 1
	if cut_lines.is_empty():
		_cut_mesh.mesh = null
	else:
		_cut_mesh.mesh = PolylineMesh.build(cut_lines, cut_widths, map_data, 0.5)
		_cut_mesh.material_override = PolylineMesh.flat_material(COLOR_CUT)
		_cut_mesh.material_override.render_priority = 1
	# Visibilité décidée par le mode Commerce seulement (`set_layer_visible`) : forcer ici
	# affichait les rubans à chaque `refresh_all` (recette Q3, ruban géant sur la Tamise).
	_apply_visible()


func set_layer_visible(value: bool) -> void:
	_wanted_visible = value
	_apply_visible()


## Lot ZG4 : masque la couche aux paliers vallée / site sans perdre l'état voulu.
func set_close_hidden(value: bool) -> void:
	if value != _close_hidden:
		_close_hidden = value
		_apply_visible()


func _apply_visible() -> void:
	visible = _wanted_visible and not _close_hidden


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
