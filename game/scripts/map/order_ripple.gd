class_name OrderRipple
extends Control

## Chantier PO5 (ADR 0097) : retour visuel d'un ordre accepté sur la carte de campagne — onde
## d'encre au point visé : un anneau qui s'étend et s'efface en `DURATION` (0,5 s), suivi d'un
## second plus pâle. Ancré au point du monde (il suit la caméra), sans intercepter la souris ;
## se libère seul. Rendu seulement. Rien en headless (`spawn` rend null).

const DURATION := 0.5
const START_RADIUS := 4.0
const END_RADIUS := 34.0
const WIDTH := 2.5
## Retard et atténuation de la seconde onde.
const ECHO_DELAY := 0.12
const ECHO_ALPHA := 0.5

var world_point: Vector3 = Vector3.ZERO
var camera: Camera3D
var elapsed: float = 0.0


## Pose une onde sous `parent` (un `CanvasLayer` ou un `Control` plein écran) au point `world`.
static func spawn(parent: Node, p_camera: Camera3D, world: Vector3) -> OrderRipple:
	if parent == null or p_camera == null or DisplayServer.get_name() == "headless":
		return null
	var ripple := OrderRipple.new()
	ripple.camera = p_camera
	ripple.world_point = world
	parent.add_child(ripple)
	return ripple


func _init() -> void:
	name = "OrderRipple"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


## Avance l'animation ; vrai une fois terminée (le nœud est alors libéré).
func advance(delta: float) -> bool:
	elapsed += delta
	if elapsed >= DURATION + ECHO_DELAY:
		queue_free()
		return true
	queue_redraw()
	return false


func _process(delta: float) -> void:
	advance(delta)


func _draw() -> void:
	if camera == null or not is_instance_valid(camera) or camera.is_position_behind(world_point):
		return
	var center := camera.unproject_position(world_point) - global_position
	_ring(center, elapsed / DURATION, 1.0)
	_ring(center, (elapsed - ECHO_DELAY) / DURATION, ECHO_ALPHA)


func _ring(center: Vector2, k: float, alpha: float) -> void:
	if k <= 0.0 or k >= 1.0:
		return
	var eased := 1.0 - pow(1.0 - k, 3.0)  # sortie rapide, fin douce
	var ink := HudStyle.INK
	ink.a = alpha * (1.0 - k)
	draw_arc(center, lerpf(START_RADIUS, END_RADIUS, eased), 0.0, TAU, 40, ink, WIDTH * (1.0 - 0.5 * k), true)
