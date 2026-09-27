class_name CampaignCamera
extends Node3D

## Caméra RTS : point de focus au sol, distance (zoom), lacet (Q/E), tangage
## dérivé du zoom (30° de près, plus rasant en vue comté → 70° de loin), amortissement.
## Entrées : WASD/flèches, bords d'écran (désactivable), molette ou pad (deux doigts / pincement), glisser molette.
##
## Lot ZG4 (ADR 0036) : caméra rapprochée quand la pyramide de relief est en cache (`relief`
## posé par la carte) : distance minimale selon l'étage le plus fin sous le point visé
## (`CloseCameraProfile`, ≈ 5 unités sur E2, 1,5 sur E4, 0,3 dans les zones E5-E7), adoucie dans
## l'espace ; tangage de plus en plus rasant sous 22 unités ; point visé posé sur le sol
## (`ground_height`) ; caméra jamais sous le relief ; plans `near` / `far` suivant la distance.
##
## Chantier PO5 (ADR 0097) : ressenti lu dans `data/ui/camera_feel.json` (`CameraFeel`) —
## inertie du déplacement (vitesse qui monte vers celle demandée puis décroît en exponentielle
## au relâchement, glisser au bouton du milieu compris), zoom lissé vers `target_distance`, et
## glissement de focus de `focus_glide_s` (0,4 s, courbe douce) au lieu d'un saut pour
## `look_at_point`. `snap()` applique tout de suite les cibles (tests, captures).

## Lot C6 : vue « comté » (≈ 40 km, ~55 unités à l'écran) au zoom maximal.
@export var min_distance: float = 22.0
## CV3-0 (#1) : 1500 -> 2600 (était trop court pour cadrer la France entière depuis Paris,
## le Midi restait hors champ ; 1 unité ≈ 719 m, Paris-Marseille ≈ 918 unités).
@export var max_distance: float = 2600.0
@export var pitch_near_deg: float = 30.0
@export var pitch_far_deg: float = 70.0
## Distance à laquelle le tangage atteint `pitch_far_deg` (bornée par max_distance,
## réglée sur la taille de carte dans `setup`).
@export var pitch_far_distance: float = 2600.0
@export var pan_speed: float = 1.2         # unités/s par unité de distance
@export var rotate_speed_deg: float = 90.0
@export var zoom_step: float = 0.15
@export var edge_pan_enabled: bool = true
@export var edge_margin_px: float = 14.0
## Lot L1 : zoom plus proche au-dessus des villes emblématiques (Paris) ; `close_zones` liste
## leurs cercles (x, z, rayon) en unités carte, fournis par `SettlementLayer.landmark_zones`.
@export var close_min_distance: float = 7.0
var close_zones: PackedVector3Array = PackedVector3Array()
## VH4 (ADR 0078) : zones du plancher provisoire ZG4b (villes emblématiques sans ville 1:1) ;
## tant qu'elles ne sont pas fournies (`floor_zones_set`), `close_zones` sert.
var floor_zones: PackedVector3Array = PackedVector3Array()
var floor_zones_set := false
## Lot ZG4 : réglages de la caméra rapprochée ; `relief` (`ReliefPyramid` : étages disponibles) et
## `ground_height(x, y) -> float` (surface affichée, `TerrainBuilder.surface_height_at`) sont posés
## par la carte ; null / vide = comportement historique.
var profile: CloseCameraProfile = CloseCameraProfile.load_default()
var relief: Object = null
var ground_height: Callable = Callable()
var _occlusion_lift: float = 0.0
var _snapping: bool = false
var _soft_min_key := Vector3(INF, INF, INF)
var _soft_min_value: float = 0.0

var focus: Vector3 = Vector3.ZERO
var distance: float = 500.0
var yaw: float = 0.0
var target_focus: Vector3 = Vector3.ZERO
var target_distance: float = 500.0
var target_yaw: float = 0.0
var bounds: Rect2 = Rect2(0, 0, 4096, 4096)

