class_name BattleCamera
extends Node3D

## Caméra RTS de bataille : pivot au sol (`target`), lacet, distance ; inclinaison automatique
## (rasante de près, plongeante de loin). W A S D (positions physiques) et bords d'écran pour
## se déplacer, molette pour zoomer, Q / E pour tourner, glisser bouton du milieu pour panoramiquer.

@export var min_distance: float = 25.0
@export var max_distance: float = 900.0
@export var pan_speed: float = 1.1
@export var rotate_speed: float = 1.6
@export var edge_margin: int = 8

var target: Vector3 = Vector3(600, 0, 200)
var yaw: float = 0.0
var distance: float = 220.0
var edge_pan_enabled: bool = true
var bounds: Rect2 = Rect2(-200, -200, 1600, 1200)
var height_at: Callable = func(_x: float, _z: float) -> float: return 0.0

var _target_distance: float = 220.0
var _dragging: bool = false

@onready var camera: Camera3D = $Camera3D


func look_at_point(point: Vector3, p_distance: float, p_yaw: float) -> void:
	target = point
	distance = p_distance
	_target_distance = p_distance
	yaw = p_yaw
	_apply()


func _process(delta: float) -> void:
	var move := Vector2.ZERO
	if Input.is_physical_key_pressed(KEY_W) or Input.is_physical_key_pressed(KEY_UP):
		move.y -= 1.0
	if Input.is_physical_key_pressed(KEY_S) or Input.is_physical_key_pressed(KEY_DOWN):
		move.y += 1.0
	if Input.is_physical_key_pressed(KEY_A) or Input.is_physical_key_pressed(KEY_LEFT):
		move.x -= 1.0
	if Input.is_physical_key_pressed(KEY_D) or Input.is_physical_key_pressed(KEY_RIGHT):
		move.x += 1.0
	if edge_pan_enabled and DisplayServer.get_name() != "headless":
		var mouse := get_viewport().get_mouse_position()
		var size := get_viewport().get_visible_rect().size
		if Rect2(Vector2.ZERO, size).has_point(mouse):
			if mouse.x < edge_margin:
				move.x -= 1.0
			elif mouse.x > size.x - edge_margin:
				move.x += 1.0
			if mouse.y < edge_margin:
				move.y -= 1.0
			elif mouse.y > size.y - edge_margin:
				move.y += 1.0
	if Input.is_physical_key_pressed(KEY_Q):
		yaw += rotate_speed * delta
	if Input.is_physical_key_pressed(KEY_E):
		yaw -= rotate_speed * delta
	if move != Vector2.ZERO:
		_pan(move.normalized() * distance * pan_speed * delta)
	distance = lerpf(distance, _target_distance, clampf(delta * 10.0, 0.0, 1.0))
	_apply()


func _pan(screen_move: Vector2) -> void:
	# Avant = direction de la caméra projetée au sol.
	var forward := Vector3(-sin(yaw), 0, -cos(yaw))
	var right := Vector3(cos(yaw), 0, -sin(yaw))
	target += right * screen_move.x - forward * screen_move.y
	target.x = clampf(target.x, bounds.position.x, bounds.end.x)
	target.z = clampf(target.z, bounds.position.y, bounds.end.y)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index == MOUSE_BUTTON_WHEEL_UP and button.pressed:
			_target_distance = clampf(_target_distance * 0.88, min_distance, max_distance)
		elif button.button_index == MOUSE_BUTTON_WHEEL_DOWN and button.pressed:
			_target_distance = clampf(_target_distance * 1.14, min_distance, max_distance)
		elif button.button_index == MOUSE_BUTTON_MIDDLE:
			_dragging = button.pressed
	elif event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		_pan(-motion.relative * distance * 0.0022)


func _apply() -> void:
	target.y = height_at.call(target.x, target.z)
	var t := clampf((distance - min_distance) / (max_distance - min_distance), 0.0, 1.0)
	var pitch := lerpf(deg_to_rad(17.0), deg_to_rad(60.0), t)
	var offset := Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch)) * distance
	var eye := target + offset
	eye.y = maxf(eye.y, height_at.call(eye.x, eye.z) + 4.0)
	if camera == null:
		return
	camera.global_position = eye
	camera.look_at(target, Vector3.UP)
