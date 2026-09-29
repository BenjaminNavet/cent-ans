class_name NavalHud
extends CanvasLayer

## Interface des batailles navales (lot NV1), dans le style parchemin d'UB1 (`BattleUiKit`) :
## bandeau (titre, lieu, vent et avantage du vent, barre d'équilibre des forces), cartes des
## navires du joueur (coque, feu, équipage), ordres (Aborder, Tirer, Éperonner, Flèches
## enflammées, Tenir, Se désengager), vitesse, chronique des événements, écran de fin.
## Aucune règle : les ordres partent en signaux vers la scène, qui les donne au cœur.
## Bandeau des navires (NV2) : cartes pleines tant qu'elles tiennent sur une ligne, sinon
## cartes compactes sur deux lignes, et défilement horizontal (molette, flèches) au-delà.

signal card_clicked(ship_id: int, additive: bool)
signal card_double_clicked(ship_id: int)
signal order_pressed(order: String)
signal speed_pressed(speed: float)
signal return_pressed

const ORDERS := [
	["board", "Aborder", "Grappins et passerelles sur le navire visé (clic droit sur un navire ennemi)"],
	["shoot", "Tirer", "Volées des châteaux sur le navire visé, à distance"],
	["ram", "Éperonner", "Galères seulement : éperon dans le flanc du navire visé"],
	["fire_arrows", "Flèches enflammées", "Traits enflammés : incendient les navires touchés"],
	["hold", "Tenir", "Rester sur place, tirer sur ce qui approche"],
	["disengage", "Se désengager", "Couper les grappins et fuir vent arrière"],
]
const SPEEDS := [["❚❚", 0.0], ["×1", 1.0], ["×2", 2.0], ["×4", 4.0]]
## Cartes des navires : pleines (une ligne) ou compactes (deux lignes au plus).
const CARD_FULL := Vector2(170, 96)
const CARD_COMPACT_MIN_WIDTH := 150.0
const CARD_COMPACT_MAX_WIDTH := 210.0
const CARD_COMPACT_HEIGHT := 50.0
const CARD_GAP := 5
const CARD_ROWS_MAX := 2

var player_side: String = "attacker"
var colors: Dictionary = {}
var _title: Label
var _subtitle: Label
var _wind: Label
var _balance: Control
var _share: float = 0.5
var _clock: Label
var _cards: GridContainer
var _card_scroll: ScrollContainer
var _scroll_left: Button
var _scroll_right: Button
var _compact := false
var _card_nodes: Dictionary = {}  # id -> {panel, name, hull, crew, status}
var _orders: HBoxContainer
var _order_buttons: Dictionary = {}
var _hint: Label
var _log: VBoxContainer
var _info: Label
var _result: PanelContainer = null
var _selected: Array = []
var _last_click: Dictionary = {"id": -1, "ms": -10000}


