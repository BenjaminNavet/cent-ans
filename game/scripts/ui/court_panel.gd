class_name CourtPanel
extends PanelContainer

## Panneau « Cour » (bouton de la barre, touche C) : liste des personnages vivants de la
## faction du joueur (`CampaignSim.get_faction_characters` + `get_character`), triable par
## âge/nom, filtrable par rôle. Portrait placeholder = carré de couleur de faction + initiales.
## Aucune règle ici : la simulation fournit rôle, titre, âge.
## C3 : onglet « Arbre familial » (`FamilyTreeView`, `CampaignSim.get_family_tree`) ; le panneau
## s'élargit tant que l'arbre est affiché.

signal character_selected(character_id: String)
signal closed

const SORT_RANK := 2
const SORT_AGE := 0
const SORT_NAME := 1
const FILTER_ALL := 0
const FILTER_GENERAL := 1
const FILTER_GOVERNOR := 2
const FILTER_COURT := 3
const TAB_LIST := 0
const TAB_TREE := 1
const TREE_UP := 2  # générations d'ascendants
const TREE_DOWN := 3  # générations de descendants
const LIST_WIDTH := 540.0
const TREE_WIDTH := 1180.0

@onready var title_label: Label = %TitleLabel
@onready var sort_option: OptionButton = %SortOption
@onready var filter_option: OptionButton = %FilterOption
@onready var rows_list: VBoxContainer = %RowsList
@onready var close_button: Button = %CloseButton
@onready var empty_label: Label = %EmptyLabel

var _rows: Array[Dictionary] = []
var _sort_mode: int = SORT_RANK
var _filter_mode: int = FILTER_ALL

## C3 : onglets, vue de l'arbre, personnage au centre de l'arbre.
var tab_bar: TabBar
var family_tree: FamilyTreeView
var tree_box: VBoxContainer
var tree_root_id: String = ""
## Simulation interrogée pour l'arbre (défaut : `SimFacade.sim`) ; injectable pour les tests.
var sim_source: Object = null
var _tree_hint: Label
var _current_tab: int = TAB_LIST
## C7 : bord droit à ne pas dépasser (fiche de personnage ouverte à droite) ; INF = libre.
var max_right: float = INF


func _ready() -> void:
	Lettrine.attach(title_label)  # UI1 : titre à lettrine enluminée
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
	_build_tree_tab()


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
		show_tab(TAB_LIST)
	if tree_root_id == "" or not _rows.any(func(row: Dictionary) -> bool: return str(row.get("id", "")) == tree_root_id):
		tree_root_id = str(rows[0].get("id", "")) if not rows.is_empty() else ""
	_render()
	if _current_tab == TAB_TREE:
		refresh_tree()
	show()


# --- C3 : arbre familial ----------------------------------------------------------------------


func _build_tree_tab() -> void:
	var vbox := get_node("VBox") as VBoxContainer
	tab_bar = TabBar.new()
	tab_bar.name = "Tabs"
	tab_bar.add_tab("Liste")
	tab_bar.add_tab("Arbre familial")
	tab_bar.set_tab_icon(TAB_LIST, HudStyle.icon("hud_court"))
	tab_bar.set_tab_icon(TAB_TREE, HudStyle.icon("class_nobility"))
	tab_bar.set_tab_icon_max_width(TAB_LIST, 18)
	tab_bar.set_tab_icon_max_width(TAB_TREE, 18)
	# Onglets au registre parchemin (le thème n'en définit pas) : actif clair filet rubrique.
	for pair in [["tab_selected", HudStyle.PARCHMENT_LIGHT, HudStyle.RUBRIC], ["tab_unselected", HudStyle.PARCHMENT_DARK, HudStyle.INK_SOFT],
			["tab_hovered", HudStyle.PARCHMENT, HudStyle.INK]]:
		var style := HudStyle.card_box(pair[1], pair[2], 1)
		style.border_width_bottom = 3 if pair[0] == "tab_selected" else 1
		style.set_content_margin_all(6)
		style.content_margin_left = 12
		style.content_margin_right = 12
		tab_bar.add_theme_stylebox_override(pair[0], style)
	for color_name in ["font_selected_color", "font_hovered_color", "font_unselected_color"]:
		tab_bar.add_theme_color_override(color_name, HudStyle.INK if color_name != "font_unselected_color" else HudStyle.INK_SOFT)
	tab_bar.tab_changed.connect(func(tab: int) -> void: show_tab(tab))
	vbox.add_child(tab_bar)
	vbox.move_child(tab_bar, 1)

	tree_box = VBoxContainer.new()
	tree_box.name = "TreeBox"
	tree_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	tree_box.visible = false
	var toolbar := HBoxContainer.new()
	toolbar.add_theme_constant_override("separation", 6)
	_tree_hint = Label.new()
	_tree_hint.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_tree_hint.add_theme_font_size_override("font_size", 12)
	_tree_hint.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	toolbar.add_child(_tree_hint)
	for spec in [["Recentrer sur le souverain", "⌂", func() -> void: recenter_tree("")],
			["Dézoomer", "−", func() -> void: family_tree.set_zoom(family_tree.zoom / 1.2)],
			["Zoomer", "+", func() -> void: family_tree.set_zoom(family_tree.zoom * 1.2)]]:
		var button := Button.new()
		button.text = spec[1]
		button.tooltip_text = spec[0]
		button.custom_minimum_size = Vector2(30, 0)
		button.pressed.connect(spec[2])
		toolbar.add_child(button)
	tree_box.add_child(toolbar)
	family_tree = FamilyTreeView.new()
	family_tree.name = "FamilyTree"
	family_tree.custom_minimum_size = Vector2(0, 480)
	family_tree.character_selected.connect(func(id: String) -> void: character_selected.emit(id))
	family_tree.recenter_requested.connect(func(id: String) -> void: recenter_tree(id))
	tree_box.add_child(family_tree)
	vbox.add_child(tree_box)


