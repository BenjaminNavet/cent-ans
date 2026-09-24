class_name BattleHud
extends CanvasLayer

## HUD parchemin de la bataille (construit en code sur `parchment_theme.tres`) : barre du haut
## (nom, horloge, météo, rapport de forces), journal, en bas les cartes compactes du joueur
## (`UnitCard`) rangées par « bataille » (`BattleGroups`), boutons d'ordres, minicarte
## (`BattleMinimap`) et boutons de vitesse ; aide F1 (F5b, audit UI § 3.2). L'écran de fin est
## `BattleResultScreen` (B2), posé par la scène sur `root`.
## Aucune règle : tout vient de `BattleSim.get_units()` et des événements.

signal card_clicked(unit_id: int, additive: bool)
signal card_double_clicked(unit_id: int)  # B3 / T6 : centrer la caméra sur ce régiment
signal command_pressed(command: String)
signal speed_pressed(index: int)  # -1 : pause, 0..2 : index dans BattleScene.SPEEDS
signal minimap_clicked(world: Vector2)

const UNIT_CARD := preload("res://scripts/battle/unit_card.gd")
const MINIMAP := preload("res://scripts/battle/battle_minimap.gd")
const SPEED_TOOLTIPS := ["Pause (Espace)", "Vitesse ×1 (+ / −)", "Vitesse ×2 (+ / −)", "Vitesse ×4 (+ / −)"]
const HELP_TEXT := """[b]Bataille — commandes[/b] (F1 : fermer)
• Espace : pause (ordres possibles en pause) · + / − : vitesse ×1, ×2, ×4 (boutons en bas à droite).
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter) · clic sur une carte : sélectionner · double clic sur une carte : centrer la caméra dessus.
• Clic droit : déplacer ou attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne.
• Ctrl+1..9 : enregistrer la sélection en groupe · 1..9 : rappeler le groupe (deux fois : centrer la caméra).
• F : formation · G : tir à volonté · H : halte · Z X V B N : ordres du chef · Échap : désélectionner.
• Bannières au-dessus des troupes : clic = sélection, clic droit sur l'ennemi = attaque · U : masquer / afficher.
• Caméra : W A S D, molette, Q / E, bouton du milieu ; clic sur la minicarte : y aller.
• C : verrouille la caméra sur la sélection (ou le général) ; suivi doux, bouton du milieu = orbite ; W A S D ou glisser libèrent la caméra."""

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const INK := Color(0.22, 0.14, 0.07)
const MAX_LOG := 9
const CATEGORY_ICON := {"infantry": "⚔", "archer": "➶", "cavalry": "♞", "siege": "⚙", "tower": "♜", "ram": "⚒"}

var root: Control
var title_label: Label
var clock_label: Label
var weather_label: Label
var site_label: Label  # B6 : « Sol sec · été · village · haies »
var _active_speed: int = 0  # -1 : pause
var balance_bar: Control
var balance_label: Label
var log_box: VBoxContainer
var cards_box: HBoxContainer
var help_panel: PanelContainer  # aide F1 (remplace la ligne d'aide permanente)
var siege_panel: PanelContainer
var siege_label: Label
var minimap: BattleMinimap
var groups := BattleGroups.new()
var _speed_buttons: Array[Button] = []
var _battle_columns: Dictionary = {}  # "vanguard"/"main"/"rear" -> VBoxContainer
var _cards: Dictionary = {}  # unit id -> UnitCard
var _balance: Array = [1, 1]
var _colors: Array = [Color.RED, Color.BLUE]
var player_faction: String = ""  # B2 : blason des vignettes
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
	# Météo et, dessous, le site en une ligne compacte (B6).
	var sky := VBoxContainer.new()
	sky.add_theme_constant_override("separation", 0)
	box.add_child(sky)
	weather_label = _label("")
	sky.add_child(weather_label)
	site_label = _label("", 12)
	site_label.visible = false
	sky.add_child(site_label)
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
	cards_box.add_theme_constant_override("separation", 14)
	scroll.add_child(cards_box)
	for key in BattleGroups.ORDER:
		var column := VBoxContainer.new()
		column.name = "Battle_%s" % key
		column.set_meta("battle", key)
		column.visible = false  # montrée dès sa première carte
		column.add_theme_constant_override("separation", 2)
		column.add_child(_label(str(BattleGroups.LABELS[key]), 12))
		var row := HBoxContainer.new()
		row.name = "Cards"
		row.add_theme_constant_override("separation", 3)
		column.add_child(row)
		cards_box.add_child(column)
		_battle_columns[key] = column
	var buttons := GridContainer.new()
	buttons.columns = 2
	outer.add_child(buttons)
	for entry in [
		["Formation (F)", "formation"], ["Tir à volonté (G)", "fire_at_will"],
		["Halte (H)", "halt"], ["Retraite", "withdraw"], ["Retraite générale", "withdraw_all"],
	]:
		var button := Button.new()
		button.text = entry[0]
		button.focus_mode = Control.FOCUS_NONE
		button.pressed.connect(func() -> void: command_pressed.emit(entry[1]))
		buttons.add_child(button)
	outer.add_child(_build_corner())
	_build_help()


