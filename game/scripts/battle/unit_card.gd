class_name UnitCard
extends RichPanel

## Carte d'unité compacte du HUD de bataille (F5b, audit UI § 3.2) : icône de classe, effectif,
## nom sur deux lignes au plus (jamais coupé au milieu d'un mot), barres fines moral / fatigue /
## munitions, état. La formation et le détail chiffré passent dans l'infobulle.
## Aucune règle : la carte n'affiche que le dictionnaire de `BattleSim.get_units()`.

signal clicked(unit_id: int, additive: bool)

const WIDTH := 71.0
const INNER_WIDTH := WIDTH - 8.0
const NAME_SIZE := 10
const NAME_MIN_SIZE := 8
const NAME_LINES := 2
const INK := Color(0.22, 0.14, 0.07)
const BORDER := Color(0.42, 0.29, 0.16)
const BORDER_SELECTED := Color(0.95, 0.75, 0.15)
const CATEGORY_GLYPH := {"infantry": "⚔", "archer": "➶", "cavalry": "♞", "siege": "⚙", "tower": "♜", "ram": "⚒"}
const FORMATION_LABELS := {"line": "ligne", "column": "colonne", "square": "schiltron", "wedge": "coin"}

var unit_id: int = -1
var unit_type: String = ""
var unit_name: String = ""
var style: StyleBoxFlat
var name_label: Label
var count_label: Label
var morale_bar: ProgressBar
var fatigue_bar: ProgressBar
var ammo_bar: ProgressBar
var state_label: Label
var groups_label: Label  # numéros des groupes Ctrl+1..9 de l'unité
var _tooltip_key: String = ""


## Nom découpé en `max_lines` lignes de `width` px au plus, uniquement entre deux mots ; ce qui
## ne tient pas est remplacé par « … » après le dernier mot entier. Un mot seul trop long reste
## entier (la carte s'élargit plutôt que de le couper).
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


static func _thin_bar(color: Color) -> ProgressBar:
	var bar := ProgressBar.new()
	bar.custom_minimum_size = Vector2(INNER_WIDTH, 4)
	bar.show_percentage = false
	bar.max_value = 100
	var fill := StyleBoxFlat.new()
	fill.bg_color = color
	var background := StyleBoxFlat.new()
	background.bg_color = Color(0.55, 0.47, 0.33, 0.6)
	bar.add_theme_stylebox_override("fill", fill)
	bar.add_theme_stylebox_override("background", background)
	bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return bar


func _label(size: int) -> Label:
	var label := Label.new()
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", INK)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return label


## Construit la carte pour `unit` (une fois, après l'ajout à l'arbre : la police du thème sert
## à mesurer le nom).
func setup(unit: Dictionary, icon_library: Node) -> void:
	unit_id = int(unit["id"])
	unit_type = str(unit.get("type", ""))
	unit_name = str(unit["name"]) + (" ★" if bool(unit["is_general"]) else "")
	name = "UnitCard%d" % unit_id
	style = StyleBoxFlat.new()
	style.bg_color = Color(0.93, 0.87, 0.72)
	style.border_color = BORDER
	style.set_border_width_all(2)
	style.set_corner_radius_all(3)
	style.set_content_margin_all(3)
	add_theme_stylebox_override("panel", style)
	custom_minimum_size = Vector2(WIDTH, 0)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 1)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(box)
	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 2)
	head.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(head)
	count_label = _label(12)
	if icon_library != null:
		var fallback := "unit_category_" + str(unit.get("render", "infantry"))
		var icon_id: String = unit_type if icon_library.call("has_icon", unit_type) else fallback
		head.add_child(icon_library.call("make_rect", icon_id, 18.0, "unit"))
	else:
		count_label.text = str(CATEGORY_GLYPH.get(str(unit.get("render", "")), "⚔"))
	head.add_child(count_label)
	groups_label = _label(9)
	groups_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	groups_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	groups_label.add_theme_color_override("font_color", Color(0.55, 0.35, 0.05))
	head.add_child(groups_label)
	name_label = _label(NAME_SIZE)
	name_label.name = "Name"
	box.add_child(name_label)
	_fit_name_label()
	morale_bar = _thin_bar(Color(0.25, 0.45, 0.8))
	fatigue_bar = _thin_bar(Color(0.75, 0.45, 0.15))
	ammo_bar = _thin_bar(Color(0.35, 0.5, 0.3))
	for bar in [morale_bar, fatigue_bar, ammo_bar]:
		box.add_child(bar)
	state_label = _label(9)
	state_label.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08))
	state_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_WORD_ELLIPSIS
	state_label.clip_text = true
	state_label.custom_minimum_size = Vector2(INNER_WIDTH, 0)
	box.add_child(state_label)
	gui_input.connect(_on_gui_input)


## Nom sur deux lignes au plus ; la police rétrécit (jusqu'à 8 px) avant de recourir à « … ».
func _fit_name_label() -> void:
	var font := name_label.get_theme_font("font")
	var size := NAME_SIZE
	var text := fit_name(unit_name, font, size, INNER_WIDTH, NAME_LINES)
	while text.ends_with("…") and size > NAME_MIN_SIZE:
		size -= 1
		text = fit_name(unit_name, font, size, INNER_WIDTH, NAME_LINES)
	name_label.add_theme_font_size_override("font_size", size)
	name_label.text = text


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		clicked.emit(unit_id, (event as InputEventMouseButton).shift_pressed)


func set_groups(numbers: Array[int]) -> void:
	var parts := PackedStringArray()
	for number in numbers:
		parts.append(str(number))
	groups_label.text = "".join(parts)


## Met la carte à jour depuis le dictionnaire de l'unité.
func refresh(unit: Dictionary, is_selected: bool) -> void:
	var present: bool = unit["present"]
	count_label.text = str(int(unit["soldiers"]))
	morale_bar.value = float(unit["morale"])
	fatigue_bar.value = float(unit["fatigue"])
	var can_shoot := bool(unit["can_shoot"])
	ammo_bar.visible = can_shoot
	if can_shoot:
		ammo_bar.value = 100.0 * float(unit["ammo"]) / maxf(float(unit["max_ammo"]), 1.0)
	var state := str(unit["state_label"])
	if bool(unit["left_field"]):
		state = "hors du champ"
	elif not present:
		state = "anéantie"
	state_label.text = state
	style.border_color = BORDER_SELECTED if is_selected else BORDER
	style.bg_color = Color(0.93, 0.87, 0.72) if present else Color(0.7, 0.65, 0.55)
	if str(unit["state"]) == "routing":
		style.bg_color = Color(0.9, 0.7, 0.62)
	_refresh_tooltip(unit)


## Infobulle : fiche du type (F2) + effectif, moral, fatigue, munitions et formation courants.
func _refresh_tooltip(unit: Dictionary) -> void:
	var detail := "Effectif : %d / %d · moral %d · fatigue %d" % [int(unit["soldiers"]), int(unit["initial_soldiers"]), int(unit["morale"]), int(unit["fatigue"])]
	if bool(unit["can_shoot"]):
		detail += "\nMunitions : %d / %d%s" % [int(unit["ammo"]), int(unit["max_ammo"]), "" if bool(unit["fire_at_will"]) else " (tir retenu)"]
	detail += "\nFormation : %s" % formation_label(str(unit["formation"]))
	if detail == _tooltip_key:
		return
	_tooltip_key = detail
	tooltip_text = RichTooltip.unit(unit_type, {"name": unit_name}) + "\n" + detail
