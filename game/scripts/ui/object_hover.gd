class_name ObjectHover
extends DecorHover

## WH hover (ADR 0271) : bulle riche différée au survol d'un objet de la carte (armée, colonie,
## armée perdue de vue). Même décompte que `DecorHover` (immobilité, délai du réglage
## `interface/decor_hover_delay`, fermeture à l'éloignement) ; au lieu d'une fiche codex, affiche
## le BBCode rendu par `text_provider` (point écran → texte, "" : rien). La bulle ne capte pas la
## souris et se pose près du curseur, dans la fenêtre. Présentation pure.

## Callable(screen_position: Vector2) -> String (BBCode).
var text_provider: Callable
var _layer: CanvasLayer = null
var shown_text := ""


func _ready() -> void:
	super._ready()
	_layer = CanvasLayer.new()
	_layer.name = "ObjectHoverLayer"
	_layer.layer = 90
	add_child(_layer)


func _fire() -> void:
	state = State.FIRED
	set_process(false)
	if not text_provider.is_valid():
		return
	if not ignore_gui and is_inside_tree() and get_viewport().gui_get_hovered_control() != null:
		return
	var text := str(text_provider.call(_anchor))
	if text == "":
		return
	var panel := TooltipHost.from_bbcode(text) as PanelContainer
	if panel == null:
		return
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for child in panel.find_children("*", "Control", true, false):
		(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	_layer.add_child(panel)
	var viewport_size := get_viewport().get_visible_rect().size if is_inside_tree() else Vector2(1280, 720)
	var size := panel.get_combined_minimum_size()
	panel.position = Vector2(
		clampf(_anchor.x + 18.0, 4.0, maxf(viewport_size.x - size.x - 4.0, 4.0)),
		clampf(_anchor.y + 18.0, 4.0, maxf(viewport_size.y - size.y - 4.0, 4.0)))
	_bubble = panel
	shown_text = text
	shown_id = "object"
	state = State.SHOWN
	bubble_shown.emit(shown_id)


func _close() -> void:
	if _bubble != null and is_instance_valid(_bubble):
		_bubble.queue_free()
	_bubble = null
	shown_id = ""
	shown_text = ""