var _dragging := false
## PO5 : vitesse de déplacement dans le plan de la carte (unités/s), inertie comprise.
var pan_velocity: Vector3 = Vector3.ZERO
## PO5 : glissement de focus en cours (`look_at_point`) : départ, durée, temps écoulé.
var _glide_active := false
var _glide_from_focus := Vector3.ZERO
var _glide_from_distance := 0.0
var _glide_distance := false
var _glide_duration := 0.0
var _glide_elapsed := 0.0
## PO5 : déplacement du glisser accumulé depuis la dernière image, et vitesse estimée.
var _drag_accum := Vector3.ZERO
var _drag_velocity := Vector3.ZERO

@onready var camera: Camera3D = $Camera3D


func setup(map_bounds: Rect2, initial_distance: float) -> void:
	bounds = map_bounds
	pitch_far_distance = clampf(maxf(map_bounds.size.x, map_bounds.size.y) * 0.45, _pitch_reference() + 1.0, max_distance)
	target_focus = Vector3(map_bounds.get_center().x, 0.0, map_bounds.get_center().y)
	target_distance = clampf(initial_distance, min_distance, max_distance)
	snap()


## Applique immédiatement les cibles (pas d'amortissement).
func snap() -> void:
	_ground_target()
	focus = target_focus
	distance = target_distance
	yaw = target_yaw
	_glide_active = false
	pan_velocity = Vector3.ZERO
	_snapping = true
	_apply_transform()
	_snapping = false


func look_at_point(point: Vector3, new_distance: float = -1.0) -> void:
	target_focus = point
	if new_distance > 0.0:
		target_distance = clampf(new_distance, min_distance_at(point), max_distance)
	var glide := CameraFeel.get_value("campaign", "focus_glide_s")
	if Accessibility.reduce_motion() or glide <= 0.0:  # U12 : coupe franche au lieu d'un travelling
		snap()
		return
	# PO5 : glissement de durée fixe (pas de saut, pas de longue traîne exponentielle).
	_glide_active = true
	_glide_from_focus = focus
	_glide_from_distance = distance
	_glide_distance = new_distance > 0.0
	_glide_duration = glide
	_glide_elapsed = 0.0
	pan_velocity = Vector3.ZERO


## Vrai pendant un glissement de focus (`look_at_point`).
func is_gliding() -> bool:
	return _glide_active


## Distance minimale au-dessus d'un point : plus courte dans une ville emblématique (L1) et,
## avec la pyramide de relief (ZG4), selon l'étage le plus fin disponible autour du point
## (champ adouci, `CloseCameraProfile.soft_min_distance`). `min_distance` reste un plafond
## (`--camera-min` le baisse pour les essais).
func min_distance_at(point: Vector3) -> float:
	var result := min_distance
	for zone in close_zones:
		if Vector2(point.x, point.z).distance_to(Vector2(zone.x, zone.y)) <= zone.z:
			result = minf(close_min_distance, min_distance)
			break
	if relief != null and profile != null:
		result = minf(result, _soft_min(point))
		# ZG4b : plancher provisoire au-dessus des villes emblématiques (levé par VH4).
		var zones := floor_zones if floor_zones_set else close_zones
		result = maxf(result, minf(profile.landmark_floor(Vector2(point.x, point.z), zones), min_distance))
	return result


## Distance minimale par étage, mémorisée tant que le point ne bouge pas de plus de 2 % de la
## distance courante (le champ varie à l'échelle de l'unité).
func _soft_min(point: Vector3) -> float:
	var cell := maxf(distance * 0.02, 0.01)
	var key := Vector3(roundf(point.x / cell), roundf(point.z / cell), cell)
	if key != _soft_min_key:
		_soft_min_key = key
		_soft_min_value = profile.soft_min_distance(Vector2(point.x, point.z), relief)
	return _soft_min_value


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			_zoom_by(1.0 - zoom_step)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			_zoom_by(1.0 + zoom_step)
		elif mb.button_index == MOUSE_BUTTON_MIDDLE:
			if _dragging and not mb.pressed:
				# PO5 : la carte file encore un peu au relâchement du glisser.
				pan_velocity = _drag_velocity * CameraFeel.get_value("campaign", "drag_release_inertia")
			elif mb.pressed:
				pan_velocity = Vector3.ZERO
				_drag_accum = Vector3.ZERO
				_drag_velocity = Vector3.ZERO
			_dragging = mb.pressed
	elif event is InputEventPanGesture:
		# Pad macOS : glissement vertical à deux doigts = molette continue.
		_zoom_by(exp(zoom_step * (event as InputEventPanGesture).delta.y))
	elif event is InputEventMagnifyGesture:
		# Pad macOS : pincement (facteur > 1 = écarter les doigts = rapprocher).
		_zoom_by(1.0 / maxf((event as InputEventMagnifyGesture).factor, 0.01))
	elif event is InputEventMouseMotion and _dragging:
		var motion := event as InputEventMouseMotion
		var viewport_height := float(get_viewport().get_visible_rect().size.y)
		var units_per_px := distance * 1.6 / maxf(viewport_height, 1.0)
		var before := target_focus
		_pan(Vector2(-motion.relative.x, -motion.relative.y) * units_per_px)
		_drag_accum += target_focus - before


