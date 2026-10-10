class_name UnitRosterController
extends Node

## Liste « Mes unités » (touche U) : armées et agents du joueur sur la carte de campagne, à la
## manière de Total War. Rendu, UI et entrées seulement ; tout vient de `CampaignSim` :
## - armées (`get_army_ids` / `get_army`) : chef, effectif, lieu, ordre en cours et portée
##   restante (`movement_left` / `movement_max`, `movement_km` = km de plaine) ;
## - agents (`get_agents`, `get_agent_reachable`) : type, lieu, pas restants, colonies à portée.
## Chaque section se déroule ou se replie ; un clic sur une ligne sélectionne l'unité et y
## porte la caméra. Les unités sans mouvement restant sont estompées.

const PANEL_WIDTH := 340.0
const MIN_HEIGHT := 160.0
const FOCUS_DISTANCE := 220.0
const KIND_LETTERS := AgentController.KIND_LETTERS
## UX5-U : seuils d'alerte (affichage seulement ; l'état vient du cœur).
const LOW_SUPPLY := 40
const UNDERSTRENGTH_RATIO := 0.6
const SORTS := {"alert": "Alertes", "men": "Effectif", "move": "Mouvement", "place": "Lieu"}
const FILTERS := {"leaderless": "Sans chef", "siege": "En siège", "spent": "À bout"}

var map: Node = null  # CampaignMap
var panel: PanelContainer = null

var _list: VBoxContainer
var _scroll: ScrollContainer
var _expanded := {"armies": true, "agents": true}
var _rows: Dictionary = {}  # "army:<id>" / "agent:<id>" → Button
var _shown_selection := ""
var _placed_top := -1.0
var _sort_mode := "alert"
var _filters := {"leaderless": false, "siege": false, "spent": false}
var _bar: VBoxContainer
var _foot: Label
var _ordered_keys: Array = []  # lignes d'armée dans l'ordre affiché (Tab)


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "UnitRosterController"
	_build_panel()
	if map.movement_ctl != null:
		map.movement_ctl.army_moved.connect(func(_id: String, _report: Dictionary) -> void: refresh())
	get_viewport().size_changed.connect(_layout)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_army_ids")


# --- Panneau ------------------------------------------------------------------------------


func _build_panel() -> void:
	panel = PanelContainer.new()
	panel.name = "UnitRoster"
	panel.theme = load("res://scenes/ui/parchment_theme.tres")
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	panel.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	panel.add_child(box)
	var head := HBoxContainer.new()
	box.add_child(head)
	var title := HudStyle.label("Montre des hommes d’armes", HudStyle.FONT_TITLE)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	var close := Button.new()
	close.text = "✕"
	TooltipHost.attach_plain(close, "close_list_u")
	close.pressed.connect(func() -> void: panel.hide())
	head.add_child(close)
	_bar = VBoxContainer.new()
	_bar.name = "SortFilterBar"
	_bar.add_theme_constant_override("separation", 2)
	box.add_child(_bar)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(_scroll)
	_list = VBoxContainer.new()
	_list.add_theme_constant_override("separation", 3)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_list)
	_foot = HudStyle.label("", HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	_foot.name = "RealmFoot"
	_foot.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_foot)
	_build_bar()
	panel.hide()
	# Le conseiller (VO1) s'efface tant que la liste est ouverte : même coin de l'écran.
	panel.add_to_group(PanelStack.BLOCKING_GROUP)
	# Registre dans la zone `SIDE_PANEL` (ouvrir la province ou la chronique le ferme).
	UiZones.put(UiZones.Zone.SIDE_PANEL, panel)


func toggle() -> void:
	if panel.visible:
		panel.hide()
		return
	if map.holdings_ctl != null and map.holdings_ctl.is_open():  # Un seul panneau à la fois
		map.holdings_ctl.toggle()
	panel.show()
	refresh()


func is_open() -> bool:
	return panel != null and panel.visible


