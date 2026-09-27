class_name BattleCamera
extends Node3D

## Caméra RTS de bataille : pivot au sol (`target`), lacet, distance ; inclinaison automatique
## (rasante de près, plongeante de loin). A1-06 : courbe d'inclinaison adoucie
## (presque à hauteur d'homme au zoom maximal, plongée au loin), visée relevée vers la poitrine
## des soldats de près, hauteur minimale au-dessus du relief qui descend avec le zoom. W A S D (positions physiques) et bords d'écran pour
## se déplacer, molette ou pad (deux doigts / pincement) pour zoomer, Q / E pour tourner, glisser bouton du milieu pour panoramiquer.
##
## B3 / T6 : suivi de régiment — `follow_unit(id, position_of)` verrouille `target` sur la
## position (fournie par une Callable, pour ne pas dépendre de `BattleScene`) du régiment ou du
## général, avec un lissage exponentiel (`FOLLOW_LERP`) ; la distance et l'inclinaison restent
## celles choisies par le joueur (zoom conservé). Bouton du milieu = orbite (lacet) pendant le
## suivi au lieu de panoramiquer. Toute action manuelle (W A S D / bords d'écran, `look_at_point`
## appelé par un clic minicarte ou un rappel de groupe) rend la main à la caméra libre.
##
## Chantier PO5 (ADR 0097) : zoom lissé au taux `battle.zoom_damping` de
## `data/ui/camera_feel.json` ; `glide_to` (clic minicarte, rappel de groupe, double clic sur une
## carte) glisse en `focus_glide_s` (0,4 s) au lieu de sauter. `look_at_point` reste une coupe
## franche (plans de caméra scriptés : harangue, étendard, gros plans, captures). Glissement
## instantané en headless (sauf `glide_in_headless`) et avec « Réduire les animations ».

@export var min_distance: float = 12.0
@export var max_distance: float = 900.0
@export var pan_speed: float = 1.1
@export var rotate_speed: float = 1.6
@export var edge_margin: int = 8

const FOLLOW_LERP := 3.0
## A1-06 : inclinaison (degrés) au zoom minimal et maximal, exposant de la courbe (< 1 : on
## quitte vite la vue rasante en dézoomant), hauteur de visée (m) et garde au sol de près/loin.
const PITCH_NEAR_DEG := 6.0
const PITCH_FAR_DEG := 62.0
const PITCH_CURVE := 0.55
const AIM_HEIGHT_NEAR := 1.7
const GROUND_CLEARANCE_NEAR := 1.6
const GROUND_CLEARANCE_FAR := 4.0
const CLOSE_RANGE := 80.0
const ORBIT_MOUSE_SPEED := 0.006

var target: Vector3 = Vector3(600, 0, 200)
var yaw: float = 0.0
var distance: float = 220.0
var edge_pan_enabled: bool = true
var bounds: Rect2 = Rect2(-200, -200, 1600, 1200)
var height_at: Callable = func(_x: float, _z: float) -> float: return 0.0

var follow_id: int = -1
var follow_position_of: Callable = Callable()

var _target_distance: float = 220.0
var _dragging: bool = false
## PO5 : glissement en cours (`glide_to`) : départ, arrivée, durée, temps écoulé.
var _glide_active := false
var _glide_from := Vector3.ZERO
var _glide_to := Vector3.ZERO
var _glide_from_yaw := 0.0
var _glide_to_yaw := 0.0
var _glide_from_distance := 0.0
var _glide_duration := 0.0
var _glide_elapsed := 0.0
## Tests : anime le glissement même en headless.
var glide_in_headless := false

@onready var camera: Camera3D = $Camera3D


## Jump caméra (clic minicarte, rappel de groupe, double-clic sur une carte) : rend la main à la
## caméra libre (annule un suivi éventuel).
func look_at_point(point: Vector3, p_distance: float, p_yaw: float) -> void:
	stop_follow()
	_glide_active = false
	target = point
	distance = p_distance
	_target_distance = p_distance
	yaw = p_yaw
	_apply()


## PO5 : même cadrage que `look_at_point`, atteint par un glissement de `focus_glide_s`.
func glide_to(point: Vector3, p_distance: float, p_yaw: float) -> void:
	var duration := CameraFeel.get_value("battle", "focus_glide_s")
	var headless := DisplayServer.get_name() == "headless" and not glide_in_headless
	if duration <= 0.0 or headless or Accessibility.reduce_motion():
		look_at_point(point, p_distance, p_yaw)
		return
	stop_follow()
	_glide_active = true
	_glide_from = target
	_glide_to = point
	_glide_from_yaw = yaw
	_glide_to_yaw = p_yaw
	_glide_from_distance = distance
	_target_distance = clampf(p_distance, min_distance, max_distance)
	_glide_duration = duration
	_glide_elapsed = 0.0


## Termine tout de suite un glissement en cours (tests, captures).
func snap() -> void:
	if _glide_active:
		_glide_active = false
		target = _glide_to
		yaw = _glide_to_yaw
	distance = _target_distance
	_apply()


