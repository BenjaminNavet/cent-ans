class_name BattleHud
extends CanvasLayer

## HUD parchemin de la bataille (construit en code sur `parchment_theme.tres`) : barre du haut
## (nom, horloge, météo, rapport de forces), journal, en bas les cartes compactes du joueur
## (`UnitCard`) rangées par « bataille » (`BattleGroups`), boutons d'ordres, minicarte
## (`BattleMinimap`) et boutons de vitesse ; aide F1, écran de fin (F5b, audit UI § 3.2).
## Aucune règle : tout vient de `BattleSim.get_units()` et des événements.

signal card_clicked(unit_id: int, additive: bool)
signal command_pressed(command: String)
signal return_pressed
signal speed_pressed(index: int)  # -1 : pause, 0..2 : index dans BattleScene.SPEEDS
signal minimap_clicked(world: Vector2)

const UNIT_CARD := preload("res://scripts/battle/unit_card.gd")
const MINIMAP := preload("res://scripts/battle/battle_minimap.gd")
const SPEED_TOOLTIPS := ["Pause (Espace)", "Vitesse ×1 (+ / −)", "Vitesse ×2 (+ / −)", "Vitesse ×4 (+ / −)"]
const HELP_TEXT := """[b]Bataille — commandes[/b] (F1 : fermer)
• Espace : pause (ordres possibles en pause) · + / − : vitesse ×1, ×2, ×4 (boutons en bas à droite).
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter) · clic sur une carte : sélectionner.
• Clic droit : déplacer ou attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne.
• Ctrl+1..9 : enregistrer la sélection en groupe · 1..9 : rappeler le groupe (deux fois : centrer la caméra).
• F : formation · G : tir à volonté · H : halte · Z X V B N : ordres du chef · Échap : désélectionner.
• Caméra : W A S D, molette, Q / E, bouton du milieu ; clic sur la minicarte : y aller."""

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const INK := Color(0.22, 0.14, 0.07)
const MAX_LOG := 9
const CATEGORY_ICON := {"infantry": "⚔", "archer": "➶", "cavalry": "♞", "siege": "⚙", "tower": "♜", "ram": "⚒"}

var root: Control
var title_label: Label
var clock_label: Label
var weather_label: Label
var _active_speed: int = 0  # -1 : pause
var balance_bar: Control
var balance_label: Label
var log_box: VBoxContainer
var cards_box: HBoxContainer
var end_panel: PanelContainer
var end_title: Label
var end_body: Label
var help_label: Label
var siege_panel: PanelContainer
var siege_label: Label
var _cards: Dictionary = {}  # unit id -> {panel, name, count, morale, fatigue, ammo, state}
var _balance: Array = [1, 1]
var _colors: Array = [Color.RED, Color.BLUE]
var _log_lines: Array[String] = []


func _ready() -> void:
	root = Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.theme = load(THEME_PATH)
	add_child(root)
	_build_top_bar()
	_build_log()
	_build_bottom()
	_build_end_panel()


func _label(text: String, size: int = 16) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", INK)
	return label


func _build_top_bar() -> void:
	var panel := PanelContainer.new()
	panel.anchor_left = 0.5
	panel.anchor_right = 0.5
	panel.offset_left = -470
	panel.offset_right = 470
	panel.offset_top = 8
	root.add_child(panel)
	var box := HBoxContainer.new()
	box.add_theme_constant_override("separation", 18)
	panel.add_child(box)
	title_label = _label("Bataille", 20)
	box.add_child(title_label)
	clock_label = _label("00:00")
	box.add_child(clock_label)
	weather_label = _label("")
	box.add_child(weather_label)
	var balance := VBoxContainer.new()
	balance.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(balance)
	balance_label = _label("", 13)
	balance_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	balance.add_child(balance_label)
	balance_bar = Control.new()
	balance_bar.custom_minimum_size = Vector2(220, 12)
	balance_bar.draw.connect(_draw_balance)
	balance.add_child(balance_bar)
	# Siège (M8) : murailles, brèches, porte, place centrale ; caché en bataille rangée.
	siege_panel = PanelContainer.new()
	siege_panel.anchor_left = 0.5
	siege_panel.anchor_right = 0.5
	siege_panel.offset_left = -300
	siege_panel.offset_right = 300
	siege_panel.offset_top = 62
	siege_panel.visible = false
	root.add_child(siege_panel)
	siege_label = _label("", 14)
	siege_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	siege_panel.add_child(siege_label)


func _draw_balance() -> void:
	var size := balance_bar.size
	var total := maxf(float(_balance[0] + _balance[1]), 1.0)
	var split := size.x * float(_balance[0]) / total
	balance_bar.draw_rect(Rect2(0, 0, split, size.y), _colors[0])
	balance_bar.draw_rect(Rect2(split, 0, size.x - split, size.y), _colors[1])
	balance_bar.draw_rect(Rect2(Vector2.ZERO, size), INK, false, 1.0)