func show_tab(tab: int) -> void:
	_current_tab = tab
	if tab_bar.current_tab != tab:
		tab_bar.set_current_tab(tab)
	var tree := tab == TAB_TREE
	for path in ["VBox/Controls", "VBox/Separator2", "VBox/Scroll"]:
		(get_node(path) as Control).visible = not tree
	empty_label.visible = not tree and rows_list.get_child_count() == 0 and not _rows.is_empty()
	tree_box.visible = tree
	offset_right = offset_left + (_tree_width() if tree else LIST_WIDTH)
	if tree:
		refresh_tree()


func _tree_width() -> float:
	var viewport_width := get_viewport_rect().size.x if is_inside_tree() else TREE_WIDTH
	var room := minf(viewport_width - offset_left - 16.0, max_right - offset_left)
	return clampf(room, LIST_WIDTH, TREE_WIDTH)


## C7 : limite le bord droit du panneau (l'arbre se rétrécit quand la fiche est ouverte).
func set_max_right(value: float) -> void:
	if is_equal_approx(value, max_right) or (is_inf(value) and is_inf(max_right)):
		return
	max_right = value
	if _current_tab == TAB_TREE:
		offset_right = offset_left + _tree_width()


## Recentre l'arbre sur `character_id` (vide = premier personnage de la cour, le souverain).
func recenter_tree(character_id: String) -> void:
	tree_root_id = character_id if character_id != "" else (str(_rows[0].get("id", "")) if not _rows.is_empty() else "")
	refresh_tree()


func refresh_tree() -> void:
	var sim: Object = sim_source
	if sim == null:
		var facade := get_node_or_null("/root/SimFacade")
		sim = facade.get("sim") if facade != null else null
	if sim == null or not sim.has_method("get_family_tree") or tree_root_id == "":
		family_tree.show_tree({})
		_tree_hint.text = "Arbre indisponible (simulation sans liens de parenté)."
		return
	var tree: Dictionary = sim.call("get_family_tree", tree_root_id, TREE_UP, TREE_DOWN)
	family_tree.show_tree(tree)
	_tree_hint.text = "%d personnages sur %d générations — clic : fiche ; clic droit : recentrer ; Ctrl + molette : zoom" % [
		family_tree.node_count(), family_tree.generation_count()]


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
	empty_label.visible = filtered.is_empty() and _current_tab == TAB_LIST
	for row in filtered:
		rows_list.add_child(_make_row(row))


## Lot U10 (audit A3, P1) : la ligne entière est cliquable (survol surligné, curseur main) ;
## plus de bouton « Voir » répété.
func _make_row(row: Dictionary) -> Control:
	var character_id: String = str(row.get("id", ""))
	var frame := CourtRow.new()
	frame.name = "CourtRow_%s" % character_id.validate_node_name()
	frame.tooltip_text = "Ouvrir la fiche de %s" % str(row.get("name", "?"))
	frame.activated.connect(func() -> void: character_selected.emit(character_id))
	var line := HBoxContainer.new()
	line.add_theme_constant_override("separation", 10)
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	frame.add_child(line)

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
	# C7 : une ligne trop longue n'élargit plus le panneau (la fiche se range à sa droite).
	sub_label.clip_text = true
	sub_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	sub_label.tooltip_text = sub_label.text
	sub_label.mouse_filter = Control.MOUSE_FILTER_PASS
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

	var chevron := Label.new()
	chevron.text = "›"
	chevron.add_theme_font_size_override("font_size", 22)
	chevron.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	line.add_child(chevron)
	for child in line.find_children("*", "Control", true, false):
		if (child as Control).mouse_filter == Control.MOUSE_FILTER_STOP:
			(child as Control).mouse_filter = Control.MOUSE_FILTER_PASS
	return frame


## Ligne de la Cour cliquable (lot U10).
class CourtRow:
	extends PanelContainer

	signal activated

	var _normal: StyleBoxFlat
	var _hover: StyleBoxFlat

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		focus_mode = Control.FOCUS_ALL
		_normal = StyleBoxFlat.new()
		_normal.bg_color = Color(0, 0, 0, 0)
		_normal.set_content_margin_all(3)
		_hover = HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD)
		_hover.set_content_margin_all(3)
		add_theme_stylebox_override("panel", _normal)
		mouse_entered.connect(func() -> void: add_theme_stylebox_override("panel", _hover))
		mouse_exited.connect(func() -> void: add_theme_stylebox_override("panel", _normal))
		focus_entered.connect(func() -> void: add_theme_stylebox_override("panel", _hover))
		focus_exited.connect(func() -> void: add_theme_stylebox_override("panel", _normal))

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click != null and click.pressed and click.button_index == MOUSE_BUTTON_LEFT:
			activated.emit()
			accept_event()
		elif event.is_action_pressed("ui_accept"):
			activated.emit()
			accept_event()


static func _initials(name: String) -> String:
	var parts := name.split(" ", false)
	var text := ""
	for part in parts:
		if part.length() > 0 and text.length() < 2:
			text += part.substr(0, 1).to_upper()
	return text if text != "" else "?"
