class_name HoldingsController
extends Node

## Liste « Colonies » (touche B), lot HL2 : accès rapide aux colonies du joueur façon Total
## War, groupées par province repliable. Rendu, UI et entrées seulement ; toutes les données
## viennent de `CampaignSim.get_holdings_overview(faction)` (lot HL1) :
## `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 2.
## Un clic sur une colonie ouvre son panneau (`SettlementController.open_settlement`) ; un clic
## sur « ⌖ » d'une province centre la caméra et ouvre le panneau de province. On ne construit
## ni ne recrute depuis cette liste.

const PANEL_WIDTH := 440.0
const MIN_HEIGHT := 160.0
const FOCUS_DISTANCE := 220.0
## Filtres exclusifs, dans l'ordre d'affichage.
const FILTERS := ["all", "idle", "upgrade", "endangered"]
const SORTS := ["income", "name", "unrest"]

var map: Node = null  # CampaignMap
var panel: PanelContainer = null

var _list: VBoxContainer
var _scroll: ScrollContainer
var _treasury_label: Label
var _income_label: Label
var _filter_buttons: Dictionary = {}  # key → Button
var _sort_buttons: Dictionary = {}  # key → Button
var _filter := "all"
var _sort := "income"
## Repli manuel d'une province (id → bool) : conservé jusqu'à la fin de la partie.
var _expanded: Dictionary = {}
var _overview: Dictionary = {}
var _visible_settlement_count := 0
var _visible_province_count := 0
var _placed_top := -1.0


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "HoldingsController"
	_build_panel()
	get_viewport().size_changed.connect(_layout)


## Vrai si la simulation expose l'API des colonies (vraie `CampaignSim`).
func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_holdings_overview")


# --- Panneau ------------------------------------------------------------------------------


func _build_panel() -> void:
	panel = PanelContainer.new()
	panel.name = "Holdings"
	panel.theme = load("res://scenes/ui/parchment_theme.tres")
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	panel.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	panel.add_child(box)

	var head := HBoxContainer.new()
	box.add_child(head)
	var title := HudStyle.label("Colonies (B)", HudStyle.FONT_TITLE)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	var close := Button.new()
	close.text = "✕"
	TooltipHost.attach_plain(close, "close_list_b")
	close.pressed.connect(func() -> void: panel.hide())
	head.add_child(close)

	var treasury_row := HBoxContainer.new()
	treasury_row.add_theme_constant_override("separation", 10)
	box.add_child(treasury_row)
	_treasury_label = HudStyle.label("", HudStyle.FONT_BODY)
	_income_label = HudStyle.label("", HudStyle.FONT_BODY)
	treasury_row.add_child(_treasury_label)
	treasury_row.add_child(_income_label)

	var filter_row := HBoxContainer.new()
	filter_row.add_theme_constant_override("separation", 4)
	box.add_child(filter_row)
	for key in FILTERS:
		var button := Button.new()
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.pressed.connect(func() -> void: set_filter(key))
		filter_row.add_child(button)
		_filter_buttons[key] = button

	var sort_row := HBoxContainer.new()
	sort_row.add_theme_constant_override("separation", 4)
	box.add_child(sort_row)
	sort_row.add_child(HudStyle.label("Tri :", HudStyle.FONT_SMALL, HudStyle.INK_SOFT))
	var sort_texts := {"income": "Revenu", "name": "Nom", "unrest": "Ordre public"}
	for key in SORTS:
		var button := Button.new()
		button.text = str(sort_texts[key])
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
		button.pressed.connect(func() -> void: set_sort(key))
		sort_row.add_child(button)
		_sort_buttons[key] = button

	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(_scroll)
	_list = VBoxContainer.new()
	_list.add_theme_constant_override("separation", 3)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_list)

	panel.hide()
	panel.add_to_group(PanelStack.BLOCKING_GROUP)
	map.ui.add_child(panel)


## Ouvre ou ferme le panneau ; l'ouvrir ferme « Mes unités » (même coin de l'écran).
func toggle() -> void:
	if panel.visible:
		panel.hide()
		return
	if map.units_ctl != null and map.units_ctl.is_open():
		map.units_ctl.toggle()
	panel.show()
	refresh()


func is_open() -> bool:
	return panel != null and panel.visible


## Reconstruit la liste depuis `get_holdings_overview`. Sans effet si le panneau est fermé.
func refresh() -> void:
	if not is_open() or not available():
		return
	_overview = map.sim.call("get_holdings_overview", map.player_faction)
	_rebuild()
	_layout.call_deferred()


