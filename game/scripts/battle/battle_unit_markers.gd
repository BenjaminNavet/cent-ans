class_name BattleUnitMarkers
extends Control

## Bannières flottantes d'unité (lot B2) : au-dessus de chaque régiment présent,
## un repère à taille écran constante (plaque aux couleurs du camp, icône de classe, barres
## d'effectif et de moral, pastilles d'état, étoile du général). Un seul Control plein écran
## dessine tous les repères ; `_has_point` ne le rend cliquable que sur eux, le reste du champ
## reste à la scène. Clic gauche : sélection ; clic droit sur un repère ennemi : attaque.
## Deux repères qui se chevauchent sont dépilés vers le haut (un trait les relie à leur troupe).
## Lot B7 : en vue très lointaine (distance de caméra au-delà de `CLUSTER_ON`), les repères
## proches d'un même camp se regroupent en une pastille (couleur du camp, nombre de régiments,
## effectif et moral cumulés) ; clic = sélection de tout le groupe. De près, rien ne change.
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
## Décalages essayés, en largeurs / hauteurs de repère, quand un repère en chevauche un autre.
const MAX_STACK := 16
const NUDGES := [Vector2(0, -1), Vector2(1, 0), Vector2(-1, 0), Vector2(1, -1), Vector2(-1, -1), Vector2(0, -2), Vector2(2, 0), Vector2(-2, 0)]
const INK := Color(0.16, 0.10, 0.05)
const PARCHMENT := Color(0.95, 0.90, 0.78)
const GOLD := Color(1.0, 0.82, 0.22)
const ROUT_RED := Color(0.85, 0.12, 0.08)
## B7 : regroupement en vue lointaine (distance de caméra en m, hystérésis contre le
## clignotement) ; deux troupes d'un camp à moins de `CLUSTER_PX` pixels écran se regroupent.
const CLUSTER_ON := 480.0
const CLUSTER_OFF := 420.0
const CLUSTER_PX := 70.0
const CLUSTER_R := 15.0  # rayon de la pastille de groupe

var icon_library: Node = null
var side_colors: Dictionary = {}
var player_side: String = "attacker"
## id -> {rect, anchor, unit, units} ; reconstruit à chaque `update`. Un groupe (B7) est rangé
## sous l'id de son premier régiment ; `units` liste les unités du repère (une seule hors groupe).
var _placed: Dictionary = {}
var _key_of: Dictionary = {}  # id de régiment -> clé de son repère dans `_placed`
var clustered: bool = false  # B7 : vue lointaine, repères regroupés
var _order: Array[int] = []
var _selected: Array = []
var hovered: int = -1  # repère sous la souris
var world_hover: int = -1  # troupe survolée sur le terrain (fournie par la scène)
var _icons: Dictionary = {}  # id -> Texture2D


func _ready() -> void:
	name = "UnitMarkers"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	icon_library = get_node_or_null("/root/IconLibrary")


func setup(colors: Dictionary, p_player_side: String) -> void:
	side_colors = colors
	player_side = p_player_side


## `anchors` : id -> position écran du haut de la troupe (absent = hors champ de la caméra).
## `camera_distance` (B7) : au-delà de `CLUSTER_ON`, les repères proches se regroupent.
func update(units: Array, anchors: Dictionary, selected: Array, camera_distance: float = 0.0) -> void:
	_selected = selected
	_placed.clear()
	_key_of.clear()
	_order.clear()
	if not visible:
		return
	clustered = camera_distance > (CLUSTER_OFF if clustered else CLUSTER_ON)
	var entries: Array = []
	for unit in units:
		var id := int(unit["id"])
		if not bool(unit["present"]) or not anchors.has(id):
			continue
		entries.append([id, anchors[id], unit, [unit]])
	if clustered:
		entries = cluster_entries(entries)
	# Les plus proches de la caméra (bas de l'écran) d'abord : ils gardent leur place.
	entries.sort_custom(func(a: Array, b: Array) -> bool: return (a[1] as Vector2).y > (b[1] as Vector2).y)
	var rects: Array[Rect2] = []
	for entry in entries:
		var anchor: Vector2 = entry[1]
		var base := Rect2(anchor.x - WIDTH * 0.5, anchor.y - HEIGHT, WIDTH, HEIGHT)
		var rect := base
		if _overlaps(base, rects):
			var placed := false
			for nudge in NUDGES:
				var candidate := Rect2(base.position + Vector2(nudge.x * (WIDTH + 14.0 + GAP), nudge.y * (HEIGHT + GAP)), base.size)
				if not _overlaps(candidate, rects):
					rect = candidate
					placed = true
					break
			# Foule (vue très éloignée) : on empile au-dessus jusqu'à une place libre.
			var level := 3
			while not placed and level < MAX_STACK:
				rect = Rect2(base.position - Vector2(0, level * (HEIGHT + GAP)), base.size)
				placed = not _overlaps(rect, rects)
				level += 1
		rects.append(rect)
		_placed[entry[0]] = {"rect": rect, "anchor": anchor, "unit": entry[2], "units": entry[3]}
		for member in entry[3]:
			_key_of[int(member["id"])] = entry[0]
		_order.append(entry[0])
	var mouse := get_local_mouse_position()
	hovered = marker_at(mouse)
	queue_redraw()


