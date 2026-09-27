class_name UnitCard
extends RichPanel

## Carte d'unité en vignette (lot B2, après F5b) : illustration du type d'unité en
## fond (`res://assets/illustrations/<type>.jpg` si elle existe ; sinon composition du blason de
## la faction, de la couleur du camp et de l'icône de classe), effectif en gros, barres fines
## moral / fatigue / munitions, état en pastille, étoile du général, numéros de groupe. Le nom,
## la formation et le détail chiffré passent dans l'infobulle riche (RichTooltip).
## Aucune règle : la carte n'affiche que le dictionnaire de `BattleSim.get_units()`.

signal clicked(unit_id: int, additive: bool)
## B3 / T6 : double-clic = centrer la caméra sur ce régiment (comme TW).
signal double_clicked(unit_id: int)

const WIDTH := 64.0
const HEIGHT := 94.0
const ILLUSTRATIONS_DIR := "res://assets/illustrations/"
const INK := Color(0.22, 0.14, 0.07)
const BORDER := Color(0.42, 0.29, 0.16)
const BORDER_SELECTED := Color(0.95, 0.75, 0.15)
const FORMATION_LABELS := {"line": "ligne", "column": "colonne", "square": "schiltron", "wedge": "coin"}
const BAR_H := 4.0

var unit_id: int = -1
var unit_type: String = ""
var unit_name: String = ""
var side_color: Color = Color(0.3, 0.3, 0.6)
var style: StyleBoxFlat
var art: Control  # vignette dessinée
var illustration: Texture2D = null
var heraldry: Texture2D = null
var class_icon: Texture2D = null
## DA5b : l'icône de classe est une miniature peinte encadrée (pas de pastille de parchemin).
var class_icon_is_miniature: bool = false
var is_general: bool = false
var _unit: Dictionary = {}
var _selected: bool = false
var _groups: String = ""
## CB1 : membre d'un groupe verrouillé (cadenas dessiné en code faute d'icône DA5).
var locked: bool = false
var _tooltip_key: String = ""


