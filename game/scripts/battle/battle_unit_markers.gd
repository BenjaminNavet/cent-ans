class_name BattleUnitMarkers
extends Control

## Bannières flottantes d'unité à la Total War (lot B2) : au-dessus de chaque régiment présent,
## un repère à taille écran constante (plaque aux couleurs du camp, icône de classe, barres
## d'effectif et de moral, pastilles d'état, étoile du général). Un seul Control plein écran
## dessine tous les repères ; `_has_point` ne le rend cliquable que sur eux, le reste du champ
## reste à la scène. Clic gauche : sélection ; clic droit sur un repère ennemi : attaque.
## Deux repères qui se chevauchent sont dépilés vers le haut (un trait les relie à leur troupe).
## Aucune règle : tout vient du dictionnaire de `BattleSim.get_units()` ; seuils d'affichage
## (moral bas, épuisement) purement visuels.

signal marker_clicked(unit_id: int, additive: bool)
signal marker_right_clicked(unit_id: int)

const WIDTH := 38.0
const PLAQUE := 32.0  # hauteur de la plaque (icône)
const BAR_H := 4.0
const MORALE_H := 3.0
const TIP := 7.0  # pointe vers la troupe
const HEIGHT := PLAQUE + 2.0 + BAR_H + 1.0 + MORALE_H + TIP
const GAP := 2.0
const MAX_STACK := 8
const INK := Color(0.16, 0.10, 0.05)
const PARCHMENT := Color(0.95, 0.90, 0.78)
const GOLD := Color(1.0, 0.82, 0.22)
const ROUT_RED := Color(0.85, 0.12, 0.08)
const EXHAUSTED_FATIGUE := 60.0  # même seuil que la simulation (vitesse réduite)

var icon_library: Node = null
var side_colors: Dictionary = {}
var player_side: String = "attacker"
## id -> {rect, anchor, unit} ; reconstruit à chaque `update`.
var _placed: Dictionary = {}
var _order: Array[int] = []
var _selected: Array = []
var hovered: int = -1  # repère sous la souris
var world_hover: int = -1  # troupe survolée sur le terrain (fournie par la scène)
var _icons: Dictionary = {}  # id -> Texture2D


func _ready() -> void:
	name = "UnitMarkers"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	icon_library = get_node_or_null("/root/IconLibrary")


func setup(colors: Dictionary, p_player_side: String) -> void:
	side_colors = colors
	player_side = p_player_side


## `anchors` : id -> position écran du haut de la troupe (absent = hors champ de la caméra).
func update(units: Array, anchors: Dictionary, selected: Array) -> void:
	_selected = selected
	_placed.clear()
	_order.clear()
	if not visible:
		return
	var entries: Array = []
	for unit in units:
		var id := int(unit["id"])
		if not bool(unit["present"]) or not anchors.has(id):
			continue
		entries.append([id, anchors[id], unit])
	# Les plus proches de la caméra (bas de l'écran) d'abord : ils gardent leur place.
	entries.sort_custom(func(a: Array, b: Array) -> bool: return (a[1] as Vector2).y > (b[1] as Vector2).y)
	var rects: Array[Rect2] = []
	for entry in entries:
		var anchor: Vector2 = entry[1]
		var rect := Rect2(anchor.x - WIDTH * 0.5, anchor.y - HEIGHT, WIDTH, HEIGHT)
		for _i in MAX_STACK:
			var hit := _overlap(rect, rects)
			if hit.size.x <= 0.0:
				break
			rect.position.y = hit.position.y - HEIGHT - GAP
		rects.append(rect)
		_placed[entry[0]] = {"rect": rect, "anchor": anchor, "unit": entry[2]}
		_order.append(entry[0])
	var mouse := get_local_mouse_position()
	hovered = marker_at(mouse)
	queue_redraw()


static func _overlap(rect: Rect2, rects: Array[Rect2]) -> Rect2:
	for other in rects:
		if other.grow(GAP * 0.5).intersects(rect):
			return other
	return Rect2()


func marker_count() -> int:
	return _placed.size()


func marker_rect(unit_id: int) -> Rect2:
	return (_placed[unit_id]["rect"] as Rect2) if _placed.has(unit_id) else Rect2()


## Id du repère sous `point` (coordonnées locales), -1 sinon ; le dernier dessiné l'emporte.
func marker_at(point: Vector2) -> int:
	for i in range(_order.size() - 1, -1, -1):
		var id: int = _order[i]
		if (_placed[id]["rect"] as Rect2).has_point(point):
			return id
	return -1