func _zoom_by(factor: float) -> void:
	target_distance = clampf(target_distance * factor, min_distance_at(target_focus), max_distance)


func _process(delta: float) -> void:
	var pan := Vector2.ZERO
	pan.x += Input.get_action_strength("map_pan_right") - Input.get_action_strength("map_pan_left")
	pan.y += Input.get_action_strength("map_pan_down") - Input.get_action_strength("map_pan_up")
	if edge_pan_enabled:
		pan += _edge_pan_vector()
	_update_pan_velocity(pan, delta)
	var rotate := Input.get_action_strength("map_rotate_right") - Input.get_action_strength("map_rotate_left")
	target_yaw += deg_to_rad(rotate_speed_deg) * rotate * delta
	if not close_zones.is_empty() or relief != null:
		target_distance = maxf(target_distance, min_distance_at(target_focus))
	_ground_target()

	var zoom_t := 1.0 - exp(-CameraFeel.get_value("campaign", "zoom_damping") * delta)
	var yaw_t := 1.0 - exp(-CameraFeel.get_value("campaign", "rotate_damping") * delta)
	if _glide_active:
		_glide_elapsed += delta
		var k := clampf(_glide_elapsed / maxf(_glide_duration, 1e-4), 0.0, 1.0)
		var eased := k * k * (3.0 - 2.0 * k)
		focus = _glide_from_focus.lerp(target_focus, eased)
		distance = lerpf(_glide_from_distance, target_distance, eased) if _glide_distance else lerpf(distance, target_distance, zoom_t)
		if k >= 1.0:
			_glide_active = false
	else:
		var follow_t := 1.0 - exp(-CameraFeel.get_value("campaign", "follow_damping") * delta)
		focus = focus.lerp(target_focus, follow_t)
		distance = lerpf(distance, target_distance, zoom_t)
	yaw = lerp_angle(yaw, target_yaw, yaw_t)
	_apply_transform()


## PO5 : inertie. La vitesse monte vers celle demandée (clavier, bords d'écran) puis décroît en
## exponentielle une fois la commande relâchée ; un déplacement annule le glissement de focus.
func _update_pan_velocity(pan: Vector2, delta: float) -> void:
	if _dragging:
		if delta > 0.0:
			_drag_velocity = _drag_velocity.lerp(_drag_accum / delta, 0.5)
		_drag_accum = Vector3.ZERO
		_glide_active = false
		return
	if pan != Vector2.ZERO:
		_glide_active = false
		var wanted := _plane_vector(pan.normalized() * pan_speed * distance)
		pan_velocity = pan_velocity.lerp(wanted, 1.0 - exp(-CameraFeel.get_value("campaign", "pan_accel") * delta))
	else:
		pan_velocity *= exp(-CameraFeel.get_value("campaign", "pan_friction") * delta)
		if pan_velocity.length() < distance * 0.002:
			pan_velocity = Vector3.ZERO
	if pan_velocity != Vector3.ZERO:
		if pan == Vector2.ZERO:
			_glide_active = false
		_move_target(pan_velocity * delta)


## Déplacement dans le plan de la carte, relatif à l'orientation de la caméra.
func _pan(screen_delta: Vector2) -> void:
	_move_target(_plane_vector(screen_delta))