func _ready() -> void:
	layer = 5
	var root := Control.new()
	root.name = "Root"
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(root)
	# Bandeau du haut.
	var top := PanelContainer.new()
	top.name = "TopBanner"
	top.add_theme_stylebox_override("panel", BattleUiKit.page_box(10))
	top.set_anchors_preset(Control.PRESET_CENTER_TOP)
	top.position = Vector2(-330, 8)
	top.custom_minimum_size = Vector2(660, 0)
	root.add_child(top)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 3)
	top.add_child(column)
	_title = BattleUiKit.label("", 26, BattleUiKit.INK, true)
	_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(_title)
	_subtitle = BattleUiKit.label("", 15, BattleUiKit.INK_SOFT)
	_subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(_subtitle)
	_wind = BattleUiKit.label("", 15, BattleUiKit.INK)
	_wind.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(_wind)
	_balance = Control.new()
	_balance.custom_minimum_size = Vector2(600, 14)
	_balance.draw.connect(func() -> void: BattleUiKit.draw_balance(_balance, _share, colors.get(player_side, Color.BLUE), colors.get(_enemy(), Color.RED)))
	column.add_child(_balance)
	# Horloge et vitesse (haut droite).
	var speed_box := PanelContainer.new()
	speed_box.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(6))
	speed_box.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	speed_box.position = Vector2(-250, 10)
	root.add_child(speed_box)
	var speed_row := HBoxContainer.new()
	speed_box.add_child(speed_row)
	_clock = BattleUiKit.label("0:00", 18, BattleUiKit.INK, false, true)
	_clock.custom_minimum_size = Vector2(60, 0)
	speed_row.add_child(_clock)
	for entry in SPEEDS:
		var button := Button.new()
		button.text = str(entry[0])
		BattleUiKit.button_font(button, 15)
		var value := float(entry[1])
		button.pressed.connect(func() -> void: speed_pressed.emit(value))
		speed_row.add_child(button)
	# Chronique (gauche).
	var log_box := PanelContainer.new()
	log_box.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(8, Color(BattleUiKit.PARCHMENT, 0.85)))
	log_box.position = Vector2(10, 130)
	log_box.custom_minimum_size = Vector2(330, 0)
	root.add_child(log_box)
	_log = VBoxContainer.new()
	log_box.add_child(_log)
	_log.add_child(BattleUiKit.label("Chronique", 17, BattleUiKit.RUBRIC, true))
	# Bas : navire choisi, ordres, cartes.
	var bottom := VBoxContainer.new()
	bottom.name = "Bottom"
	bottom.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	bottom.offset_top = -214
	bottom.offset_bottom = -8
	bottom.offset_left = 10
	bottom.offset_right = -10
	bottom.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bottom.add_theme_constant_override("separation", 4)
	root.add_child(bottom)
	var order_panel := PanelContainer.new()
	order_panel.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(6))
	order_panel.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	bottom.add_child(order_panel)
	var order_column := VBoxContainer.new()
	order_panel.add_child(order_column)
	_info = BattleUiKit.label("Choisissez un navire (clic), puis clic droit sur un navire ennemi pour l'aborder.", 14, BattleUiKit.INK_SOFT)
	_info.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	order_column.add_child(_info)
	_orders = HBoxContainer.new()
	_orders.alignment = BoxContainer.ALIGNMENT_CENTER
	order_column.add_child(_orders)
	for entry in ORDERS:
		var button := Button.new()
		button.text = str(entry[1])
		button.tooltip_text = str(entry[2])
		BattleUiKit.button_font(button, 15)
		var key := str(entry[0])
		button.pressed.connect(func() -> void: order_pressed.emit(key))
		_orders.add_child(button)
		_order_buttons[key] = button
	_hint = BattleUiKit.label("", 13, BattleUiKit.RUBRIC)
	_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	order_column.add_child(_hint)
	# Bandeau des cartes : flèches de défilement de part et d'autre.
	var strip := HBoxContainer.new()
	strip.name = "CardStrip"
	strip.add_theme_constant_override("separation", 4)
	strip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bottom.add_child(strip)
	_scroll_left = _scroll_button("‹", -1)
	strip.add_child(_scroll_left)
	_card_scroll = ScrollContainer.new()
	_card_scroll.name = "CardScroll"
	_card_scroll.custom_minimum_size = Vector2(0, CARD_FULL.y + 8)
	_card_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_card_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_card_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_SHOW_NEVER
	_card_scroll.gui_input.connect(_on_cards_wheel)
	strip.add_child(_card_scroll)
	_scroll_right = _scroll_button("›", 1)
	strip.add_child(_scroll_right)
	_cards = GridContainer.new()
	_cards.name = "Cards"
	_cards.add_theme_constant_override("h_separation", CARD_GAP)
	_cards.add_theme_constant_override("v_separation", CARD_GAP - 1)
	_card_scroll.add_child(_cards)
	get_viewport().size_changed.connect(_layout_cards)
	_layout_cards()


func _scroll_button(text: String, direction: int) -> Button:
	var button := Button.new()
	button.text = text
	RichTooltip.attach_plain(button, "naval_scroll_ships")
	BattleUiKit.button_font(button, 22)
	button.custom_minimum_size = Vector2(26, 0)
	button.visible = false
	button.pressed.connect(func() -> void: _scroll_cards(direction))
	return button


func _scroll_cards(direction: int) -> void:
	var step := int(_card_width() + CARD_GAP) * 3
	_card_scroll.scroll_horizontal = maxi(0, _card_scroll.scroll_horizontal + direction * step)


