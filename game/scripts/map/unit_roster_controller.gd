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

var map: Node = null  # CampaignMap
var panel: PanelContainer = null

var _list: VBoxContainer
var _scroll: ScrollContainer
var _expanded := {"armies": true, "agents": true}
var _rows: Dictionary = {}  # "army:<id>" / "agent:<id>" → Button
var _shown_selection := ""
var _placed_top := -1.0


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
	var title := HudStyle.label("Mes unités (U)", HudStyle.FONT_TITLE)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	var close := Button.new()
	close.text = "✕"
	TooltipHost.attach_plain(close, "close_list_u")
	close.pressed.connect(func() -> void: panel.hide())
	head.add_child(close)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(_scroll)
	_list = VBoxContainer.new()
	_list.add_theme_constant_override("separation", 3)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_list)
	panel.hide()
	# Le conseiller (VO1) s'efface tant que la liste est ouverte : même coin de l'écran.
	panel.add_to_group(PanelStack.BLOCKING_GROUP)
	# PO1 : registre dans la zone `SIDE_PANEL` (ouvrir la province ou la chronique le ferme).
	UiZones.put(UiZones.Zone.SIDE_PANEL, panel)


func toggle() -> void:
	if panel.visible:
		panel.hide()
		return
	if map.holdings_ctl != null and map.holdings_ctl.is_open():  # HL2 : un seul panneau à la fois
		map.holdings_ctl.toggle()
	panel.show()
	refresh()


func is_open() -> bool:
	return panel != null and panel.visible


## Reconstruit la liste (ouverture, ordre, fin de tour). Sans effet si le panneau est fermé.
func refresh() -> void:
	if not is_open() or not available():
		return
	for child in _list.get_children():
		child.queue_free()
	_rows.clear()
	var armies := _army_entries()
	var agents := _agent_entries()
	_add_section("armies", "Armées", armies.size())
	if _expanded["armies"]:
		for entry in armies:
			_add_row(entry)
		if armies.is_empty():
			_list.add_child(HudStyle.label("Aucune armée en campagne.", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	if map.agents_ctl != null and map.agents_ctl.available():
		_add_section("agents", "Agents", agents.size())
		if _expanded["agents"]:
			for entry in agents:
				_add_row(entry)
			if agents.is_empty():
				_list.add_child(HudStyle.label("Aucun agent à votre service (G pour recruter).", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	_shown_selection = ""
	_update_selection()
	_layout.call_deferred()  # les lignes libérées comptent encore dans la taille minimale


func _add_section(key: String, text: String, count: int) -> void:
	var header := Button.new()
	header.flat = true
	header.alignment = HORIZONTAL_ALIGNMENT_LEFT
	header.text = "%s %s (%d)" % ["▾" if _expanded[key] else "▸", text, count]
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
	var title := "⚔ Ost de %s" % (general if general != "" else "l'ost sans chef")
	var units: Array = army.get("units", [])
	var men := 0
	for unit in units:
		men += int(unit.get("strength", 0))
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
	return {"key": "army:" + army_id, "id": army_id, "title": title, "detail": detail,
		"range": range_text, "ratio": ratio,
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


## PO1 : hauteur de la zone `SIDE_PANEL` (repère de mise à jour de `_layout`).
func _top() -> float:
	return UiZones.rect(UiZones.Zone.SIDE_PANEL).size.y


## PO1 : la liste remplit la zone `SIDE_PANEL` (placée par `UiLayout`) ; elle défile au-delà.
func _layout() -> void:
	if not is_open():
		return
	var room := _top()
	_placed_top = room
	var header_height := 48.0
	var wanted := _list.get_combined_minimum_size().y
	_scroll.custom_minimum_size = Vector2(PANEL_WIDTH - 20.0, clampf(wanted, 0.0, maxf(MIN_HEIGHT, room - header_height)))