func is_gliding() -> bool:
	return _glide_active


## Verrouille la caméra sur le régiment `id` ; `position_of` rend sa position (Vector3) ou `null`
## si le régiment a disparu du champ (auto-libération).
func follow_unit(id: int, position_of: Callable) -> void:
	follow_id = id
	follow_position_of = position_of


func stop_follow() -> void:
	follow_id = -1


## Lettre de caméra tenue, sauf avec Ctrl/Cmd : ces combinaisons sont des raccourcis
## (CB0 : Ctrl/Cmd+A = tout sélectionner), pas un déplacement.
func _key_held(physical: Key) -> bool:
	if Input.is_key_pressed(KEY_CTRL) or Input.is_key_pressed(KEY_META):
		return false
	return Input.is_physical_key_pressed(physical)


func is_following() -> bool:
	return follow_id >= 0


func _process(delta: float) -> void:
	var move := Vector2.ZERO
	if _key_held(KEY_W) or Input.is_physical_key_pressed(KEY_UP):
		move.y -= 1.0
	if _key_held(KEY_S) or Input.is_physical_key_pressed(KEY_DOWN):
		move.y += 1.0
	if _key_held(KEY_A) or Input.is_physical_key_pressed(KEY_LEFT):
		move.x -= 1.0
	if _key_held(KEY_D) or Input.is_physical_key_pressed(KEY_RIGHT):
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
	if _key_held(KEY_Q):
		yaw += rotate_speed * delta
	if _key_held(KEY_E):
		yaw -= rotate_speed * delta
	if move != Vector2.ZERO:
		if follow_id >= 0:
			stop_follow()
		_glide_active = false
		_pan(move.normalized() * distance * pan_speed * delta)
	if follow_id >= 0:
		var point: Variant = follow_position_of.call(follow_id) if follow_position_of.is_valid() else null
		if point == null:
			stop_follow()
		else:
			target = target.lerp(point as Vector3, clampf(delta * FOLLOW_LERP, 0.0, 1.0))
			target.x = clampf(target.x, bounds.position.x, bounds.end.x)
			target.z = clampf(target.z, bounds.position.y, bounds.end.y)
	if _glide_active:
		_glide_elapsed += delta
		var k := clampf(_glide_elapsed / maxf(_glide_duration, 1e-4), 0.0, 1.0)
		var eased := k * k * (3.0 - 2.0 * k)
		target = _glide_from.lerp(_glide_to, eased)
		yaw = lerp_angle(_glide_from_yaw, _glide_to_yaw, eased)
		distance = lerpf(_glide_from_distance, _target_distance, eased)
		if k >= 1.0:
			_glide_active = false
	else:
		distance = lerpf(distance, _target_distance, 1.0 - exp(-CameraFeel.get_value("battle", "zoom_damping") * delta))
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
			_zoom_by(0.88)
		elif button.button_index == MOUSE_BUTTON_WHEEL_DOWN and button.pressed:
			_zoom_by(1.14)
		elif button.button_index == MOUSE_BUTTON_MIDDLE:
			_dragging = button.pressed
	elif event is InputEventPanGesture:
		# Pad macOS : glissement vertical à deux doigts = molette continue.
		_zoom_by(exp(0.13 * (event as InputEventPanGesture).delta.y))
	elif event is InputEventMagnifyGesture:
		# Pad macOS : pincement (facteur > 1 = écarter les doigts = rapprocher).
		_zoom_by(1.0 / maxf((event as InputEventMagnifyGesture).factor, 0.01))
	elif event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		_glide_active = false
		if follow_id >= 0:
			yaw += -motion.relative.x * ORBIT_MOUSE_SPEED
		else:
			_pan(-motion.relative * distance * 0.0022)


func _zoom_by(factor: float) -> void:
	_target_distance = clampf(_target_distance * factor, min_distance, max_distance)


func _apply() -> void:
	target.y = height_at.call(target.x, target.z)
	var t := clampf((distance - min_distance) / (max_distance - min_distance), 0.0, 1.0)
	var pitch := deg_to_rad(lerpf(PITCH_NEAR_DEG, PITCH_FAR_DEG, pow(t, PITCH_CURVE)))
	# 0 au zoom maximal, 1 à partir de CLOSE_RANGE : visée et garde au sol s'y ajustent.
	var close := smoothstep(min_distance, CLOSE_RANGE, distance)
	var aim := target + Vector3.UP * lerpf(AIM_HEIGHT_NEAR, 0.0, close)
	var offset := Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch)) * distance
	var eye := aim + offset
	eye.y = maxf(eye.y, height_at.call(eye.x, eye.z) + lerpf(GROUND_CLEARANCE_NEAR, GROUND_CLEARANCE_FAR, close))
	if camera == null:
		return
	camera.global_position = eye
	camera.look_at(aim, Vector3.UP)
