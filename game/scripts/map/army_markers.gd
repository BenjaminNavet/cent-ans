class_name ArmyMarkers
extends Node3D

## Marqueurs d'armées (`army_marker.tscn`) posés au centroïde de leur province, décalés
## quand plusieurs armées partagent une province. `refresh(sim)` reconstruit l'ensemble
## (appelé après chaque fin de tour ou ordre) ; `pick_screen` renvoie l'armée sous le
## curseur (priorité sur la province).

const MARKER_SCENE := preload("res://scenes/map/army_marker.tscn")
const PICK_RADIUS_PX := 26.0
## Échelle du marqueur = distance caméra × facteur, bornée.
const SCALE_PER_DISTANCE := 0.014
const MIN_SCALE := 0.8
const MAX_SCALE := 14.0

var map_data: MapData
var camera: Camera3D
var selected_army: String = ""

var _markers: Dictionary = {}  # army_id → ArmyMarker
var _current_scale: float = 1.0


func setup(data: MapData, view_camera: Camera3D) -> void:
	map_data = data
	camera = view_camera


## Reconstruit les marqueurs depuis la simulation. `color_of(faction_id) -> Color`.
func refresh(sim: Object, color_of: Callable, player_faction: String) -> void:
	for marker in _markers.values():
		marker.queue_free()
	_markers.clear()
	if sim == null or map_data == null:
		return
	var per_province: Dictionary = {}
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if army.is_empty():
			continue
		var location: String = str(army.get("location", ""))
		var centroid := map_data.centroid_of_id(location)
		if centroid.x < 0.0:
			continue
		var marker: ArmyMarker = MARKER_SCENE.instantiate()
		add_child(marker)
		var faction: String = str(army.get("faction", ""))
		marker.setup(army_id, army, color_of.call(faction), faction == player_faction)
		marker.base_position = Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y)
		var stack: int = per_province.get(location, 0)
		per_province[location] = stack + 1
		marker.offset_dir = Vector2.ZERO if stack == 0 else Vector2.RIGHT.rotated(stack * TAU / 6.0)
		marker.set_selected(army_id == selected_army)
		marker.apply_scale(_current_scale)
		_markers[army_id] = marker
	if not _markers.has(selected_army):
		selected_army = ""


func set_selected(army_id: String) -> void:
	selected_army = army_id
	for id in _markers:
		_markers[id].set_selected(id == army_id)


func has_army(army_id: String) -> bool:
	return _markers.has(army_id)


func army_count() -> int:
	return _markers.size()


## Armée la plus proche du point écran dans `PICK_RADIUS_PX`, sinon "".
func pick_screen(screen_position: Vector2) -> String:
	if camera == null:
		return ""
	var best := ""
	var best_distance := PICK_RADIUS_PX
	for id in _markers:
		var marker: ArmyMarker = _markers[id]
		var world := marker.pick_position()
		if camera.is_position_behind(world):
			continue
		var distance := camera.unproject_position(world).distance_to(screen_position)
		if distance < best_distance:
			best_distance = distance
			best = id
	return best


## Position monde d'une armée (pour cadrer la caméra), Vector3.ZERO si absente.
func world_position_of(army_id: String) -> Vector3:
	var marker: ArmyMarker = _markers.get(army_id)
	return marker.base_position if marker != null else Vector3.ZERO


func update_scale(camera_distance: float) -> void:
	var new_scale := clampf(camera_distance * SCALE_PER_DISTANCE, MIN_SCALE, MAX_SCALE)
	if absf(new_scale - _current_scale) < 0.01:
		return
	_current_scale = new_scale
	for marker in _markers.values():
		marker.apply_scale(_current_scale)
