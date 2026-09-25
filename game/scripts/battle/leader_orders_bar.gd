extends CanvasLayer

## Barre des ordres du chef (F10b, `docs/design/battle-orders.md`) : une rangée de boutons en bas
## à gauche, au-dessus des cartes d'unités. Aucune règle ici : la liste, la disponibilité, la
## raison d'un refus et la recharge viennent de `BattleSim.get_leader_orders(side)` ; un clic (ou
## un raccourci) envoie `{type: "leader_order", order, units}` par `BattleScene.issue`.
## Branché par `battle_scene.gd` (`add_child(LEADER_ORDERS_BAR.new(self))` à la fin de `begin`).

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const INK := Color(0.22, 0.14, 0.07)
const INK_MUTED := Color(0.42, 0.34, 0.26)
const REFRESH := 0.2
## Raccourcis, dans l'ordre de la barre (touches libres : 1-3 vitesse, F/G/H ordres, Q/E/WASD caméra,
## C/M/T carte de campagne). Touches **physiques** (position QWERTY), comme la caméra : en AZERTY
## la rangée du bas donne W X V B N et ne recoupe jamais Z Q S D (audit A3 B1).
const HOTKEYS := [KEY_Z, KEY_X, KEY_V, KEY_B, KEY_N]
const ICON_DIR := "res://assets/ui/orders/"
## Repli quand l'icône PNG n'existe pas : un glyphe par nature d'ordre.
const GLYPHS := {"war_cry": "✠", "rally": "⚑", "dismount": "♞", "pavise": "▮", "no_quarter": "⚔"}
## Au-dessus du bandeau des cartes d'unités de `battle_hud.gd` (panneau haut de 182 px) et de sa
## ligne d'aide.
const BOTTOM_MARGIN := 216.0
const BUTTON_SIZE := Vector2(108, 62)

var scene: Node = null  # BattleScene
var panel: PanelContainer
var row: HBoxContainer
var _buttons: Dictionary = {}  # order id -> {button, glyph, name, key, shade, timer}
var _orders: Array = []
var _timer: float = 0.0


func _init(p_scene: Node = null) -> void:
	scene = p_scene
	layer = 2
	name = "LeaderOrdersBar"


func _ready() -> void:
	panel = PanelContainer.new()
	panel.theme = load(THEME_PATH)
	panel.anchor_top = 1.0
	panel.anchor_bottom = 1.0
	panel.offset_left = 8
	panel.grow_vertical = Control.GROW_DIRECTION_BEGIN
	panel.offset_bottom = -BOTTOM_MARGIN
	panel.offset_top = -BOTTOM_MARGIN - BUTTON_SIZE.y - 30
	add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	panel.add_child(box)
	var title := Label.new()
	title.text = "Ordres du chef"
	title.add_theme_font_size_override("font_size", 13)
	title.add_theme_color_override("font_color", INK_MUTED)
	box.add_child(title)
	row = HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	box.add_child(row)
	refresh()


func _process(delta: float) -> void:
	_timer -= delta
	if _timer <= 0.0:
		_timer = REFRESH
		refresh()


func _battle() -> Object:
	return scene.get("battle") if scene != null else null


func _side() -> String:
	return str(scene.get("player_side")) if scene != null else "attacker"


## Relit la barre depuis la simulation (création des boutons au premier appel).
func refresh() -> void:
	var battle := _battle()
	if battle == null or not battle.has_method("get_leader_orders"):
		visible = false
		return
	_orders = battle.call("get_leader_orders", _side())
	visible = not _orders.is_empty() and not bool(battle.call("is_finished"))
	for i in _orders.size():
		var order: Dictionary = _orders[i]
		var id := str(order["id"])
		if not _buttons.has(id):
			_buttons[id] = _make_button(order, i)
		_update_button(_buttons[id], order)


func _make_button(order: Dictionary, index: int) -> Dictionary:
	var button := OrderButton.new()
	button.custom_minimum_size = BUTTON_SIZE
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(_on_pressed.bind(str(order["id"])))
	row.add_child(button)
	var inner := VBoxContainer.new()
	inner.set_anchors_preset(Control.PRESET_FULL_RECT)
	inner.alignment = BoxContainer.ALIGNMENT_CENTER
	inner.add_theme_constant_override("separation", 0)
	inner.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button.add_child(inner)
	var icon_node: Control
	var icon_path := ICON_DIR + str(order.get("icon", "")) + ".png"
	if str(order.get("icon", "")) != "" and ResourceLoader.exists(icon_path):
		var tex := TextureRect.new()
		tex.texture = load(icon_path)
		tex.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		tex.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		tex.custom_minimum_size = Vector2(0, 28)
		icon_node = tex
	else:
		var glyph := Label.new()
		glyph.text = GLYPHS.get(str(order.get("kind", "")), "✦")
		glyph.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		glyph.add_theme_font_size_override("font_size", 24)
		glyph.add_theme_color_override("font_color", INK)
		icon_node = glyph
	icon_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	inner.add_child(icon_node)
	var name_label := Label.new()
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	name_label.custom_minimum_size = Vector2(BUTTON_SIZE.x - 8, 0)
	name_label.add_theme_font_size_override("font_size", 12)
	name_label.add_theme_color_override("font_color", INK)
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	inner.add_child(name_label)
	# Raccourci en haut à gauche, recharge en surimpression.
	var key := Label.new()
	key.text = physical_label(HOTKEYS[index]) if index < HOTKEYS.size() else ""
	key.position = Vector2(5, 1)
	key.add_theme_font_size_override("font_size", 11)
	key.add_theme_color_override("font_color", INK_MUTED)
	key.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button.add_child(key)
	var shade := ColorRect.new()
	shade.color = Color(0.1, 0.07, 0.04, 0.35)
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	shade.set_anchors_preset(Control.PRESET_FULL_RECT)
	button.add_child(shade)
	# Recharge en haut à droite (face au raccourci) : ne recouvre plus le nom (audit A3 B5).
	var timer := Label.new()
	timer.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	timer.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	timer.offset_right = -5
	timer.offset_top = 0
	timer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	timer.add_theme_font_size_override("font_size", 14)
	timer.add_theme_color_override("font_color", Color(1, 0.95, 0.85))
	timer.add_theme_color_override("font_outline_color", Color(0.1, 0.06, 0.03))
	timer.add_theme_constant_override("outline_size", 5)
	timer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button.add_child(timer)
	return {"button": button, "name": name_label, "shade": shade, "timer": timer, "key": key.text}


