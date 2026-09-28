class_name VictoryController
extends Node

## Objectifs de campagne et fin de partie (M10) : panneau « Objectifs » (touche O, entrée du menu),
## écran de victoire / défaite / fin de campagne quand `get_outcome` change d'état. Séparé de
## `campaign_map.gd` ; celui-ci n'appelle que `setup`, `after_end_turn` et `handle_input`.

var map: Node = null  # CampaignMap
var panel: PanelContainer
var list: VBoxContainer
var score_label: Label
var end_dialog: Control
var end_art: TextureRect
var end_title: Label
var end_caption: Label
var end_text: Label
var _announced: String = "ongoing"
## Q1 : largeur des libellés à retour à la ligne (panneau de 620 px, marges comprises).
const WRAP_WIDTH := 580.0


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
	title.add_theme_font_size_override("font_size", UiType.size(UiType.HEADING))
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

	# AR1 : écran de fin illustré — enluminure plein écran, cartouche de parchemin en bas.
	end_dialog = Control.new()
	end_dialog.name = "CampaignEnding"
	end_dialog.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	end_dialog.mouse_filter = Control.MOUSE_FILTER_STOP
	var night := ColorRect.new()
	night.color = Color(0.06, 0.04, 0.02)
	night.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	end_dialog.add_child(night)
	end_art = TextureRect.new()
	end_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	end_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	end_art.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	end_art.modulate = Color(0.9, 0.87, 0.82)
	end_dialog.add_child(end_art)
	var shade := TextureRect.new()
	var gradient := Gradient.new()
	gradient.set_color(0, Color(0, 0, 0, 0))
	gradient.set_color(1, Color(0.05, 0.03, 0.01, 0.85))
	var gradient_texture := GradientTexture2D.new()
	gradient_texture.gradient = gradient
	gradient_texture.fill_from = Vector2(0, 0.35)
	gradient_texture.fill_to = Vector2(0, 1)
	shade.texture = gradient_texture
	shade.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	shade.stretch_mode = TextureRect.STRETCH_SCALE
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	end_dialog.add_child(shade)
	var card := PanelContainer.new()
	card.theme = theme
	card.add_theme_stylebox_override("panel", HudStyle.illuminated_box())  # page enluminée UI1
	card.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	card.custom_minimum_size = Vector2(720, 0)
	card.grow_horizontal = Control.GROW_DIRECTION_BOTH
	card.grow_vertical = Control.GROW_DIRECTION_BEGIN
	card.offset_left = -360
	card.offset_right = 360
	card.offset_bottom = -48
	end_dialog.add_child(card)
	var end_box := VBoxContainer.new()
	end_box.add_theme_constant_override("separation", 10)
	card.add_child(end_box)
	end_title = Label.new()
	# P2f : bannière de fin de partie, hors des 4 paliers UiType (dramatique voulu, cf. docs/wip/p2f-fonts.md).
	end_title.add_theme_font_size_override("font_size", 40)
	end_title.add_theme_color_override("font_color", Color(0.45, 0.10, 0.06))
	end_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	end_box.add_child(end_title)
	end_caption = Label.new()
	end_caption.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	end_caption.add_theme_color_override("font_color", Color(0.40, 0.30, 0.18))
	end_caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	end_box.add_child(end_caption)
	end_text = Label.new()
	end_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	end_text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	end_text.custom_minimum_size.x = 680
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
	menu.pressed.connect(func() -> void: SceneFader.go(map.START_MENU_SCENE))
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
			var left := hold - streak
			score_label.text += "\nTous les objectifs sont remplis : encore %d %s à les tenir pour l'emporter." % [left, "saison" if left <= 1 else "saisons"]
		else:
			score_label.text += "\nLa victoire exige de tenir tous les objectifs %d saisons d'affilée." % hold
	score_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	score_label.custom_minimum_size.x = WRAP_WIDTH
	# Audit A3 D2 : le score n'était pas expliqué.
	score_label.mouse_filter = Control.MOUSE_FILTER_PASS
	RichTooltip.attach_plain(score_label, "campaign_score")
	if objectives.is_empty():
		var none := Label.new()
		none.text = "Cette faction n'a pas d'objectifs historiques : survivre et prospérer."
		list.add_child(none)
	for objective in objectives:
		var row := VBoxContainer.new()
		var head := Label.new()
		var done := bool(objective["done"])
		head.text = "%s %s — %s" % ["✔" if done else "☐", objective["title"], objective["progress"]]
		head.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
		head.add_theme_color_override("font_color", Color(0.15, 0.45, 0.15) if done else Color(0.35, 0.22, 0.10))
		row.add_child(head)
		var text := Label.new()
		text.text = str(objective["description"])
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.custom_minimum_size.x = WRAP_WIDTH
		text.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
		row.add_child(text)
		list.add_child(row)
	panel.show()
	_fit_centered.call_deferred(panel)


## Q1 : les libellés à retour à la ligne gonflaient la hauteur minimale au premier calcul (panneau
## étiré hors de l'écran, vide en bas) ; une fois la largeur connue, on le recentre à sa taille.
static func _fit_centered(p: Control) -> void:
	if not is_instance_valid(p):
		return
	p.reset_size()
	p.set_anchors_and_offsets_preset(Control.PRESET_CENTER, Control.PRESET_MODE_MINSIZE)


func after_end_turn() -> void:
	if not available():
		return
	var outcome: Dictionary = map.sim.call("get_outcome")
	var state := str(outcome.get("state", "ongoing"))
	if state == "ongoing" or state == _announced:
		return
	_announced = state
	show_ending(state, str(outcome.get("text", "")), int(outcome.get("score", 0)))


## Écran de fin illustré (AR1) pour `state` (`victory`, `defeat`, `ended`).
func show_ending(state: String, text: String, score: int) -> void:
	var plate := ArtPlates.ending("campaign_defeat" if state == "defeat" else "campaign_victory")
	end_art.texture = ArtPlates.texture(plate)
	end_title.text = {"victory": str(plate.get("title", "Victoire !")), "defeat": str(plate.get("title", "Défaite")), "ended": "Fin de la campagne"}.get(state, "Fin")
	end_caption.text = str(plate.get("caption", ""))
	end_text.text = "%s\nScore final : %d." % [text, score]
	end_dialog.show()


func handle_input(event: InputEvent) -> bool:
	if event.is_action_pressed("map_toggle_objectives") and not event.is_echo():  # U7
		toggle_panel()
		return true
	return false
