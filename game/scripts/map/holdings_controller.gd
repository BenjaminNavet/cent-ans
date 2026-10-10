class_name HoldingsController
extends Node

## Liste « Colonies » (touche B) : accès rapide aux colonies du joueur façon Total
## War, groupées par province repliable. Rendu, UI et entrées seulement ; toutes les données
## viennent de `CampaignSim.get_holdings_overview(faction)` :
## `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 2.
## Un clic sur une colonie ouvre son panneau (`SettlementController.open_settlement`) ; un clic
## sur « ⌖ » d'une province centre la caméra et ouvre le panneau de province. On ne construit
## ni ne recrute depuis cette liste.

const PANEL_WIDTH := 440.0
const MIN_HEIGHT := 160.0
const FOCUS_DISTANCE := 220.0
## Filtres cumulables (« all » les efface), dans l'ordre d'affichage.
const FILTERS := ["all", "idle", "upgrade", "endangered"]
const SORTS := ["income", "name", "unrest"]
## Colonnes d'une colonie : clé de tri, titre, largeur fixe (0 = prend le reste).
const COLUMNS := [
	{"key": "name", "title": "Nom", "width": 0.0},
	{"key": "income", "title": "Cens", "width": 64.0},
	{"key": "build", "title": "Ouvrage", "width": 104.0},
	{"key": "garrison", "title": "Garde", "width": 48.0},
	{"key": "alerts", "title": "Alertes", "width": 64.0},
]
const SCROLLBAR_ROOM := 12.0
const ROW_HEIGHT := 26.0


## Sceau de cire d'alerte (rond, glyphe au centre) ; l'infobulle porte le sens.
class _Seal:
	extends Control
	var glyph := ""
	var wax: Color = HudStyle.WAX

	func _init() -> void:
		custom_minimum_size = Vector2(17, 17)
		mouse_filter = Control.MOUSE_FILTER_PASS

	func _draw() -> void:
		var center := size * 0.5
		HudStyle.draw_wax_seal(self, center, 7.5, wax, glyph.unicode_at(0) if not glyph.is_empty() else 7)
		var font := ThemeDB.fallback_font
		draw_string(font, Vector2(0, center.y + 4.0), glyph, HORIZONTAL_ALIGNMENT_CENTER, size.x, 10, HudStyle.PARCHMENT_LIGHT)


var map: Node = null  # CampaignMap
var panel: PanelContainer = null

var _list: VBoxContainer
var _scroll: ScrollContainer
var _treasury_label: Label
var _income_label: Label
var _filter_buttons: Dictionary = {}  # key → Button
var _sort_buttons: Dictionary = {}  # key → Button
var _filters: Dictionary = {}  # clé de filtre actif → true (cumulés : ET)
var _search := ""
var _sort := "income"
## Tri des colonies dans chaque province ("" = ordre de la simulation).
var _col_sort := ""
var _col_asc := true
var _search_edit: LineEdit
var _header_buttons: Dictionary = {}  # clé de colonne → Button
var _totals_label: Label
var _totals_box: HBoxContainer
var _totals: Dictionary = {}
var _order: Dictionary = {}  # province → ids des colonies affichées, dans l'ordre
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
	var title := HudStyle.label("Censier du royaume", HudStyle.FONT_TITLE)
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
		button.pressed.connect(func() -> void: toggle_filter(key))
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

	_search_edit = LineEdit.new()
	_search_edit.placeholder_text = "Chercher une colonie…"
	_search_edit.clear_button_enabled = true
	_search_edit.text_changed.connect(set_search)
	box.add_child(_search_edit)

	var column_head := HBoxContainer.new()
	column_head.name = "ColumnHead"
	column_head.add_theme_constant_override("separation", 0)
	box.add_child(column_head)
	_fill_head(column_head)

	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(_scroll)
	_list = VBoxContainer.new()
	_list.add_theme_constant_override("separation", 3)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_list)

	_totals_box = HBoxContainer.new()
	_totals_box.name = "Totals"
	_totals_box.add_theme_constant_override("separation", 0)
	box.add_child(_totals_box)

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
	var scroll_before := _scroll.scroll_vertical
	for child in _list.get_children():
		_list.remove_child(child)
		child.queue_free()
	_update_header()
	_visible_settlement_count = 0
	_visible_province_count = 0
	_order.clear()
	_totals = {"income": 0, "garrison": 0, "idle": 0}
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
		_sort_settlements(visible_settlements)
		_visible_province_count += 1
		_visible_settlement_count += visible_settlements.size()
		var ids: Array = []
		for settlement in visible_settlements:
			ids.append(str(settlement.get("id", "")))
			_totals["income"] += int(settlement.get("income", 0))
			_totals["garrison"] += int(settlement.get("garrison_strength", 0))
			if bool(settlement.get("idle", false)):
				_totals["idle"] += 1
		_order[str(province["province"])] = ids
		var has_danger := (settlements as Array).any(func(s) -> bool: return bool(s.get("endangered", false)))
		var expanded := is_filtering() or _province_expanded(str(province["province"]), has_danger)
		_add_province_row(province, expanded, visible_settlements)
	_update_totals()
	_restore_scroll.call_deferred(scroll_before)


