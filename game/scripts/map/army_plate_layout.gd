class_name ArmyPlateLayout
extends RefCounted

## Placement à l'écran des plaques d'effectif : projette l'ancre de chaque marqueur,
## masque les plaques hors champ / trop lointaines, puis écarte les chevauchements avec
## `LabelPlacer`. Les plaques et marqueurs restent la propriété d'`ArmyMarkers`.

## Portée des plaques sous `ArmyScale.CLOSE_KNEE_DISTANCE`, en multiples de la
## distance caméra.
const CLOSE_PLATE_RANGE_FACTOR := 40.0
## Période de recalcul du placement des plaques quand la caméra est immobile (les
## armées animées et les noms de ville qui apparaissent sont repris à ce rythme).
const PLACEMENT_INTERVAL := 0.25

var _placer := LabelPlacer.new()
var _offsets: Dictionary = {}  # army_id → décalage écran
var _camera_transform := Transform3D()
var _view := Vector2.ZERO
var _timer := 0.0
var _dirty := true


## Force un recalcul au prochain `update` (armées ou sélection changées).
func mark_dirty() -> void:
	_dirty = true


func tick(delta: float) -> void:
	_timer -= delta


## `plates` / `markers` : army_id → PanelContainer / ArmyMarker ; `camera_distance` < 0 si inconnue ;
## `obstacles` : `func(camera) -> Array[Rect2]` (peut être invalide).
func update(
	camera: Camera3D,
	viewport_rect: Rect2,
	plates: Dictionary,
	markers: Dictionary,
	selected_army: String,
	camera_distance: float,
	obstacles: Callable
) -> void:
	var bases: Array = []  # [{id, rect, marker}] des plaques visibles
	for id in plates:
		var plate: PanelContainer = plates[id]
		var marker: ArmyMarker = markers.get(id)
		if marker == null:
			plate.visible = false
			continue
		var anchor := marker.plate_anchor()
		if camera.is_position_behind(anchor) or not marker.is_visible_in_tree():
			plate.visible = false
			continue
		# Vues vallée / site (rasantes) : pas de plaques d'armées lointaines sur l'horizon.
		if camera_distance >= 0.0 and camera_distance < ArmyScale.CLOSE_KNEE_DISTANCE and id != selected_army \
				and camera.global_position.distance_to(anchor) > CLOSE_PLATE_RANGE_FACTOR * camera_distance:
			plate.visible = false
			continue
		var screen := camera.unproject_position(anchor)
		var size := plate.get_combined_minimum_size()
		plate.size = size
		var lift := -2.0 if marker.plate_below() else size.y + 2.0
		var base := (screen - Vector2(size.x * 0.5, lift)).round()
		plate.visible = viewport_rect.grow(40.0).has_point(screen)
		if plate.visible:
			bases.append({"id": id, "rect": Rect2(base, size), "marker": marker})
		plate.position = base + _offsets.get(id, Vector2.ZERO)
	if _due(camera, viewport_rect.size):
		_place(camera, plates, bases, selected_army, obstacles)


## Recalcul quand la caméra ou la vue change, quand les armées changent, sinon à
## `PLACEMENT_INTERVAL` ; entre deux, les plaques gardent leur décalage (pas de clignotement).
func _due(camera: Camera3D, view: Vector2) -> bool:
	var moved := not camera.global_transform.is_equal_approx(_camera_transform) or view != _view
	if not (moved or _dirty or _timer <= 0.0):
		return false
	_camera_transform = camera.global_transform
	_view = view
	_dirty = false
	_timer = PLACEMENT_INTERVAL
	return true


func _place(camera: Camera3D, plates: Dictionary, bases: Array, selected_army: String, obstacles: Callable) -> void:
	# Priorité : armée sélectionnée, armées du joueur, gros effectifs (gardent leur place).
	bases.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return _before(a, b, selected_army))
	var blocked: Array = obstacles.call(camera) if obstacles.is_valid() else []
	_offsets = _placer.place(bases, blocked)
	for entry in bases:
		var plate: PanelContainer = plates[entry["id"]]
		plate.position = (entry["rect"] as Rect2).position + _offsets.get(entry["id"], Vector2.ZERO)


static func _before(a: Dictionary, b: Dictionary, selected_army: String) -> bool:
	var ma: ArmyMarker = a["marker"]
	var mb: ArmyMarker = b["marker"]
	var sa: bool = a["id"] == selected_army
	var sb: bool = b["id"] == selected_army
	if sa != sb:
		return sa
	if ma.is_player != mb.is_player:
		return ma.is_player
	if ma.men != mb.men:
		return ma.men > mb.men
	return str(a["id"]) < str(b["id"])


## CV3-0 (#7) : rectangles écran des plaques affichées (position finale, après placement).
static func visible_rects(plates: Dictionary) -> Array:
	var rects: Array = []
	for id in plates:
		var plate: PanelContainer = plates[id]
		if plate.visible:
			rects.append(Rect2(plate.position, plate.size))
	return rects