## Reconstruit la liste (ouverture, ordre, fin de tour, tri, filtre) en gardant le défilement.
## Sans effet si le panneau est fermé.
func refresh() -> void:
	if not is_open() or not available():
		return
	var scroll_before := _scroll.scroll_vertical
	for child in _list.get_children():
		_list.remove_child(child)
		child.queue_free()
	_rows.clear()
	_ordered_keys.clear()
	var all_armies := _army_entries()
	var armies := _sorted_filtered(all_armies)
	var agents := _agent_entries()
	var count_text := "%d/%d" % [armies.size(), all_armies.size()] if armies.size() != all_armies.size() else str(armies.size())
	_add_section("armies", "Armées", count_text)
	if _expanded["armies"]:
		for entry in armies:
			_add_army_row(entry)
			_ordered_keys.append(entry["key"])
		if armies.is_empty():
			_list.add_child(HudStyle.label("Aucune armée en campagne." if all_armies.is_empty() else "Aucun ost ne correspond aux filtres.", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	if map.agents_ctl != null and map.agents_ctl.available():
		_add_section("agents", "Agents", str(agents.size()))
		if _expanded["agents"]:
			for entry in agents:
				_add_row(entry)
			if agents.is_empty():
				_list.add_child(HudStyle.label("Aucun agent à votre service (G pour recruter).", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	_update_foot(all_armies)
	_shown_selection = ""
	_update_selection()
	_layout.call_deferred()  # les lignes libérées comptent encore dans la taille minimale
	_restore_scroll.call_deferred(scroll_before)


func _restore_scroll(value: int) -> void:
	if _scroll != null and is_open():
		_scroll.scroll_vertical = value


func _add_section(key: String, text: String, count: String) -> void:
	var header := Button.new()
	header.flat = true
	header.alignment = HORIZONTAL_ALIGNMENT_LEFT
	header.text = "%s %s (%s)" % ["▾" if _expanded[key] else "▸", text, count]
	TooltipHost.attach_plain(header, "roster_toggle", {"title": "Replier la liste" if _expanded[key] else "Dérouler la liste"})
	header.add_theme_font_size_override("font_size", HudStyle.FONT_BODY + 1)
	header.pressed.connect(func() -> void:
		_expanded[key] = not _expanded[key]
		refresh())
	_list.add_child(header)


## Ligne cliquable : titre, détail, jauge de portée et texte de portée.
func _add_row(entry: Dictionary) -> void:
	var row := Button.new()
	row.name = str(entry["key"]).replace(":", "_")
	row.toggle_mode = true
	row.focus_mode = Control.FOCUS_NONE
	row.add_theme_stylebox_override("normal", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.PARCHMENT_DARK))
	row.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD))
	row.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	row.add_theme_stylebox_override("hover_pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	row.tooltip_text = str(entry["tooltip"])
	row.custom_minimum_size = Vector2(PANEL_WIDTH - 24.0, 0)
	var margin := MarginContainer.new()
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 5)
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	row.add_child(margin)
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_theme_constant_override("separation", 1)
	margin.add_child(box)
	var faded := float(entry["ratio"]) <= 0.0
	var ink := HudStyle.INK_FADED if faded else HudStyle.INK
	var title := HudStyle.label(str(entry["title"]), HudStyle.FONT_BODY, ink)
	title.clip_text = true
	box.add_child(title)
	var detail := HudStyle.label(str(entry["detail"]), HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	detail.clip_text = true
	box.add_child(detail)
	var range_row := HBoxContainer.new()
	range_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	range_row.add_theme_constant_override("separation", 6)
	box.add_child(range_row)
	var gauge := ProgressBar.new()
	gauge.mouse_filter = Control.MOUSE_FILTER_IGNORE
	gauge.show_percentage = false
	gauge.max_value = 1.0
	gauge.step = 0.0
	gauge.value = float(entry["ratio"])
	gauge.custom_minimum_size = Vector2(90, 8)
	gauge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var fill := StyleBoxFlat.new()
	fill.bg_color = HudStyle.gauge_color(float(entry["ratio"]))
	gauge.add_theme_stylebox_override("fill", fill)
	var background := StyleBoxFlat.new()
	background.bg_color = HudStyle.PARCHMENT_DARK
	gauge.add_theme_stylebox_override("background", background)
	range_row.add_child(gauge)
	range_row.add_child(HudStyle.label(str(entry["range"]), HudStyle.FONT_SMALL, ink))
	row.custom_minimum_size.y = 62
	var key := str(entry["key"])
	row.pressed.connect(func() -> void: focus_entry(key))
	_list.add_child(row)
	_rows[key] = row


# --- Données ------------------------------------------------------------------------------


## Armées du joueur : `{key, id, title, detail, range, ratio, tooltip}`.
func _army_entries() -> Array:
	var result: Array = []
	for army_id in map.player_army_ids():
		var army: Dictionary = map.sim.call("get_army", army_id)
		if army.is_empty():
			continue
		result.append(army_entry(army_id, army))
	return result


func army_entry(army_id: String, army: Dictionary) -> Dictionary:
	var general := str(army.get("general_name", ""))
	var title := "⚔ Ost de %s" % general if general != "" else "⚔ Ost sans chef"
	var units: Array = army.get("units", [])
	var men := 0
	var max_men := 0
	var morale_sum := 0
	for unit in units:
		men += int(unit.get("strength", 0))
		max_men += int(unit.get("max_strength", unit.get("strength", 0)))
		morale_sum += int(unit.get("morale", 0))
	var detail := "%s · %s hommes · %s" % [FrText.count(units.size(), "unité"), Money.digits(men), _army_place(army)]
	var left := int(army.get("movement_left", army.get("movement_points", 0)))
	var allowance := maxi(1, int(army.get("movement_max", left)))
	var ratio := clampf(float(left) / float(allowance), 0.0, 1.0)
	var range_text: String
	if bool(army.get("embarked", false)):
		range_text = "En mer"
		ratio = 0.0
	elif left <= 0:
		range_text = "Plus de mouvement cette saison"
	elif army.has("movement_km"):
		range_text = "Portée ≈ %d km de plaine (%d %%)" % [roundi(float(army["movement_km"])), roundi(ratio * 100.0)]
	else:
		range_text = "Portée %d / %d" % [left, allowance]
	var order := _army_order(army)
	if order != "":
		detail += " — " + order
	# UX5-U : champs de la ligne à deux niveaux, des alertes et du tri.
	var stance := str(army.get("stance", "normal"))
	var embarked := bool(army.get("embarked", false))
	var supply := int(army.get("supply", 100))
	var leaderless := general == "" and str(army.get("general", "")) == ""
	var state := "stationed"
	if embarked:
		state = "embarked"
	elif stance == "siege":
		state = "siege"
	elif stance == "entrenched" and str(army.get("settlement", "")) == "":
		state = "camp"  # retranché en rase campagne : exposé
	elif stance == "entrenched":
		state = "entrenched"
	elif order == "en marche" or order.begins_with("en marche"):
		state = "march"
	var alerts: Array[String] = []
	if leaderless:
		alerts.append("leaderless")
	if supply < LOW_SUPPLY:
		alerts.append("supply")
	if max_men > 0 and float(men) < UNDERSTRENGTH_RATIO * float(max_men):
		alerts.append("strength")
	if left <= 0 and not embarked:
		alerts.append("spent")
	var morale := roundi(float(morale_sum) / float(units.size())) if not units.is_empty() else 0
	return {"key": "army:" + army_id, "id": army_id, "title": title, "detail": detail,
		"range": range_text, "ratio": ratio,
		"general_id": str(army.get("general", "")), "faction": str(army.get("faction", "")),
		"men": men, "max_men": max_men, "morale": morale, "supply": supply, "state": state, "alerts": alerts,
		"leaderless": leaderless, "left": left, "place": _army_place(army), "order": order,
		"tooltip": "%s\n%s\n%s\nClic : sélectionner et centrer la carte." % [title, detail, range_text]}


func _army_place(army: Dictionary) -> String:
	var settlement := str(army.get("settlement", army.get("location", "")))
	if settlement != "" and map.settlements_ctl != null:
		var name_text: String = map.settlements_ctl.settlement_name(settlement)
		if name_text != "" and name_text != settlement:
			return name_text
	var province := str(army.get("location_province", ""))
	return "en campagne, %s" % map.province_name_of(province) if province != "" else "en campagne"


func _army_order(army: Dictionary) -> String:
	if str(army.get("stance", "")) == "siege":
		return "siège"
	var planned: Variant = army.get("planned_path", [])
	if (planned is Array or planned is PackedVector2Array) and planned.size() > 0:
		return "en marche"
	var path: Array = army.get("path_provinces", army.get("path", []))
	if not path.is_empty():
		return "en marche vers %s" % map.province_name_of(str(path[-1]))
	return ""


## Agents du joueur : même forme que `army_entry`.
func _agent_entries() -> Array:
	var result: Array = []
	if map.agents_ctl == null or not map.agents_ctl.available():
		return result
	for agent in map.sim.call("get_agents"):
		if str(agent.get("faction", "")) != map.player_faction:
			continue
		result.append(agent_entry(agent, map.sim.call("get_agent_reachable", str(agent.get("id", "")))))
	return result


func agent_entry(agent: Dictionary, reachable: Dictionary) -> Dictionary:
	var agent_id := str(agent.get("id", ""))
	var kind := str(agent.get("kind", ""))
	var title := "%s %s — %s" % [KIND_LETTERS.get(kind, "✦"), agent.get("name", ""), agent.get("kind_name", "")]
	var level := int(agent.get("level", 1))
	var detail := "%s · niveau %d" % [agent.get("location_name", ""), level]
	if bool(agent.get("acted", false)):
		detail += " — a agi cette saison"
	var left := int(agent.get("movement_points", 0))
	var allowance := maxi(1, int(agent.get("max_movement_points", left)))
	var ratio := clampf(float(left) / float(allowance), 0.0, 1.0)
	var targets := 0
	for id in reachable:
		if str(id) != str(agent.get("location", "")):
			targets += 1
	var range_text := "Plus de mouvement cette saison" if left <= 0 \
		else "Portée : %s à portée" % FrText.count(targets, "colonie")
	return {"key": "agent:" + agent_id, "id": agent_id, "title": title, "detail": detail,
		"range": range_text, "ratio": ratio,
		"tooltip": "%s\n%s\n%s\nClic : sélectionner et centrer la carte." % [title, detail, range_text]}


# --- Tri, filtres, pied (UX5-U) ------------------------------------------------------------


func _build_bar() -> void:
	var sort_row := HBoxContainer.new()
	sort_row.name = "SortRow"
	sort_row.add_theme_constant_override("separation", 3)
	sort_row.add_child(HudStyle.label("Trier", HudStyle.FONT_SMALL, HudStyle.INK_SOFT))
	var group := ButtonGroup.new()
	for mode in SORTS:
		var button := _bar_button("Sort_" + mode, SORTS[mode])
		button.button_group = group
		button.set_pressed_no_signal(mode == _sort_mode)
		button.pressed.connect(func() -> void:
			_sort_mode = mode
			refresh())
		sort_row.add_child(button)
	_bar.add_child(sort_row)
	var filter_row := HBoxContainer.new()
	filter_row.name = "FilterRow"
	filter_row.add_theme_constant_override("separation", 3)
	filter_row.add_child(HudStyle.label("Montrer", HudStyle.FONT_SMALL, HudStyle.INK_SOFT))
	for key in FILTERS:
		var button := _bar_button("Filter_" + key, FILTERS[key])
		button.toggled.connect(func(on: bool) -> void:
			_filters[key] = on
			refresh())
		filter_row.add_child(button)
	_bar.add_child(filter_row)


func _bar_button(button_name: String, text: String) -> Button:
	var button := Button.new()
	button.name = button_name
	button.text = text
	button.toggle_mode = true
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	button.add_theme_stylebox_override("normal", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.PARCHMENT_DARK))
	button.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD))
	button.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	button.add_theme_stylebox_override("hover_pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	return button


func set_sort(mode: String) -> void:
	if not SORTS.has(mode):
		return
	_sort_mode = mode
	var button := _bar.find_child("Sort_" + mode, true, false) as Button
	if button != null:
		button.set_pressed_no_signal(true)
	refresh()


func set_filter(key: String, on: bool) -> void:
	if not _filters.has(key):
		return
	_filters[key] = on
	var button := _bar.find_child("Filter_" + key, true, false) as Button
	if button != null:
		button.set_pressed_no_signal(on)
	refresh()


## Entrées d'ost filtrées (filtres cumulables : toutes les conditions cochées) puis triées.
func _sorted_filtered(entries: Array) -> Array:
	var result: Array = []
	for entry in entries:
		if _filters["leaderless"] and not bool(entry.get("leaderless", false)):
			continue
		if _filters["siege"] and str(entry.get("state", "")) != "siege":
			continue
		if _filters["spent"] and not (entry.get("alerts", []) as Array).has("spent"):
			continue
		result.append(entry)
	var mode := _sort_mode
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		match mode:
			"men":
				if int(a["men"]) != int(b["men"]):
					return int(a["men"]) > int(b["men"])
			"move":
				if float(a["ratio"]) != float(b["ratio"]):
					return float(a["ratio"]) > float(b["ratio"])
			"place":
				if str(a["place"]) != str(b["place"]):
					return str(a["place"]).naturalnocasecmp_to(str(b["place"])) < 0
			_:
				var alerts_a := (a["alerts"] as Array).size()
				var alerts_b := (b["alerts"] as Array).size()
				if alerts_a != alerts_b:
					return alerts_a > alerts_b
		return str(a["key"]) < str(b["key"]))
	return result