func _build_log() -> void:
	var panel := PanelContainer.new()
	panel.anchor_left = 1.0
	panel.anchor_right = 1.0
	panel.offset_left = -372
	panel.offset_right = -12
	panel.offset_top = 70
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.add_child(panel)
	log_box = VBoxContainer.new()
	log_box.add_theme_constant_override("separation", 2)
	panel.add_child(log_box)
	log_box.add_child(_label("Journal de bataille", 17))


func _build_bottom() -> void:
	var panel := PanelContainer.new()
	panel.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	panel.offset_left = 8
	panel.offset_right = -8
	panel.offset_top = -182
	panel.offset_bottom = -8
	root.add_child(panel)
	var outer := HBoxContainer.new()
	outer.add_theme_constant_override("separation", 10)
	panel.add_child(outer)
	var scroll := ScrollContainer.new()
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	outer.add_child(scroll)
	cards_box = HBoxContainer.new()
	cards_box.add_theme_constant_override("separation", 6)
	scroll.add_child(cards_box)
	var buttons := GridContainer.new()
	buttons.columns = 2
	outer.add_child(buttons)
	for entry in [
		["Formation (F)", "formation"], ["Tir à volonté (G)", "fire_at_will"],
		["Halte (H)", "halt"], ["Pause (Espace)", "pause"],
		["Retraite", "withdraw"], ["Retraite générale", "withdraw_all"],
	]:
		var button := Button.new()
		button.text = entry[0]
		button.focus_mode = Control.FOCUS_NONE
		button.pressed.connect(func() -> void: command_pressed.emit(entry[1]))
		buttons.add_child(button)
	help_label = _label("Clic gauche : sélection (Maj : ajouter, glisser : rectangle) · Clic droit : déplacer / attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne · 1/2/3 : vitesse · WASD, molette, Q/E : caméra", 12)
	help_label.anchor_top = 1.0
	help_label.anchor_bottom = 1.0
	help_label.offset_left = 14
	help_label.offset_top = -210
	help_label.add_theme_color_override("font_color", Color(0.97, 0.94, 0.85))
	help_label.add_theme_color_override("font_outline_color", Color(0.1, 0.07, 0.03))
	help_label.add_theme_constant_override("outline_size", 4)
	root.add_child(help_label)


func _build_end_panel() -> void:
	end_panel = PanelContainer.new()
	end_panel.anchor_left = 0.5
	end_panel.anchor_right = 0.5
	end_panel.anchor_top = 0.5
	end_panel.anchor_bottom = 0.5
	end_panel.offset_left = -290
	end_panel.offset_right = 290
	end_panel.offset_top = -190
	end_panel.visible = false
	root.add_child(end_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 12)
	end_panel.add_child(box)
	end_title = _label("Victoire", 30)
	end_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(end_title)
	end_body = _label("", 16)
	end_body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(end_body)
	var button := Button.new()
	button.text = "Retour à la campagne"
	button.pressed.connect(func() -> void: return_pressed.emit())
	box.add_child(button)


# --- Mises à jour ---------------------------------------------------------------------


func set_title(text: String, weather: String, colors: Array) -> void:
	title_label.text = text
	weather_label.text = weather
	_colors = colors


func set_clock(seconds: float, speed: float, paused: bool) -> void:
	var total := int(seconds)
	clock_label.text = "%02d:%02d" % [total / 60, total % 60]
	_active_speed = -1 if paused else maxi(BattleScene.SPEEDS.find(speed), 0)


func set_siege_status(text: String) -> void:
	siege_panel.visible = text != ""
	siege_label.text = text


func set_balance(player_name: String, player_strength: int, enemy_name: String, enemy_strength: int) -> void:
	_balance = [player_strength, enemy_strength]
	balance_label.text = "%s %d — %d %s" % [player_name, player_strength, enemy_strength, enemy_name]
	balance_bar.queue_redraw()


func add_events(events: Array) -> void:
	for event in events:
		var seconds := int(float(event.get("time", 0.0)))
		_log_lines.push_front("%02d:%02d  %s" % [seconds / 60, seconds % 60, str(event.get("text_fr", ""))])
	while _log_lines.size() > MAX_LOG:
		_log_lines.pop_back()
	for i in range(log_box.get_child_count() - 1, 0, -1):
		log_box.get_child(i).queue_free()
	for line in _log_lines:
		var label := _label(line, 13)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(330, 0)
		log_box.add_child(label)


func log_line_count() -> int:
	return _log_lines.size()


