class_name CampaignCamera
extends Node3D

## Caméra RTS : point de focus au sol, distance (zoom), lacet (Q/E), tangage
## dérivé du zoom (30° de près, plus rasant en vue comté → 70° de loin), amortissement.
## Entrées : WASD/flèches, bords d'écran (désactivable), molette, glisser molette.

## Lot C6 : vue « comté » (≈ 40 km, ~55 unités à l'écran) au zoom maximal.
@export var min_distance: float = 22.0
@export var max_distance: float = 1500.0
@export var pitch_near_deg: float = 30.0
@export var pitch_far_deg: float = 70.0
## Distance à laquelle le tangage atteint `pitch_far_deg` (bornée par max_distance,
## réglée sur la taille de carte dans `setup`).
@export var pitch_far_distance: float = 1500.0
@export var pan_speed: float = 1.2         # unités/s par unité de distance
@export var rotate_speed_deg: float = 90.0
@export var zoom_step: float = 0.15
@export var edge_pan_enabled: bool = true
@export var edge_margin_px: float = 14.0
@export var damping: float = 10.0
## Lot L1 : zoom plus proche au-dessus des villes emblématiques (Paris) ; `close_zones` liste
## leurs cercles (x, z, rayon) en unités carte, fournis par `SettlementLayer.landmark_zones`.
@export var close_min_distance: float = 7.0
var close_zones: PackedVector3Array = PackedVector3Array()

var focus: Vector3 = Vector3.ZERO
var distance: float = 500.0
var yaw: float = 0.0
var target_focus: Vector3 = Vector3.ZERO
var target_distance: float = 500.0
var target_yaw: float = 0.0
var bounds: Rect2 = Rect2(0, 0, 4096, 4096)

var _dragging := false

@onready var camera: Camera3D = $Camera3D


func setup(map_bounds: Rect2, initial_distance: float) -> void:
	bounds = map_bounds
	pitch_far_distance = clampf(maxf(map_bounds.size.x, map_bounds.size.y) * 0.45, min_distance + 1.0, max_distance)
	target_focus = Vector3(map_bounds.get_center().x, 0.0, map_bounds.get_center().y)
	target_distance = clampf(initial_distance, min_distance, max_distance)
	snap()


## Applique immédiatement les cibles (pas d'amortissement).
func snap() -> void:
	focus = target_focus
	distance = target_distance
	yaw = target_yaw
	_apply_transform()


func look_at_point(point: Vector3, new_distance: float = -1.0) -> void:
	target_focus = point
	if new_distance > 0.0:
		target_distance = clampf(new_distance, min_distance_at(point), max_distance)


## Distance minimale au-dessus d'un point : plus courte dans une ville emblématique.
func min_distance_at(point: Vector3) -> float:
	for zone in close_zones:
		if Vector2(point.x, point.z).distance_to(Vector2(zone.x, zone.y)) <= zone.z:
			return minf(close_min_distance, min_distance)
	return min_distance


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			target_distance = clampf(target_distance * (1.0 - zoom_step), min_distance_at(target_focus), max_distance)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			target_distance = clampf(target_distance * (1.0 + zoom_step), min_distance_at(target_focus), max_distance)
		elif mb.button_index == MOUSE_BUTTON_MIDDLE:
			_dragging = mb.pressed
	elif event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		var viewport_height := float(get_viewport().get_visible_rect().size.y)
		var units_per_px := distance * 1.6 / maxf(viewport_height, 1.0)
		_pan(Vector2(-motion.relative.x, -motion.relative.y) * units_per_px)


func _process(delta: float) -> void:
	var pan := Vector2.ZERO
	pan.x += Input.get_action_strength("map_pan_right") - Input.get_action_strength("map_pan_left")
	pan.y += Input.get_action_strength("map_pan_down") - Input.get_action_strength("map_pan_up")
	if edge_pan_enabled:
		pan += _edge_pan_vector()
	if pan != Vector2.ZERO:
		_pan(pan.normalized() * pan_speed * distance * delta)
	var rotate := Input.get_action_strength("map_rotate_right") - Input.get_action_strength("map_rotate_left")
	target_yaw += deg_to_rad(rotate_speed_deg) * rotate * delta
	if not close_zones.is_empty():
		target_distance = maxf(target_distance, min_distance_at(target_focus))

	var t := 1.0 - exp(-damping * delta)
	focus = focus.lerp(target_focus, t)
	distance = lerpf(distance, target_distance, t)
	yaw = lerp_angle(yaw, target_yaw, t)
	_apply_transform()


## Déplacement dans le plan de la carte, relatif à l'orientation de la caméra.
func _pan(screen_delta: Vector2) -> void:
	var right := Vector3(cos(yaw), 0.0, -sin(yaw))
	var forward := Vector3(-sin(yaw), 0.0, -cos(yaw))
	target_focus += right * screen_delta.x - forward * screen_delta.y
	target_focus.x = clampf(target_focus.x, bounds.position.x, bounds.end.x)
	target_focus.z = clampf(target_focus.z, bounds.position.y, bounds.end.y)


func _edge_pan_vector() -> Vector2:
	var viewport := get_viewport()
	if viewport == null or not DisplayServer.window_is_focused():
		return Vector2.ZERO
	var size := viewport.get_visible_rect().size
	var mouse := viewport.get_mouse_position()
	if mouse.x < 0.0 or mouse.y < 0.0 or mouse.x > size.x or mouse.y > size.y:
		return Vector2.ZERO
	var v := Vector2.ZERO
	if mouse.x < edge_margin_px:
		v.x -= 1.0
	elif mouse.x > size.x - edge_margin_px:
		v.x += 1.0
	if mouse.y < edge_margin_px:
		v.y -= 1.0
	elif mouse.y > size.y - edge_margin_px:
		v.y += 1.0
	return v


func pitch_deg() -> float:
	var zoom_t := inverse_lerp(min_distance, pitch_far_distance, distance)
	return lerpf(pitch_near_deg, pitch_far_deg, clampf(zoom_t, 0.0, 1.0))


func _apply_transform() -> void:
	if camera == null:
		return
	var pitch := deg_to_rad(pitch_deg())
	var horizontal := cos(pitch) * distance
	var offset := Vector3(sin(yaw) * horizontal, sin(pitch) * distance, cos(yaw) * horizontal)
	camera.global_position = focus + offset
	camera.look_at(focus, Vector3.UP)
