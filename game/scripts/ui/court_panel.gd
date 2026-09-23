class_name CourtPanel
extends PanelContainer

## Panneau « Cour » (bouton de la barre, touche C) : liste des personnages vivants de la
## faction du joueur (`CampaignSim.get_faction_characters` + `get_character`), triable par
## âge/nom, filtrable par rôle. Portrait placeholder = carré de couleur de faction + initiales.
## Aucune règle ici : la simulation fournit rôle, titre, âge.

signal character_selected(character_id: String)
signal closed

const SORT_RANK := 2
const SORT_AGE := 0
const SORT_NAME := 1
const FILTER_ALL := 0
const FILTER_GENERAL := 1
const FILTER_GOVERNOR := 2
const FILTER_COURT := 3

@onready var title_label: Label = %TitleLabel
@onready var sort_option: OptionButton = %SortOption
@onready var filter_option: OptionButton = %FilterOption
@onready var rows_list: VBoxContainer = %RowsList
@onready var close_button: Button = %CloseButton
@onready var empty_label: Label = %EmptyLabel

var _rows: Array[Dictionary] = []
var _sort_mode: int = SORT_RANK
var _filter_mode: int = FILTER_ALL


func _ready() -> void:
	sort_option.add_item("Rang", SORT_RANK)
	sort_option.add_item("Âge", SORT_AGE)
	sort_option.add_item("Nom", SORT_NAME)
	sort_option.item_selected.connect(func(index: int) -> void:
		_sort_mode = sort_option.get_item_id(index)
		_render())
	filter_option.add_item("Tous", FILTER_ALL)
	filter_option.add_item("Généraux", FILTER_GENERAL)
	filter_option.add_item("Gouverneurs", FILTER_GOVERNOR)
	filter_option.add_item("À la cour", FILTER_COURT)
	filter_option.item_selected.connect(func(index: int) -> void:
		_filter_mode = filter_option.get_item_id(index)
		_render())
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())


## `rows` : `[{id, name, epithet, age, title, role, sex, faction}]` déjà résolus par
## `CampaignMap` via `get_character`. `faction_label`/`faction_color` : identité affichée.
## `preset_filter` : `FILTER_*` à présélectionner (-1 = garder le filtre courant), utilisé
## par le bouton « Cour » du panneau de province (filtre sur les gouverneurs).
func show_court(rows: Array[Dictionary], faction_label: String, faction_color: Color, preset_filter: int = -1) -> void:
	_rows = rows
	title_label.text = "Cour — %s" % faction_label
	title_label.add_theme_color_override("font_color", faction_color.darkened(0.3))
	if preset_filter != -1:
		_filter_mode = preset_filter
		filter_option.select(filter_option.get_item_index(preset_filter))
	_render()
	show()


func _render() -> void:
	for child in rows_list.get_children():
		child.queue_free()
	var filtered: Array[Dictionary] = []
	for row in _rows:
		var role: String = str(row.get("role", ""))
		match _filter_mode:
			FILTER_GENERAL:
				if not role.begins_with("général"):
					continue
			FILTER_GOVERNOR:
				if not role.begins_with("gouverneur"):
					continue
			FILTER_COURT:
				if role != "à la cour":
					continue
		filtered.append(row)
	if _sort_mode == SORT_RANK:
		pass  # ordre de `get_faction_characters` : dirigeant, héritier, puis par âge
	elif _sort_mode == SORT_AGE:
		filtered.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["age"]) > int(b["age"]))
	else:
		filtered.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return str(a["name"]) < str(b["name"]))
	empty_label.visible = filtered.is_empty()
	for row in filtered:
		rows_list.add_child(_make_row(row))


func _make_row(row: Dictionary) -> Control:
	var line := HBoxContainer.new()
	line.add_theme_constant_override("separation", 10)
	line.mouse_filter = Control.MOUSE_FILTER_PASS

	var portrait := PanelContainer.new()
	portrait.custom_minimum_size = Vector2(36, 36)
	var swatch := ColorRect.new()
	swatch.color = SimFacade.faction_color(str(row.get("faction", "")))
	swatch.custom_minimum_size = Vector2(36, 36)
	portrait.add_child(swatch)
	var initials := Label.new()
	initials.text = _initials(str(row.get("name", "?")))
	initials.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	initials.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	initials.add_theme_color_override("font_color", Color(1, 1, 1))
	initials.add_theme_font_size_override("font_size", 14)
	initials.set_anchors_preset(Control.PRESET_FULL_RECT)
	swatch.add_child(initials)
	# M10 assets : portrait peint (ou blason de faction) par-dessus le placeholder.
	PortraitLoader.overlay_portrait(swatch, str(row.get("id", "")), str(row.get("faction", "")), Vector2(48, 48))
	line.add_child(portrait)

	var name_box := VBoxContainer.new()
	name_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_box.add_theme_constant_override("separation", 0)
	var name_label := Label.new()
	var epithet: String = str(row.get("epithet", ""))
	name_label.text = "%s%s" % [str(row.get("name", "?")), " « %s »" % epithet if epithet != "" else ""]
	name_label.add_theme_font_size_override("font_size", 16)
	name_box.add_child(name_label)
	var sub_label := Label.new()
	sub_label.text = "%s ans — %s — %s" % [int(row.get("age", 0)), str(row.get("title", "")), str(row.get("role", ""))]
	sub_label.add_theme_font_size_override("font_size", 12)
	name_box.add_child(sub_label)
	line.add_child(name_box)

	# F2 : icône du rôle (général, gouverneur, cour) avec infobulle de la branche associée.
	var role: String = str(row.get("role", ""))
	var branch := "court"
	var role_icon := "hud_court"
	if role.begins_with("général"):
		branch = "command"
		role_icon = "hud_army"
	elif role.begins_with("gouverneur"):
		branch = "governance"
		role_icon = "hud_governor"
	var role_chip := IconChip.create(role_icon, "", RichTooltip.branch(branch), 24.0)
	line.add_child(role_chip)
	line.move_child(role_chip, 1)

	var open_button := Button.new()
	open_button.text = "Voir"
	var character_id: String = str(row.get("id", ""))
	open_button.pressed.connect(func() -> void: character_selected.emit(character_id))
	line.add_child(open_button)
	return line


static func _initials(name: String) -> String:
	var parts := name.split(" ", false)
	var text := ""
	for part in parts:
		if part.length() > 0 and text.length() < 2:
			text += part.substr(0, 1).to_upper()
	return text if text != "" else "?"