func _update_foot(all_armies: Array) -> void:
	var men := 0
	for entry in all_armies:
		men += int(entry.get("men", 0))
	var upkeep := 0
	if map.sim.has_method("get_faction_economy"):
		upkeep = int(map.sim.call("get_faction_economy", map.player_faction).get("army_upkeep", 0))
	_foot.text = "Montre du royaume : %s hommes · %s · entretien %s par saison" % [
		Money.digits(men), FrText.count(all_armies.size(), "ost"), Money.amount(upkeep)]


## Tab / Maj+Tab : ost suivant (`step` = 1) ou précédent (-1) dans l'ordre affiché ; sélection et
## caméra comme un clic. Retourne faux si la liste est fermée ou vide (le raccourci reprend alors son rôle).
func cycle(step: int) -> bool:
	if not is_open() or _ordered_keys.is_empty():
		return false
	var target := CampaignHotkeys.next_in_cycle(_ordered_keys, _current_selection(), step)
	if target == "":
		return false
	focus_entry(target)
	return true


## Ligne d'ost à deux niveaux (+ lieu en petit) : mini-sceau, titre, état, alertes ; effectif,
## moral, vivres, jauge de mouvement.
func _add_army_row(entry: Dictionary) -> void:
	var row := _make_row_button(entry)
	var margin := _row_margin(row)
	var hbox := HBoxContainer.new()
	hbox.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hbox.add_theme_constant_override("separation", 6)
	margin.add_child(hbox)
	var seal := SealChip.new()
	seal.setup(str(entry["general_id"]), str(entry["faction"]), bool(entry["leaderless"]))
	hbox.add_child(seal)
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override("separation", 1)
	hbox.add_child(box)
	var faded := float(entry["ratio"]) <= 0.0
	var ink := HudStyle.INK_FADED if faded else HudStyle.INK
	var line1 := HBoxContainer.new()
	line1.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line1.add_theme_constant_override("separation", 3)
	box.add_child(line1)
	var title := HudStyle.label(str(entry["title"]).trim_prefix("⚔ "), HudStyle.FONT_BODY, ink)
	title.clip_text = true
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line1.add_child(title)
	var state := str(entry["state"])
	var state_chip := _chip(_state_glyph(state), _state_text(state, entry), HudStyle.INK_SOFT)
	state_chip.name = "State_" + state
	line1.add_child(state_chip)
	for alert in entry["alerts"]:
		var chip := _chip(_alert_glyph(alert), _alert_text(alert, entry), HudStyle.RUBRIC, true)
		chip.name = "Alert_" + str(alert)
		line1.add_child(chip)
	var line2 := HBoxContainer.new()
	line2.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line2.add_theme_constant_override("separation", 6)
	box.add_child(line2)
	var counts := "%s/%s" % [Money.digits(int(entry["men"])), Money.digits(int(entry["max_men"]))]
	var stats := HudStyle.label("%s · moral %d · vivres %d %%" % [counts, int(entry["morale"]), int(entry["supply"])], HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	stats.clip_text = true
	stats.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line2.add_child(stats)
	line2.add_child(_gauge(float(entry["ratio"]), 52.0))
	var place := HudStyle.label(str(entry["place"]), HudStyle.FONT_SMALL - 1, HudStyle.INK_FADED)
	place.clip_text = true
	box.add_child(place)
	row.custom_minimum_size.y = 66
	_register_row(row, entry)


func _make_row_button(entry: Dictionary) -> Button:
	var row := Button.new()
	row.name = str(entry["key"]).replace(":", "_")
	row.toggle_mode = true
	row.focus_mode = Control.FOCUS_NONE
	row.add_theme_stylebox_override("normal", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.PARCHMENT_DARK))
	row.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD))
	row.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	row.add_theme_stylebox_override("hover_pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.WAX, 2))
	row.tooltip_text = str(entry["tooltip"])
	row.custom_minimum_size = Vector2(PANEL_WIDTH - 24.0, 0)
	return row


func _row_margin(row: Button) -> MarginContainer:
	var margin := MarginContainer.new()
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 5)
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	row.add_child(margin)
	return margin


