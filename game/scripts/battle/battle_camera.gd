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
## celles choisies par le joueur (zoom conservé). Toute action manuelle (W A S D / bords d'écran,
## `look_at_point` appelé par un clic minicarte ou un rappel de groupe) rend la main à la caméra
## libre.
##
## CB3 : bouton du milieu maintenu = glisser horizontal fait tourner (lacet), glisser vertical
## incline (`manual_pitch_deg`, borné 5°-85°) ; pendant un suivi, il orbite seulement (lacet).
## Maj + bouton du milieu = panoramique (ancien comportement par défaut). Une inclinaison
## manuelle suspend la courbe d'inclinaison automatique de `_apply` (`manual_pitch`) jusqu'au
## prochain recentrage (`look_at_point`, `follow_unit`).

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
## CB3 : bornes et vitesse de l'inclinaison manuelle (degrés par pixel glissé, bouton du milieu).
const MANUAL_PITCH_MIN_DEG := 5.0
const MANUAL_PITCH_MAX_DEG := 85.0
const MANUAL_PITCH_DRAG_SPEED := 0.15

var target: Vector3 = Vector3(600, 0, 200)
var yaw: float = 0.0
var distance: float = 220.0
var edge_pan_enabled: bool = true
var bounds: Rect2 = Rect2(-200, -200, 1600, 1200)
var height_at: Callable = func(_x: float, _z: float) -> float: return 0.0

## CB3 : inclinaison manuelle (bouton du milieu, glisser vertical) : suspend la courbe
## automatique tant que `manual_pitch` est vrai ; `look_at_point` et `follow_unit` la remettent
## à zéro (recentrage).
var manual_pitch: bool = false
var manual_pitch_deg: float = 0.0

var follow_id: int = -1
var follow_position_of: Callable = Callable()

var _target_distance: float = 220.0
var _dragging: bool = false
## CB3 : dernière inclinaison réellement appliquée (auto ou manuelle), point de départ d'une
## nouvelle inclinaison manuelle pour éviter un saut au premier pixel glissé.
var _last_pitch_deg: float = PITCH_NEAR_DEG

@onready var camera: Camera3D = $Camera3D


## Jump caméra (clic minicarte, rappel de groupe, double-clic sur une carte) : rend la main à la
## caméra libre (annule un suivi éventuel).
func look_at_point(point: Vector3, p_distance: float, p_yaw: float) -> void:
	stop_follow()
	target = point
	distance = p_distance
	_target_distance = p_distance
	yaw = p_yaw
	manual_pitch = false  # CB3 : un recentrage rend la main à l'inclinaison automatique.
	_apply()


## Verrouille la caméra sur le régiment `id` ; `position_of` rend sa position (Vector3) ou `null`
## si le régiment a disparu du champ (auto-libération).
func follow_unit(id: int, position_of: Callable) -> void:
	follow_id = id
	follow_position_of = position_of
	manual_pitch = false  # CB3 : recentrage.


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
		_pan(move.normalized() * distance * pan_speed * delta)
	if follow_id >= 0:
		var point: Variant = follow_position_of.call(follow_id) if follow_position_of.is_valid() else null
		if point == null:
			stop_follow()
		else:
			target = target.lerp(point as Vector3, clampf(delta * FOLLOW_LERP, 0.0, 1.0))
			target.x = clampf(target.x, bounds.position.x, bounds.end.x)
			target.z = clampf(target.z, bounds.position.y, bounds.end.y)
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
			_zoom_by(0.88)
		elif button.button_index == MOUSE_BUTTON_WHEEL_DOWN and button.pressed:
			_zoom_by(1.14)
		elif button.button_index == MOUSE_BUTTON_MIDDLE:
			_dragging = button.pressed
			# CB3 : un glisser de rotation/inclinaison part de l'inclinaison déjà affichée (pas
			# de saut) ; Maj ou un suivi en cours restent le panoramique / l'orbite d'avant.
			if button.pressed and follow_id < 0 and not button.shift_pressed:
				manual_pitch_deg = _last_pitch_deg
	elif event is InputEventPanGesture:
		# Pad macOS : glissement vertical à deux doigts = molette continue.
		_zoom_by(exp(0.13 * (event as InputEventPanGesture).delta.y))
	elif event is InputEventMagnifyGesture:
		# Pad macOS : pincement (facteur > 1 = écarter les doigts = rapprocher).
		_zoom_by(1.0 / maxf((event as InputEventMagnifyGesture).factor, 0.01))
	elif event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		if follow_id >= 0:
			yaw += -motion.relative.x * ORBIT_MOUSE_SPEED  # suivi : orbite (lacet) seulement
		elif motion.shift_pressed:
			_pan(-motion.relative * distance * 0.0022)  # CB3 : Maj + bouton du milieu = panoramique
		else:
			# CB3 : bouton du milieu seul = rotation (lacet) + inclinaison (tangage), bornée.
			yaw += -motion.relative.x * ORBIT_MOUSE_SPEED
			manual_pitch = true
			manual_pitch_deg = clampf(
				manual_pitch_deg - motion.relative.y * MANUAL_PITCH_DRAG_SPEED,
				MANUAL_PITCH_MIN_DEG,
				MANUAL_PITCH_MAX_DEG
			)


func _zoom_by(factor: float) -> void:
	_target_distance = clampf(_target_distance * factor, min_distance, max_distance)


func _apply() -> void:
	target.y = height_at.call(target.x, target.z)
	var pitch_deg: float
	if manual_pitch:
		# CB3 : inclinaison manuelle (bouton du milieu) : suspend la courbe automatique.
		pitch_deg = clampf(manual_pitch_deg, MANUAL_PITCH_MIN_DEG, MANUAL_PITCH_MAX_DEG)
	else:
		var t := clampf((distance - min_distance) / (max_distance - min_distance), 0.0, 1.0)
		pitch_deg = lerpf(PITCH_NEAR_DEG, PITCH_FAR_DEG, pow(t, PITCH_CURVE))
	_last_pitch_deg = pitch_deg
	var pitch := deg_to_rad(pitch_deg)
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
