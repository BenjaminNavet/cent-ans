class_name WindowResizeGrip
extends Control

## RG1 : poignée de redimensionnement (coin bas droit) d'une fenêtre centrale ou compagnon.
## Posée une seule fois par `PanelStack.register` (`attach`). Glisser agrandit ou rétrécit le
## panneau (`size` et `custom_minimum_size`), borné entre sa taille minimale initiale et
## l'écran. La taille choisie est mémorisée pour la session (pas sur disque) et signalée par
## la méta `USER_SIZED` (les mises en page ne la défont pas).

const USER_SIZED := &"ui_user_sized"
const GRIP_SIZE := 18.0
## Marge laissée entre la fenêtre et le bord de l'écran.
const SCREEN_MARGIN := 4.0

## Nom du panneau → dernière taille choisie (durée de la session).
static var _remembered: Dictionary = {}

var _panel: Control
var _floor := Vector2.ZERO
var _dragging := false


## Pose la poignée sur `panel` (idempotent) et applique la taille mémorisée.
static func attach(panel: Control) -> WindowResizeGrip:
	var existing := panel.get_node_or_null("ResizeGrip") as WindowResizeGrip
	if existing != null:
		return existing
	var grip := WindowResizeGrip.new()
	grip.name = "ResizeGrip"
	grip._panel = panel
	grip._floor = panel.get_combined_minimum_size()
	panel.add_child(grip)
	return grip


static func forget_all() -> void:
	_remembered.clear()


func _init() -> void:
	top_level = true
	custom_minimum_size = Vector2(GRIP_SIZE, GRIP_SIZE)
	size = custom_minimum_size
	mouse_default_cursor_shape = Control.CURSOR_FDIAGSIZE
	mouse_filter = Control.MOUSE_FILTER_STOP
	z_index = 10


func _ready() -> void:
	set_process(true)
	if _remembered.has(_panel.name):
		_apply.call_deferred(_remembered[_panel.name])
	_follow()


func _process(_delta: float) -> void:
	_follow()


func _follow() -> void:
	visible = _panel != null and _panel.is_visible_in_tree()
	if visible:
		global_position = _panel.get_global_rect().end - size


func _draw() -> void:
	var tri := PackedVector2Array([Vector2(size.x - 1.0, 1.0), Vector2(size.x - 1.0, size.y - 1.0), Vector2(1.0, size.y - 1.0)])
	draw_colored_polygon(tri, Color(0.45, 0.32, 0.16, 0.75))
	draw_line(Vector2(size.x - 1.0, size.y * 0.45), Vector2(size.x * 0.45, size.y - 1.0), Color(0.95, 0.88, 0.7, 0.8), 1.0)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
		_dragging = (event as InputEventMouseButton).pressed
		if _dragging and not _panel.has_meta(USER_SIZED):
			_floor = _panel.get_combined_minimum_size()
		accept_event()
	elif event is InputEventMouseMotion and _dragging:
		drag_by((event as InputEventMouseMotion).relative)
		accept_event()


## Agrandit (ou rétrécit) le panneau de `delta` pixels, borné.
func drag_by(delta: Vector2) -> void:
	_apply(_panel.size + delta)


func _apply(wanted: Vector2) -> void:
	var view := _panel.get_viewport_rect().size
	var origin := _panel.get_global_rect().position
	var ceiling := Vector2(view.x - origin.x - SCREEN_MARGIN, view.y - origin.y - SCREEN_MARGIN)
	var clamped := Vector2(
		clampf(wanted.x, _floor.x, maxf(_floor.x, ceiling.x)),
		clampf(wanted.y, _floor.y, maxf(_floor.y, ceiling.y)))
	_panel.set_meta(USER_SIZED, true)
	_panel.custom_minimum_size = clamped
	_panel.size = clamped
	_remembered[_panel.name] = clamped
	_follow()
