class_name VictoryController
extends Node

## Objectifs de campagne et fin de partie (M10) : panneau « Objectifs » (touche O, entrée du menu),
## écran de victoire / défaite / fin de campagne quand `get_outcome` change d'état. Séparé de
## `campaign_map.gd` ; celui-ci n'appelle que `setup`, `after_end_turn` et `handle_input`.

var map: Node = null  # CampaignMap
var panel: PanelContainer
var list: VBoxContainer
var score_label: Label
var end_dialog: PanelContainer
var end_title: Label
var end_text: Label
var _announced: String = "ongoing"


func setup(campaign_map: Node) -> void:
	map = campaign_map
	var theme: Theme = load("res://scenes/ui/parchment_theme.tres")
	panel = _centered_panel(theme, Vector2(620, 460))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	var header := HBoxContainer.new()
	var title := Label.new()
	title.text = "Objectifs de campagne"
	title.add_theme_font_size_override("font_size", 22)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	var close := Button.new()
	close.text = "×"
	close.pressed.connect(func() -> void: panel.hide())
	header.add_child(close)
	box.add_child(header)
	score_label = Label.new()
	box.add_child(score_label)
	box.add_child(HSeparator.new())
	list = VBoxContainer.new()
	list.add_theme_constant_override("separation", 10)
	box.add_child(list)
	map.ui.add_child(panel)
	panel.hide()

	end_dialog = _centered_panel(theme, Vector2(560, 260))
	var end_box := VBoxContainer.new()
	end_box.add_theme_constant_override("separation", 12)
	end_dialog.add_child(end_box)
	end_title = Label.new()
	end_title.add_theme_font_size_override("font_size", 30)
	end_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	end_box.add_child(end_title)
	end_text = Label.new()
	end_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	end_text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	end_text.size_flags_vertical = Control.SIZE_EXPAND_FILL
	end_box.add_child(end_text)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	buttons.add_theme_constant_override("separation", 16)
	var keep := Button.new()
	keep.text = "Continuer à jouer"
	keep.pressed.connect(func() -> void: end_dialog.hide())
	buttons.add_child(keep)
	var menu := Button.new()
	menu.text = "Menu principal"
	menu.pressed.connect(func() -> void: map.get_tree().change_scene_to_file(map.START_MENU_SCENE))
	buttons.add_child(menu)
	end_box.add_child(buttons)
	map.ui.add_child(end_dialog)
	end_dialog.hide()
	var popup: PopupMenu = map.ui.menu_button.get_popup()
	popup.add_item("Objectifs (O)", 900)
	popup.id_pressed.connect(func(id: int) -> void:
		if id == 900:
			toggle_panel())
	if available():
		_announced = str((map.sim.call("get_outcome") as Dictionary).get("state", "ongoing"))


static func _centered_panel(theme: Theme, size: Vector2) -> PanelContainer:
	var p := PanelContainer.new()
	p.theme = theme
	p.set_anchors_preset(Control.PRESET_CENTER)
	p.custom_minimum_size = size
	p.offset_left = -size.x / 2
	p.offset_right = size.x / 2
	p.offset_top = -size.y / 2
	p.offset_bottom = size.y / 2
	return p


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_objectives")


func toggle_panel() -> void:
	if panel.visible:
		panel.hide()
	else:
		open_panel()


func open_panel() -> void:
	if not available():
		map.ui.show_toast("Objectifs indisponibles avec cette simulation.", true)
		return
	for child in list.get_children():
		child.queue_free()
	var objectives: Array = map.sim.call("get_objectives", map.player_faction)
	var info: Dictionary = SimFacadeRef.info(map, map.player_faction)
	var outcome: Dictionary = map.sim.call("get_outcome")
	score_label.text = "%s — échéance %d — score actuel : %d" % [
		str(info.get("victory_summary", "")), int(info.get("victory_end_year", 0)), int(outcome.get("score", 0))]
	var hold := int(outcome.get("hold_turns", 1))
	if hold > 1:
		var streak := int(outcome.get("victory_streak", 0))
		if streak > 0:
			score_label.text += "\nTous les objectifs sont remplis : encore %d saison(s) à les tenir pour l'emporter." % (hold - streak)
		else:
			score_label.text += "\nLa victoire exige de tenir tous les objectifs %d saisons d'affilée." % hold
	score_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	if objectives.is_empty():
		var none := Label.new()
		none.text = "Cette faction n'a pas d'objectifs historiques : survivre et prospérer."
		list.add_child(none)
	for objective in objectives:
		var row := VBoxContainer.new()
		var head := Label.new()
		var done := bool(objective["done"])
		head.text = "%s %s — %s" % ["✔" if done else "☐", objective["title"], objective["progress"]]
		head.add_theme_font_size_override("font_size", 17)
		head.add_theme_color_override("font_color", Color(0.15, 0.45, 0.15) if done else Color(0.35, 0.22, 0.10))
		row.add_child(head)
		var text := Label.new()
		text.text = str(objective["description"])
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.add_theme_font_size_override("font_size", 13)
		row.add_child(text)
		list.add_child(row)
	panel.show()


func after_end_turn() -> void:
	if not available():
		return
	var outcome: Dictionary = map.sim.call("get_outcome")
	var state := str(outcome.get("state", "ongoing"))
	if state == "ongoing" or state == _announced:
		return
	_announced = state
	end_title.text = {"victory": "Victoire !", "defeat": "Défaite", "ended": "Fin de la campagne"}.get(state, "Fin")
	end_text.text = "%s\nScore final : %d." % [outcome.get("text", ""), int(outcome.get("score", 0))]
	end_dialog.show()


func handle_input(event: InputEvent) -> bool:
	if event is InputEventKey and event.pressed and not event.echo and (event as InputEventKey).physical_keycode == KEY_O:
		toggle_panel()
		return true
	return false