func _has_point(point: Vector2) -> bool:
	return visible and marker_at(point) >= 0


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		var button := event as InputEventMouseButton
		var id := marker_at(button.position)
		if id < 0:
			return
		if button.button_index == MOUSE_BUTTON_LEFT:
			marker_clicked.emit(id, button.shift_pressed)
			accept_event()
		elif button.button_index == MOUSE_BUTTON_RIGHT:
			marker_right_clicked.emit(id)
			accept_event()
	elif event is InputEventMouseMotion:
		var id := marker_at((event as InputEventMouseMotion).position)
		if id != hovered:
			hovered = id
			queue_redraw()


func toggle() -> void:
	visible = not visible
	if not visible:
		_placed.clear()
		_order.clear()


# --- Dessin ---------------------------------------------------------------------------


func _draw() -> void:
	var blink := fmod(Time.get_ticks_msec() / 1000.0, 0.6) < 0.3
	for id in _order:
		_draw_marker(id, _placed[id], blink)
	var focus := hovered if hovered >= 0 else world_hover
	if _placed.has(focus):
		_draw_name(_placed[focus])


func _draw_marker(id: int, entry: Dictionary, blink: bool) -> void:
	var unit: Dictionary = entry["unit"]
	var rect: Rect2 = entry["rect"]
	var anchor: Vector2 = entry["anchor"]
	var side := str(unit["side"])
	var color: Color = side_colors.get(side, Color.GRAY)
	var state := str(unit["state"])
	var routing := state == "routing"
	var is_selected := _selected.has(id)
	var is_hovered := id == hovered or id == world_hover
	var frame := color
	if routing and blink:
		frame = ROUT_RED
	var plaque := Rect2(rect.position, Vector2(WIDTH, PLAQUE))
	# Trait de rappel quand le repère a été dépilé.
	var tip_y := rect.end.y
	if anchor.y - tip_y > 1.0:
		draw_line(Vector2(anchor.x, tip_y), anchor, Color(INK, 0.7), 1.0)
	# Pointe vers la troupe.
	var tip := PackedVector2Array([
		Vector2(rect.position.x + WIDTH * 0.5 - 6.0, tip_y - TIP),
		Vector2(rect.position.x + WIDTH * 0.5 + 6.0, tip_y - TIP),
		Vector2(rect.position.x + WIDTH * 0.5, tip_y),
	])
	draw_colored_polygon(tip, frame)
	if is_selected:
		draw_rect(plaque.grow(3.0), GOLD, false, 2.5)
	elif is_hovered:
		draw_rect(plaque.grow(2.0), Color(1, 1, 1, 0.85), false, 1.5)
	# Plaque : cadre aux couleurs du camp, fond parchemin, icône de classe.
	draw_rect(plaque, frame)
	var inner := plaque.grow(-3.0)
	var fill := PARCHMENT if not routing else Color(0.95, 0.78, 0.72)
	if not is_hovered and not is_selected:
		fill = fill.darkened(0.06)
	draw_rect(inner, fill)
	var icon := _icon_for(unit)
	if icon != null:
		var size := inner.size.y - 2.0
		draw_texture_rect(icon, Rect2(inner.get_center() - Vector2(size, size) * 0.5, Vector2(size, size)), false)
	draw_rect(plaque, INK, false, 1.0)
	if bool(unit["is_general"]):
		_draw_star(plaque.position + Vector2(6.5, 6.5), 5.0)
	# Effectif et moral.
	var y := plaque.end.y + 2.0
	var ratio := clampf(float(unit["soldiers"]) / maxf(float(unit["initial_soldiers"]), 1.0), 0.0, 1.0)
	_draw_bar(Rect2(rect.position.x, y, WIDTH, BAR_H), ratio, Color(0.93, 0.9, 0.8))
	var morale := clampf(float(unit["morale"]) / 100.0, 0.0, 1.0)
	_draw_bar(Rect2(rect.position.x, y + BAR_H + 1.0, WIDTH, MORALE_H), morale, morale_color(morale))
	# Pastilles d'état (colonne à droite de la plaque).
	var badges: Array[String] = []
	if routing:
		badges.append("rout")
	elif state == "shooting":
		badges.append("shoot")
	elif state == "charging" or (bool(unit.get("running", false)) and int(unit.get("target", -1)) >= 0):
		badges.append("charge")
	elif state == "melee":
		badges.append("melee")
	if float(unit["fatigue"]) >= EXHAUSTED_FATIGUE:
		badges.append("tired")
	for i in badges.size():
		_draw_badge(badges[i], Vector2(plaque.end.x + 1.0, plaque.position.y + 1.0 + i * 13.0), blink)