func _bar(color: Color) -> ProgressBar:
	var bar := ProgressBar.new()
	bar.custom_minimum_size = Vector2(120, 9)
	bar.show_percentage = false
	bar.max_value = 100
	var fill := StyleBoxFlat.new()
	fill.bg_color = color
	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0.55, 0.47, 0.33, 0.6)
	bar.add_theme_stylebox_override("fill", fill)
	bar.add_theme_stylebox_override("background", bg)
	return bar


## Cartes des unités du joueur (créées une fois, mises à jour ensuite).
func update_cards(units: Array, side: String, selected: Array) -> void:
	for unit in units:
		if str(unit["side"]) != side:
			continue
		var id := int(unit["id"])
		if not _cards.has(id):
			_cards[id] = _make_card(unit)
		var card: Dictionary = _cards[id]
		var present: bool = unit["present"]
		card["count"].text = "%d / %d" % [int(unit["soldiers"]), int(unit["initial_soldiers"])]
		card["morale"].value = float(unit["morale"])
		card["fatigue"].value = float(unit["fatigue"])
		if bool(unit["can_shoot"]):
			card["ammo"].text = "Munitions : %d / %d%s" % [int(unit["ammo"]), int(unit["max_ammo"]), "" if bool(unit["fire_at_will"]) else " (tir retenu)"]
		else:
			card["ammo"].text = "Formation : %s" % _formation_label(str(unit["formation"]))
		var state := str(unit["state_label"])
		if bool(unit["left_field"]):
			state = "hors du champ"
		elif not present:
			state = "anéantie"
		card["state"].text = state
		var style: StyleBoxFlat = card["style"]
		style.border_color = Color(0.95, 0.75, 0.15) if selected.has(id) else Color(0.42, 0.29, 0.16)
		style.bg_color = Color(0.93, 0.87, 0.72) if present else Color(0.7, 0.65, 0.55)
		if str(unit["state"]) == "routing":
			style.bg_color = Color(0.9, 0.7, 0.62)


func _formation_label(key: String) -> String:
	match key:
		"column":
			return "colonne"
		"square":
			return "schiltron"
		"wedge":
			return "coin"
	return "ligne"


func _make_card(unit: Dictionary) -> Dictionary:
	var panel := RichPanel.new()  # F2 : infobulle riche du type d'unité
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.93, 0.87, 0.72)
	style.border_color = Color(0.42, 0.29, 0.16)
	style.set_border_width_all(2)
	style.set_corner_radius_all(3)
	style.content_margin_left = 6
	style.content_margin_right = 6
	style.content_margin_top = 4
	style.content_margin_bottom = 4
	panel.add_theme_stylebox_override("panel", style)
	panel.custom_minimum_size = Vector2(142, 0)
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	var id := int(unit["id"])
	panel.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			card_clicked.emit(id, event.shift_pressed))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 1)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	panel.add_child(box)
	# F2 : icône du type d'unité (repli sur la catégorie de rendu), infobulle riche.
	var unit_type := str(unit.get("type", ""))
	panel.tooltip_text = RichTooltip.unit(unit_type, {"name": str(unit["name"])})
	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 3)
	var name_prefix := ""
	var library := get_node_or_null("/root/IconLibrary")
	if library != null:
		var fallback := "unit_category_" + str(unit.get("render", "infantry"))
		var icon_id: String = unit_type if library.call("has_icon", unit_type) else fallback
		head.add_child(library.call("make_rect", icon_id, 22.0, "unit"))
	else:
		name_prefix = str(CATEGORY_ICON.get(str(unit["render"]), "⚔")) + " "
	var name_label := _label("%s%s%s" % [name_prefix, str(unit["name"]), " ★" if bool(unit["is_general"]) else ""], 13)
	name_label.clip_text = true
	name_label.custom_minimum_size = Vector2(104, 0)
	head.add_child(name_label)
	box.add_child(head)
	var count := _label("", 13)
	box.add_child(count)
	box.add_child(_label("Moral", 11))
	var morale := _bar(Color(0.25, 0.45, 0.8))
	box.add_child(morale)
	box.add_child(_label("Fatigue", 11))
	var fatigue := _bar(Color(0.75, 0.45, 0.15))
	box.add_child(fatigue)
	var ammo := _label("", 11)
	box.add_child(ammo)
	var state := _label("", 12)
	state.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08))
	box.add_child(state)
	for child in box.get_children():
		if child is Control:
			(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	cards_box.add_child(panel)
	return {"panel": panel, "style": style, "count": count, "morale": morale, "fatigue": fatigue, "ammo": ammo, "state": state}


func show_end(title: String, body: String) -> void:
	end_title.text = title
	end_body.text = body
	end_panel.visible = true