func _rebuild() -> void:
	for child in _list.get_children():
		child.queue_free()
	_update_header()
	_visible_settlement_count = 0
	_visible_province_count = 0
	var provinces: Array = (_overview.get("provinces", []) as Array).duplicate()
	match _sort:
		"name":
			provinces.sort_custom(func(a, b) -> bool: return str(a["name"]) < str(b["name"]))
		"unrest":
			provinces.sort_custom(func(a, b) -> bool: return int(a["unrest"]) > int(b["unrest"]))
		_:
			provinces.sort_custom(func(a, b) -> bool: return int(a["income"]) > int(b["income"]))
	for province in provinces:
		var settlements: Array = province.get("settlements", [])
		var visible_settlements: Array = []
		for settlement in settlements:
			if _matches_filter(settlement):
				visible_settlements.append(settlement)
		if visible_settlements.is_empty():
			continue
		_visible_province_count += 1
		_visible_settlement_count += visible_settlements.size()
		var has_danger := (settlements as Array).any(func(s) -> bool: return bool(s.get("endangered", false)))
		var expanded := _filter != "all" or _province_expanded(str(province["province"]), has_danger)
		_add_province_row(province, expanded, visible_settlements)


func _matches_filter(settlement: Dictionary) -> bool:
	match _filter:
		"idle":
			return bool(settlement.get("idle", false))
		"upgrade":
			return not (settlement.get("options_available", []) as Array).is_empty()
		"endangered":
			return bool(settlement.get("endangered", false))
		_:
			return true


func _province_expanded(province_id: String, has_danger: bool) -> bool:
	if _expanded.has(province_id):
		return bool(_expanded[province_id])
	return has_danger


func _update_header() -> void:
	_treasury_label.text = "Trésor : %s" % Money.amount(int(_overview.get("treasury", 0)))
	var net := int(_overview.get("net_income_last_turn", 0))
	_income_label.text = "Revenu net : %s" % Money.signed(net)
	_income_label.add_theme_color_override("font_color", Money.color_of(net))
	(_filter_buttons["all"] as Button).text = "Tout"
	(_filter_buttons["idle"] as Button).text = "⚒ libre %d" % int(_overview.get("count_idle", 0))
	(_filter_buttons["upgrade"] as Button).text = "▲ %d" % int(_overview.get("count_upgrade", 0))
	(_filter_buttons["endangered"] as Button).text = "⚠ %d" % int(_overview.get("count_endangered", 0))
	for key in _filter_buttons:
		(_filter_buttons[key] as Button).set_pressed_no_signal(key == _filter)
	for key in _sort_buttons:
		(_sort_buttons[key] as Button).set_pressed_no_signal(key == _sort)


# --- Lignes ---------------------------------------------------------------------------


func _add_province_row(province: Dictionary, expanded: bool, visible_settlements: Array) -> void:
	var province_id := str(province["province"])
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 4)
	_list.add_child(row)

	var header := Button.new()
	header.flat = true
	header.focus_mode = Control.FOCUS_NONE
	header.alignment = HORIZONTAL_ALIGNMENT_LEFT
	header.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var revolt_seasons := int(province.get("revolt_seasons", 0))
	var revolt_needed := int(province.get("revolt_seasons_needed", 1))
	var revolt_text := ""
	if revolt_seasons > 0:
		var remaining := maxi(revolt_needed - revolt_seasons, 0)
		revolt_text = " ⚠ révolte dans %s" % FrText.count(remaining, "saison")
	header.text = "%s %s · %s · ⚒ %d/%d%s" % [
		"▾" if expanded else "▸", province.get("name", province_id),
		Money.amount(int(province.get("income", 0))), int(province.get("slots_busy", 0)),
		int(province.get("slots_total", 0)), revolt_text]
	TooltipHost.attach_plain(header, "holdings_weighted_unrest", {"body": "%d %%" % int(province.get("unrest", 0))})
	header.pressed.connect(func() -> void:
		_expanded[province_id] = not expanded
		_rebuild())
	row.add_child(header)

	var focus_button := Button.new()
	focus_button.text = "⌖"
	focus_button.focus_mode = Control.FOCUS_NONE
	TooltipHost.attach_plain(focus_button, "province_focus_open")
	focus_button.pressed.connect(func() -> void: focus_province(province_id))
	row.add_child(focus_button)

	if expanded:
		for settlement in visible_settlements:
			_add_settlement_row(settlement, province)