## B7 : regroupe les entrées [id, ancre, unité, unités] d'un même camp dont les ancres écran sont
## à moins de `CLUSTER_PX` du centre d'un groupe (glouton, dans l'ordre reçu : stable d'une image
## à l'autre). Un groupe garde l'id et l'unité de son premier régiment, prend l'ancre moyenne et
## la liste de ses unités.
static func cluster_entries(entries: Array) -> Array:
	var groups: Array = []  # [camp, somme des ancres, unités]
	for entry in entries:
		var side := str(entry[2]["side"])
		var anchor: Vector2 = entry[1]
		var best := -1
		var best_distance := CLUSTER_PX
		for i in groups.size():
			if groups[i][0] != side:
				continue
			var center: Vector2 = groups[i][1] / float((groups[i][2] as Array).size())
			var distance := center.distance_to(anchor)
			if distance < best_distance:
				best_distance = distance
				best = i
		if best < 0:
			groups.append([side, anchor, [entry[2]]])
		else:
			groups[best][1] += anchor
			(groups[best][2] as Array).append(entry[2])
	var result: Array = []
	for group in groups:
		var members: Array = group[2]
		result.append([int(members[0]["id"]), group[1] / float(members.size()), members[0], members])
	return result


## Chevauchement avec un repère déjà placé (pastilles d'état comprises, à droite).
static func _overlaps(rect: Rect2, rects: Array[Rect2]) -> bool:
	var wide := rect.grow_individual(0, 0, 14.0, 0)
	for other in rects:
		if other.grow_individual(GAP, GAP, 14.0 + GAP, GAP).intersects(wide):
			return true
	return false


func marker_count() -> int:
	return _placed.size()


## Rectangle du repère d'un régiment (celui de son groupe s'il est regroupé, B7).
func marker_rect(unit_id: int) -> Rect2:
	return (_placed[_key_of[unit_id]]["rect"] as Rect2) if _key_of.has(unit_id) else Rect2()


## Ids des régiments d'un repère (plusieurs pour un groupe, B7).
func marker_members(key: int) -> Array[int]:
	var ids: Array[int] = []
	if _placed.has(key):
		for unit in _placed[key]["units"]:
			ids.append(int(unit["id"]))
	return ids


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
			# Groupe (B7) : clic = tout le groupe sélectionné.
			var members := marker_members(id)
			for i in members.size():
				marker_clicked.emit(members[i], button.shift_pressed or i > 0)
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
		_key_of.clear()
		_order.clear()


# --- Dessin ---------------------------------------------------------------------------


func _draw() -> void:
	var blink := fmod(Time.get_ticks_msec() / 1000.0, 0.6) < 0.3
	# Traits de rappel des repères décalés d'abord, sous toutes les plaques.
	for id in _order:
		var rect: Rect2 = _placed[id]["rect"]
		var anchor: Vector2 = _placed[id]["anchor"]
		var tip_point := Vector2(rect.get_center().x, rect.end.y)
		if tip_point.distance_to(anchor) > 1.0:
			draw_line(tip_point, anchor, Color(INK, 0.7), 1.0)
			draw_circle(anchor, 2.0, Color(INK, 0.7))
	for id in _order:
		if (_placed[id]["units"] as Array).size() > 1:
			_draw_cluster(_placed[id], blink)
		else:
			_draw_marker(id, _placed[id], blink)
	var focus := hovered if hovered >= 0 else int(_key_of.get(world_hover, -1))
	if _placed.has(focus):
		_draw_name(_placed[focus])


func _draw_marker(id: int, entry: Dictionary, blink: bool) -> void:
	var unit: Dictionary = entry["unit"]
	var rect: Rect2 = entry["rect"]
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
	var badges := state_badges(unit)
	for i in badges.size():
		draw_badge(self, badges[i], Vector2(plaque.end.x + 1.0, plaque.position.y + 1.0 + i * 13.0), blink)