func _draw_bar(rect: Rect2, ratio: float, color: Color) -> void:
	draw_rect(rect, Color(0.1, 0.07, 0.04, 0.85))
	draw_rect(Rect2(rect.position, Vector2(rect.size.x * ratio, rect.size.y)), color)


static func morale_color(ratio: float) -> Color:
	if ratio >= 0.6:
		return Color(0.35, 0.72, 0.3)
	if ratio >= 0.3:
		return Color(0.92, 0.68, 0.15)
	return Color(0.85, 0.2, 0.12)


func _draw_star(center: Vector2, radius: float) -> void:
	var points := PackedVector2Array()
	for i in 10:
		var angle := -PI * 0.5 + i * PI / 5.0
		var r := radius if i % 2 == 0 else radius * 0.45
		points.append(center + Vector2(cos(angle), sin(angle)) * r)
	draw_colored_polygon(points, GOLD)
	draw_polyline(points + PackedVector2Array([points[0]]), INK, 1.0)


## Pastille ronde de 12 px, pictogramme vectoriel (pas de glyphe de police).
func _draw_badge(kind: String, top_left: Vector2, blink: bool) -> void:
	var c := top_left + Vector2(6, 6)
	var bg := PARCHMENT
	if kind == "rout":
		bg = ROUT_RED if blink else Color(0.55, 0.08, 0.05)
	draw_circle(c, 6.5, bg)
	draw_arc(c, 6.5, 0, TAU, 16, INK, 1.0)
	var ink := INK if kind != "rout" else Color.WHITE
	match kind:
		"shoot":  # flèche en diagonale
			draw_line(c + Vector2(-3.5, 3.5), c + Vector2(3.5, -3.5), ink, 1.5)
			draw_colored_polygon(PackedVector2Array([c + Vector2(3.8, -3.8), c + Vector2(0.2, -3.2), c + Vector2(3.2, -0.2)]), ink)
		"charge":  # double chevron
			for dx in [-2.5, 1.0]:
				draw_polyline(PackedVector2Array([c + Vector2(dx, -3.5), c + Vector2(dx + 3, 0), c + Vector2(dx, 3.5)]), ink, 1.5)
		"melee":  # épées croisées
			draw_line(c + Vector2(-3.5, -3.5), c + Vector2(3.5, 3.5), ink, 1.5)
			draw_line(c + Vector2(3.5, -3.5), c + Vector2(-3.5, 3.5), ink, 1.5)
		"tired":  # goutte
			draw_circle(c + Vector2(0, 1.2), 2.6, Color(0.2, 0.4, 0.75))
			draw_colored_polygon(PackedVector2Array([c + Vector2(-2.3, 0.2), c + Vector2(0, -4.0), c + Vector2(2.3, 0.2)]), Color(0.2, 0.4, 0.75))
		"rout":  # drapeau blanc
			draw_line(c + Vector2(-2.5, 4), c + Vector2(-2.5, -4), ink, 1.2)
			draw_rect(Rect2(c + Vector2(-2.5, -4), Vector2(6, 4)), ink)


func _draw_name(entry: Dictionary) -> void:
	var unit: Dictionary = entry["unit"]
	var rect: Rect2 = entry["rect"]
	var font := get_theme_default_font()
	var size := 13
	var text := "%s — %d" % [str(unit["name"]), int(unit["soldiers"])]
	var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
	var box := Rect2(rect.get_center().x - width * 0.5 - 5.0, rect.position.y - 22.0, width + 10.0, 18.0)
	draw_rect(box, Color(PARCHMENT, 0.95))
	draw_rect(box, INK, false, 1.0)
	draw_string(font, box.position + Vector2(5, 14), text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, INK)


func _icon_for(unit: Dictionary) -> Texture2D:
	var id := int(unit["id"])
	if _icons.has(id):
		return _icons[id]
	var texture: Texture2D = null
	if icon_library != null:
		var category := "unit_category_" + str(unit.get("render", "infantry"))
		var icon_id: String = category if icon_library.call("has_icon", category) else str(unit.get("type", ""))
		texture = icon_library.call("get_icon", icon_id, "unit")
	_icons[id] = texture
	return texture