## Nom découpé en `max_lines` lignes de `width` px au plus, uniquement entre deux mots ; ce qui
## ne tient pas est remplacé par « … » après le dernier mot entier. Un mot seul trop long reste
## entier. (Sert encore aux libellés courts ; la carte n'affiche plus le nom.)
static func fit_name(text: String, font: Font, size: int, width: float, max_lines: int) -> String:
	var words := text.split(" ", false)
	var lines: Array[String] = []
	var current := ""
	var index := 0
	while index < words.size():
		var candidate := words[index] if current == "" else current + " " + words[index]
		if current == "" or font.get_string_size(candidate, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x <= width:
			current = candidate
			index += 1
			continue
		lines.append(current)
		current = ""
		if lines.size() == max_lines:
			break
	if current != "" and lines.size() < max_lines:
		lines.append(current)
	if index < words.size():
		var last := lines[lines.size() - 1]
		while font.get_string_size(last + " …", HORIZONTAL_ALIGNMENT_LEFT, -1, size).x > width and last.contains(" "):
			last = last.substr(0, last.rfind(" "))
		lines[lines.size() - 1] = last + " …"
	return "\n".join(lines)


static func formation_label(key: String) -> String:
	return str(FORMATION_LABELS.get(key, "ligne"))


## Illustration peinte du type d'unité, null si absente.
static func illustration_for(type_id: String) -> Texture2D:
	if type_id == "":
		return null
	return PortraitLoader.load_texture(ILLUSTRATIONS_DIR + type_id + ".jpg")


## Construit la carte pour `unit` du camp `faction` (couleur `color`).
func setup(unit: Dictionary, icon_library: Node, faction: String = "", color: Color = Color(0.3, 0.3, 0.6)) -> void:
	unit_id = int(unit["id"])
	unit_type = str(unit.get("type", ""))
	is_general = bool(unit["is_general"])
	unit_name = str(unit["name"]) + (" ★" if is_general else "")
	side_color = color
	name = "UnitCard%d" % unit_id
	style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.15, 0.1)
	style.border_color = BORDER
	style.set_border_width_all(2)
	style.set_corner_radius_all(3)
	style.set_content_margin_all(2)
	add_theme_stylebox_override("panel", style)
	custom_minimum_size = Vector2(WIDTH, HEIGHT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	illustration = illustration_for(unit_type)
	heraldry = PortraitLoader.heraldry_texture(faction)
	if icon_library != null:
		var category := "unit_category_" + str(unit.get("render", "infantry"))
		var icon_id: String = category if icon_library.call("has_icon", category) else unit_type
		class_icon = icon_library.call("get_icon", icon_id, "unit")
		class_icon_is_miniature = icon_library.has_method("is_entity") and bool(icon_library.call("is_entity", icon_id, "unit"))
	art = Control.new()
	art.name = "Art"
	art.mouse_filter = Control.MOUSE_FILTER_IGNORE
	art.clip_contents = true
	art.draw.connect(_draw_art)
	add_child(art)
	gui_input.connect(_on_gui_input)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var button := event as InputEventMouseButton
		if button.double_click:
			double_clicked.emit(unit_id)
		else:
			clicked.emit(unit_id, button.shift_pressed)


func set_groups(numbers: Array[int]) -> void:
	var parts := PackedStringArray()
	for number in numbers:
		parts.append(str(number))
	var text := "".join(parts)
	if text != _groups:
		_groups = text
		art.queue_redraw()


## CB1 : cadenas du groupe verrouillé.
func set_locked(value: bool) -> void:
	if value != locked:
		locked = value
		art.queue_redraw()
		_tooltip_key = ""


func has_illustration() -> bool:
	return illustration != null


## Met la carte à jour depuis le dictionnaire de l'unité.
func refresh(unit: Dictionary, is_selected: bool) -> void:
	_unit = unit
	_selected = is_selected
	style.border_color = BORDER_SELECTED if is_selected else BORDER
	style.set_border_width_all(3 if is_selected else 2)
	art.queue_redraw()
	_refresh_tooltip(unit)


## État court de la carte (hors du champ, réserve, anéantie, sinon libellé de la simulation).
static func state_text(unit: Dictionary) -> String:
	if bool(unit["left_field"]):
		return "hors du champ"
	if bool(unit.get("reserve", false)):
		return "en réserve"
	if not bool(unit["present"]):
		return "anéantie"
	return str(unit["state_label"])


func _draw_art() -> void:
	if _unit.is_empty():
		return
	var size := art.size
	var body := Rect2(Vector2.ZERO, size)
	# Fond : illustration recadrée en portrait, sinon camp + blason + icône.
	if illustration != null:
		var tex := illustration.get_size()
		var crop_w := minf(tex.x, tex.y * size.x / maxf(size.y, 1.0))
		art.draw_texture_rect_region(illustration, body, Rect2((tex.x - crop_w) * 0.5, 0, crop_w, tex.y))
	else:
		art.draw_rect(body, side_color.darkened(0.35))
		if heraldry != null:
			art.draw_texture_rect(heraldry, Rect2(size.x * 0.12, size.y * 0.1, size.x * 0.76, size.x * 0.76 * 1.15), false, Color(1, 1, 1, 0.35))
		if class_icon != null:
			var icon_size := size.x * 0.62
			art.draw_circle(Vector2(size.x * 0.5, size.y * 0.42), icon_size * 0.52, Color(0.95, 0.9, 0.78, 0.9))
			art.draw_texture_rect(class_icon, Rect2(Vector2(size.x * 0.5, size.y * 0.42) - Vector2(icon_size, icon_size) * 0.5, Vector2(icon_size, icon_size)), false)
	# Bandeau aux couleurs du camp, icône de classe, étoile, groupes.
	art.draw_rect(Rect2(0, 0, size.x, 4), side_color)
	if illustration != null and class_icon != null:
		if class_icon_is_miniature:
			art.draw_texture_rect(class_icon, Rect2(1, 5, 20, 20), false)
		else:
			art.draw_rect(Rect2(1, 5, 17, 17), Color(0.95, 0.9, 0.78, 0.92))
			art.draw_texture_rect(class_icon, Rect2(2, 6, 15, 15), false)
	var font := get_theme_default_font()
	if is_general:
		_draw_star(Vector2(size.x - 8, 12), 6.0)
	if _groups != "":
		art.draw_string_outline(font, Vector2(3, 34), _groups, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, 3, Color(0.1, 0.06, 0.03))
		art.draw_string(font, Vector2(3, 34), _groups, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color(1, 0.82, 0.3))
	if locked:
		draw_padlock(art, Vector2(9, 46 if _groups != "" else 33), 1.0)
	# Bas : dégradé sombre, effectif en gros, barres fines.
	var shade_top := size.y - 34.0
	art.draw_polygon(PackedVector2Array([Vector2(0, shade_top), Vector2(size.x, shade_top), Vector2(size.x, size.y), Vector2(0, size.y)]),
		PackedColorArray([Color(0, 0, 0, 0), Color(0, 0, 0, 0), Color(0, 0, 0, 0.85), Color(0, 0, 0, 0.85)]))
	var count := str(int(_unit["soldiers"]))
	# UB1 / U9 : santé (effectif restant), moral, munitions ; la fatigue passe en pastille « épuisée ».
	var health := float(_unit["soldiers"]) / maxf(float(_unit.get("initial_soldiers", _unit["soldiers"])), 1.0)
	var bars: Array = [[health, Color(0.86, 0.80, 0.58)], [float(_unit["morale"]) / 100.0, BattleUnitMarkers.morale_color(float(_unit["morale"]) / 100.0)]]
	if bool(_unit["can_shoot"]):
		bars.append([float(_unit["ammo"]) / maxf(float(_unit["max_ammo"]), 1.0), Color(0.55, 0.75, 0.45)])
	var bar_y := size.y - 2.0 - bars.size() * (BAR_H + 1.0)
	art.draw_string_outline(font, Vector2(3, bar_y - 3), count, HORIZONTAL_ALIGNMENT_LEFT, -1, 19, 4, Color(0.08, 0.05, 0.02))
	art.draw_string(font, Vector2(3, bar_y - 3), count, HORIZONTAL_ALIGNMENT_LEFT, -1, 19, Color(1, 0.97, 0.88))
	for bar in bars:
		var rect := Rect2(2, bar_y, size.x - 4, BAR_H)
		art.draw_rect(rect, Color(0.15, 0.1, 0.06, 0.9))
		art.draw_rect(Rect2(rect.position, Vector2(rect.size.x * clampf(float(bar[0]), 0.0, 1.0), BAR_H)), bar[1])
		bar_y += BAR_H + 1.0
	# État : pastille en haut à droite (sous l'étoile), voile gris hors du champ, rouge en déroute.
	var badges := BattleUnitMarkers.state_badges(_unit)
	var blink := fmod(Time.get_ticks_msec() / 1000.0, 0.6) < 0.3
	for i in badges.size():
		BattleUnitMarkers.draw_badge(art, badges[i], Vector2(size.x - 19, 19 + i * 18), blink, 1.3)
	if not bool(_unit["present"]) or bool(_unit["left_field"]):
		art.draw_rect(body, Color(0.25, 0.22, 0.2, 0.65))
		_draw_cross(size)
	elif str(_unit["state"]) == "routing":
		art.draw_rect(body, Color(0.8, 0.1, 0.05, 0.3 if blink else 0.15))
	elif bool(_unit.get("reserve", false)):
		art.draw_rect(body, Color(0.9, 0.85, 0.7, 0.35))


func _draw_star(center: Vector2, radius: float) -> void:
	var points := PackedVector2Array()
	for i in 10:
		var angle := -PI * 0.5 + i * PI / 5.0
		points.append(center + Vector2(cos(angle), sin(angle)) * (radius if i % 2 == 0 else radius * 0.45))
	art.draw_colored_polygon(points, BattleUnitMarkers.GOLD)
	art.draw_polyline(points + PackedVector2Array([points[0]]), INK, 1.0)


## CB1 : cadenas (anse et corps) centré en `center`, à l'échelle `k` ; glyphe provisoire en
## attendant une icône DA5, comme les curseurs de CB-M2.
static func draw_padlock(canvas: CanvasItem, center: Vector2, k: float) -> void:
	var dark := Color(0.1, 0.06, 0.03)
	var gold := Color(1, 0.82, 0.3)
	var body := Rect2(center + Vector2(-5.5, -1.5) * k, Vector2(11, 8.5) * k)
	canvas.draw_arc(center + Vector2(0, -2) * k, 3.6 * k, PI, TAU, 10, dark, 3.4 * k)
	canvas.draw_arc(center + Vector2(0, -2) * k, 3.6 * k, PI, TAU, 10, gold, 1.6 * k)
	canvas.draw_rect(body.grow(1.2 * k), dark)
	canvas.draw_rect(body, gold)
	canvas.draw_circle(center + Vector2(0, 2.2) * k, 1.3 * k, dark)


## Croix de Saint-André discrète sur une unité anéantie ou sortie du champ.
func _draw_cross(size: Vector2) -> void:
	var color := Color(0.1, 0.06, 0.04, 0.8)
	art.draw_line(Vector2(size.x * 0.25, size.y * 0.2), Vector2(size.x * 0.75, size.y * 0.6), color, 3.0)
	art.draw_line(Vector2(size.x * 0.75, size.y * 0.2), Vector2(size.x * 0.25, size.y * 0.6), color, 3.0)


## Infobulle : fiche du type (F2) + état, effectif, moral, fatigue, munitions et formation.
func _refresh_tooltip(unit: Dictionary) -> void:
	var detail := "État : %s" % state_text(unit)
	if BattleUnitMarkers.is_exhausted(unit):
		detail += " · épuisée"
	detail += "\nEffectif : %d / %d · moral %d · fatigue %d" % [int(unit["soldiers"]), int(unit["initial_soldiers"]), int(unit["morale"]), int(unit["fatigue"])]
	if bool(unit["can_shoot"]):
		detail += "\nMunitions : %d / %d%s" % [int(unit["ammo"]), int(unit["max_ammo"]), "" if bool(unit["fire_at_will"]) else " (tir retenu)"]
	detail += "\nFormation : %s" % formation_label(str(unit["formation"]))
	if _groups != "":
		detail += "\nGroupe(s) : %s" % _groups
	if locked:
		detail += "\nGroupe verrouillé : se déplace d'un bloc (Ctrl+G : déverrouiller)"
	if detail == _tooltip_key:
		return
	_tooltip_key = detail
	tooltip_text = RichTooltip.unit(unit_type, {"name": unit_name}) + "\n" + detail