func _restore_scroll(value: int) -> void:
	if _scroll != null:
		_scroll.scroll_vertical = value


func is_filtering() -> bool:
	return not _filters.is_empty() or not _search.strip_edges().is_empty()


func _matches_filter(settlement: Dictionary) -> bool:
	if _filters.has("idle") and not bool(settlement.get("idle", false)):
		return false
	if _filters.has("upgrade") and (settlement.get("options_available", []) as Array).is_empty():
		return false
	if _filters.has("endangered") and not bool(settlement.get("endangered", false)):
		return false
	var needle := _search.strip_edges().to_lower()
	if not needle.is_empty() and str(settlement.get("name", "")).to_lower().find(needle) < 0:
		return false
	return true


func _sort_key(settlement: Dictionary, column: String) -> Variant:
	match column:
		"name":
			return str(settlement.get("name", "")).to_lower()
		"income":
			return int(settlement.get("income", 0))
		"build":
			if settlement.has("construction"):
				return int((settlement["construction"] as Dictionary).get("turns_left", 0))
			return -1 if bool(settlement.get("idle", false)) else 9999
		"garrison":
			return int(settlement.get("garrison_strength", 0))
		"alerts":
			return alerts_of(settlement).size()
	return 0


func _sort_settlements(list: Array) -> void:
	if _col_sort.is_empty():
		return
	var column := _col_sort
	var ascending := _col_asc
	list.sort_custom(func(a, b) -> bool:
		var ka: Variant = _sort_key(a, column)
		var kb: Variant = _sort_key(b, column)
		if ka == kb:
			return str(a.get("name", "")) < str(b.get("name", ""))
		return (ka < kb) if ascending else (ka > kb))


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
		var active: bool = _filters.is_empty() if key == "all" else _filters.has(key)
		(_filter_buttons[key] as Button).set_pressed_no_signal(active)
	for key in _sort_buttons:
		(_sort_buttons[key] as Button).set_pressed_no_signal(key == _sort)
	for key in _header_buttons:
		var title := ""
		for column in COLUMNS:
			if column["key"] == key:
				title = str(column["title"]).to_upper()
		var arrow := ""
		if key == _col_sort:
			arrow = " ▲" if _col_asc else " ▼"
		(_header_buttons[key] as Button).text = title + arrow