func _register_row(row: Button, entry: Dictionary) -> void:
	var key := str(entry["key"])
	row.pressed.connect(func() -> void: focus_entry(key))
	_list.add_child(row)
	_rows[key] = row


func _gauge(ratio: float, width: float) -> ProgressBar:
	var gauge := ProgressBar.new()
	gauge.mouse_filter = Control.MOUSE_FILTER_IGNORE
	gauge.show_percentage = false
	gauge.max_value = 1.0
	gauge.step = 0.0
	gauge.value = ratio
	gauge.custom_minimum_size = Vector2(width, 8)
	gauge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var fill := StyleBoxFlat.new()
	fill.bg_color = HudStyle.gauge_color(ratio)
	gauge.add_theme_stylebox_override("fill", fill)
	var background := StyleBoxFlat.new()
	background.bg_color = HudStyle.PARCHMENT_DARK
	gauge.add_theme_stylebox_override("background", background)
	return gauge


## Pastille à infobulle : pictogramme (`GeneralSeal.GlyphIcon`) ou, sans pictogramme, une lettre.
func _chip(glyph: String, tip: String, color: Color, boxed: bool = false) -> Control:
	var holder := PanelContainer.new()
	holder.mouse_filter = Control.MOUSE_FILTER_PASS  # l'infobulle s'affiche, le clic passe à la ligne
	holder.tooltip_text = tip
	var style := HudStyle.card_box(HudStyle.PARCHMENT_LIGHT if boxed else Color(0, 0, 0, 0), color if boxed else Color(0, 0, 0, 0))
	style.set_content_margin_all(2)
	holder.add_theme_stylebox_override("panel", style)
	if glyph.length() == 1 and not glyph.is_valid_identifier():
		var letter := HudStyle.label(glyph, HudStyle.FONT_SMALL, color)
		letter.mouse_filter = Control.MOUSE_FILTER_IGNORE
		holder.add_child(letter)
	else:
		var icon := GeneralSeal.GlyphIcon.new()
		icon.glyph = glyph
		icon.color = color
		icon.custom_minimum_size = Vector2(16, 16)
		holder.add_child(icon)
	return holder


