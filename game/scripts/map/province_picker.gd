class_name ProvincePicker
extends Node

## Sélection de province sous le curseur : rayon caméra → intersection avec le
## plan Y = 0, raffinée par itérations contre la hauteur échantillonnée, puis
## lecture de `province_ids.png` en (x, z).

signal province_hovered(index: int)
signal province_selected(index: int)
## Clic droit (sans glisser) sur une province (index 0 = mer / hors carte).
signal province_right_clicked(index: int)

const REFINE_ITERATIONS := 8
const REFINE_EPSILON := 0.02
const CLICK_MAX_DRAG_PX := 6.0

var camera: Camera3D
var map_data: MapData
var hovered_index: int = 0
var selected_index: int = 0

## Interception des clics gauches : `Callable(screen_position: Vector2) -> bool` ; si elle
## renvoie vrai (ex. une armée a été cliquée), la sélection de province n'a pas lieu.
var click_interceptor: Callable = Callable()

var _mouse_dirty := false
var _press_position := Vector2.ZERO
var _press_button := 0


func setup(view_camera: Camera3D, data: MapData) -> void:
	camera = view_camera
	map_data = data


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_mouse_dirty = true
	elif event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index != MOUSE_BUTTON_LEFT and mb.button_index != MOUSE_BUTTON_RIGHT:
			return
		if mb.pressed:
			_press_position = mb.position
			_press_button = mb.button_index
		elif mb.button_index == _press_button and mb.position.distance_to(_press_position) <= CLICK_MAX_DRAG_PX:
			if mb.button_index == MOUSE_BUTTON_RIGHT:
				province_right_clicked.emit(pick_screen(mb.position))
			elif click_interceptor.is_valid() and click_interceptor.call(mb.position):
				return
			else:
				select_index(pick_screen(mb.position))


func _process(_delta: float) -> void:
	if not _mouse_dirty or camera == null:
		return
	_mouse_dirty = false
	var index := pick_screen(get_viewport().get_mouse_position())
	if index != hovered_index:
		hovered_index = index
		province_hovered.emit(index)


func select_index(index: int) -> void:
	selected_index = index
	province_selected.emit(index)


## Index de province sous un point écran (0 si mer ou hors carte).
func pick_screen(screen_position: Vector2) -> int:
	var hit := pick_ray_screen(screen_position)
	if hit.is_empty():
		return 0
	return province_at_world(hit["x"], hit["z"])


## Point du terrain sous un point écran : {"x", "y", "z"} ou {} si aucun.
func pick_ray_screen(screen_position: Vector2) -> Dictionary:
	if camera == null or map_data == null:
		return {}
	var origin := camera.project_ray_origin(screen_position)
	var direction := camera.project_ray_normal(screen_position)
	return pick_ray(origin, direction)


func pick_ray(origin: Vector3, direction: Vector3) -> Dictionary:
	if direction.y >= -0.001:
		return {}
	# Point fixe : t tel que origin.y + t·dir.y = h(x(t), z(t)). Converge vite
	# tant que pente × cot(tangage) < 1, ce que garantit l'exagération 0,02.
	var height := 0.0
	var point := origin
	var previous_t := INF
	for _i in REFINE_ITERATIONS:
		var t := (height - origin.y) / direction.y
		if t < 0.0:
			return {}
		point = origin + direction * t
		if point.x < 0.0 or point.z < 0.0 or point.x >= map_data.size.x or point.z >= map_data.size.y:
			return {}
		height = map_data.surface_world_at(point.x, point.z)
		if absf(t - previous_t) < REFINE_EPSILON:
			break
		previous_t = t
	return {"x": point.x, "y": height, "z": point.z}


func province_at_world(x: float, z: float) -> int:
	if map_data == null:
		return 0
	return map_data.province_index_at(x, z)