## Vecteur écran (x vers la droite, y vers le bas) → vecteur au sol selon le lacet.
func _plane_vector(screen_delta: Vector2) -> Vector3:
	var right := Vector3(cos(yaw), 0.0, -sin(yaw))
	var forward := Vector3(-sin(yaw), 0.0, -cos(yaw))
	return right * screen_delta.x - forward * screen_delta.y


func _move_target(offset: Vector3) -> void:
	target_focus += offset
	var clamped_x := clampf(target_focus.x, bounds.position.x, bounds.end.x)
	var clamped_z := clampf(target_focus.z, bounds.position.y, bounds.end.y)
	if clamped_x != target_focus.x:
		pan_velocity.x = 0.0
	if clamped_z != target_focus.z:
		pan_velocity.z = 0.0
	target_focus.x = clamped_x
	target_focus.z = clamped_z


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


## Tangage : courbe historique (C6, `pitch_near_deg` → `pitch_far_deg`) au-dessus de la distance
## de référence (22 unités), de plus en plus rasant en deçà (ZG4, `CloseCameraProfile`).
func pitch_deg() -> float:
	var reference := _pitch_reference()
	var zoom_t := inverse_lerp(reference, pitch_far_distance, distance)
	var pitch := lerpf(pitch_near_deg, pitch_far_deg, clampf(zoom_t, 0.0, 1.0))
	if distance < reference and profile != null:
		return profile.close_pitch_deg(distance, pitch_near_deg)
	return pitch


func _pitch_reference() -> float:
	return profile.pitch_reference_distance if profile != null else min_distance


## Point visé posé sur la surface affichée (ZG4) : à 200 m de distance, un point visé à Y = 0
## mettrait la caméra sous les collines.
func _ground_target() -> void:
	if ground_height.is_valid():
		target_focus.y = float(ground_height.call(target_focus.x, target_focus.z))


func _apply_transform() -> void:
	if camera == null:
		return
	var pitch := deg_to_rad(pitch_deg())
	var horizontal := cos(pitch) * distance
	var offset := Vector3(sin(yaw) * horizontal, sin(pitch) * distance, cos(yaw) * horizontal)
	var eye := focus + offset
	# ZG4 : jamais sous le relief (garde au sol proportionnelle à la distance) et point visé jamais
	# caché par une crête entre lui et la caméra (vue rasante) : la caméra monte d'autant, vite à la
	# montée, lentement à la descente (pas de tremblement en panoramique).
	if ground_height.is_valid() and profile != null:
		var clear := profile.clearance(distance)
		var needed := float(ground_height.call(eye.x, eye.z)) + clear
		for k in range(1, profile.occlusion_samples + 1):
			var t := lerpf(profile.occlusion_min_t, 1.0, float(k) / profile.occlusion_samples)
			var p := focus.lerp(eye, t)
			var ground := float(ground_height.call(p.x, p.z)) + clear
			needed = maxf(needed, focus.y + (ground - focus.y) / t)
		# SZ1 : au-dessus des crêtes voisines (cercles autour de la caméra et du point visé).
		var radius := profile.crest_radius_factor * distance
		for k in profile.crest_samples:
			var angle := TAU * k / profile.crest_samples
			var ring := Vector2(cos(angle), sin(angle)) * radius
			needed = maxf(needed, float(ground_height.call(eye.x + ring.x, eye.z + ring.y)) + clear)
			needed = maxf(needed, float(ground_height.call(focus.x + ring.x, focus.z + ring.y)) * profile.crest_focus_weight + clear)
		var lift := maxf(needed - eye.y, 0.0)
		var rate := 1.0 if _snapping else (0.5 if lift > _occlusion_lift else 0.06)
		_occlusion_lift = lerpf(_occlusion_lift, lift, rate)
		eye.y += _occlusion_lift
		eye.y = maxf(eye.y, float(ground_height.call(eye.x, eye.z)) + clear)
	camera.global_position = eye
	camera.look_at(focus + Vector3.UP * (profile.look_up(distance) if profile != null and ground_height.is_valid() else 0.0), Vector3.UP)
	if profile != null:
		camera.near = profile.near_plane(distance)
		camera.far = profile.far_plane(distance)