static func _state_glyph(state: String) -> String:
	match state:
		"siege":
			return "stance_siege"
		"camp", "entrenched":
			return "stance_entrenched"
		"march":
			return "movement"
		"embarked":
			return "≈"
	return "infantry"


static func _state_text(state: String, entry: Dictionary) -> String:
	match state:
		"siege":
			return "Au siège"
		"camp":
			return "Campement retranché, exposé en rase campagne"
		"entrenched":
			return "Retranché"
		"march":
			return str(entry.get("order", "En marche")).capitalize() if str(entry.get("order", "")) != "" else "En marche"
		"embarked":
			return "Embarqué, en mer"
	return "Stationné"


static func _alert_glyph(alert: String) -> String:
	match alert:
		"leaderless":
			return "!"
		"supply":
			return "supply"
		"strength":
			return "infantry"
		"spent":
			return "movement"
	return "!"


static func _alert_text(alert: String, entry: Dictionary) -> String:
	match alert:
		"leaderless":
			return "Sans chef : nommez un capitaine depuis la cour."
		"supply":
			return "Vivres bas : %d %% (seuil %d %%)." % [int(entry["supply"]), LOW_SUPPLY]
		"strength":
			return "Sous-effectif : %s/%s hommes (moins de %d %% du complet)." % [
				Money.digits(int(entry["men"])), Money.digits(int(entry["max_men"])), roundi(UNDERSTRENGTH_RATIO * 100.0)]
		"spent":
			return "Plus de mouvement cette saison."
	return alert