## Molette sur le bandeau : défilement horizontal.
func _on_cards_wheel(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button == null or not button.pressed:
		return
	if button.button_index in [MOUSE_BUTTON_WHEEL_DOWN, MOUSE_BUTTON_WHEEL_RIGHT]:
		_card_scroll.scroll_horizontal += int(_card_width() + CARD_GAP)
		_card_scroll.accept_event()
	elif button.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_LEFT]:
		_card_scroll.scroll_horizontal = maxi(0, _card_scroll.scroll_horizontal - int(_card_width() + CARD_GAP))
		_card_scroll.accept_event()


## Largeur disponible pour les cartes (écran moins les marges et les flèches).
func _strip_width() -> float:
	var arrows := 60.0 if _scroll_right != null and _scroll_right.visible else 0.0
	return get_viewport().get_visible_rect().size.x - 20.0 - arrows


func _card_width() -> float:
	if not _compact:
		return CARD_FULL.x
	var count := maxi(_card_nodes.size(), 1)
	var columns := ceili(float(count) / CARD_ROWS_MAX)
	var fit := (_strip_width() - CARD_GAP * (columns - 1)) / columns
	return clampf(fit, CARD_COMPACT_MIN_WIDTH, CARD_COMPACT_MAX_WIDTH)


## Cartes pleines sur une ligne si elles tiennent, sinon compactes sur deux lignes ;
## flèches et molette quand même cela déborde.
func _layout_cards() -> void:
	if _cards == null:
		return
	var count := maxi(_card_nodes.size(), 1)
	var width := _strip_width()
	_compact = count * (CARD_FULL.x + CARD_GAP) - CARD_GAP > width
	var card_size := CARD_FULL
	var rows := 1
	if _compact:
		rows = CARD_ROWS_MAX if count > 1 else 1
		card_size = Vector2(_card_width(), CARD_COMPACT_HEIGHT)
	var columns := ceili(float(count) / rows)
	_cards.columns = maxi(columns, 1)
	var content := columns * (card_size.x + CARD_GAP) - CARD_GAP
	var overflow := content > width + 0.5
	_scroll_left.visible = overflow
	_scroll_right.visible = overflow
	_card_scroll.custom_minimum_size = Vector2(0, rows * (card_size.y + CARD_GAP) + 4)
	for id in _card_nodes:
		_style_card(_card_nodes[id], card_size)


func _style_card(card: Dictionary, card_size: Vector2) -> void:
	(card["panel"] as PanelContainer).custom_minimum_size = card_size
	(card["kind"] as Label).visible = not _compact
	(card["crew"] as Label).visible = not _compact
	(card["hull"] as ProgressBar).custom_minimum_size = Vector2(card_size.x - 20.0, 6 if _compact else 8)


## Mode du bandeau (tests) : `compact`, nombre de colonnes, défilement.
func card_layout() -> Dictionary:
	return {
		"compact": _compact,
		"columns": _cards.columns,
		"scroll": _scroll_right.visible,
		"card_width": _card_width(),
		"strip_width": _strip_width(),
	}


func _enemy() -> String:
	return "defender" if player_side == "attacker" else "attacker"


func set_header(title: String, subtitle: String, p_player_side: String, p_colors: Dictionary) -> void:
	player_side = p_player_side
	colors = p_colors
	_title.text = title
	_subtitle.text = subtitle


func set_wind(text: String) -> void:
	_wind.text = text


func set_balance(player_strength: float, enemy_strength: float) -> void:
	var share := player_strength / maxf(player_strength + enemy_strength, 0.001)
	if absf(share - _share) > 0.002:
		_share = share
		_balance.queue_redraw()


func set_clock(seconds: float) -> void:
	_clock.text = "%d:%02d" % [int(seconds) / 60, int(seconds) % 60]


func set_hint(text: String) -> void:
	_hint.text = text


