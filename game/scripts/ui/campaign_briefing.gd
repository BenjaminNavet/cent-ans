class_name CampaignBriefing
extends Control

## TW trans (top9) : fenêtre « Briefing de campagne » du premier tour (avant le tutoriel) :
## objectifs de la faction, voisins hostiles (guerres déclarées), trois premiers conseils.
## Une seule fois par nouvelle partie ; désactivable (réglage `interface/campaign_briefing`, case
## « Ne plus afficher »). Textes dans `data/ui/transitions.json` ; objectifs et guerres lus
## dans la simulation (`get_objectives`, `get_faction_summary`). Aucune règle de jeu.

signal closed

static var _texts := JsonLookup.new("ui/transitions.json")

var panel: PanelContainer
var start_button: Button
var never_again: CheckBox


static func texts() -> Dictionary:
	return _texts.data().get("briefing", {})


## Vrai au tour 1 d'une nouvelle partie (tour 0, pas une sauvegarde chargée), hors capture, si le
## réglage le permet.
static func should_show(turn: int, enabled: bool, capture: bool, from_save: bool = false) -> bool:
	return turn <= 0 and enabled and not capture and not from_save


## Contenu : `{title, intro, objectives, neighbours, tips}` (listes de lignes).
static func build_model(faction_name: String, date_label: String, objectives: Array, hostile_names: Array) -> Dictionary:
	var t := texts()
	var objective_lines: Array = []
	for objective in objectives:
		objective_lines.append("• %s" % str((objective as Dictionary).get("title", "")))
	if objective_lines.is_empty():
		objective_lines.append(str(t.get("no_objectives", "")))
	var neighbour_lines: Array = []
	for hostile in hostile_names:
		neighbour_lines.append("• %s" % str(hostile))
	if neighbour_lines.is_empty():
		neighbour_lines.append(str(t.get("no_neighbours", "")))
	return {
		"title": str(t.get("title", "")),
		"intro": str(t.get("intro", "")).format({"faction": faction_name, "date": date_label}),
		"objectives": objective_lines,
		"neighbours": neighbour_lines,
		"tips": (t.get("tips", []) as Array).slice(0, 3),
	}


## Construit la fenêtre d'après `model` (voir `build_model`).
func show_briefing(model: Dictionary) -> void:
	name = "CampaignBriefing"
	theme = load("res://scenes/ui/parchment_theme.tres")  # boutons et case parchemin
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var veil := ColorRect.new()
	veil.color = Color(0.08, 0.05, 0.02, 0.55)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(veil)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(center)
	panel = PanelContainer.new()
	panel.name = "Panel"
	panel.add_theme_stylebox_override("panel", BattleUiKit.illuminated_box(30))
	panel.custom_minimum_size = Vector2(620, 0)
	center.add_child(panel)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 14)
	panel.add_child(margin)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	margin.add_child(box)
	box.add_child(_line(str(model["title"]), 30, BattleUiKit.INK, true))
	box.add_child(_line(str(model["intro"]), 17, Color(0.42, 0.33, 0.22), false))
	box.add_child(BattleUiKit.rule())
	for section in [["objectives_title", "objectives"], ["neighbours_title", "neighbours"], ["tips_title", "tips"]]:
		box.add_child(_line(str(texts().get(section[0], "")), 19, BattleUiKit.INK, true))
		for line in model[section[1]]:
			var label := _line(str(line) if section[1] != "tips" else "• " + str(line), 15, BattleUiKit.INK, false)
			label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			label.custom_minimum_size = Vector2(560, 0)
			box.add_child(label)
	box.add_child(BattleUiKit.rule())
	never_again = CheckBox.new()
	never_again.name = "NeverAgain"
	never_again.text = str(texts().get("never_again", ""))
	box.add_child(never_again)
	start_button = Button.new()
	start_button.name = "Start"
	start_button.text = str(texts().get("start", ""))
	start_button.custom_minimum_size = Vector2(260, 44)
	start_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	BattleUiKit.button_font(start_button, 19)
	start_button.pressed.connect(_on_start)
	box.add_child(start_button)
	visible = true


func _line(text: String, size: int, color: Color, bold: bool) -> Label:
	var label := BattleUiKit.label(text, size, color, false, bold)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return label


func is_open() -> bool:
	return visible and is_inside_tree()


func _on_start() -> void:
	var settings := get_node_or_null("/root/Settings")
	if never_again != null and never_again.button_pressed and settings != null:
		settings.call("set_value", "interface/campaign_briefing", false)
	visible = false
	closed.emit()
	queue_free()