## Mini-sceau du chef : médaillon de cire, portrait ou armes ; sceau vide hachuré sans chef.
class SealChip:
	extends Control

	const RADIUS := 17.0
	var empty := false
	var _portrait: Texture2D
	var _heraldry: Texture2D
	var _faction := ""

	func setup(character_id: String, faction_id: String, leaderless: bool) -> void:
		empty = leaderless
		_faction = faction_id
		custom_minimum_size = Vector2(RADIUS * 2.0 + 4.0, RADIUS * 2.0 + 4.0)
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		size_flags_vertical = Control.SIZE_SHRINK_CENTER
		if not leaderless and character_id != "":
			_portrait = PortraitLoader.portrait_texture(character_id)
		if not leaderless and _portrait == null:
			_heraldry = PortraitLoader.heraldry_texture(faction_id)

	func _draw() -> void:
		var center := Vector2(RADIUS + 2.0, RADIUS + 2.0)
		if empty:
			draw_circle(center, RADIUS, HudStyle.PARCHMENT_DARK)
			var step := 6.0
			var offset := -RADIUS * 2.0
			while offset < RADIUS * 2.0:
				# hachures diagonales limitées au disque
				var a := center + Vector2(offset, -RADIUS)
				var b := center + Vector2(offset + RADIUS * 2.0, RADIUS)
				var d := (b - a).normalized()
				var to_center := center - a
				var t := to_center.dot(d)
				var closest := a + d * t
				var dist := closest.distance_to(center)
				if dist < RADIUS - 1.0:
					var half := sqrt(RADIUS * RADIUS - dist * dist)
					draw_line(closest - d * half, closest + d * half, HudStyle.INK_FADED, 1.0)
				offset += step
			draw_arc(center, RADIUS, 0.0, TAU, 32, HudStyle.RUBRIC, 1.5, true)
			return
		var inner := HudStyle.draw_wax_seal(self, center, RADIUS, HudStyle.WAX, hash(_faction) % 97)
		if _portrait != null:
			HudStyle.draw_texture_disc(self, _portrait, center, inner - 1.0)
		else:
			draw_circle(center, inner - 1.0, HudStyle.PARCHMENT_LIGHT)
			if _heraldry != null:
				HudStyle.draw_texture_fit(self, _heraldry, center, inner * 1.3)
			else:
				HudStyle.draw_glyph(self, "infantry", center, inner, HudStyle.INK_SOFT)
		draw_arc(center, inner - 1.0, 0.0, TAU, 32, HudStyle.GOLD, 1.5, true)