## Cartes des navires du joueur (état courant de `get_ships`).
func update_cards(ships: Array, selected: Array) -> void:
	_selected = selected
	for ship in ships:
		if str(ship["side"]) != player_side:
			continue
		var id := int(ship["id"])
		if not _card_nodes.has(id):
			_card_nodes[id] = _make_card(ship)
			_layout_cards()
		var card: Dictionary = _card_nodes[id]
		var status := str(ship["status"])
		var men := int(round(float(ship["soldiers"])))
		var sailors := int(round(float(ship["sailors"])))
		(card["crew"] as Label).text = "%d hommes · %d marins" % [men, sailors]
		var hull := float(ship["hull"]) / maxf(float(ship["hull_max"]), 1.0)
		(card["hull"] as ProgressBar).value = hull * 100.0
		var text := _status_text(ship)
		if _compact and status == "afloat":
			text = "%d h · %s" % [men + sailors, text]
		(card["status"] as Label).text = text
		RichTooltip.attach_plain(card["panel"] as PanelContainer, "naval_ship_card", {"title": "%s (%s)" % [ship["name"], ship["class_name"]], "body": "%d hommes, %d marins" % [men, sailors]})
		(card["status"] as Label).add_theme_color_override("font_color", BattleUiKit.RUBRIC if float(ship["fire"]) > 0.05 or status != "afloat" else BattleUiKit.INK_SOFT)
		var panel: PanelContainer = card["panel"]
		var chosen := selected.has(id)
		var box := BattleUiKit.parchment_box(6, BattleUiKit.PARCHMENT_LIGHT if chosen else BattleUiKit.PARCHMENT, BattleUiKit.GOLD if chosen else BattleUiKit.INK_SOFT, 3 if chosen else 1)
		panel.add_theme_stylebox_override("panel", box)
		panel.modulate = Color(1, 1, 1, 0.55) if status in ["sunk", "captured", "escaped", "abandoned"] else Color.WHITE


func _status_text(ship: Dictionary) -> String:
	match str(ship["status"]):
		"captured":
			return "Pris"
		"sunk":
			return "Coulé"
		"sinking":
			return "Sombre"
		"escaped":
			return "Échappé"
		"abandoned":
			return "Abandonné"
	var parts: Array[String] = []
	if float(ship["fire"]) > 0.05:
		parts.append("En feu (%d %%)" % int(float(ship["fire"]) * 100.0))
	if (ship.get("grappled", PackedInt32Array()) as PackedInt32Array).size() > 0:
		parts.append("Grappiné")
	if bool(ship.get("fleeing", false)):
		parts.append("Fuit")
	if int(ship.get("chain", -1)) >= 0:
		parts.append("Enchaîné")
	if parts.is_empty():
		parts.append(ORDER_LABELS.get(str(ship.get("order", "hold")), ""))
	return " · ".join(parts)


const ORDER_LABELS := {"hold": "Tient", "move": "Fait route", "board": "Va aborder", "shoot": "Tire", "ram": "Éperonne", "disengage": "Se dégage"}


func _make_card(ship: Dictionary) -> Dictionary:
	var id := int(ship["id"])
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(170, 96)
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	RichTooltip.attach_plain(panel, "naval_ship_name", {"title": "%s (%s)" % [ship["name"], ship["class_name"]]})
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 1)
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	panel.add_child(column)
	var title := BattleUiKit.label(("⚑ " if bool(ship.get("flagship", false)) else "") + str(ship["name"]), 15, BattleUiKit.INK, false, true)
	title.clip_text = true
	title.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(title)
	var kind := BattleUiKit.label(str(ship["class_name"]), 13, BattleUiKit.INK_FADED)
	kind.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(kind)
	var hull := ProgressBar.new()
	hull.custom_minimum_size = Vector2(150, 8)
	hull.show_percentage = false
	hull.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var fill := StyleBoxFlat.new()
	fill.bg_color = BattleUiKit.GOOD
	hull.add_theme_stylebox_override("fill", fill)
	RichTooltip.attach_plain(hull, "naval_hull")
	column.add_child(hull)
	var crew := BattleUiKit.label("", 13, BattleUiKit.INK)
	crew.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(crew)
	var status := BattleUiKit.label("", 13, BattleUiKit.INK_SOFT)
	status.clip_text = true
	status.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(status)
	panel.gui_input.connect(func(event: InputEvent) -> void:
		_on_cards_wheel(event)
		if event is InputEventMouseButton and (event as InputEventMouseButton).pressed and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
			var now := Time.get_ticks_msec()
			if int(_last_click["id"]) == id and now - int(_last_click["ms"]) < 350:
				card_double_clicked.emit(id)
			else:
				card_clicked.emit(id, (event as InputEventMouseButton).shift_pressed or (event as InputEventMouseButton).ctrl_pressed)
			_last_click = {"id": id, "ms": now})
	_cards.add_child(panel)
	return {"panel": panel, "hull": hull, "crew": crew, "status": status, "kind": kind}