## Rangée d'en-têtes de colonnes (petites capitales, clic = tri).
func _fill_head(head: HBoxContainer) -> void:
	for index in COLUMNS.size():
		var column: Dictionary = COLUMNS[index]
		if index > 0:
			head.add_child(_rule())
		var button := Button.new()
		button.flat = true
		button.focus_mode = Control.FOCUS_NONE
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT if index == 0 else HORIZONTAL_ALIGNMENT_CENTER
		button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL - 2)
		button.add_theme_color_override("font_color", HudStyle.INK_SOFT)
		button.add_theme_color_override("font_hover_color", HudStyle.INK)
		_size_cell(button, float(column["width"]))
		var key := str(column["key"])
		button.pressed.connect(func() -> void: sort_by_column(key))
		button.tooltip_text = "Trier par « %s » ; un second clic inverse le sens." % str(column["title"]).to_lower()
		head.add_child(button)
		_header_buttons[key] = button
	var room := Control.new()
	room.custom_minimum_size = Vector2(SCROLLBAR_ROOM, 0)
	head.add_child(room)


func _size_cell(cell: Control, width: float) -> void:
	if width <= 0.0:
		cell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		cell.custom_minimum_size.x = 80.0
	else:
		cell.custom_minimum_size.x = width
		cell.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN


func _rule() -> ColorRect:
	var rule := ColorRect.new()
	rule.color = Color(HudStyle.INK_SOFT, 0.35)
	rule.custom_minimum_size = Vector2(1, 0)
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rule


func _update_totals() -> void:
	for child in _totals_box.get_children():
		_totals_box.remove_child(child)
		child.queue_free()
	var texts := [
		"Total (%d)" % _visible_settlement_count,
		Money.amount(int(_totals["income"])),
		FrText.count(int(_totals["idle"]), "chantier") + " libre%s" % ("s" if int(_totals["idle"]) > 1 else ""),
		str(_totals["garrison"]),
		"",
	]
	for index in COLUMNS.size():
		if index > 0:
			_totals_box.add_child(_rule())
		var cell := HudStyle.label(str(texts[index]), HudStyle.FONT_SMALL, HudStyle.INK)
		cell.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT if index == 0 else HORIZONTAL_ALIGNMENT_CENTER
		cell.clip_text = true
		_size_cell(cell, float(COLUMNS[index]["width"]))
		_totals_box.add_child(cell)
	var room := Control.new()
	room.custom_minimum_size = Vector2(SCROLLBAR_ROOM, 0)
	_totals_box.add_child(room)


## Alertes d'une colonie : tableau de {id, glyph, wax, text}.
func alerts_of(settlement: Dictionary) -> Array:
	var alerts: Array = []
	if settlement.has("siege"):
		var turns := int((settlement["siege"] as Dictionary).get("turns_left", 0))
		alerts.append({"id": "siege", "glyph": "⚔", "wax": HudStyle.RUBRIC, "text": "Assiégée (%s)" % FrText.count(turns, "tour")})
	var reasons: Array = settlement.get("danger_reasons", [])
	if reasons.has("revolt_countdown") or reasons.has("unrest"):
		alerts.append({"id": "unrest", "glyph": "⚠", "wax": HudStyle.RUBRIC, "text": "Troubles : agitation ou révolte menaçante dans la province"})
	if bool(settlement.get("occupied", false)):
		alerts.append({"id": "occupied", "glyph": "✕", "wax": HudStyle.RUBRIC, "text": "Occupée par l'ennemi"})
	if bool(settlement.get("idle", false)):
		alerts.append({"id": "idle", "glyph": "⚒", "wax": HudStyle.WAX, "text": "Chantier libre : rien n'est en construction"})
	if bool(settlement.get("upgrade_available", false)):
		alerts.append({"id": "upgrade", "glyph": "▲", "wax": HudStyle.WAX_GREEN, "text": "Promotion possible"})
	# No "empty recruit queue" seal: it lit almost every settlement and drowned the real alerts.
	return alerts


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
	row.name = "Row_" + str(settlement.get("id", ""))
	row.flat = false
	row.focus_mode = Control.FOCUS_NONE
	row.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	row.add_theme_stylebox_override("normal", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.PARCHMENT_DARK))
	row.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD))
	row.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))

	var name_text := str(settlement.get("name", ""))
	if bool(settlement.get("is_city", false)):
		name_text += " (cité)"
	var build_text := ""
	if settlement.has("construction"):
		var construction: Dictionary = settlement["construction"]
		build_text = "%s · %dt" % [construction.get("name", ""), int(construction.get("turns_left", 0))]
	elif bool(settlement.get("idle", false)):
		build_text = "libre"
	else:
		build_text = "—"
	var garrison := int(settlement.get("garrison_strength", 0))

	var cells := HBoxContainer.new()
	cells.name = "Cells"
	cells.add_theme_constant_override("separation", 0)
	cells.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	cells.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(cells)
	var values := [name_text, Money.amount(int(settlement.get("income", 0))), build_text, str(garrison)]
	for index in COLUMNS.size():
		if index > 0:
			cells.add_child(_rule())
		var width := float(COLUMNS[index]["width"])
		if index < values.size():
			var cell := HudStyle.label(str(values[index]), HudStyle.FONT_SMALL, HudStyle.INK)
			cell.clip_text = true
			cell.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
			cell.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
			cell.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT if index == 0 else HORIZONTAL_ALIGNMENT_CENTER
			cell.mouse_filter = Control.MOUSE_FILTER_IGNORE
			_size_cell(cell, width)
			cells.add_child(cell)
		else:
			var seals := HBoxContainer.new()
			seals.name = "Alerts"
			seals.alignment = BoxContainer.ALIGNMENT_CENTER
			seals.add_theme_constant_override("separation", 2)
			seals.mouse_filter = Control.MOUSE_FILTER_IGNORE
			_size_cell(seals, width)
			for alert in alerts_of(settlement):
				var seal := _Seal.new()
				seal.glyph = str(alert["glyph"])
				seal.wax = alert["wax"]
				seal.name = "Seal_" + str(alert["id"])
				seal.tooltip_text = str(alert["text"])
				seals.add_child(seal)
			cells.add_child(seals)
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