# --- Sélection ----------------------------------------------------------------------------


## Sélectionne l'unité `key` ("army:<id>" / "agent:<id>") et porte la caméra sur elle.
func focus_entry(key: String) -> void:
	var parts := key.split(":", true, 1)
	if parts.size() != 2:
		return
	var id := parts[1]
	var world := Vector3.ZERO
	if parts[0] == "army":
		map.select_army(id)
		world = map.armies.world_position_of(id)
	elif parts[0] == "agent" and map.agents_ctl != null:
		map.agents_ctl.select_agent(id)
		var agent: Dictionary = map.sim.call("get_agent", id)
		world = map.settlement_layer.world_position_of(str(agent.get("location", "")))
	if world != Vector3.ZERO:
		map.camera_rig.look_at_point(world, minf(map.camera_rig.distance, FOCUS_DISTANCE))
	_shown_selection = ""
	_update_selection()


func _current_selection() -> String:
	if map.selected_army != "":
		return "army:" + map.selected_army
	if map.agents_ctl != null and map.agents_ctl.selected_agent != "":
		return "agent:" + map.agents_ctl.selected_agent
	return ""


func _update_selection() -> void:
	var current := _current_selection()
	if current == _shown_selection:
		return
	_shown_selection = current
	for key in _rows:
		(_rows[key] as Button).set_pressed_no_signal(key == current)


func row_count() -> int:
	return _rows.size()


func _process(_delta: float) -> void:
	if is_open():
		_update_selection()
		if not is_equal_approx(_top(), _placed_top):
			_layout()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("map_toggle_units") and not event.is_echo():
		toggle()
		get_viewport().set_input_as_handled()


## Hauteur de la zone `SIDE_PANEL` (repère de mise à jour de `_layout`).
func _top() -> float:
	return UiZones.rect(UiZones.Zone.SIDE_PANEL).size.y


## La liste remplit la zone `SIDE_PANEL` (placée par `UiLayout`) ; elle défile au-delà.
func _layout() -> void:
	if not is_open():
		return
	var room := _top()
	_placed_top = room
	var header_height := 48.0
	var wanted := _list.get_combined_minimum_size().y
	_scroll.custom_minimum_size = Vector2(PANEL_WIDTH - 20.0, clampf(wanted, 0.0, maxf(MIN_HEIGHT, room - header_height)))