## Libellé de la touche physique `keycode` sur la disposition active (Z physique → « W » en AZERTY).
static func physical_label(keycode: Key) -> String:
	var label := keycode
	if DisplayServer.get_name() != "headless":
		label = DisplayServer.keyboard_get_label_from_physical(keycode)
	if label == KEY_NONE:
		label = keycode
	return OS.get_keycode_string(label)


## Raccourcis des ordres, séparés par des espaces (« W X V B N » en AZERTY).
static func hotkey_labels() -> String:
	var labels: PackedStringArray = []
	for keycode: Key in HOTKEYS:
		labels.append(physical_label(keycode))
	return " ".join(labels)


func _update_button(entry: Dictionary, order: Dictionary) -> void:
	var button: OrderButton = entry["button"]
	var available := bool(order["available"])
	var remaining := float(order.get("cooldown_remaining", 0.0))
	var name_label: Label = entry["name"]
	name_label.text = str(order["name"])
	button.disabled = not available
	button.modulate = Color(1, 1, 1) if available else Color(0.85, 0.8, 0.75, 0.6)
	var shade: ColorRect = entry["shade"]
	var total := float(order.get("cooldown", 0.0))
	var timer: Label = entry["timer"]
	if remaining > 0.0 and total > 0.0:
		# Rideau de recharge : la part sombre descend à mesure que l'ordre revient.
		shade.visible = true
		shade.anchor_top = 1.0 - clampf(remaining / total, 0.0, 1.0)
		shade.offset_top = 0
		timer.text = "%d s" % int(ceil(remaining))
	else:
		shade.visible = false
		timer.text = ""
	button.order = order
	button.hotkey = str(entry["key"])
	button.tooltip_text = str(order["label"])  # déclenche l'infobulle riche


func _on_pressed(id: String) -> void:
	give(id)


## Donne l'ordre `id` : les ordres « sélection » visent les régiments choisis (tous les régiments
## concernés si la sélection est vide ou n'en contient aucun), les autres n'en ont pas besoin.
func give(id: String) -> Dictionary:
	var battle := _battle()
	if battle == null:
		return {}
	var units: Array[int] = []
	if scene.has_method("_available_selection"):
		units = scene.call("_available_selection")
	var command := {"type": "leader_order", "order": id, "units": units}
	var result: Dictionary = scene.call("issue", command)
	if not bool(result.get("ok", false)) and not units.is_empty() and str(result.get("error", "")).contains("désignés"):
		# Aucun régiment choisi ne peut obéir : l'ordre vaut pour tous ceux qui le peuvent.
		command["units"] = []
		result = scene.call("issue", command)
	_timer = 0.0
	return result


func _unhandled_input(event: InputEvent) -> void:
	if not visible or _battle() == null:
		return
	if event is InputEventKey and event.pressed and not event.echo and not event.ctrl_pressed:
		var index := HOTKEYS.find((event as InputEventKey).physical_keycode)
		if index >= 0 and index < _orders.size():
			give(str(_orders[index]["id"]))
			get_viewport().set_input_as_handled()


## Bouton d'ordre avec infobulle riche (nom, libellé de la faction, description, état).
class OrderButton extends Button:
	var order: Dictionary = {}
	var hotkey: String = ""

	func _make_custom_tooltip(_for_text: String) -> Object:
		var box := PanelContainer.new()
		box.theme = load(THEME_PATH)
		var text := RichTextLabel.new()
		text.bbcode_enabled = true
		text.fit_content = true
		text.custom_minimum_size = Vector2(340, 0)
		text.add_theme_color_override("default_color", INK)
		var lines: Array[String] = []
		lines.append("[b]%s[/b]   [color=#6b5a45][%s][/color]" % [order.get("name", ""), hotkey])
		if str(order.get("label", "")) != str(order.get("name", "")):
			lines.append("[i]%s[/i]" % order.get("label", ""))
		lines.append(str(order.get("description", "")))
		var facts: Array[String] = []
		var cooldown := float(order.get("cooldown", 0.0))
		if cooldown > 0.0:
			facts.append("recharge %d s" % int(cooldown))
		var per_battle = order.get("uses_per_battle", null)
		if per_battle != null:
			facts.append("%d fois par bataille" % int(per_battle))
		if not facts.is_empty():
			lines.append("[color=#6b5a45]%s[/color]" % " · ".join(facts))
		if not bool(order.get("available", false)):
			lines.append("[color=#8a2a1a]Indisponible : %s[/color]" % order.get("reason", ""))
		text.text = "\n".join(lines)
		box.add_child(text)
		return box
