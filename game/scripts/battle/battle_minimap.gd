class_name BattleMinimap
extends Control

## Minicarte de bataille (F5b) : champ, régiments en points aux couleurs des camps, cadre de la
## caméra ; clic (ou glisser) = déplacer la caméra. Pure présentation.

signal clicked(world: Vector2)

const SIZE := Vector2(168, 104)
const INK := Color(0.22, 0.14, 0.07)
## CB5 : durée du repère pulsé (secondes), un peu plus court que la colonne d'alertes.
const PING_SECONDS := 2.5

var field_size: Vector2 = Vector2(1200, 800)
var side_colors: Dictionary = {}
var _terrain: Dictionary = {}
var _relief: ImageTexture = null
var _dots: Array = []  # [Vector2 monde, Color]
var _frame := PackedVector2Array()  # cadre de la caméra au sol (monde x, z)
var _ping_world: Vector2 = Vector2.ZERO  # CB5 : lieu d'une alerte cliquée
var _ping_time: float = -1.0  # < 0 : pas de repère actif


func _init() -> void:
	custom_minimum_size = SIZE
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true  # bois et boues débordant du champ
	TooltipHost.attach_plain(self, "battle_minimap_click_camera")
	set_process(false)  # CB5 : seulement pendant un repère pulsé


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


## CB5 : fait pulser un repère à `world` (monde x/z) pendant `PING_SECONDS`, lancé par un clic
## sur une alerte de `BattleAlertsColumn`.
func ping(world: Vector2) -> void:
	_ping_world = world
	_ping_time = 0.0
	set_process(true)
	queue_redraw()


func _process(delta: float) -> void:
	if _ping_time < 0.0:
		set_process(false)
		return
	_ping_time += delta
	if _ping_time >= PING_SECONDS:
		_ping_time = -1.0
		set_process(false)
	queue_redraw()


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
	# B8 : haies, fossés et clôtures (site de bataille B5/B6).
	for obstacle in _terrain.get("obstacles", []):
		var kind: String = str(obstacle.get("kind", "hedge"))
		var color := Color(0.28, 0.4, 0.18, 0.9)  # haie
		var width := 2.0
		if kind == "ditch":
			color = Color(0.35, 0.3, 0.22, 0.9)
			width = 1.5
		elif kind == "fence":
			color = Color(0.5, 0.4, 0.28, 0.9)
			width = 1.0
		elif kind == "palisade":  # CV3-2 : camp retranché
			color = Color(0.42, 0.27, 0.12, 1.0)
			width = 2.5
		draw_line(to_map(obstacle["a"]), to_map(obstacle["b"]), color, width)
	# B8 : village (emprise + maisons).
	var village: Dictionary = _terrain.get("village", {})
	if village.has("x"):
		draw_circle(to_map(Vector2(float(village["x"]), float(village["z"]))), float(village.get("radius", 0.0)) * scale_x, Color(0.55, 0.42, 0.3, 0.45))
		for house in village.get("houses", []):
			draw_circle(to_map(Vector2(float(house["x"]), float(house["z"]))), maxf(float(house.get("length", 6.0)) * 0.25 * scale_x, 1.0), Color(0.42, 0.28, 0.18, 0.85))
	# B8 : côte (trait le long du rivage).
	var coast: Dictionary = _terrain.get("coast", {})
	if coast.has("shore_x"):
		var shore_x := float(coast["shore_x"])
		var top := to_map(Vector2(shore_x, 0.0))
		var bottom := to_map(Vector2(shore_x, field_size.y))
		draw_line(top, bottom, Color(0.3, 0.45, 0.65), 2.0)
	# EP3 : routes (sous l'eau et les ponts), ruisseaux, rivière, gués, ponts.
	for road in _terrain.get("roads", []):
		var road_line := PackedVector2Array()
		for point in road["points"]:
			road_line.append(to_map(point))
		if road_line.size() >= 2:
			draw_polyline(road_line, Color(0.62, 0.5, 0.34, 0.85), 1.5 if str(road.get("kind", "")) == "main" else 1.0)
	for stream in _terrain.get("streams", []):
		var stream_line := PackedVector2Array()
		for point in stream["points"]:
			stream_line.append(to_map(point))
		if stream_line.size() >= 2:
			draw_polyline(stream_line, Color(0.36, 0.52, 0.7), 1.0)
	var river: Dictionary = _terrain.get("river", {})
	if river.has("points"):
		var line := PackedVector2Array()
		for point in river["points"]:
			line.append(to_map(point))
		draw_polyline(line, Color(0.3, 0.45, 0.65), maxf(float(river.get("width", 10.0)) * scale_x, 2.0))
		for ford in river.get("fords", []):
			var fx := float(ford["x"])
			var half := float(ford["half_width"])
			var fz := float(ford["z"])
			draw_line(to_map(Vector2(fx - half, fz)), to_map(Vector2(fx + half, fz)), Color(0.72, 0.8, 0.86), maxf(float(river.get("width", 10.0)) * scale_x * 0.6, 2.0))
	for bridge in _terrain.get("bridges", []):
		var centre := Vector2(float(bridge["x"]), float(bridge["z"]))
		var along := Vector2(cos(float(bridge["yaw"])), sin(float(bridge["yaw"]))) * float(bridge["length"]) * 0.5
		draw_line(to_map(centre - along), to_map(centre + along), Color(0.35, 0.26, 0.18), 3.0)
	for dot in _dots:
		draw_rect(Rect2(to_map(dot[0]) - Vector2(2, 2), Vector2(4, 4)), dot[1])
	if _frame.size() >= 3:
		var outline := PackedVector2Array()
		for point in _frame:
			outline.append(to_map(point))
		outline.append(outline[0])
		draw_polyline(outline, Color(1, 0.97, 0.85), 1.5)
	if _ping_time >= 0.0:
		# CB5 : anneau qui grandit et s'efface (clic sur une alerte de la colonne).
		var t := _ping_time / PING_SECONDS
		var center := to_map(_ping_world)
		draw_arc(center, lerpf(2.0, 14.0, t), 0.0, TAU, 24, Color(1.0, 0.85, 0.3, 1.0 - t), 2.5)
	draw_rect(rect, INK, false, 1.5)


func _gui_input(event: InputEvent) -> void:
	var pressed: bool = event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	var dragged: bool = event is InputEventMouseMotion and (event.button_mask & MOUSE_BUTTON_MASK_LEFT) != 0
	if pressed or dragged:
		clicked.emit(to_world(event.position))
		accept_event()


## Infobulle en sections (`attach_plain` ne pose pas `plain_tooltip_host.gd` sur une classe scriptée).
func _make_custom_tooltip(for_text: String) -> Object:
	return TooltipHost.bubble(for_text, self)