## Coin bas droit : minicarte, puis boutons-icônes de vitesse (pause, ×1, ×2, ×4).
func _build_corner() -> VBoxContainer:
	var corner := VBoxContainer.new()
	corner.add_theme_constant_override("separation", 4)
	minimap = MINIMAP.new()
	minimap.name = "Minimap"
	minimap.clicked.connect(func(world: Vector2) -> void: minimap_clicked.emit(world))
	corner.add_child(minimap)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	row.add_theme_constant_override("separation", 3)
	corner.add_child(row)
	for index in range(-1, 3):
		var button := Button.new()
		button.name = "Speed%d" % index
		button.custom_minimum_size = Vector2(38, 22)
		button.focus_mode = Control.FOCUS_NONE
		button.toggle_mode = true
		button.tooltip_text = SPEED_TOOLTIPS[index + 1]
		button.pressed.connect(func() -> void: speed_pressed.emit(index))
		button.draw.connect(_draw_speed_icon.bind(button, index))
		row.add_child(button)
		_speed_buttons.append(button)
	return corner


## Pictogramme dessiné (pas de glyphe de police) : deux barres pour la pause, 1 à 3 triangles.
func _draw_speed_icon(button: Button, index: int) -> void:
	var active := index == _active_speed
	var color := Color(0.6, 0.1, 0.08) if active and index < 0 else (Color(0.55, 0.35, 0.02) if active else INK)
	var center := button.size * 0.5
	if active:
		button.draw_rect(Rect2(Vector2(2, 2), button.size - Vector2(4, 4)), Color(0.95, 0.75, 0.15), false, 2.0)
	if index < 0:
		button.draw_rect(Rect2(center + Vector2(-6, -6), Vector2(4, 12)), color)
		button.draw_rect(Rect2(center + Vector2(2, -6), Vector2(4, 12)), color)
		return
	var count := index + 1
	var left := center.x - count * 4.5
	for i in count:
		var x := left + i * 9.0
		button.draw_colored_polygon(PackedVector2Array([Vector2(x, center.y - 6), Vector2(x + 9, center.y), Vector2(x, center.y + 6)]), color)


# --- Mises à jour ---------------------------------------------------------------------


func set_title(text: String, weather: String, colors: Array) -> void:
	title_label.text = text
	weather_label.text = weather
	_colors = colors


## B6 : le site de la bataille (sol, saison, village, haies, côte), vide pour le masquer.
func set_site(text: String) -> void:
	site_label.text = text
	site_label.visible = text != ""


func set_clock(seconds: float, speed: float, paused: bool) -> void:
	var total := int(seconds)
	clock_label.text = "%02d:%02d" % [total / 60, total % 60]
	var active := -1 if paused else maxi(BattleScene.SPEEDS.find(speed), 0)
	if active == _active_speed and not _speed_buttons.is_empty() and _speed_buttons[active + 1].button_pressed:
		return
	_active_speed = active
	for i in _speed_buttons.size():
		_speed_buttons[i].set_pressed_no_signal(i - 1 == active)
		_speed_buttons[i].queue_redraw()


func active_speed() -> int:
	return _active_speed


func _build_help() -> void:
	help_panel = PanelContainer.new()
	help_panel.name = "BattleHelp"
	help_panel.anchor_left = 0.5
	help_panel.anchor_right = 0.5
	help_panel.offset_left = -380
	help_panel.offset_right = 380
	help_panel.offset_top = 110
	help_panel.visible = false
	root.add_child(help_panel)
	var text := RichTextLabel.new()
	text.bbcode_enabled = true
	text.fit_content = true
	text.add_theme_color_override("default_color", INK)
	text.text = HELP_TEXT
	help_panel.add_child(text)


func toggle_help() -> void:
	help_panel.visible = not help_panel.visible


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


## Cartes des unités du joueur (créées une fois dans la colonne de leur « bataille »).
func update_cards(units: Array, side: String, selected: Array) -> void:
	for unit in units:
		if str(unit["side"]) != side:
			continue
		var id := int(unit["id"])
		if not _cards.has(id):
			_cards[id] = _make_card(unit)
		var card: UnitCard = _cards[id]
		card.refresh(unit, selected.has(id))
		card.set_groups(groups.numbers_of(id))


func _make_card(unit: Dictionary) -> UnitCard:
	var battle_key := BattleGroups.default_battle(unit)
	var column: VBoxContainer = _battle_columns[battle_key]
	column.visible = true
	var card: UnitCard = UNIT_CARD.new()
	(column.get_node("Cards") as HBoxContainer).add_child(card)
	card.setup(unit, get_node_or_null("/root/IconLibrary"), player_faction, _colors[0])
	card.clicked.connect(func(id: int, additive: bool) -> void: card_clicked.emit(id, additive))
	card.double_clicked.connect(func(id: int) -> void: card_double_clicked.emit(id))
	return card


func card_count() -> int:
	return _cards.size()


## Clé de « bataille » de la carte de l'unité ("" si pas de carte).
func card_battle(unit_id: int) -> String:
	if not _cards.has(unit_id):
		return ""
	return str((_cards[unit_id] as Node).get_parent().get_parent().get_meta("battle", ""))


# --- F5c : message éphémère (refus de déploiement, sortie de la garnison) ---------------

const TOAST_SECONDS := 4.0
var toast_label: Label = null
var _toast_serial: int = 0


func show_toast(text: String, is_error: bool = true) -> void:
	if toast_label == null:
		var panel := PanelContainer.new()
		panel.name = "Toast"
		panel.anchor_left = 0.5
		panel.anchor_right = 0.5
		panel.anchor_top = 0.3
		panel.anchor_bottom = 0.3
		panel.offset_left = -300
		panel.offset_right = 300
		panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
		root.add_child(panel)
		toast_label = _label("", 16)
		toast_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		toast_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		panel.add_child(toast_label)
	toast_label.text = text
	toast_label.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if is_error else INK)
	toast_label.get_parent().visible = true
	_toast_serial += 1
	var serial := _toast_serial
	get_tree().create_timer(TOAST_SECONDS).timeout.connect(func() -> void:
		if serial == _toast_serial and toast_label != null:
			toast_label.get_parent().visible = false)
