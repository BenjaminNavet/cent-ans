class_name MapLegend
extends PanelContainer

## Lot UX1 (audit A3, C14 / U14) : légende de la carte de campagne, panneau parchemin ouvert
## par le bouton « Légende » de la minicarte. Textes dans `data/ui/map_legend.json` (schéma
## `data/schemas/map_legend.schema.json`) ; chaque symbole a son échantillon dessiné
## (`LegendSample`). Les sections marquées `modes` ne s'affichent que dans ce mode de carte
## (politique, mécontentement, diplomatie, religion). Rendu seulement.

signal closed

const DATA_PATH := "ui/map_legend.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const WIDTH := 380.0
const MODES := ["political", "unrest", "diplomacy", "religion"]

static var _data: Dictionary = {}

var mode: String = "political"
## {player_color, player_faction, factions: [[id, Color, nom]]} (voir `LegendSample`).
var context: Dictionary = {}

var _mode_label: Label
var _sections_box: VBoxContainer
var _scroll: ScrollContainer


## Contenu de `data/ui/map_legend.json` (mis en cache), {} si absent.
static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		# Jeux de données réduits (fixtures des tests) : légende du jeu complet.
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("MapLegend: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Sections affichées dans le mode de carte `map_mode` (sections sans `modes` : toujours).
static func sections_for(map_mode: String) -> Array:
	var result: Array = []
	for section in data().get("sections", []):
		var modes: Array = (section as Dictionary).get("modes", [])
		if modes.is_empty() or modes.has(map_mode):
			result.append(section)
	return result


func _init() -> void:
	name = "MapLegend"
	theme = load("res://scenes/ui/parchment_theme.tres")
	add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(WIDTH, 0)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	var title := HudStyle.label(str(data().get("title", "Légende de la carte")), HudStyle.FONT_TITLE, HudStyle.RUBRIC)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	var close := Button.new()
	close.name = "Close"
	close.text = "×"
	close.tooltip_text = "Fermer la légende"
	close.focus_mode = Control.FOCUS_NONE
	close.pressed.connect(close_legend)
	header.add_child(close)
	_mode_label = HudStyle.label("", HudStyle.FONT_BODY, HudStyle.INK_SOFT)
	box.add_child(_mode_label)
	var intro := str(data().get("intro", ""))
	if intro != "":
		var intro_label := HudStyle.label(intro, HudStyle.FONT_SMALL, HudStyle.INK_FADED)
		intro_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		intro_label.custom_minimum_size = Vector2(WIDTH - 24.0, 0)
		box.add_child(intro_label)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(_scroll)
	_sections_box = VBoxContainer.new()
	_sections_box.add_theme_constant_override("separation", 3)
	_sections_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_sections_box)


## Construit la légende pour `map_mode` avec les couleurs de `legend_context`.
func build(map_mode: String, legend_context: Dictionary) -> void:
	context = legend_context
	mode = map_mode if MODES.has(map_mode) else "political"
	_mode_label.text = str((data().get("mode_titles", {}) as Dictionary).get(mode, ""))
	for child in _sections_box.get_children():
		_sections_box.remove_child(child)
		child.queue_free()
	for section in sections_for(mode):
		var heading := HudStyle.label(str(section.get("title", "")), HudStyle.FONT_BODY, HudStyle.RUBRIC)
		heading.name = "Section_" + str(section.get("id", ""))
		_sections_box.add_child(heading)
		for entry in section.get("entries", []):
			_sections_box.add_child(_entry_row(entry))
		_sections_box.add_child(HSeparator.new())


## Change de mode de carte (reconstruit seulement si le mode change).
func set_mode(map_mode: String) -> void:
	if map_mode != mode:
		build(map_mode, context)


## Nombre d'entrées affichées (tests).
func entry_count() -> int:
	var count := 0
	for child in _sections_box.get_children():
		if child.name.begins_with("Entry"):
			count += 1
	return count


## Libellés des entrées affichées (tests).
func entry_labels() -> PackedStringArray:
	var labels := PackedStringArray()
	for child in _sections_box.get_children():
		if child.name.begins_with("Entry"):
			labels.append(str(child.get_meta("label", "")))
	return labels


## Hauteur maximale du panneau (au-delà, les sections défilent).
func set_max_height(height: float) -> void:
	var content := _sections_box.get_combined_minimum_size().y
	var target := clampf(content, 0.0, maxf(height - 110.0, 120.0))
	if not is_equal_approx(_scroll.custom_minimum_size.y, target):
		_scroll.custom_minimum_size = Vector2(0, target)
		reset_size()


func close_legend() -> void:
	hide()
	closed.emit()


func _entry_row(entry: Dictionary) -> Control:
	var row := HBoxContainer.new()
	row.name = "Entry_%d" % _sections_box.get_child_count()
	row.set_meta("label", str(entry.get("label", "")))
	row.add_theme_constant_override("separation", 8)
	row.add_child(LegendSample.build(entry.get("sample", {}), context))
	var text_box := VBoxContainer.new()
	text_box.add_theme_constant_override("separation", 0)
	text_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(text_box)
	text_box.add_child(HudStyle.label(str(entry.get("label", "")), HudStyle.FONT_BODY, HudStyle.INK))
	var text := HudStyle.label(str(entry.get("text", "")), HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size = Vector2(WIDTH - LegendSample.SIZE.x - 44.0, 0)
	text_box.add_child(text)
	return row