## Filtre exclusif : "all" (efface) / "idle" / "upgrade" / "endangered".
func set_filter(key: String) -> void:
	if not FILTERS.has(key):
		return
	_filters.clear()
	if key != "all":
		_filters[key] = true
	refresh()


## Bascule un filtre ; les filtres actifs se cumulent (ET). « all » les efface.
func toggle_filter(key: String) -> void:
	if not FILTERS.has(key):
		return
	if key == "all":
		_filters.clear()
	elif _filters.has(key):
		_filters.erase(key)
	else:
		_filters[key] = true
	refresh()


func active_filters() -> Array:
	return _filters.keys()


## Recherche par nom de colonie (sans casse).
func set_search(text: String) -> void:
	_search = text
	if _search_edit != null and _search_edit.text != text:
		_search_edit.text = text
	refresh()


## Tri des colonies par colonne ; un second appel sur la même colonne inverse le sens.
func sort_by_column(key: String) -> void:
	if _col_sort == key:
		_col_asc = not _col_asc
	else:
		_col_sort = key
		_col_asc = key == "name"
	refresh()


func column_sort() -> String:
	return _col_sort + (" asc" if _col_asc else " desc")


## Ids des colonies affichées d'une province, dans l'ordre du tri.
func settlement_order(province_id: String) -> Array:
	return _order.get(province_id, [])


## Totaux des colonies affichées : income, garrison, idle.
func totals() -> Dictionary:
	return _totals


## Replie / déplie une province.
func set_expanded(province_id: String, expanded: bool) -> void:
	_expanded[province_id] = expanded
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
	var header_height := 210.0
	var wanted := _list.get_combined_minimum_size().y
	_scroll.custom_minimum_size = Vector2(PANEL_WIDTH - 20.0, clampf(wanted, 0.0, maxf(MIN_HEIGHT, bottom - top - header_height)))
	panel.position = Vector2(map.ui.HUD_MARGIN, top)
	panel.size = Vector2.ZERO
	panel.call_deferred("reset_size")


func _process(_delta: float) -> void:
	if is_open() and not is_equal_approx(_top(), _placed_top):
		_layout()