## Ordres possibles pour la sélection (Éperonner : galères seulement).
func set_orders_enabled(any_selected: bool, can_ram: bool, fire_arrows_on: bool) -> void:
	for key in _order_buttons:
		var button: Button = _order_buttons[key]
		button.disabled = not any_selected or (key == "ram" and not can_ram)
	(_order_buttons["fire_arrows"] as Button).text = "Flèches enflammées : %s" % ("oui" if fire_arrows_on else "non")


func set_info(text: String) -> void:
	_info.text = text


## Ajoute des lignes à la chronique (six dernières gardées).
func add_log(lines: Array) -> void:
	for line in lines:
		var label := BattleUiKit.label(str(line[0]), 14, line[1] if line.size() > 1 else BattleUiKit.INK)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(310, 0)
		_log.add_child(label)
	while _log.get_child_count() > 7:
		var old := _log.get_child(1)
		_log.remove_child(old)
		old.queue_free()


## Écran de fin : vainqueur, pertes, navires pris et coulés de chaque camp.
func show_result(outcome: Dictionary, side_names: Dictionary, button_text: String) -> void:
	if _result != null:
		return
	_result = PanelContainer.new()
	_result.name = "Result"
	PanelStack.mark_blocking(_result)  # Q4
	_result.add_theme_stylebox_override("panel", BattleUiKit.illuminated_box(40))  # clear the 38 px ivy frame (Q3)
	_result.set_anchors_preset(Control.PRESET_CENTER)
	_result.custom_minimum_size = Vector2(620, 0)
	_result.position = Vector2(-310, -230)
	get_node("Root").add_child(_result)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 8)
	_result.add_child(column)
	var winner := str(outcome.get("winner", ""))
	var headline := "Aucun vainqueur : les flottes se séparent"
	var tint := BattleUiKit.INK
	if winner == player_side:
		headline = "Victoire sur mer !"
		tint = BattleUiKit.GOOD
	elif winner != "" and winner != "<null>":
		headline = "Défaite sur mer"
		tint = BattleUiKit.RUBRIC
	var title := BattleUiKit.label(headline, 32, tint, true)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(title)
	var duration := float(outcome.get("duration", 0.0))
	var sub := BattleUiKit.label("Combat de %d min %02d s" % [int(duration) / 60, int(duration) % 60], 15, BattleUiKit.INK_SOFT)
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(sub)
	column.add_child(BattleUiKit.rule())
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 24)
	column.add_child(grid)
	var rows := [["", "attacker", "defender"]]
	for row in [["Hommes au départ", "men_start"], ["Tués", "men_lost"], ["Noyés", "drowned"], ["Prisonniers", "prisoners"]]:
		rows.append(row)
	for cell in ["", str(side_names.get("attacker", "Assaillant")), str(side_names.get("defender", "Défenseur"))]:
		grid.add_child(BattleUiKit.label(cell, 17, BattleUiKit.RUBRIC, false, true))
	for row in rows.slice(1):
		grid.add_child(BattleUiKit.label(str(row[0]), 16))
		for side in ["attacker", "defender"]:
			grid.add_child(BattleUiKit.label(BattleUiKit.thousands(int((outcome.get(side, {}) as Dictionary).get(row[1], 0))), 16))
	for fate in [["Navires pris", "captured"], ["Navires coulés", "sunk"], ["Navires échappés", "escaped"]]:
		grid.add_child(BattleUiKit.label(str(fate[0]), 16))
		for side in ["attacker", "defender"]:
			var count := 0
			for ship in (outcome.get(side, {}) as Dictionary).get("ships", []):
				if str(ship.get("fate", "")) == str(fate[1]):
					count += 1
			grid.add_child(BattleUiKit.label(str(count), 16))
	var prizes: Array = (outcome.get(player_side, {}) as Dictionary).get("prizes", [])
	if not prizes.is_empty():
		var prize_label := BattleUiKit.label("Prises ramenées au port : %d navire%s." % [prizes.size(), "s" if prizes.size() > 1 else ""], 16, BattleUiKit.GOOD, false, true)
		column.add_child(prize_label)
	column.add_child(BattleUiKit.rule())
	var button := Button.new()
	button.text = button_text
	BattleUiKit.button_font(button, 20)
	button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	button.pressed.connect(func() -> void: return_pressed.emit())
	column.add_child(button)


func result_shown() -> bool:
	return _result != null
