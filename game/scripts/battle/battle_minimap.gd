class_name BattleMinimap
extends Control

## Minicarte de bataille (F5b) : champ, régiments en points aux couleurs des camps, cadre de la
## caméra ; clic (ou glisser) = déplacer la caméra. Pure présentation.

signal clicked(world: Vector2)

const SIZE := Vector2(180, 120)
const INK := Color(0.22, 0.14, 0.07)

var field_size: Vector2 = Vector2(1200, 800)
var side_colors: Dictionary = {}
var _terrain: Dictionary = {}
var _relief: ImageTexture = null
var _dots: Array = []  # [Vector2 monde, Color]
var _frame := PackedVector2Array()  # cadre de la caméra au sol (monde x, z)


func _init() -> void:
	custom_minimum_size = SIZE
	mouse_filter = Control.MOUSE_FILTER_STOP
	tooltip_text = "Minicarte : cliquer pour y porter la caméra"


## `terrain` : dictionnaire de `BattleSim.get_terrain()` ; `colors` : camp → couleur.
func setup(terrain: Dictionary, colors: Dictionary) -> void:
	_terrain = terrain
	side_colors = colors
	field_size = Vector2(float(terrain.get("width", 1200.0)), float(terrain.get("depth", 800.0)))
	var nx := int(terrain.get("nx", 0))
	var nz := int(terrain.get("nz", 0))
	var heights: PackedFloat32Array = terrain.get("heights", PackedFloat32Array())
	if nx < 2 or nz < 2 or heights.size() != nx * nz:
		return
	var low := INF
	var high := -INF
	for h in heights:
		low = minf(low, h)
		high = maxf(high, h)
	var image := Image.create(nx, nz, false, Image.FORMAT_RGB8)
	for z in nz:
		for x in nx:
			var t := (heights[z * nx + x] - low) / maxf(high - low, 0.01)
			image.set_pixel(x, z, Color(0.78, 0.74, 0.55).lerp(Color(0.55, 0.47, 0.33), t))
	_relief = ImageTexture.create_from_image(image)
	queue_redraw()


## Régiments (points) et cadre de la caméra, rafraîchis par la scène.
func update(units: Array, frame: PackedVector2Array) -> void:
	_dots.clear()
	for unit in units:
		if bool(unit["present"]):
			var color: Color = side_colors.get(str(unit["side"]), Color.WHITE)
			_dots.append([Vector2(float(unit["x"]), float(unit["z"])), color])
	_frame = frame
	queue_redraw()


func dot_count() -> int:
	return _dots.size()


## Retournée de 180° quand le joueur regarde vers +z (attaquant) : son camp reste en bas.
var flipped: bool = false


func to_map(world: Vector2) -> Vector2:
	var t := world / field_size
	if flipped:
		t = Vector2.ONE - t
	return t * size


func to_world(local: Vector2) -> Vector2:
	var t := (local / size).clamp(Vector2.ZERO, Vector2.ONE)
	if flipped:
		t = Vector2.ONE - t
	return t * field_size


func _draw() -> void:
	var rect := Rect2(Vector2.ZERO, size)
	if _relief != null:
		draw_texture_rect(_relief, Rect2(size, -size) if flipped else rect, false)
	else:
		draw_rect(rect, Color(0.78, 0.74, 0.55))
	var scale_x := size.x / field_size.x
	for zone in _terrain.get("forests", []):
		draw_circle(to_map(Vector2(float(zone["x"]), float(zone["z"]))), float(zone["radius"]) * scale_x, Color(0.3, 0.42, 0.2, 0.75))
	for zone in _terrain.get("mud", []):
		draw_circle(to_map(Vector2(float(zone["x"]), float(zone["z"]))), float(zone["radius"]) * scale_x, Color(0.45, 0.33, 0.2, 0.55))
	var river: Dictionary = _terrain.get("river", {})
	if river.has("points"):
		var line := PackedVector2Array()
		for point in river["points"]:
			line.append(to_map(point))
		draw_polyline(line, Color(0.3, 0.45, 0.65), maxf(float(river.get("width", 10.0)) * scale_x, 2.0))
	for dot in _dots:
		draw_rect(Rect2(to_map(dot[0]) - Vector2(2, 2), Vector2(4, 4)), dot[1])
	if _frame.size() >= 3:
		var outline := PackedVector2Array()
		for point in _frame:
			outline.append(to_map(point))
		outline.append(outline[0])
		draw_polyline(outline, Color(1, 0.97, 0.85), 1.5)
	draw_rect(rect, INK, false, 1.5)


func _gui_input(event: InputEvent) -> void:
	var pressed: bool = event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	var dragged: bool = event is InputEventMouseMotion and (event.button_mask & MOUSE_BUTTON_MASK_LEFT) != 0
	if pressed or dragged:
		clicked.emit(to_world(event.position))
		accept_event()