func _add_settlement_row(settlement: Dictionary, province: Dictionary) -> void:
	var row := Button.new()
	row.flat = false
	row.focus_mode = Control.FOCUS_NONE
	row.alignment = HORIZONTAL_ALIGNMENT_LEFT
	row.add_theme_stylebox_override("normal", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.PARCHMENT_DARK))
	row.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD))
	row.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))

	var name_text := str(settlement.get("name", ""))
	if bool(settlement.get("is_city", false)):
		name_text += " (cité)"

	var build_text := ""
	if settlement.has("construction"):
		var construction: Dictionary = settlement["construction"]
		build_text = "%s · %s" % [construction.get("name", ""), FrText.count(int(construction.get("turns_left", 0)), "tour")]
	elif bool(settlement.get("idle", false)):
		build_text = "libre"

	var options: Array = settlement.get("options_available", [])
	var badge := ""
	if bool(settlement.get("upgrade_available", false)):
		badge = "▲"
	elif not options.is_empty():
		badge = "△"

	var danger_text := ""
	if settlement.has("siege"):
		var siege: Dictionary = settlement["siege"]
		danger_text = "⚔ siège (%s)" % FrText.count(int(siege.get("turns_left", 0)), "tour")
	elif bool(settlement.get("occupied", false)):
		danger_text = "occupée"
	elif bool(settlement.get("endangered", false)):
		danger_text = "⚠"

	row.text = "%s   %s   %s   %s   garnison %d   %s" % [
		name_text, Money.amount(int(settlement.get("income", 0))), build_text, badge,
		int(settlement.get("garrison_strength", 0)), danger_text]
	row.tooltip_text = _settlement_tooltip(settlement, province)
	var settlement_id := str(settlement.get("id", ""))
	row.pressed.connect(func() -> void: focus_settlement(settlement_id))
	_list.add_child(row)


func _settlement_tooltip(settlement: Dictionary, province: Dictionary) -> String:
	var lines: Array = []
	var options: Array = settlement.get("options_available", [])
	if not options.is_empty():
		lines.append("Constructions possibles :")
		for option in options:
			var suffix := " (promotion)" if bool(option.get("is_upgrade", false)) else ""
			lines.append("- %s — %s%s" % [option.get("name", ""), Money.amount(int(option.get("cost", 0))), suffix])
	var reasons: Array = settlement.get("danger_reasons", [])
	if not reasons.is_empty():
		lines.append("Menaces :")
		for reason in reasons:
			lines.append("- " + _danger_label(str(reason), province))
	return "\n".join(lines)


func _danger_label(reason: String, province: Dictionary) -> String:
	match reason:
		"siege":
			return "Assiégée"
		"occupied":
			return "Occupée"
		"revolt_countdown":
			var remaining := maxi(int(province.get("revolt_seasons_needed", 1)) - int(province.get("revolt_seasons", 0)), 0)
			return "Révolte dans %s" % FrText.count(remaining, "saison")
		"unrest":
			return "Agitation au-dessus du seuil de révolte"
		_:
			return reason


# --- Filtres, tri, sélection -----------------------------------------------------------


func row_count() -> int:
	return _visible_settlement_count


func province_row_count() -> int:
	return _visible_province_count


## Filtre actif : "all" / "idle" / "upgrade" / "endangered".
func set_filter(key: String) -> void:
	if _filter == key or not FILTERS.has(key):
		return
	_filter = key
	refresh()


## Tri des provinces : "income" (défaut) / "name" / "unrest".
func set_sort(key: String) -> void:
	if _sort == key or not SORTS.has(key):
		return
	_sort = key
	refresh()


## Sélectionne la colonie `id`, y porte la caméra et ouvre son panneau.
func focus_settlement(id: String) -> void:
	if map.settlements_ctl != null:
		map.settlements_ctl.open_settlement(id, true)


## Centre la caméra sur la province `id` et ouvre son panneau (même chemin que le clic sur la
## carte : `map.picker.select_index`).
func focus_province(id: String) -> void:
	var index: int = map.map_data.index_of_id(id)
	if index <= 0:
		return
	var centroid: Vector2 = map.map_data.centroid_of_id(id)
	var world := Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y) + 6.0, centroid.y)
	map.camera_rig.look_at_point(world, minf(map.camera_rig.distance, FOCUS_DISTANCE))
	map.picker.select_index(index)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("map_toggle_holdings") and not event.is_echo():
		toggle()
		get_viewport().set_input_as_handled()


## Haut de la liste : sous la barre, sous la carte « que faire maintenant » (UX2) si affichée
## (même coin que « Mes unités »).
func _top() -> float:
	var top: float = (map.ui.get_node("TopBar") as Control).size.y + 8.0
	var hint: Control = map.next_hint.card if map.next_hint != null else null
	if hint != null and hint.visible:
		top = maxf(top, hint.position.y + hint.size.y + 8.0)
	return top


## Haut gauche, sous la barre ; la hauteur s'arrête au-dessus du journal.
func _layout() -> void:
	if not is_open():
		return
	var view := panel.get_viewport_rect().size
	var top := _top()
	_placed_top = top
	var bottom := view.y * 0.62
	var event_log: Control = map.ui.event_log
	if event_log != null and event_log.visible:
		bottom = minf(bottom, event_log.position.y - 8.0)
	var header_height := 96.0
	var wanted := _list.get_combined_minimum_size().y
	_scroll.custom_minimum_size = Vector2(PANEL_WIDTH - 20.0, clampf(wanted, 0.0, maxf(MIN_HEIGHT, bottom - top - header_height)))
	panel.position = Vector2(map.ui.HUD_MARGIN, top)
	panel.size = Vector2.ZERO
	panel.call_deferred("reset_size")


func _process(_delta: float) -> void:
	if is_open() and not is_equal_approx(_top(), _placed_top):
		_layout()