## B7 : pastille de groupe (vue lointaine) : disque aux couleurs du camp, sur deux disques
## décalés (pile), nombre de régiments au centre ; effectif et moral cumulés dessous.
func _draw_cluster(entry: Dictionary, blink: bool) -> void:
	var members: Array = entry["units"]
	var rect: Rect2 = entry["rect"]
	var color: Color = side_colors.get(str(entry["unit"]["side"]), Color.GRAY)
	var soldiers := 0.0
	var initial := 0.0
	var morale := 0.0
	var routing := false
	var is_selected := false
	var general := false
	for unit in members:
		var n := float(unit["soldiers"])
		soldiers += n
		initial += float(unit["initial_soldiers"])
		morale += float(unit["morale"]) * n
		routing = routing or str(unit["state"]) == "routing"
		is_selected = is_selected or _selected.has(int(unit["id"]))
		general = general or bool(unit["is_general"])
	var key := int(entry["unit"]["id"])
	var is_hovered: bool = key == hovered or int(_key_of.get(world_hover, -1)) == key
	var frame := ROUT_RED if routing and blink else color
	var center := Vector2(rect.position.x + WIDTH * 0.5, rect.position.y + PLAQUE * 0.5)
	var tip_y := rect.end.y
	draw_colored_polygon(PackedVector2Array([
		Vector2(center.x - 6.0, tip_y - TIP), Vector2(center.x + 6.0, tip_y - TIP), Vector2(center.x, tip_y),
	]), frame)
	# Pile : deux disques en retrait, puis la pastille.
	for offset in [Vector2(5, -4), Vector2(2.5, -2)]:
		draw_circle(center + offset, CLUSTER_R, frame.darkened(0.35))
		draw_arc(center + offset, CLUSTER_R, 0, TAU, 24, INK, 1.0)
	if is_selected:
		draw_arc(center, CLUSTER_R + 3.0, 0, TAU, 28, GOLD, 2.5)
	elif is_hovered:
		draw_arc(center, CLUSTER_R + 2.0, 0, TAU, 28, Color(1, 1, 1, 0.85), 1.5)
	draw_circle(center, CLUSTER_R, frame)
	draw_circle(center, CLUSTER_R - 3.0, PARCHMENT if not routing else Color(0.95, 0.78, 0.72))
	draw_arc(center, CLUSTER_R, 0, TAU, 24, INK, 1.0)
	var font := get_theme_default_font()
	var text := str(members.size())
	var size := 15
	var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size)
	draw_string(font, center + Vector2(-text_size.x * 0.5, text_size.y * 0.35), text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, INK)
	if general:
		_draw_star(center + Vector2(-CLUSTER_R + 2.0, -CLUSTER_R + 2.0), 5.0)
	var y := rect.position.y + PLAQUE + 2.0
	_draw_bar(Rect2(rect.position.x, y, WIDTH, BAR_H), clampf(soldiers / maxf(initial, 1.0), 0.0, 1.0), Color(0.93, 0.9, 0.8))
	var ratio := clampf(morale / maxf(soldiers, 1.0) / 100.0, 0.0, 1.0)
	_draw_bar(Rect2(rect.position.x, y + BAR_H + 1.0, WIDTH, MORALE_H), ratio, morale_color(ratio))


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


## Pastilles d'état d'une unité : déroute, tir, charge ou mêlée (une seule), puis épuisement.
static func state_badges(unit: Dictionary) -> Array[String]:
	var badges: Array[String] = []
	var state := str(unit["state"])
	if state == "routing":
		badges.append("rout")
	elif state == "shooting":
		badges.append("shoot")
	elif state == "charging" or (bool(unit.get("running", false)) and int(unit.get("target", -1)) >= 0):
		badges.append("charge")
	elif state == "melee":
		badges.append("melee")
	if is_exhausted(unit):
		badges.append("tired")
	return badges


## SV4 : épuisée au-delà du seuil où la simulation lui retire du moral (`exhausted_fatigue`,
## lu dans le cœur ; jamais épuisée si les données manquent).
static func is_exhausted(unit: Dictionary) -> bool:
	return float(unit.get("fatigue", 0.0)) > RuleValues.value("exhausted_fatigue", INF)


## Pastille ronde de 12 px (× `scale`) sur `canvas`, pictogramme vectoriel (pas de glyphe).
static func draw_badge(canvas: CanvasItem, kind: String, top_left: Vector2, blink: bool, scale: float = 1.0) -> void:
	canvas.draw_set_transform(top_left + Vector2(6, 6) * scale, 0.0, Vector2(scale, scale))
	var c := Vector2.ZERO
	var bg := PARCHMENT
	if kind == "rout":
		bg = ROUT_RED if blink else Color(0.55, 0.08, 0.05)
	canvas.draw_circle(c, 6.5, bg)
	canvas.draw_arc(c, 6.5, 0, TAU, 16, INK, 1.0)
	var ink := INK if kind != "rout" else Color.WHITE
	match kind:
		"shoot":  # flèche en diagonale
			canvas.draw_line(c + Vector2(-3.5, 3.5), c + Vector2(3.5, -3.5), ink, 1.5)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(3.8, -3.8), c + Vector2(0.2, -3.2), c + Vector2(3.2, -0.2)]), ink)
		"charge":  # double chevron
			for dx in [-2.5, 1.0]:
				canvas.draw_polyline(PackedVector2Array([c + Vector2(dx, -3.5), c + Vector2(dx + 3, 0), c + Vector2(dx, 3.5)]), ink, 1.5)
		"melee":  # épées croisées
			canvas.draw_line(c + Vector2(-3.5, -3.5), c + Vector2(3.5, 3.5), ink, 1.5)
			canvas.draw_line(c + Vector2(3.5, -3.5), c + Vector2(-3.5, 3.5), ink, 1.5)
		"tired":  # goutte
			canvas.draw_circle(c + Vector2(0, 1.2), 2.6, Color(0.2, 0.4, 0.75))
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(-2.3, 0.2), c + Vector2(0, -4.0), c + Vector2(2.3, 0.2)]), Color(0.2, 0.4, 0.75))
		"rout":  # drapeau blanc
			canvas.draw_line(c + Vector2(-2.5, 4), c + Vector2(-2.5, -4), ink, 1.2)
			canvas.draw_rect(Rect2(c + Vector2(-2.5, -4), Vector2(6, 4)), ink)
	canvas.draw_set_transform(Vector2.ZERO)


## B7 : nom d'une troupe seule ; B8 : infobulle détaillée d'un groupe (une ligne par régiment :
## nom, effectif, moral) au lieu du seul décompte global.
func _draw_name(entry: Dictionary) -> void:
	var unit: Dictionary = entry["unit"]
	var rect: Rect2 = entry["rect"]
	var font := get_theme_default_font()
	var size := 13
	var members: Array = entry["units"]
	if members.size() <= 1:
		var text := "%s — %d" % [str(unit["name"]), int(unit["soldiers"])]
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		var box := Rect2(rect.get_center().x - width * 0.5 - 5.0, rect.position.y - 22.0, width + 10.0, 18.0)
		draw_rect(box, Color(PARCHMENT, 0.95))
		draw_rect(box, INK, false, 1.0)
		draw_string(font, box.position + Vector2(5, 14), text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, INK)
		return
	# Groupe : en-tête (total) puis une ligne par régiment, plafonné pour rester lisible.
	const MAX_LINES := 8
	var total := 0
	for member in members:
		total += int(member["soldiers"])
	var lines: Array[String] = ["%d régiments — %d hommes" % [members.size(), total]]
	for i in mini(members.size(), MAX_LINES):
		var member: Dictionary = members[i]
		lines.append("%s — %d (moral %d%%)" % [str(member["name"]), int(member["soldiers"]), int(roundf(float(member["morale"])))])
	if members.size() > MAX_LINES:
		lines.append("… %d autres" % (members.size() - MAX_LINES))
	var width := 0.0
	for line in lines:
		width = maxf(width, font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x)
	var line_h := size + 5.0
	var box := Rect2(rect.get_center().x - width * 0.5 - 5.0, rect.position.y - 8.0 - line_h * lines.size(), width + 10.0, line_h * lines.size() + 6.0)
	draw_rect(box, Color(PARCHMENT, 0.95))
	draw_rect(box, INK, false, 1.0)
	for i in lines.size():
		var y := box.position.y + 14.0 + i * line_h
		draw_string(font, Vector2(box.position.x + 5.0, y), lines[i], HORIZONTAL_ALIGNMENT_LEFT, -1, size, INK)
		if i == 0 and lines.size() > 1:
			draw_line(Vector2(box.position.x + 3.0, y + 4.0), Vector2(box.end.x - 3.0, y + 4.0), Color(INK, 0.4), 1.0)


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
